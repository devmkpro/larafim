//! PostgreSQL (fase 8): download opcional, um servidor sob demanda e um
//! banco por projeto, como o MariaDB compartilhado.

use sentinel_core::t;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sentinel_runtimes::http_client;
use sentinel_runtimes::php::ini;
use sentinel_runtimes::postgres::{self, PgEntry};
use sentinel_server::postgres::{PgClient, PgError, PgService, PgSetup, PgStatus};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

pub struct PgState {
    pub svc: Arc<PgService>,
    catalog: Mutex<Option<(Instant, Vec<PgEntry>)>>,
    installing: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Default for PgState {
    fn default() -> Self {
        Self { svc: Arc::new(PgService::default()), catalog: Mutex::new(None), installing: Mutex::new(HashMap::new()) }
    }
}

pub fn pg_port(app: &AppHandle) -> u16 {
    app.state::<AppState>().config.lock().unwrap().pg_port
}

pub fn pg_setup(app: &AppHandle) -> Result<PgSetup, String> {
    let state = app.state::<AppState>();
    let (version, port) = {
        let c = state.config.lock().unwrap();
        (c.pg_version.clone(), c.pg_port)
    };
    let installed = postgres::list_installed(&postgres::postgres_dir(&state.paths));
    let pick = match &version {
        Some(major) => installed.iter().find(|m| &m.major == major),
        None => installed.first(),
    }
    .ok_or_else(|| t!("o PostgreSQL não está instalado (Configurações › Serviços)"))?;
    Ok(PgSetup {
        version: pick.version.clone(),
        install_dir: pick.dir.clone(),
        data_dir: state.paths.data_dir().join(format!("postgres-{}", pick.major)),
        port,
    })
}

fn pg_err(e: PgError) -> String {
    match e {
        PgError::PortBusy(p) => {
            let who = sentinel_server::port::listener_on(p)
                .map(|o| match o.hint {
                    Some(h) => format!("{} ({h})", o.name),
                    None => o.name,
                })
                .unwrap_or_else(|| t!("outro programa"));
            t!("A porta {p} já está em uso por {who}. Escolha outra porta para o PostgreSQL (ex.: {a}).", p = p, who = who, a = p + 1)
        }
        PgError::Failed(m) => m,
    }
}

pub async fn pg_running(app: &AppHandle) -> bool {
    app.state::<PgState>().svc.status().await.running
}

/// Liga o PostgreSQL (também ao deixar online um projeto que usa ele).
pub async fn start_pg(app: &AppHandle) -> Result<(), String> {
    let setup = pg_setup(app)?;
    let svc = app.state::<PgState>().svc.clone();
    let r = svc.start(&setup).await.map_err(pg_err);
    let _ = app.emit("services-changed", ());
    if r.is_ok() {
        crate::services::refresh_adminer(app);
        // O Adminer (PHP mais novo) precisa da extensão pgsql.
        let paths = app.state::<AppState>().paths.clone();
        if let Some(newest) = sentinel_runtimes::php::installed::list_installed(&paths.php_dir()).first() {
            enable_pg_extensions(app, newest).await;
        }
        crate::server::after_change(app, false);
    }
    r
}

pub async fn client(app: &AppHandle) -> Result<PgClient, String> {
    start_pg(app).await?;
    app.state::<PgState>().svc.client().await.map_err(pg_err)
}

/// Liga `pgsql` e `pdo_pgsql` no `php.ini` dessa versão (se ainda não
/// estiverem) e reinicia os php-cgi para valer.
pub async fn enable_pg_extensions(app: &AppHandle, php: &sentinel_runtimes::php::installed::InstalledPhp) {
    let Ok(text) = ini::read(&php.ini()) else { return };
    let ext_dir = php.dir.join("ext");
    let current = ini::list_extensions(&text, &ext_dir);
    let mut updated = text.clone();
    for name in ["pgsql", "pdo_pgsql"] {
        if let Some(e) = current.iter().find(|e| e.name == name && !e.enabled) {
            updated = ini::set_extension(&updated, &e.name, true, e.zend);
        }
    }
    if updated != text {
        if let Err(e) = ini::write(&php.ini(), &updated) {
            tracing::warn!("não foi possível ligar pgsql no PHP {}: {e}", php.version);
            return;
        }
        tracing::info!(version = php.version, "extensões do PostgreSQL ligadas");
        app.state::<crate::server::ServerState>().restart_php().await;
    }
}

/// Liga as extensões no PHP que o projeto usa.
pub async fn enable_for_project(app: &AppHandle, path: &Path) {
    let state = app.state::<AppState>();
    let Some(view) = crate::server::views(&state).into_iter().find(|v| sentinel_core::store::same_path(&v.path, path)) else {
        return;
    };
    let installed = sentinel_runtimes::php::installed::list_installed(&state.paths.php_dir());
    let Some(rt) = crate::server::php_for(&view, &installed).0 else { return };
    if let Some(php) = installed.iter().find(|p| p.version == rt.version) {
        enable_pg_extensions(app, php).await;
    }
}

// ---------- Tela de Serviços ----------

#[derive(Serialize)]
pub struct PgRow {
    major: String,
    latest: Option<String>,
    installed: Option<String>,
    update_available: bool,
    installing: bool,
}

#[derive(Serialize)]
pub struct PgOverview {
    rows: Vec<PgRow>,
    catalog_error: Option<String>,
    status: PgStatus,
    port: u16,
    selected: Option<String>,
    databases: Vec<String>,
}

async fn catalog(state: &PgState, force: bool) -> Result<Vec<PgEntry>, String> {
    if !force
        && let Some((at, c)) = state.catalog.lock().unwrap().as_ref()
        && at.elapsed() < Duration::from_secs(30 * 60)
    {
        return Ok(c.clone());
    }
    let client = http_client().map_err(|e| e.to_string())?;
    let c = postgres::fetch_catalog(&client).await.map_err(|e| e.to_string())?;
    *state.catalog.lock().unwrap() = Some((Instant::now(), c.clone()));
    Ok(c)
}

fn newer(a: &str, b: &str) -> bool {
    let k = |v: &str| v.split('.').map(|p| p.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    k(a) > k(b)
}

#[tauri::command]
pub async fn pg_overview(app: AppHandle, refresh: Option<bool>) -> CmdResult<PgOverview> {
    let state = app.state::<PgState>();
    let paths = app.state::<AppState>().paths.clone();
    let (entries, catalog_error) = match catalog(&state, refresh.unwrap_or(false)).await {
        Ok(c) => (c, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    let installed = postgres::list_installed(&postgres::postgres_dir(&paths));
    let mut majors: Vec<String> = entries.iter().map(|e| e.major.clone()).collect();
    for m in &installed {
        if !majors.contains(&m.major) {
            majors.push(m.major.clone());
        }
    }
    let installing = state.installing.lock().unwrap().clone();
    let rows = majors
        .into_iter()
        .map(|major| {
            let inst = installed.iter().find(|m| m.major == major).map(|m| m.version.clone());
            let latest = entries.iter().find(|e| e.major == major).map(|e| e.version.clone());
            PgRow {
                update_available: matches!((&inst, &latest), (Some(i), Some(l)) if newer(l, i)),
                installing: installing.contains_key(&major),
                installed: inst,
                latest,
                major,
            }
        })
        .collect();
    let status = state.svc.status().await;
    let databases = match state.svc.client().await {
        Ok(c) if status.running => c.databases().await.unwrap_or_default(),
        _ => Vec::new(),
    };
    let app_state = app.state::<AppState>();
    let (port, selected) = {
        let c = app_state.config.lock().unwrap();
        (c.pg_port, c.pg_version.clone())
    };
    Ok(PgOverview { rows, catalog_error, status, port, selected, databases })
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
pub async fn pg_install(app: AppHandle, major: String) -> CmdResult<()> {
    let state = app.state::<PgState>();
    let entry = catalog(&state, false)
        .await?
        .into_iter()
        .find(|e| e.major == major)
        .ok_or_else(|| t!("o PostgreSQL {major} não está no catálogo", major = major))?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut i = state.installing.lock().unwrap();
        if i.contains_key(&major) {
            return Err(t!("já está instalando"));
        }
        i.insert(major.clone(), cancel.clone());
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let paths = app.state::<AppState>().paths.clone();
        let emitter = app.clone();
        let m = major.clone();
        let last = Mutex::new(Instant::now() - Duration::from_secs(1));
        let on_progress = move |received: u64, total: Option<u64>| {
            let mut l = last.lock().unwrap();
            if l.elapsed() >= Duration::from_millis(100) {
                *l = Instant::now();
                let _ = emitter.emit("pg:progress", Progress { major: m.clone(), received, total });
            }
        };
        let result = async {
            let client = http_client()?;
            postgres::install(&client, &paths, &entry, &cancel, &on_progress).await?;
            // O Adminer serve também para o PostgreSQL.
            let ports = crate::services::maria_ports(&app);
            let pg = pg_port(&app);
            if let Err(e) = sentinel_runtimes::extras::ensure_adminer(&client, &paths, &ports, &[pg]).await {
                tracing::warn!("Adminer não baixado: {e}");
            }
            Ok::<(), sentinel_runtimes::Error>(())
        }
        .await;
        app.state::<PgState>().installing.lock().unwrap().remove(&major);
        let _ = app.emit("pg:done", Done { major, ok: result.is_ok(), error: result.err().map(|e| e.to_string()) });
        let _ = app.emit("services-changed", ());
    });
    Ok(())
}

#[tauri::command]
pub async fn pg_remove(app: AppHandle, major: String) -> CmdResult<()> {
    let st = app.state::<PgState>().svc.status().await;
    if st.running && st.version.as_deref().is_some_and(|v| v.split('.').next() == Some(major.as_str())) {
        return Err(t!("desligue o PostgreSQL antes de remover esta versão"));
    }
    let paths = app.state::<AppState>().paths.clone();
    for m in postgres::list_installed(&postgres::postgres_dir(&paths)).into_iter().filter(|m| m.major == major) {
        std::fs::remove_dir_all(&m.dir).map_err(|e| e.to_string())?;
    }
    // Os dados (data\postgres-<major>) ficam: reinstalar recupera os bancos.
    let _ = app.emit("services-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn pg_start(app: AppHandle) -> CmdResult<()> {
    start_pg(&app).await
}

#[tauri::command]
pub async fn pg_stop(app: AppHandle) -> CmdResult<()> {
    app.state::<PgState>().svc.stop().await;
    let _ = app.emit("services-changed", ());
    crate::server::after_change(&app, false);
    Ok(())
}

#[tauri::command]
pub async fn pg_configure(app: AppHandle, version: Option<String>, port: u16) -> CmdResult<()> {
    if port < 1024 {
        return Err(t!("use uma porta acima de 1024 (ex.: 5432 ou 5433)"));
    }
    if crate::services::maria_ports(&app).contains(&port) {
        return Err(t!("a porta {port} já é usada por um MariaDB do Sentinel", port = port));
    }
    {
        let state = app.state::<AppState>();
        let mut c = state.config.lock().unwrap();
        c.pg_version = version;
        c.pg_port = port;
        c.save(&state.paths.config_file()).map_err(|e| e.to_string())?;
        let _ = app.emit("config-changed", &*c);
    }
    if pg_running(&app).await {
        start_pg(&app).await?;
    }
    crate::services::refresh_adminer(&app);
    let _ = app.emit("services-changed", ());
    Ok(())
}
