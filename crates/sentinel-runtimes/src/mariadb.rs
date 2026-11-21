//! MariaDB para Windows: versões LTS da API oficial de downloads, zip
//! `winx64` conferido pelo SHA-256 e extraído em `runtimes\mariadb\<versão>`.

use sentinel_core::t;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sentinel_core::Paths;
use serde::Serialize;

use crate::download::{download_verified, fetch_text};
use crate::error::{Error, Result};

const API: &str = "https://downloads.mariadb.org/rest-api/mariadb/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MariaEntry {
    /// "11.8"
    pub major: String,
    /// "11.8.9"
    pub version: String,
    pub url: String,
    pub sha256: String,
    /// Data de fim do suporte (LTS).
    pub eol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledMaria {
    pub version: String,
    pub major: String,
    pub dir: PathBuf,
}

impl InstalledMaria {
    pub fn bin(&self, tool: &str) -> PathBuf {
        self.dir.join("bin").join(format!("{tool}.exe"))
    }
}

fn major_of(v: &str) -> String {
    v.split('.').take(2).collect::<Vec<_>>().join(".")
}

fn version_key(v: &str) -> Vec<u32> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// Linhas estáveis com suporte longo (LTS), da mais nova para a mais antiga.
pub fn parse_majors(json: &str) -> Result<Vec<(String, Option<String>)>> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| Error::Catalog(e.to_string()))?;
    let list = v["major_releases"].as_array().ok_or_else(|| Error::Catalog(t!("sem major_releases")))?;
    Ok(list
        .iter()
        .filter(|r| r["release_status"] == "Stable" && r["release_support_type"] == "Long Term Support")
        .filter_map(|r| Some((r["release_id"].as_str()?.to_string(), r["release_eol_date"].as_str().map(String::from))))
        .collect())
}

/// Última versão de uma linha com o zip `winx64`.
pub fn parse_release(json: &str, major: &str, eol: Option<String>) -> Result<MariaEntry> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| Error::Catalog(e.to_string()))?;
    let releases = v["releases"].as_object().ok_or_else(|| Error::Catalog(t!("sem releases")))?;
    let mut versions: Vec<&String> = releases.keys().collect();
    versions.sort_by_key(|k| std::cmp::Reverse(version_key(k)));
    for version in versions {
        let files = releases[version]["files"].as_array();
        let zip = files.into_iter().flatten().find(|f| {
            f["file_name"].as_str().is_some_and(|n| n == format!("mariadb-{version}-winx64.zip"))
        });
        if let Some(f) = zip {
            let sha = f["checksum"]["sha256sum"].as_str().unwrap_or_default().to_lowercase();
            if sha.len() != 64 {
                continue;
            }
            let file = format!("mariadb-{version}-winx64.zip");
            return Ok(MariaEntry {
                major: major.into(),
                version: version.clone(),
                url: format!("{API}{version}/{file}"),
                sha256: sha,
                eol,
            });
        }
    }
    Err(Error::Catalog(t!("nenhum zip para Windows na linha {major}", major = major)))
}

pub async fn fetch_catalog(client: &reqwest::Client) -> Result<Vec<MariaEntry>> {
    let majors = parse_majors(&fetch_text(client, API).await?)?;
    let mut out = Vec::new();
    for (major, eol) in majors.into_iter().take(4) {
        match fetch_text(client, &format!("{API}{major}/")).await.and_then(|t| parse_release(&t, &major, eol)) {
            Ok(e) => out.push(e),
            Err(e) => tracing::warn!(major, "linha do MariaDB ignorada: {e}"),
        }
    }
    if out.is_empty() {
        return Err(Error::Catalog(t!("nenhuma versão do MariaDB disponível")));
    }
    Ok(out)
}

pub fn list_installed(dir: &Path) -> Vec<InstalledMaria> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut list: Vec<InstalledMaria> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let ok = name.split('.').count() == 3 && name.split('.').all(|p| p.chars().all(|c| c.is_ascii_digit()));
            (ok && e.path().join("bin").join("mariadbd.exe").is_file()).then(|| InstalledMaria {
                major: major_of(&name),
                version: name,
                dir: e.path(),
            })
        })
        .collect();
    list.sort_by_key(|m| std::cmp::Reverse(version_key(&m.version)));
    list
}

pub async fn install(
    client: &reqwest::Client,
    paths: &Paths,
    entry: &MariaEntry,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<InstalledMaria> {
    let file = format!("mariadb-{}-winx64.zip", entry.version);
    let zip_path = paths.downloads_dir().join(&file);
    if !zip_path.is_file() {
        download_verified(client, &entry.url, &entry.sha256, &zip_path, cancel, on_progress).await?;
    }
    let root = paths.mariadb_dir();
    let tmp = root.join(format!(".tmp-{}", entry.version));
    let final_dir = root.join(&entry.version);
    {
        let (zip_path, tmp) = (zip_path.clone(), tmp.clone());
        tokio::task::spawn_blocking(move || -> Result<()> {
            if tmp.exists() {
                std::fs::remove_dir_all(&tmp).map_err(|e| Error::io(&tmp, e))?;
            }
            let f = std::fs::File::open(&zip_path).map_err(|e| Error::io(&zip_path, e))?;
            zip::ZipArchive::new(f)
                .and_then(|mut a| a.extract(&tmp))
                .map_err(|e| Error::Zip(e.to_string()))
        })
        .await
        .map_err(|e| Error::Zip(e.to_string()))??;
    }
    let inner = tmp.join(format!("mariadb-{}-winx64", entry.version));
    if !inner.join("bin").join("mariadbd.exe").is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(Error::Zip(t!("o pacote não contém mariadbd.exe")));
    }
    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir).map_err(|e| Error::io(&final_dir, e))?;
    }
    std::fs::rename(&inner, &final_dir).map_err(|e| Error::io(&final_dir, e))?;
    let _ = std::fs::remove_dir_all(&tmp);

    let installed = InstalledMaria { version: entry.version.clone(), major: entry.major.clone(), dir: final_dir };
    let out = tokio::process::Command::new(installed.bin("mariadbd"))
        .arg("--version")
        .output()
        .await
        .map_err(|e| Error::io(installed.bin("mariadbd"), e))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if !text.contains(&entry.version) {
        let _ = std::fs::remove_dir_all(&installed.dir);
        let code = out.status.code().unwrap_or_default() as u32;
        if code == 0xC000_0135 || code == 0xC000_007B {
            return Err(Error::MissingVcRedist);
        }
        return Err(Error::Verify(text.trim().to_string()));
    }
    let _ = std::fs::remove_file(&zip_path);
    // Mantém só a build atual de cada linha (os dados ficam em data\mariadb\<linha>).
    for old in list_installed(&root).into_iter().filter(|m| m.major == entry.major && m.version != entry.version) {
        let _ = std::fs::remove_dir_all(&old.dir);
    }
    tracing::info!(version = entry.version, "MariaDB instalado");
    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lts_and_release() {
        let majors = r#"{"major_releases":[
          {"release_id":"13.0","release_status":"Stable","release_support_type":"Rolling","release_eol_date":null},
          {"release_id":"12.3","release_status":"Stable","release_support_type":"Long Term Support","release_eol_date":"2029-06-12"},
          {"release_id":"11.8","release_status":"Stable","release_support_type":"Long Term Support","release_eol_date":"2028-06-04"},
          {"release_id":"13.2","release_status":"Preview","release_support_type":null,"release_eol_date":null}]}"#;
        let m = parse_majors(majors).unwrap();
        assert_eq!(m.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["12.3", "11.8"]);

        let release = r#"{"releases":{
          "11.8.8":{"files":[{"file_name":"mariadb-11.8.8-winx64.zip","checksum":{"sha256sum":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}}]},
          "11.8.9":{"files":[
            {"file_name":"mariadb-11.8.9-winx64.msi","checksum":{"sha256sum":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}},
            {"file_name":"mariadb-11.8.9-winx64.zip","checksum":{"sha256sum":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}}]}}}"#;
        let e = parse_release(release, "11.8", None).unwrap();
        assert_eq!(e.version, "11.8.9");
        assert_eq!(e.sha256, "c".repeat(64));
        assert!(e.url.ends_with("/11.8.9/mariadb-11.8.9-winx64.zip"));
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn real_catalog() {
        let c = crate::http_client().unwrap();
        let cat = fetch_catalog(&c).await.unwrap();
        assert!(cat.iter().any(|e| e.major == "11.8"));
    }
}
