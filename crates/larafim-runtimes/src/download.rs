//! Download com progresso, cancelamento e verificação SHA-256.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::{Error, Result};

/// Fontes oficiais de onde o Larafim baixa algo. Um catálogo adulterado
/// não consegue apontar o download para outro servidor.
pub const TRUSTED_HOSTS: &[&str] = &[
    "downloads.php.net",
    "windows.php.net",
    "nodejs.org",
    "downloads.mariadb.org",
    "github.com",
    "getcomposer.org",
    "curl.se",
    "xdebug.org",
];

/// Antes de baixar: só `https://` de uma fonte oficial e com um SHA-256 de verdade.
pub fn check_source(url: &str, sha256: &str) -> Result<()> {
    if !larafim_core::safety::is_https_url_on(url, TRUSTED_HOSTS) {
        return Err(Error::Invalid(larafim_core::t!("download recusado: {url} não é uma fonte oficial", url = url)));
    }
    if !larafim_core::safety::is_sha256(sha256.trim()) {
        return Err(Error::Invalid(larafim_core::t!("download recusado: SHA-256 inválido para {url}", url = url)));
    }
    Ok(())
}

/// Baixa `url` para `dest`. Grava primeiro em `dest.part` e só renomeia
/// depois de conferir o SHA-256 — um arquivo pela metade nunca vira "pronto".
/// `on_progress(recebido, total)` é chamado a cada pedaço recebido.
pub async fn download_verified(
    client: &reqwest::Client,
    url: &str,
    sha256: &str,
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<()> {
    check_source(url, sha256)?;
    let mut resp = client.get(url).send().await.map_err(Error::Http)?;
    if !resp.status().is_success() {
        return Err(Error::HttpStatus { status: resp.status().as_u16(), url: url.into() });
    }
    let total = resp.content_length();

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| Error::io(parent, e))?;
    }
    // Nome temporário único: dois downloads do mesmo arquivo ao mesmo tempo
    // (ex.: duas instalações pedindo o Composer) não se atropelam.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let part = dest.with_extension(format!("{}-{n}.part", std::process::id()));
    let mut file = tokio::fs::File::create(&part).await.map_err(|e| Error::io(&part, e))?;
    let mut hasher = Sha256::new();
    let mut received = 0u64;

    let result: Result<()> = async {
        while let Some(chunk) = resp.chunk().await.map_err(Error::Http)? {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| Error::io(&part, e))?;
            received += chunk.len() as u64;
            on_progress(received, total);
        }
        file.flush().await.map_err(|e| Error::io(&part, e))?;
        Ok(())
    }
    .await;
    drop(file);

    if let Err(e) = result {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(e);
    }

    let digest = hex::encode(hasher.finalize());
    if !digest.eq_ignore_ascii_case(sha256.trim()) {
        let _ = tokio::fs::remove_file(&part).await;
        tracing::warn!(url, expected = sha256, got = digest, "SHA-256 não confere");
        return Err(Error::ChecksumMismatch);
    }

    tokio::fs::rename(&part, dest).await.map_err(|e| Error::io(dest, e))
}

/// Baixa um texto pequeno (catálogos, arquivos de hash).
pub async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().await.map_err(Error::Http)?;
    if !resp.status().is_success() {
        return Err(Error::HttpStatus { status: resp.status().as_u16(), url: url.into() });
    }
    resp.text().await.map_err(Error::Http)
}

/// SHA-256 de um arquivo local (em hexadecimal minúsculo).
pub fn sha256_file(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| Error::io(path, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_official_sources() {
        let sha = "a".repeat(64);
        for ok in [
            "https://downloads.php.net/~windows/releases/php-8.4.28-nts-Win32-vs17-x64.zip",
            "https://github.com/axllent/mailpit/releases/download/v1.34.0/mailpit-windows-amd64.zip",
            "https://nodejs.org/dist/v22.11.0/node-v22.11.0-win-x64.zip",
        ] {
            assert!(check_source(ok, &sha).is_ok(), "{ok}");
        }
        for bad in [
            "http://downloads.php.net/php.zip",
            "https://evil.example/php.zip",
            "https://github.com.evil.example/x.zip",
            "https://github.com@evil.example/x.zip",
            "file:///C:/x.zip",
        ] {
            assert!(check_source(bad, &sha).is_err(), "{bad}");
        }
        assert!(check_source("https://github.com/x.zip", "").is_err());
        assert!(check_source("https://github.com/x.zip", "naoehash").is_err());
    }

    /// Mesmo com URL boa, o arquivo só fica se o SHA-256 bater (sem rede:
    /// a recusa acontece antes de qualquer conexão).
    #[tokio::test]
    async fn refuses_before_connecting() {
        let tmp = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        let never = AtomicBool::new(false);
        let dest = tmp.path().join("x.zip");
        let r = download_verified(&client, "https://evil.example/x.zip", &"a".repeat(64), &dest, &never, &|_, _| {}).await;
        assert!(r.is_err());
        assert!(!dest.exists());
    }

    #[test]
    fn sha256_of_known_content() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a.txt");
        std::fs::write(&f, b"abc").unwrap();
        assert_eq!(
            sha256_file(&f).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
