//! Download com progresso, cancelamento e verificação SHA-256.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::{Error, Result};

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
