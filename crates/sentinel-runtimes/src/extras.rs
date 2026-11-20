//! Arquivos de apoio baixados uma vez: certificados raiz (para o PHP falar
//! HTTPS) e o Composer.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sentinel_core::Paths;

use crate::download::{download_verified, fetch_text};
use crate::error::Result;

const CACERT_URL: &str = "https://curl.se/ca/cacert.pem";
const CACERT_SHA_URL: &str = "https://curl.se/ca/cacert.pem.sha256";
const COMPOSER_URL: &str = "https://getcomposer.org/download/latest-stable/composer.phar";
const COMPOSER_SHA_URL: &str = "https://getcomposer.org/download/latest-stable/composer.phar.sha256";

/// Onde fica o pacote de certificados usado por `curl.cainfo`/`openssl.cafile`.
pub fn cacert_path(paths: &Paths) -> PathBuf {
    paths.runtimes_dir().join("cacert.pem")
}

pub fn composer_path(paths: &Paths) -> PathBuf {
    paths.runtimes_dir().join("composer").join("composer.phar")
}

/// O arquivo de hash vem como `<hash>  <nome>` ou só `<hash>`.
fn first_token(text: &str) -> String {
    text.split_whitespace().next().unwrap_or_default().to_string()
}

async fn ensure_file(client: &reqwest::Client, url: &str, sha_url: &str, dest: &Path) -> Result<()> {
    if dest.is_file() {
        return Ok(());
    }
    let sha = first_token(&fetch_text(client, sha_url).await?);
    let never = AtomicBool::new(false);
    download_verified(client, url, &sha, dest, &never, &|_, _| {}).await
}

pub async fn ensure_cacert(client: &reqwest::Client, paths: &Paths) -> Result<PathBuf> {
    let dest = cacert_path(paths);
    ensure_file(client, CACERT_URL, CACERT_SHA_URL, &dest).await?;
    Ok(dest)
}

pub async fn ensure_composer(client: &reqwest::Client, paths: &Paths) -> Result<PathBuf> {
    let dest = composer_path(paths);
    ensure_file(client, COMPOSER_URL, COMPOSER_SHA_URL, &dest).await?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_file_formats() {
        assert_eq!(first_token("abc123  cacert.pem\n"), "abc123");
        assert_eq!(first_token("abc123\n"), "abc123");
    }
}
