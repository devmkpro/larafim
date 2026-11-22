//! Extração segura dos pacotes baixados (PHP, Node, MariaDB, PostgreSQL).
//!
//! Mesmo com o SHA-256 conferido, o zip é tratado como não confiável:
//! - nenhum arquivo pode sair da pasta de destino (`../`, caminho absoluto,
//!   letra de unidade, `:` de fluxo alternativo do NTFS);
//! - links simbólicos são recusados;
//! - há limite de quantidade de arquivos e de tamanho total descompactado
//!   (contra "zip bomb"), contado nos bytes realmente escritos.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use larafim_core::t;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_entries: usize,
    pub max_total_bytes: u64,
}

impl Default for Limits {
    /// Folgado para os maiores pacotes (MariaDB ~ 1 GB descompactado).
    fn default() -> Self {
        Self { max_entries: 100_000, max_total_bytes: 4 * 1024 * 1024 * 1024 }
    }
}

/// Caminho relativo seguro dentro do destino, ou `None` se o nome é perigoso.
fn safe_relative(name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains(['\0', ':']) || name.starts_with(['/', '\\']) {
        return None;
    }
    let mut out = PathBuf::new();
    for part in name.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.ends_with(['.', ' ']) || larafim_core::safety::is_reserved_windows_name(part) {
            return None;
        }
        out.push(part);
    }
    // Confere de novo pelos componentes que o próprio Rust enxerga.
    let normal = out.components().all(|c| matches!(c, Component::Normal(_)));
    (normal && out.components().next().is_some()).then_some(out)
}

/// Extrai `zip_path` em `dest` (que é apagado antes, se existir).
pub fn extract_zip(zip_path: &Path, dest: &Path, limits: Limits) -> Result<()> {
    let result = extract_inner(zip_path, dest, limits);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(dest);
    }
    result
}

fn extract_inner(zip_path: &Path, dest: &Path, limits: Limits) -> Result<()> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| Error::io(dest, e))?;
    }
    std::fs::create_dir_all(dest).map_err(|e| Error::io(dest, e))?;
    let file = std::fs::File::open(zip_path).map_err(|e| Error::io(zip_path, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| Error::Zip(e.to_string()))?;
    if archive.len() > limits.max_entries {
        return Err(Error::Zip(t!("o pacote tem arquivos demais ({n})", n = archive.len())));
    }

    // 1ª passada: só os cabeçalhos (sem descompactar), para recusar cedo.
    let mut declared: u64 = 0;
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).map_err(|e| Error::Zip(e.to_string()))?;
        let name = entry.name().map_err(|e| Error::Zip(e.to_string()))?.to_string();
        if entry.is_symlink() {
            return Err(Error::Zip(t!("o pacote tem um link simbólico ({name})", name = name)));
        }
        if safe_relative(&name).is_none() || entry.enclosed_name().is_none() {
            return Err(Error::Zip(t!("o pacote tem um caminho perigoso ({name})", name = name)));
        }
        declared = declared.saturating_add(entry.size());
    }
    if declared > limits.max_total_bytes {
        return Err(Error::Zip(t!("o pacote descompactado passaria do limite")));
    }

    // 2ª passada: extrai contando os bytes de verdade (o tamanho declarado pode mentir).
    let mut written: u64 = 0;
    let mut buf = vec![0u8; 64 * 1024];
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| Error::Zip(e.to_string()))?;
        let name = entry.name().map_err(|e| Error::Zip(e.to_string()))?.to_string();
        let rel = safe_relative(&name).ok_or_else(|| Error::Zip(t!("o pacote tem um caminho perigoso ({name})", name = name)))?;
        let target = dest.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| Error::io(&target, e))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut out = std::fs::File::create(&target).map_err(|e| Error::io(&target, e))?;
        loop {
            let n = entry.read(&mut buf).map_err(|e| Error::Zip(e.to_string()))?;
            if n == 0 {
                break;
            }
            written += n as u64;
            if written > limits.max_total_bytes {
                return Err(Error::Zip(t!("o pacote descompactado passaria do limite")));
            }
            out.write_all(&buf[..n]).map_err(|e| Error::io(&target, e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;

    /// Monta um zip com os nomes e conteúdos dados (o `zip` deixa gravar
    /// nomes perigosos de propósito, como um atacante faria).
    fn make_zip(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join("pacote.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
        path
    }

    #[test]
    fn extracts_normal_package() {
        let tmp = tempfile::tempdir().unwrap();
        let zip = make_zip(tmp.path(), &[("php-8.4/php.exe", b"MZ"), ("php-8.4/ext/php_curl.dll", b"dll")]);
        let dest = tmp.path().join("out");
        extract_zip(&zip, &dest, Limits::default()).unwrap();
        assert_eq!(std::fs::read(dest.join("php-8.4/php.exe")).unwrap(), b"MZ");
        assert!(dest.join("php-8.4/ext/php_curl.dll").is_file());
    }

    #[test]
    fn refuses_path_traversal() {
        for evil in [
            "../evil.txt",
            "a/../../evil.txt",
            "..\\evil.txt",
            "/abs/evil.txt",
            "\\abs\\evil.txt",
            "C:/Windows/evil.txt",
            "C:\\Windows\\evil.txt",
            "ok.txt:hidden",
            "CON",
            "dir/NUL.txt",
            "trailing.",
            "trailing ",
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let zip = make_zip(tmp.path(), &[("good.txt", b"1"), (evil, b"pwned")]);
            let dest = tmp.path().join("pkg").join("out");
            let err = extract_zip(&zip, &dest, Limits::default());
            assert!(err.is_err(), "{evil} deveria ser recusado");
            assert!(!dest.exists(), "{evil}: a pasta deveria ser apagada");
            assert!(!tmp.path().join("evil.txt").exists() && !tmp.path().join("pkg").join("evil.txt").exists());
        }
    }

    #[test]
    fn refuses_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("link.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        w.add_symlink("atalho", "C:\\Windows\\System32", SimpleFileOptions::default()).unwrap();
        w.finish().unwrap();
        assert!(extract_zip(&path, &tmp.path().join("out"), Limits::default()).is_err());
    }

    #[test]
    fn refuses_zip_bomb() {
        let tmp = tempfile::tempdir().unwrap();
        // 2 MB de zeros comprimem para quase nada.
        let zeros = vec![0u8; 2 * 1024 * 1024];
        let zip = make_zip(tmp.path(), &[("a.bin", &zeros), ("b.bin", &zeros)]);
        let small = Limits { max_entries: 10, max_total_bytes: 3 * 1024 * 1024 };
        let dest = tmp.path().join("out");
        assert!(extract_zip(&zip, &dest, small).is_err());
        assert!(!dest.exists());
    }

    #[test]
    fn refuses_too_many_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let names: Vec<String> = (0..20).map(|i| format!("f{i}.txt")).collect();
        let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x"[..])).collect();
        let zip = make_zip(tmp.path(), &entries);
        let limits = Limits { max_entries: 10, max_total_bytes: 1024 };
        assert!(extract_zip(&zip, &tmp.path().join("out"), limits).is_err());
    }

    #[test]
    fn refuses_garbage() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nao-e-zip.zip");
        std::fs::write(&path, b"<html>404</html>").unwrap();
        assert!(extract_zip(&path, &tmp.path().join("out"), Limits::default()).is_err());
    }
}
