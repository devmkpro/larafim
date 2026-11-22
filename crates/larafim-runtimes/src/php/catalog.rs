//! Catálogo oficial de builds do PHP para Windows (`releases.json`).

use larafim_core::t;
use serde::Serialize;

use crate::download::fetch_text;
use crate::error::{Error, Result};

/// O antigo `windows.php.net/downloads/releases/` redireciona para cá.
pub const RELEASES_BASE: &str = "https://downloads.php.net/~windows/releases/";

/// Última build de uma versão minor (NTS x64, a indicada para FastCGI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogEntry {
    /// "8.4"
    pub minor: String,
    /// "8.4.28"
    pub version: String,
    /// "php-8.4.28-nts-Win32-vs17-x64.zip"
    pub file: String,
    pub url: String,
    pub sha256: String,
    /// Tamanho como o catálogo informa ("31.4MB").
    pub size: String,
    /// Compilador da build ("vs17"), define qual Visual C++ é necessário.
    pub build: String,
}

pub async fn fetch_catalog(client: &reqwest::Client) -> Result<Vec<CatalogEntry>> {
    let text = fetch_text(client, &format!("{RELEASES_BASE}releases.json")).await?;
    parse_catalog(&text)
}

/// Lê o JSON e devolve uma entrada por minor, da mais nova para a mais antiga.
pub fn parse_catalog(json: &str) -> Result<Vec<CatalogEntry>> {
    let root: serde_json::Value = serde_json::from_str(json).map_err(|e| Error::Catalog(e.to_string()))?;
    let map = root.as_object().ok_or_else(|| Error::Catalog(t!("raiz não é um objeto")))?;

    let mut entries = Vec::new();
    for (minor, info) in map {
        let Some(version) = info.get("version").and_then(|v| v.as_str()) else { continue };
        if super::parse_version(version).is_none() {
            continue;
        }
        // Chave da build: "nts-vs17-x64", "nts-vc15-x64"…
        let Some((key, build)) = info.as_object().and_then(|o| {
            o.iter().find(|(k, _)| k.starts_with("nts-") && k.ends_with("-x64"))
        }) else {
            continue;
        };
        let zip = &build["zip"];
        let (Some(file), Some(sha256)) = (zip["path"].as_str(), zip["sha256"].as_str()) else { continue };

        // O nome vira parte de uma URL e de um caminho local: só aceita nome simples.
        let safe = file.ends_with(".zip")
            && !file.contains(['/', '\\'])
            && !file.contains("..")
            && sha256.len() == 64
            && sha256.chars().all(|c| c.is_ascii_hexdigit());
        if !safe {
            tracing::warn!(minor, file, "entrada do catálogo ignorada");
            continue;
        }

        entries.push(CatalogEntry {
            minor: minor.clone(),
            version: version.to_string(),
            url: format!("{RELEASES_BASE}{file}"),
            file: file.to_string(),
            sha256: sha256.to_lowercase(),
            size: zip["size"].as_str().unwrap_or_default().to_string(),
            build: key.trim_start_matches("nts-").trim_end_matches("-x64").to_string(),
        });
    }

    if entries.is_empty() {
        return Err(Error::Catalog(t!("nenhuma versão encontrada")));
    }
    entries.sort_by_key(|e| std::cmp::Reverse(super::parse_version(&e.version)));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "8.3": {
        "version": "8.3.37",
        "ts-vs16-x64": {"zip": {"path": "php-8.3.37-Win32-vs16-x64.zip", "size": "30MB", "sha256": "1111111111111111111111111111111111111111111111111111111111111111"}},
        "nts-vs16-x64": {"zip": {"path": "php-8.3.37-nts-Win32-vs16-x64.zip", "size": "29.9MB", "sha256": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}},
        "nts-vs16-x86": {"zip": {"path": "x86.zip", "size": "1MB", "sha256": "2222222222222222222222222222222222222222222222222222222222222222"}},
        "source": {"path": "php-8.3.37-src.zip"}
      },
      "8.5": {
        "version": "8.5.13",
        "nts-vs17-x64": {"zip": {"path": "php-8.5.13-nts-Win32-vs17-x64.zip", "size": "32MB", "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}
      },
      "8.4": {
        "version": "8.4.28",
        "nts-vs17-x64": {"zip": {"path": "../evil.zip", "size": "1MB", "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}}
      }
    }"#;

    #[test]
    fn picks_nts_x64_sorted_and_rejects_unsafe_paths() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let versions: Vec<_> = entries.iter().map(|e| e.version.as_str()).collect();
        assert_eq!(versions, ["8.5.13", "8.3.37"]);

        let e = &entries[1];
        assert_eq!(e.minor, "8.3");
        assert_eq!(e.file, "php-8.3.37-nts-Win32-vs16-x64.zip");
        assert_eq!(e.url, format!("{RELEASES_BASE}php-8.3.37-nts-Win32-vs16-x64.zip"));
        assert_eq!(e.sha256, "a".repeat(64));
        assert_eq!(e.build, "vs16");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_catalog("<html>").is_err());
        assert!(parse_catalog("{}").is_err());
    }

    /// Consulta o catálogo real. Rode com `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn real_catalog() {
        let client = crate::http_client().unwrap();
        let entries = fetch_catalog(&client).await.unwrap();
        assert!(entries.iter().any(|e| e.minor == "8.4"));
    }
}
