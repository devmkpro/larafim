//! Node.js para Windows: catálogo oficial (nodejs.org), instalação com
//! SHA-256 e escolha da versão por pasta.

use sentinel_core::t;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use semver::{Version, VersionReq};
use sentinel_core::store::same_path;
use sentinel_core::{Paths, ProjectsFile};
use serde::Serialize;

use crate::download::{download_verified, fetch_text};
use crate::error::{Error, Result};

pub const DIST: &str = "https://nodejs.org/dist/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeEntry {
    /// "22"
    pub major: String,
    /// "22.23.5"
    pub version: String,
    /// Nome LTS ("Jod") ou `None` para a linha "Current".
    pub lts: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledNode {
    pub version: String,
    pub major: String,
    pub dir: PathBuf,
}

impl InstalledNode {
    pub fn node_exe(&self) -> PathBuf {
        self.dir.join("node.exe")
    }
    /// Script do npm/npx que vem junto com o Node.
    pub fn npm_cli(&self, tool: &str) -> PathBuf {
        self.dir.join("node_modules").join("npm").join("bin").join(format!("{tool}-cli.js"))
    }
}

fn parse(v: &str) -> Option<Version> {
    Version::parse(v.trim().trim_start_matches('v')).ok()
}

/// Uma entrada por linha (major): as LTS a partir do 18 e a "Current" mais
/// nova. Versões de teste (alpha, rc) ficam de fora.
pub fn parse_index(json: &str) -> Result<Vec<NodeEntry>> {
    let list: Vec<serde_json::Value> = serde_json::from_str(json).map_err(|e| Error::Catalog(e.to_string()))?;
    let mut out: Vec<NodeEntry> = Vec::new();
    let mut newest_current: Option<NodeEntry> = None;
    for r in &list {
        let Some(v) = r["version"].as_str().and_then(parse) else { continue };
        if !v.pre.is_empty() || v.major < 18 {
            continue;
        }
        let has_zip = r["files"].as_array().is_some_and(|f| f.iter().any(|x| x == "win-x64-zip"));
        if !has_zip {
            continue;
        }
        let major = v.major.to_string();
        if out.iter().any(|e| e.major == major) || newest_current.as_ref().is_some_and(|e| e.major == major) {
            continue; // o índice vem do mais novo para o mais antigo
        }
        let lts = r["lts"].as_str().map(String::from);
        let entry = NodeEntry { major, version: v.to_string(), lts: lts.clone() };
        match lts {
            Some(_) => out.push(entry),
            None => {
                let newer = newest_current.as_ref().is_none_or(|c| parse(&c.version) < Some(v.clone()));
                if newer {
                    newest_current = Some(entry);
                }
            }
        }
    }
    if let Some(c) = newest_current
        && out.first().is_none_or(|lts| parse(&lts.version) < parse(&c.version))
    {
        out.insert(0, c);
    }
    out.sort_by_key(|e| std::cmp::Reverse(parse(&e.version)));
    if out.is_empty() {
        return Err(Error::Catalog(t!("nenhuma versão do Node encontrada")));
    }
    Ok(out)
}

pub async fn fetch_catalog(client: &reqwest::Client) -> Result<Vec<NodeEntry>> {
    parse_index(&fetch_text(client, &format!("{DIST}index.json")).await?)
}

pub fn list_installed(node_dir: &Path) -> Vec<InstalledNode> {
    let Ok(entries) = std::fs::read_dir(node_dir) else { return Vec::new() };
    let mut list: Vec<InstalledNode> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let v = parse(&name)?;
            e.path().join("node.exe").is_file().then(|| InstalledNode {
                major: v.major.to_string(),
                version: name,
                dir: e.path(),
            })
        })
        .collect();
    list.sort_by_key(|n| std::cmp::Reverse(parse(&n.version)));
    list
}

/// Converte `.nvmrc`/`engines` em requisito: "22", "v22.3.0", ">=20", "lts/*".
fn requirement(text: &str) -> Option<VersionReq> {
    let t = text.trim().trim_start_matches('v');
    if t.is_empty() || t.starts_with("lts") || t == "node" || t == "latest" {
        return Some(VersionReq::STAR);
    }
    if t.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) {
        let parts: Vec<_> = t.split('.').collect();
        return VersionReq::parse(&match parts.len() {
            1 => format!("~{}", parts[0]),
            2 => format!("~{}.{}", parts[0], parts[1]),
            _ => format!("={t}"),
        })
        .ok();
    }
    let alts: Vec<String> = t.split("||").map(|a| a.split_whitespace().collect::<Vec<_>>().join(",")).collect();
    VersionReq::parse(alts.first()?).ok()
}

/// Mais novo instalado que atende ao pedido.
pub fn best_installed<'a>(request: &str, installed: &'a [InstalledNode]) -> Option<&'a InstalledNode> {
    let reqs: Vec<VersionReq> = request.split("||").filter_map(requirement).collect();
    installed.iter().find(|n| {
        let v = parse(&n.version);
        v.is_some_and(|v| reqs.iter().any(|r| r.matches(&v)))
    })
}

/// O que uma pasta pede: `.nvmrc`, `.node-version` ou `engines.node`.
pub fn request_in(dir: &Path) -> Option<String> {
    for file in [".nvmrc", ".node-version"] {
        if let Ok(t) = std::fs::read_to_string(dir.join(file))
            && let Some(line) = t.lines().next().map(str::trim).filter(|l| !l.is_empty())
        {
            return Some(line.to_string());
        }
    }
    let text = std::fs::read_to_string(dir.join("package.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json.pointer("/engines/node").and_then(|v| v.as_str()).map(String::from)
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeSelection {
    pub node: Option<InstalledNode>,
    pub reason: String,
    pub project: Option<String>,
}

/// Qual Node usar numa pasta: `SENTINEL_NODE` → versão fixada no projeto →
/// pedido da pasta (`.nvmrc`/`engines`) → LTS mais novo instalado.
pub fn select(paths: &Paths, dir: &Path) -> NodeSelection {
    let installed = list_installed(&paths.node_dir());
    let out = |node: Option<&InstalledNode>, reason: String, project: Option<String>| NodeSelection {
        node: node.cloned(),
        reason,
        project,
    };
    if installed.is_empty() {
        return out(None, t!("nenhum Node instalado no Sentinel"), None);
    }
    if let Ok(forced) = std::env::var("SENTINEL_NODE")
        && !forced.trim().is_empty()
    {
        return match best_installed(&forced, &installed) {
            Some(n) => out(Some(n), format!("SENTINEL_NODE={forced}"), None),
            None => out(None, t!("SENTINEL_NODE={forced}, mas essa versão não está instalada", forced = forced), None),
        };
    }
    let store = ProjectsFile::load(&paths.projects_file()).unwrap_or_default();
    let project = dir.ancestors().find_map(|a| store.projects.iter().find(|p| same_path(&p.path, a)));
    if let Some(cfg) = project
        && let Some(major) = &cfg.node_version
    {
        return match installed.iter().find(|n| &n.major == major) {
            Some(n) => out(Some(n), t!("versão fixada no projeto ({major})", major = major), Some(cfg.domain.clone())),
            None => out(None, t!("o projeto fixa o Node {major}, que não está instalado", major = major), Some(cfg.domain.clone())),
        };
    }
    let project_name = project.map(|p| p.domain.clone());
    if let Some((req, _)) = dir.ancestors().find_map(|a| request_in(a).map(|r| (r, a.to_path_buf()))) {
        return match best_installed(&req, &installed) {
            Some(n) => out(Some(n), t!("mais novo compatível com {req}", req = req), project_name),
            None => out(None, t!("a pasta pede Node {req} e nenhuma versão instalada atende", req = req), project_name),
        };
    }
    out(installed.first(), t!("mais novo instalado"), project_name)
}

/// Instala uma versão (zip oficial win-x64, conferido pelo SHASUMS256.txt).
pub async fn install(
    client: &reqwest::Client,
    paths: &Paths,
    entry: &NodeEntry,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<InstalledNode> {
    let base = format!("{DIST}v{}/", entry.version);
    let file = format!("node-v{}-win-x64.zip", entry.version);
    let sums = fetch_text(client, &format!("{base}SHASUMS256.txt")).await?;
    let sha = sums
        .lines()
        .find_map(|l| l.split_once("  ").filter(|(_, f)| f.trim() == file).map(|(h, _)| h.to_string()))
        .ok_or_else(|| Error::Catalog(t!("{file} não está no SHASUMS256.txt", file = file)))?;

    let zip_path = paths.downloads_dir().join(&file);
    if !zip_path.is_file() {
        download_verified(client, &format!("{base}{file}"), &sha, &zip_path, cancel, on_progress).await?;
    }

    let node_dir = paths.node_dir();
    let tmp = node_dir.join(format!(".tmp-{}", entry.version));
    let final_dir = node_dir.join(&entry.version);
    let inner = format!("node-v{}-win-x64", entry.version);
    {
        let (zip_path, tmp) = (zip_path.clone(), tmp.clone());
        tokio::task::spawn_blocking(move || -> Result<()> {
            if tmp.exists() {
                std::fs::remove_dir_all(&tmp).map_err(|e| Error::io(&tmp, e))?;
            }
            let f = std::fs::File::open(&zip_path).map_err(|e| Error::io(&zip_path, e))?;
            let mut archive = zip::ZipArchive::new(f).map_err(|e| Error::Zip(e.to_string()))?;
            archive.extract(&tmp).map_err(|e| Error::Zip(e.to_string()))
        })
        .await
        .map_err(|e| Error::Zip(e.to_string()))??;
    }
    // O zip tem uma pasta "node-vX-win-x64" dentro.
    let extracted = tmp.join(&inner);
    if !extracted.join("node.exe").is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(Error::Zip(t!("o pacote não contém node.exe")));
    }
    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir).map_err(|e| Error::io(&final_dir, e))?;
    }
    std::fs::rename(&extracted, &final_dir).map_err(|e| Error::io(&final_dir, e))?;
    let _ = std::fs::remove_dir_all(&tmp);

    let installed = InstalledNode { version: entry.version.clone(), major: entry.major.clone(), dir: final_dir };
    let out = tokio::process::Command::new(installed.node_exe())
        .arg("-v")
        .output()
        .await
        .map_err(|e| Error::io(installed.node_exe(), e))?;
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if printed != format!("v{}", entry.version) {
        let _ = std::fs::remove_dir_all(&installed.dir);
        return Err(Error::Verify(printed));
    }
    let _ = std::fs::remove_file(&zip_path);
    // Mantém só a build atual de cada major.
    for old in list_installed(&node_dir).into_iter().filter(|n| n.major == entry.major && n.version != entry.version) {
        let _ = std::fs::remove_dir_all(&old.dir);
    }
    tracing::info!(version = entry.version, "Node instalado");
    Ok(installed)
}

pub fn remove_major(paths: &Paths, major: &str) -> Result<usize> {
    let node_dir = paths.node_dir();
    let mut n = 0;
    for v in list_installed(&node_dir).into_iter().filter(|v| v.major == major) {
        if v.dir.parent() == Some(node_dir.as_path()) {
            std::fs::remove_dir_all(&v.dir).map_err(|e| Error::io(&v.dir, e))?;
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"[
      {"version":"v27.0.0-alpha.1","lts":false,"files":["win-x64-zip"]},
      {"version":"v26.13.1","lts":"Lithium","files":["win-x64-zip"]},
      {"version":"v26.12.0","lts":"Lithium","files":["win-x64-zip"]},
      {"version":"v25.9.0","lts":false,"files":["win-x64-zip"]},
      {"version":"v24.22.3","lts":"Krypton","files":["win-x64-zip"]},
      {"version":"v23.11.1","lts":false,"files":["win-x64-zip"]},
      {"version":"v22.23.5","lts":"Jod","files":["win-x64-zip","osx-arm64-tar"]},
      {"version":"v20.20.2","lts":"Iron","files":["linux-x64"]},
      {"version":"v16.20.2","lts":"Gallium","files":["win-x64-zip"]}
    ]"#;

    #[test]
    fn index_keeps_lts_lines() {
        let e = parse_index(INDEX).unwrap();
        let v: Vec<_> = e.iter().map(|x| (x.version.as_str(), x.lts.is_some())).collect();
        // 25/23 são "Current" mais antigas que a LTS 26; 20 não tem zip; 16 é antiga demais.
        assert_eq!(v, [("26.13.1", true), ("24.22.3", true), ("22.23.5", true)]);
    }

    fn fake(dir: &Path, v: &str) {
        std::fs::create_dir_all(dir.join(v)).unwrap();
        std::fs::write(dir.join(v).join("node.exe"), "").unwrap();
    }

    #[test]
    fn selection() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::at(tmp.path().join("home"));
        for v in ["20.20.2", "22.23.5", "24.22.3"] {
            fake(&paths.node_dir(), v);
        }
        let app = tmp.path().join("app");
        std::fs::create_dir_all(app.join("src")).unwrap();
        std::fs::write(app.join(".nvmrc"), "v22\n").unwrap();
        assert_eq!(select(&paths, &app.join("src")).node.unwrap().version, "22.23.5");

        let api = tmp.path().join("api");
        std::fs::create_dir_all(&api).unwrap();
        std::fs::write(api.join("package.json"), r#"{"engines":{"node":">=18 <21"}}"#).unwrap();
        assert_eq!(select(&paths, &api).node.unwrap().version, "20.20.2");

        assert_eq!(select(&paths, tmp.path()).node.unwrap().version, "24.22.3");

        let old = tmp.path().join("velho");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join(".nvmrc"), "16").unwrap();
        assert!(select(&paths, &old).node.is_none());
    }

    #[test]
    fn requirements() {
        let inst = vec![InstalledNode { version: "22.3.0".into(), major: "22".into(), dir: PathBuf::new() }];
        assert!(best_installed("lts/*", &inst).is_some());
        assert!(best_installed("^20 || ^22", &inst).is_some());
        assert!(best_installed("22.3", &inst).is_some());
        assert!(best_installed("21", &inst).is_none());
    }

    /// Instalação real (~30 MB). `cargo test -p sentinel-runtimes -- --ignored real_node`
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn real_node_install() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::at(tmp.path());
        paths.ensure().unwrap();
        let client = crate::http_client().unwrap();
        let cat = fetch_catalog(&client).await.unwrap();
        let entry = cat.iter().find(|e| e.lts.is_some()).unwrap();
        let never = AtomicBool::new(false);
        let n = install(&client, &paths, entry, &never, &|_, _| {}).await.unwrap();
        assert!(n.npm_cli("npm").is_file());
    }
}
