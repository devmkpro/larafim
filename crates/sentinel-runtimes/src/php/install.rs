//! Instalação, atualização e remoção de versões do PHP.

use sentinel_core::t;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sentinel_core::Paths;
use serde::Serialize;

use super::catalog::CatalogEntry;
use super::installed::{InstalledPhp, list_installed};
use super::ini;
use crate::download::download_verified;
use crate::error::{Error, Result};
use crate::extras;

/// Etapas mostradas na interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Stage {
    Preparing,
    Downloading { received: u64, total: Option<u64> },
    Extracting,
    Configuring,
    Verifying,
}

/// Instala uma versão do catálogo. Se já houver outra build do mesmo minor
/// (atualização de patch), o `php.ini` dela é levado para a nova; a antiga
/// fica guardada para poder voltar.
pub async fn install(
    client: &reqwest::Client,
    paths: &Paths,
    entry: &CatalogEntry,
    cancel: &AtomicBool,
    on_stage: &(dyn Fn(Stage) + Send + Sync),
) -> Result<InstalledPhp> {
    on_stage(Stage::Preparing);
    let cacert = extras::ensure_cacert(client, paths).await?;

    let zip_path = paths.downloads_dir().join(&entry.file);
    if !zip_path.is_file() {
        download_verified(client, &entry.url, &entry.sha256, &zip_path, cancel, &|received, total| {
            on_stage(Stage::Downloading { received, total })
        })
        .await?;
    }

    on_stage(Stage::Extracting);
    let php_dir = paths.php_dir();
    let final_dir = php_dir.join(&entry.version);
    let tmp_dir = php_dir.join(format!(".tmp-{}", entry.version));
    {
        let (zip_path, tmp_dir) = (zip_path.clone(), tmp_dir.clone());
        tokio::task::spawn_blocking(move || extract(&zip_path, &tmp_dir))
            .await
            .map_err(|e| Error::Zip(e.to_string()))??;
    }
    if !tmp_dir.join("php.exe").is_file() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(Error::Zip(t!("o pacote não contém php.exe")));
    }

    on_stage(Stage::Configuring);
    let previous = list_installed(&php_dir)
        .into_iter()
        .find(|p| p.minor == entry.minor && p.version != entry.version);
    let ini_text = match previous.as_ref().filter(|p| p.ini().is_file()) {
        Some(prev) => ini::migrate(&ini::read(&prev.ini())?, &prev.dir, &final_dir),
        None => {
            let dev = ini::read(&tmp_dir.join("php.ini-development"))?;
            ini::generate(&dev, &final_dir, &tmp_dir, &cacert)
        }
    };
    ini::write(&tmp_dir.join("php.ini"), &ini_text)?;

    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir).map_err(|e| Error::io(&final_dir, e))?;
    }
    std::fs::rename(&tmp_dir, &final_dir).map_err(|e| Error::io(&final_dir, e))?;

    on_stage(Stage::Verifying);
    let installed = InstalledPhp { version: entry.version.clone(), minor: entry.minor.clone(), dir: final_dir };
    if let Err(e) = verify(&installed).await {
        // Instalação que não roda não fica para trás; o zip continua em
        // cache para tentar de novo sem baixar outra vez.
        let _ = std::fs::remove_dir_all(&installed.dir);
        return Err(e);
    }

    let _ = std::fs::remove_file(&zip_path);
    prune_old(&php_dir, &entry.minor, &entry.version);

    // O Composer é um bônus: se falhar, o PHP continua instalado.
    if let Err(e) = extras::ensure_composer(client, paths).await {
        tracing::warn!("Composer não foi baixado agora: {e}");
    }

    tracing::info!(version = entry.version, "PHP instalado");
    Ok(installed)
}

/// Remove todas as builds de um minor (ex.: todas as 8.3.x).
pub fn remove_minor(paths: &Paths, minor: &str) -> Result<usize> {
    let php_dir = paths.php_dir();
    let mut removed = 0;
    for p in list_installed(&php_dir).into_iter().filter(|p| p.minor == minor) {
        // Segurança: só apaga pastas dentro de runtimes\php.
        if p.dir.parent() != Some(php_dir.as_path()) {
            continue;
        }
        std::fs::remove_dir_all(&p.dir).map_err(|e| Error::io(&p.dir, e))?;
        removed += 1;
    }
    tracing::info!(minor, removed, "PHP removido");
    Ok(removed)
}

fn extract(zip_path: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| Error::io(dest, e))?;
    }
    let file = std::fs::File::open(zip_path).map_err(|e| Error::io(zip_path, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| Error::Zip(e.to_string()))?;
    // `extract` recusa caminhos que saem da pasta de destino.
    archive.extract(dest).map_err(|e| {
        let _ = std::fs::remove_dir_all(dest);
        Error::Zip(e.to_string())
    })
}

/// Mantém só a build atual e a anterior de cada minor (para poder voltar).
fn prune_old(php_dir: &Path, minor: &str, current: &str) {
    let same_minor: Vec<_> = list_installed(php_dir).into_iter().filter(|p| p.minor == minor).collect();
    for old in same_minor.iter().filter(|p| p.version != current).skip(1) {
        tracing::info!(version = old.version, "removendo build antiga");
        let _ = std::fs::remove_dir_all(&old.dir);
    }
}

/// Roda `php -v` e confere a versão. Detecta falta do Visual C++.
pub async fn verify(php: &InstalledPhp) -> Result<String> {
    let exe = php.php_exe();
    let out = hidden_command(&exe)
        .arg("-c")
        .arg(php.ini())
        .arg("-v")
        .output()
        .await
        .map_err(|e| Error::io(&exe, e))?;

    if !out.status.success() {
        // 0xC0000135 (DLL não encontrada) e 0xC000007B: falta o runtime do Visual C++.
        let code = out.status.code().unwrap_or_default() as u32;
        if code == 0xC000_0135 || code == 0xC000_007B {
            return Err(Error::MissingVcRedist);
        }
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(Error::Verify(t!("código {a}: {b}", a = format!("{:#x}", code), b = stderr.trim())));
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let first = stdout.lines().next().unwrap_or_default().to_string();
    if !first.starts_with(&format!("PHP {} ", php.version)) {
        return Err(Error::Verify(first));
    }
    Ok(first)
}

/// Processo sem abrir janela de console (o app é de interface gráfica).
fn hidden_command(exe: &PathBuf) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(exe);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.kill_on_drop(true);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(path: &Path, files: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, content) in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extract_and_prune() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("php.zip");
        make_zip(&zip_path, &[("php.exe", "x"), ("ext/php_curl.dll", "x"), ("php.ini-development", "[PHP]")]);
        let dest = tmp.path().join("out");
        extract(&zip_path, &dest).unwrap();
        assert!(dest.join("php.exe").is_file());
        assert!(dest.join("ext").join("php_curl.dll").is_file());

        let php_dir = tmp.path().join("php");
        for v in ["8.4.26", "8.4.27", "8.4.28", "8.3.37"] {
            std::fs::create_dir_all(php_dir.join(v)).unwrap();
            std::fs::write(php_dir.join(v).join("php.exe"), "").unwrap();
        }
        prune_old(&php_dir, "8.4", "8.4.28");
        let left: Vec<_> = list_installed(&php_dir).into_iter().map(|p| p.version).collect();
        assert_eq!(left, ["8.4.28", "8.4.27", "8.3.37"]);
    }

    #[test]
    fn remove_minor_only_touches_that_minor() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::at(tmp.path());
        for v in ["8.4.27", "8.4.28", "8.3.37"] {
            let d = paths.php_dir().join(v);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("php.exe"), "").unwrap();
        }
        assert_eq!(remove_minor(&paths, "8.4").unwrap(), 2);
        let left: Vec<_> = list_installed(&paths.php_dir()).into_iter().map(|p| p.version).collect();
        assert_eq!(left, ["8.3.37"]);
    }

    /// Instalação real (baixa ~30 MB). Rode com `cargo test -- --ignored real_install`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn real_install() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::at(tmp.path());
        paths.ensure().unwrap();
        let client = crate::http_client().unwrap();
        let catalog = super::super::fetch_catalog(&client).await.unwrap();
        let entry = catalog.iter().find(|e| e.minor == "8.4").unwrap();
        let never = AtomicBool::new(false);
        let php = install(&client, &paths, entry, &never, &|_| {}).await.unwrap();
        let line = verify(&php).await.unwrap();
        assert!(line.starts_with("PHP 8.4."));
        let ini = std::fs::read_to_string(php.ini()).unwrap();
        let exts = ini::list_extensions(&ini, &php.dir.join("ext"));
        assert!(exts.iter().any(|e| e.name == "curl" && e.enabled));
    }
}
