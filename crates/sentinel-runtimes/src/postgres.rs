//! PostgreSQL para Windows: builds do projeto theseus-rs/postgresql-binaries
//! (zip de ~50 MB por versão, publicado no GitHub com SHA-256), extraído em
//! `runtimes\postgres\<versão>`.

use sentinel_core::t;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sentinel_core::Paths;
use serde::Serialize;

use crate::download::{download_verified, fetch_text};
use crate::error::{Error, Result};

const API: &str = "https://api.github.com/repos/theseus-rs/postgresql-binaries/releases?per_page=40";
const URL_PREFIX: &str = "https://github.com/theseus-rs/postgresql-binaries/";
const TARGET: &str = "x86_64-pc-windows-msvc";
/// Majors oferecidas (as que ainda têm suporte da comunidade).
const MIN_MAJOR: u32 = 15;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PgEntry {
    /// "18"
    pub major: String,
    /// "18.7.0"
    pub version: String,
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledPg {
    pub version: String,
    pub major: String,
    pub dir: PathBuf,
}

impl InstalledPg {
    pub fn bin(&self, tool: &str) -> PathBuf {
        self.dir.join("bin").join(format!("{tool}.exe"))
    }
}

pub fn postgres_dir(paths: &Paths) -> PathBuf {
    paths.runtimes_dir().join("postgres")
}

fn version_key(v: &str) -> Vec<u32> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

fn major_of(v: &str) -> String {
    v.split('.').next().unwrap_or_default().to_string()
}

/// A build mais nova de cada major estável, da mais nova para a mais antiga.
pub fn parse_releases(json: &str) -> Result<Vec<PgEntry>> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| Error::Catalog(e.to_string()))?;
    let releases = v.as_array().ok_or_else(|| Error::Catalog(t!("resposta inesperada do GitHub")))?;
    let mut best: Vec<PgEntry> = Vec::new();
    for r in releases {
        if r["prerelease"].as_bool().unwrap_or(true) || r["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        let version = r["tag_name"].as_str().unwrap_or_default().to_string();
        if version.split('.').count() != 3 || !version.split('.').all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit())) {
            continue;
        }
        let major = major_of(&version);
        if major.parse::<u32>().unwrap_or(0) < MIN_MAJOR {
            continue;
        }
        let name = format!("postgresql-{version}-{TARGET}.zip");
        let Some(asset) = r["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"] == name.as_str())) else {
            continue;
        };
        let url = asset["browser_download_url"].as_str().unwrap_or_default().to_string();
        let sha = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).unwrap_or_default().to_lowercase();
        if !url.starts_with(URL_PREFIX) || sha.len() != 64 || !sha.bytes().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let entry = PgEntry { major: major.clone(), version, url, sha256: sha };
        match best.iter_mut().find(|e| e.major == major) {
            Some(e) if version_key(&entry.version) > version_key(&e.version) => *e = entry,
            Some(_) => {}
            None => best.push(entry),
        }
    }
    best.sort_by_key(|e| std::cmp::Reverse(version_key(&e.version)));
    Ok(best)
}

pub async fn fetch_catalog(client: &reqwest::Client) -> Result<Vec<PgEntry>> {
    parse_releases(&fetch_text(client, API).await?)
}

pub fn list_installed(dir: &Path) -> Vec<InstalledPg> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut list: Vec<InstalledPg> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let ok = name.split('.').count() == 3 && name.split('.').all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()));
            (ok && e.path().join("bin").join("postgres.exe").is_file()).then(|| InstalledPg {
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
    entry: &PgEntry,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<InstalledPg> {
    let file = format!("postgresql-{}-{TARGET}.zip", entry.version);
    let zip_path = paths.downloads_dir().join(&file);
    if !zip_path.is_file() {
        download_verified(client, &entry.url, &entry.sha256, &zip_path, cancel, on_progress).await?;
    }
    let root = postgres_dir(paths);
    let tmp = root.join(format!(".tmp-{}", entry.version));
    let final_dir = root.join(&entry.version);
    {
        let (zip_path, tmp) = (zip_path.clone(), tmp.clone());
        tokio::task::spawn_blocking(move || -> Result<()> {
            crate::archive::extract_zip(&zip_path, &tmp, crate::archive::Limits::default())
        })
        .await
        .map_err(|e| Error::Zip(e.to_string()))??;
    }
    let inner = tmp.join(format!("postgresql-{}-{TARGET}", entry.version));
    if !inner.join("bin").join("postgres.exe").is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(Error::Zip(t!("o pacote não contém postgres.exe")));
    }
    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir).map_err(|e| Error::io(&final_dir, e))?;
    }
    std::fs::rename(&inner, &final_dir).map_err(|e| Error::io(&final_dir, e))?;
    let _ = std::fs::remove_dir_all(&tmp);

    let installed = InstalledPg { version: entry.version.clone(), major: entry.major.clone(), dir: final_dir };
    let out = tokio::process::Command::new(installed.bin("postgres"))
        .arg("--version")
        .output()
        .await
        .map_err(|e| Error::io(installed.bin("postgres"), e))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    // "postgres (PostgreSQL) 18.7" — o zip usa "18.7.0", o binário "18.7".
    let short = entry.version.trim_end_matches(".0");
    if !text.contains(short) {
        let _ = std::fs::remove_dir_all(&installed.dir);
        let code = out.status.code().unwrap_or_default() as u32;
        if code == 0xC000_0135 || code == 0xC000_007B {
            return Err(Error::MissingVcRedist);
        }
        return Err(Error::Verify(text.trim().to_string()));
    }
    let _ = std::fs::remove_file(&zip_path);
    // Mantém só a build atual de cada major (os dados ficam em data\postgres-<major>).
    for old in list_installed(&root).into_iter().filter(|m| m.major == entry.major && m.version != entry.version) {
        let _ = std::fs::remove_dir_all(&old.dir);
    }
    tracing::info!(version = entry.version, "PostgreSQL instalado");
    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(tag: &str, pre: bool, sha: &str) -> String {
        format!(
            r#"{{"tag_name":"{tag}","prerelease":{pre},"draft":false,"assets":[
              {{"name":"postgresql-{tag}-aarch64-pc-windows-msvc.zip","browser_download_url":"https://github.com/theseus-rs/postgresql-binaries/releases/download/{tag}/a.zip","digest":"sha256:{sha}"}},
              {{"name":"postgresql-{tag}-x86_64-pc-windows-msvc.zip","browser_download_url":"https://github.com/theseus-rs/postgresql-binaries/releases/download/{tag}/postgresql-{tag}-x86_64-pc-windows-msvc.zip","digest":"sha256:{sha}"}}]}}"#
        )
    }

    #[test]
    fn newest_build_per_major() {
        let a = "A".repeat(64);
        let json = format!(
            "[{},{},{},{},{},{}]",
            rel("18.7.0", false, &a),
            rel("19.0.0", true, &a),
            rel("17.12.0", false, &a),
            rel("17.11.0", false, &a),
            rel("18.6.0", false, &a),
            rel("14.20.0", false, &a)
        );
        let list = parse_releases(&json).unwrap();
        assert_eq!(list.iter().map(|e| e.version.as_str()).collect::<Vec<_>>(), ["18.7.0", "17.12.0"]);
        assert_eq!(list[0].major, "18");
        assert_eq!(list[0].sha256, "a".repeat(64));
        assert!(list[0].url.ends_with("x86_64-pc-windows-msvc.zip"));
    }

    #[test]
    fn skips_without_digest() {
        let json = format!("[{}]", rel("18.7.0", false, "zz"));
        assert!(parse_releases(&json).unwrap().is_empty());
    }
}
