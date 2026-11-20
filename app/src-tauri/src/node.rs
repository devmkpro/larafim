//! Comandos de versões do Node.js: catálogo, instalar, remover.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sentinel_runtimes::http_client;
use sentinel_runtimes::node::{self, NodeEntry};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

#[derive(Default)]
pub struct NodeState {
    catalog: Mutex<Option<(Instant, Vec<NodeEntry>)>>,
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

#[derive(Serialize)]
pub struct NodeRow {
    major: String,
    installed: Option<String>,
    latest: Option<String>,
    lts: Option<String>,
    update_available: bool,
    installing: bool,
}

#[derive(Serialize)]
pub struct NodeOverview {
    rows: Vec<NodeRow>,
    catalog_error: Option<String>,
}

async fn catalog(state: &NodeState, force: bool) -> Result<Vec<NodeEntry>, String> {
    if !force
        && let Some((at, e)) = state.catalog.lock().unwrap().as_ref()
        && at.elapsed() < Duration::from_secs(30 * 60)
    {
        return Ok(e.clone());
    }
    let client = http_client().map_err(|e| e.to_string())?;
    let e = node::fetch_catalog(&client).await.map_err(|e| e.to_string())?;
    *state.catalog.lock().unwrap() = Some((Instant::now(), e.clone()));
    Ok(e)
}

#[tauri::command]
pub async fn node_overview(
    app: State<'_, AppState>,
    state: State<'_, NodeState>,
    refresh: Option<bool>,
) -> CmdResult<NodeOverview> {
    let cat = catalog(&state, refresh.unwrap_or(false)).await;
    let installed = node::list_installed(&app.paths.node_dir());
    let running = state.running.lock().unwrap();
    let (entries, catalog_error) = match cat {
        Ok(e) => (e, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    let mut majors: Vec<String> = entries.iter().map(|e| e.major.clone()).collect();
    for n in &installed {
        if !majors.contains(&n.major) {
            majors.push(n.major.clone());
        }
    }
    majors.sort_by_key(|m| std::cmp::Reverse(m.parse::<u32>().unwrap_or(0)));
    let rows = majors
        .into_iter()
        .map(|major| {
            let inst = installed.iter().find(|n| n.major == major).map(|n| n.version.clone());
            let entry = entries.iter().find(|e| e.major == major);
            let update_available = match (&inst, entry) {
                (Some(i), Some(e)) => semver::Version::parse(&e.version).ok() > semver::Version::parse(i).ok(),
                _ => false,
            };
            NodeRow {
                installing: running.contains_key(&major),
                latest: entry.map(|e| e.version.clone()),
                lts: entry.and_then(|e| e.lts.clone()),
                installed: inst,
                update_available,
                major,
            }
        })
        .collect();
    Ok(NodeOverview { rows, catalog_error })
}

#[derive(Clone, Serialize)]
struct Progress {
    major: String,
    received: u64,
    total: Option<u64>,
}

#[derive(Clone, Serialize)]
struct Done {
    major: String,
    ok: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn node_install(app: AppHandle, state: State<'_, NodeState>, major: String) -> CmdResult<()> {
    let entry = catalog(&state, false)
        .await?
        .into_iter()
        .find(|e| e.major == major)
        .ok_or_else(|| format!("o Node {major} não está no catálogo"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut r = state.running.lock().unwrap();
        if r.contains_key(&major) {
            return Err(format!("o Node {major} já está sendo instalado"));
        }
        r.insert(major.clone(), cancel.clone());
    }
    tauri::async_runtime::spawn(async move {
        let paths = app.state::<AppState>().paths.clone();
        let emitter = app.clone();
        let m = major.clone();
        let last = Mutex::new(Instant::now() - Duration::from_secs(1));
        let on_progress = move |received: u64, total: Option<u64>| {
            let mut l = last.lock().unwrap();
            if l.elapsed() >= Duration::from_millis(100) {
                *l = Instant::now();
                let _ = emitter.emit("node:progress", Progress { major: m.clone(), received, total });
            }
        };
        let result = match http_client() {
            Ok(c) => node::install(&c, &paths, &entry, &cancel, &on_progress).await,
            Err(e) => Err(e),
        };
        app.state::<NodeState>().running.lock().unwrap().remove(&major);
        let _ = app.emit(
            "node:done",
            Done { major, ok: result.is_ok(), error: result.err().map(|e| e.to_string()) },
        );
    });
    Ok(())
}

#[tauri::command]
pub fn node_cancel(state: State<'_, NodeState>, major: String) {
    if let Some(f) = state.running.lock().unwrap().get(&major) {
        f.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn node_remove(app: State<'_, AppState>, major: String) -> CmdResult<()> {
    node::remove_major(&app.paths, &major).map(|_| ()).map_err(|e| e.to_string())
}

/// Majors instalados (para o seletor do projeto).
#[tauri::command]
pub fn node_installed(app: State<'_, AppState>) -> Vec<String> {
    let mut v: Vec<String> = node::list_installed(&app.paths.node_dir()).into_iter().map(|n| n.major).collect();
    v.dedup();
    v
}
