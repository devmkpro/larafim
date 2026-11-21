//! Ferramentas da fase 7: Mailpit (caixa de e-mails local) e Xdebug.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sentinel_core::Paths;
use serde::Serialize;

use crate::download::{download_verified, fetch_text};
use crate::error::{Error, Result};
use crate::php::ini;
use crate::php::installed::InstalledPhp;

// ---------- Mailpit ----------

const MAILPIT_API: &str = "https://api.github.com/repos/axllent/mailpit/releases/latest";
const MAILPIT_ASSET: &str = "mailpit-windows-amd64.zip";

pub fn mailpit_dir(paths: &Paths) -> PathBuf {
    paths.runtimes_dir().join("mailpit")
}

pub fn mailpit_exe(paths: &Paths) -> PathBuf {
    mailpit_dir(paths).join("mailpit.exe")
}

pub fn mailpit_installed(paths: &Paths) -> bool {
    mailpit_exe(paths).is_file()
}

/// (versão, url, sha256) da última versão, pela API do GitHub.
pub fn parse_mailpit_release(json: &str) -> Result<(String, String, String)> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| Error::Catalog(e.to_string()))?;
    let tag = v["tag_name"].as_str().unwrap_or_default().to_string();
    let asset = v["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"] == MAILPIT_ASSET))
        .ok_or_else(|| Error::Catalog("Mailpit sem pacote para Windows".into()))?;
    let url = asset["browser_download_url"].as_str().unwrap_or_default().to_string();
    let sha = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).unwrap_or_default().to_lowercase();
    if !url.starts_with("https://github.com/axllent/mailpit/") || sha.len() != 64 {
        return Err(Error::Catalog("dados do Mailpit incompletos (sem SHA-256)".into()));
    }
    Ok((tag, url, sha))
}

pub async fn install_mailpit(client: &reqwest::Client, paths: &Paths) -> Result<String> {
    let (tag, url, sha) = parse_mailpit_release(&fetch_text(client, MAILPIT_API).await?)?;
    let zip_path = paths.downloads_dir().join(MAILPIT_ASSET);
    let never = AtomicBool::new(false);
    download_verified(client, &url, &sha, &zip_path, &never, &|_, _| {}).await?;
    let dir = mailpit_dir(paths);
    let (z, d) = (zip_path.clone(), dir.clone());
    tokio::task::spawn_blocking(move || -> Result<()> {
        std::fs::create_dir_all(&d).map_err(|e| Error::io(&d, e))?;
        let f = std::fs::File::open(&z).map_err(|e| Error::io(&z, e))?;
        let mut a = zip::ZipArchive::new(f).map_err(|e| Error::Zip(e.to_string()))?;
        let mut entry = a.by_name("mailpit.exe").map_err(|e| Error::Zip(e.to_string()))?;
        let exe = d.join("mailpit.exe");
        let mut out = std::fs::File::create(&exe).map_err(|e| Error::io(&exe, e))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| Error::io(&exe, e))?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Zip(e.to_string()))??;
    let _ = std::fs::remove_file(&zip_path);
    std::fs::write(dir.join("VERSION"), &tag).map_err(|e| Error::io(&dir, e))?;
    tracing::info!(version = tag, "Mailpit instalado");
    Ok(tag)
}

// ---------- Xdebug ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct XdebugBuild {
    pub file: String,
    pub sha256: String,
    pub version: String,
    /// "8.4"
    pub php: String,
}

/// Lê a página de downloads do xdebug.org: cada link de DLL vem com o SHA-256.
pub fn parse_xdebug_page(html: &str) -> Vec<XdebugBuild> {
    let mut out = Vec::new();
    for part in html.split("SHA256:&nbsp;").skip(1) {
        let sha: String = part.chars().take(64).collect();
        if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let Some(start) = part.find("/files/") else { continue };
        let rest = &part[start + 7..];
        let Some(end) = rest.find(['\'', '"']) else { continue };
        let file = &rest[..end];
        // php_xdebug-3.5.3-8.4-nts-vs17-x86_64.dll
        let Some(inner) = file.strip_prefix("php_xdebug-").and_then(|f| f.strip_suffix("-x86_64.dll")) else { continue };
        let parts: Vec<&str> = inner.split('-').collect();
        if parts.len() != 4 || parts[2] != "nts" || file.contains(['/', '\\']) {
            continue;
        }
        out.push(XdebugBuild { file: file.into(), sha256: sha.to_lowercase(), version: parts[0].into(), php: parts[1].into() });
    }
    out
}

/// Build mais nova para a versão do PHP (NTS, x64). Prefere versões estáveis;
/// alpha/beta/RC só quando o PHP é tão novo que ainda não tem estável.
pub fn pick_xdebug<'a>(builds: &'a [XdebugBuild], php_minor: &str) -> Option<&'a XdebugBuild> {
    let stable = |b: &XdebugBuild| b.version.split('.').all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()));
    let key = |b: &XdebugBuild| {
        let nums = b
            .version
            .split('.')
            .map(|p| p.chars().take_while(char::is_ascii_digit).collect::<String>().parse::<u32>().unwrap_or(0))
            .collect::<Vec<_>>();
        // "0alpha1" → estágio alpha(1) < beta(2) < rc(3), depois o número.
        let tail = b.version.rsplit('.').next().unwrap_or("").trim_start_matches(|c: char| c.is_ascii_digit()).to_ascii_lowercase();
        let stage = if tail.starts_with("alpha") { 1 } else if tail.starts_with("beta") { 2 } else if tail.starts_with("rc") { 3 } else { 4 };
        let stage_num = tail.trim_start_matches(|c: char| c.is_ascii_alphabetic()).parse::<u32>().unwrap_or(0);
        (stable(b), nums, stage, stage_num)
    };
    builds.iter().filter(|b| b.php == php_minor).max_by_key(|b| key(b))
}

fn xdebug_dll(php: &InstalledPhp) -> PathBuf {
    php.dir.join("ext").join("php_xdebug.dll")
}

#[derive(Debug, Clone, Serialize)]
pub struct XdebugState {
    pub downloaded: bool,
    pub enabled: bool,
}

pub fn xdebug_state(php: &InstalledPhp) -> XdebugState {
    let ini_text = std::fs::read_to_string(php.ini()).unwrap_or_default();
    let enabled = ini_text.lines().any(|l| {
        let t = l.trim();
        t.starts_with("zend_extension") && t.contains("php_xdebug")
    });
    XdebugState { downloaded: xdebug_dll(php).is_file(), enabled }
}

/// Baixa a DLL certa para essa versão do PHP (uma vez).
pub async fn ensure_xdebug(client: &reqwest::Client, php: &InstalledPhp) -> Result<String> {
    let html = fetch_text(client, "https://xdebug.org/download").await?;
    let builds = parse_xdebug_page(&html);
    let build = pick_xdebug(&builds, &php.minor)
        .ok_or_else(|| Error::Invalid(format!("ainda não há Xdebug para o PHP {} no Windows", php.minor)))?;
    let dll = xdebug_dll(php);
    let never = AtomicBool::new(false);
    download_verified(
        client,
        &format!("https://xdebug.org/files/{}", build.file),
        &build.sha256,
        &dll,
        &never,
        &|_, _| {},
    )
    .await?;
    Ok(build.version.clone())
}

/// Liga/desliga o Xdebug no php.ini (modo debug, só quando pedido pelo
/// navegador/IDE — `trigger` — para não deixar tudo lento).
pub fn set_xdebug(php: &InstalledPhp, enabled: bool) -> Result<()> {
    let path = php.ini();
    let mut text = ini::read(&path)?;
    let dll = xdebug_dll(php);
    let line = format!("zend_extension=\"{}\"", dll.display());
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| {
            let t = l.trim().trim_start_matches(';').trim();
            if t.starts_with("zend_extension") && t.contains("php_xdebug") {
                found = true;
                if enabled { line.clone() } else { format!(";{line}") }
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found && enabled {
        lines.push(String::new());
        lines.push("[xdebug]".into());
        lines.push(line);
    }
    text = lines.join("\r\n") + "\r\n";
    if enabled {
        for (k, v) in [
            ("xdebug.mode", "debug,develop"),
            ("xdebug.start_with_request", "trigger"),
            ("xdebug.client_host", "127.0.0.1"),
            ("xdebug.client_port", "9003"),
        ] {
            text = ini::set_directive(&text, k, v);
        }
    }
    ini::write(&path, &text)
}

/// Para mostrar ao usuário: configuração do VS Code (launch.json).
pub fn vscode_launch(docroot_mapping: Option<&Path>) -> String {
    let _ = docroot_mapping;
    r#"{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Sentinel: ouvir Xdebug",
      "type": "php",
      "request": "launch",
      "port": 9003
    }
  ]
}"#
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdebug_page() {
        let html = r#"<a title="SHA256:&nbsp;f073de91bea046106abf4d6071c963ea71e58571df6ce58948ceca89d121cb2d" href='/files/php_xdebug-3.5.3-8.4-nts-vs17-x86_64.dll'>8.4 NTS</a>
        <a title="SHA256:&nbsp;a3ffd4d5b1f9ca0766b1f3ed321246808a51ce4ba49dc32ae195bfafc4d4d9a2" href='/files/php_xdebug-3.5.3-8.4-vs17-x86_64.dll'>8.4 TS</a>
        <a title="SHA256:&nbsp;48398f54d7bf311728eafeb9999a5a55be5b5feb83b9af8f3115fef4b71dd2f7" href='/files/php_xdebug-3.4.2-8.4-nts-vs17-x86_64.dll'>old</a>"#;
        let b = parse_xdebug_page(html);
        assert_eq!(b.len(), 2, "só NTS");
        let pick = pick_xdebug(&b, "8.4").unwrap();
        assert_eq!(pick.version, "3.5.3");
        assert!(pick_xdebug(&b, "8.6").is_none());
    }

    #[test]
    fn xdebug_prefers_stable() {
        let build = |v: &str, php: &str| XdebugBuild { file: String::new(), sha256: String::new(), version: v.into(), php: php.into() };
        let b = [build("3.6.0alpha1", "8.4"), build("3.5.3", "8.4"), build("3.6.0beta2", "8.6"), build("3.6.0alpha1", "8.6")];
        assert_eq!(pick_xdebug(&b, "8.4").unwrap().version, "3.5.3");
        // PHP novo sem versão estável: usa a pré-versão mais recente.
        assert_eq!(pick_xdebug(&b, "8.6").unwrap().version, "3.6.0beta2");
    }

    #[test]
    fn toggle_ini() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("8.4.28");
        std::fs::create_dir_all(dir.join("ext")).unwrap();
        std::fs::write(dir.join("php.ini"), "[PHP]\r\nmemory_limit = 512M\r\n").unwrap();
        let php = InstalledPhp { version: "8.4.28".into(), minor: "8.4".into(), dir: dir.clone() };
        set_xdebug(&php, true).unwrap();
        assert!(xdebug_state(&php).enabled);
        let text = std::fs::read_to_string(dir.join("php.ini")).unwrap();
        assert!(text.contains("xdebug.mode = debug,develop"));
        set_xdebug(&php, false).unwrap();
        assert!(!xdebug_state(&php).enabled);
        set_xdebug(&php, true).unwrap();
        let text = std::fs::read_to_string(dir.join("php.ini")).unwrap();
        assert_eq!(text.matches("php_xdebug.dll").count(), 1, "não duplica a linha");
    }

    #[test]
    fn mailpit_release() {
        let json = r#"{"tag_name":"v1.34.0","assets":[{"name":"mailpit-windows-amd64.zip","browser_download_url":"https://github.com/axllent/mailpit/releases/download/v1.34.0/mailpit-windows-amd64.zip","digest":"sha256:8f080d41374bf22686606374a0a483c8563c1bf29ae078179792392ff8c911d7"}]}"#;
        let (tag, url, sha) = parse_mailpit_release(json).unwrap();
        assert_eq!(tag, "v1.34.0");
        assert!(url.ends_with(".zip"));
        assert_eq!(sha.len(), 64);
    }
}
