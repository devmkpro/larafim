//! Bancos que já existem no PC (MySQL/MariaDB do Laragon, XAMPP…):
//! cadastrar, testar, detectar e usar nos projetos.

use larafim_core::t;
use std::path::{Path, PathBuf};

use larafim_core::DbConnection;
use larafim_runtimes::mariadb;
use larafim_server::dbclient::{DbClient, has_client};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

/// Para a interface: sem a senha.
#[derive(Serialize)]
pub struct ConnectionInfo {
    id: String,
    name: String,
    kind: String,
    host: String,
    port: u16,
    user: String,
    has_password: bool,
    client_dir: Option<PathBuf>,
    /// Projetos que usam esta conexão.
    used_by: Vec<String>,
}

/// O que vem do formulário. `password: None` = manter a senha guardada.
#[derive(Deserialize)]
pub struct ConnectionInput {
    id: Option<String>,
    name: String,
    kind: String,
    host: String,
    port: u16,
    user: String,
    password: Option<String>,
    client_dir: Option<PathBuf>,
}

pub fn find(app: &AppHandle, id: &str) -> Result<DbConnection, String> {
    app.state::<AppState>()
        .config
        .lock()
        .unwrap()
        .db_connections
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .ok_or_else(|| t!("conexão \"{id}\" não encontrada", id = id))
}

/// Programas de linha de comando para falar com o servidor: os do próprio
/// servidor (MySQL) ou os do MariaDB do Larafim.
fn client_dir(app: &AppHandle, kind: &str, own: Option<&Path>) -> Option<PathBuf> {
    let larafim = mariadb::list_installed(&app.state::<AppState>().paths.mariadb_dir())
        .first()
        .map(|m| m.dir.join("bin"));
    let own = own.filter(|d| has_client(d)).map(Path::to_path_buf);
    let order = if kind == "mysql" { [own, larafim] } else { [larafim, own] };
    order.into_iter().flatten().find(|d| has_client(d))
}

pub fn client_for(app: &AppHandle, c: &DbConnection) -> Result<DbClient, String> {
    let bin_dir = client_dir(app, &c.kind, c.client_dir.as_deref()).ok_or_else(|| 
        t!("não achei os programas mysql/mysqldump desse servidor. Informe a pasta deles na conexão ou instale o MariaDB do Larafim (só para ter as ferramentas)."),
    )?;
    Ok(DbClient {
        bin_dir,
        host: c.host.clone(),
        port: c.port,
        user: c.user.clone(),
        password: crate::secret::unprotect(&c.password_protected)?,
    })
}

fn slug(name: &str) -> String {
    let s: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { t!("banco") } else { s }
}

fn to_connection(app: &AppHandle, input: ConnectionInput) -> Result<DbConnection, String> {
    let existing = input.id.as_deref().and_then(|id| find(app, id).ok());
    let password_protected = match (&input.password, &existing) {
        (Some(p), _) => crate::secret::protect(p)?,
        (None, Some(e)) => e.password_protected.clone(),
        (None, None) => String::new(),
    };
    let id = match input.id {
        Some(id) => id,
        None => {
            let base = slug(&input.name);
            let used: Vec<String> = app.state::<AppState>().config.lock().unwrap().db_connections.iter().map(|c| c.id.clone()).collect();
            (1..).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|c| !used.contains(c)).unwrap()
        }
    };
    let c = DbConnection {
        id,
        name: input.name.trim().to_string(),
        kind: input.kind,
        host: input.host.trim().to_string(),
        port: input.port,
        user: input.user.trim().to_string(),
        password_protected,
        client_dir: input.client_dir.filter(|d| !d.as_os_str().is_empty()),
    };
    c.validate().map_err(|e| e.to_string())?;
    Ok(c)
}

#[tauri::command]
pub fn db_connections(app: AppHandle) -> Vec<ConnectionInfo> {
    let state = app.state::<AppState>();
    let projects = state.projects.lock().unwrap().projects.clone();
    state
        .config
        .lock()
        .unwrap()
        .db_connections
        .iter()
        .map(|c| ConnectionInfo {
            used_by: projects
                .iter()
                .filter(|p| p.database.as_deref() == Some(&format!("external:{}", c.id)))
                .map(|p| p.domain.clone())
                .collect(),
            id: c.id.clone(),
            name: c.name.clone(),
            kind: c.kind.clone(),
            host: c.host.clone(),
            port: c.port,
            user: c.user.clone(),
            has_password: !c.password_protected.is_empty(),
            client_dir: c.client_dir.clone(),
        })
        .collect()
}

/// Testa sem salvar. Devolve a versão do servidor.
#[tauri::command]
pub async fn db_connection_test(app: AppHandle, input: ConnectionInput) -> CmdResult<String> {
    let c = to_connection(&app, input)?;
    client_for(&app, &c)?.version().await
}

#[tauri::command]
pub fn db_connection_save(app: AppHandle, input: ConnectionInput) -> CmdResult<String> {
    let c = to_connection(&app, input)?;
    let state = app.state::<AppState>();
    let mut cfg = state.config.lock().unwrap();
    match cfg.db_connections.iter_mut().find(|x| x.id == c.id) {
        Some(x) => *x = c.clone(),
        None => cfg.db_connections.push(c.clone()),
    }
    cfg.save(&state.paths.config_file()).map_err(|e| e.to_string())?;
    let _ = app.emit("config-changed", &*cfg);
    let _ = app.emit("services-changed", ());
    tracing::info!(id = c.id, "conexão de banco salva");
    Ok(c.id)
}

#[tauri::command]
pub fn db_connection_remove(app: AppHandle, id: String) -> CmdResult<()> {
    let state = app.state::<AppState>();
    let users: Vec<String> = state
        .projects
        .lock()
        .unwrap()
        .projects
        .iter()
        .filter(|p| p.database.as_deref() == Some(&format!("external:{id}")))
        .map(|p| p.domain.clone())
        .collect();
    if !users.is_empty() {
        return Err(t!("a conexão é usada por: {a}. Troque o banco desses projetos antes.", a = users.join(", ")));
    }
    let mut cfg = state.config.lock().unwrap();
    cfg.db_connections.retain(|c| c.id != id);
    cfg.save(&state.paths.config_file()).map_err(|e| e.to_string())?;
    let _ = app.emit("config-changed", &*cfg);
    let _ = app.emit("services-changed", ());
    Ok(())
}

/// Um banco encontrado rodando no PC.
#[derive(Serialize)]
pub struct DetectedDb {
    port: u16,
    kind: String,
    process: String,
    hint: Option<String>,
    client_dir: Option<PathBuf>,
    /// Já existe uma conexão para essa porta.
    known: bool,
}

/// Procura MySQL/MariaDB escutando nas portas comuns (3306–3310), sem contar
/// o MariaDB do próprio Larafim.
#[tauri::command]
pub async fn db_detect(app: AppHandle) -> Vec<DetectedDb> {
    let own_pid = app.state::<crate::services::ServicesState>().maria.status().await.pid;
    let known: Vec<u16> = app.state::<AppState>().config.lock().unwrap().db_connections.iter().map(|c| c.port).collect();
    (3306..=3310)
        .filter_map(|port| {
            let owner = larafim_server::port::listener_on(port)?;
            if Some(owner.pid) == own_pid {
                return None;
            }
            let name = owner.name.to_lowercase();
            let kind = if name.contains("mariadb") {
                "mariadb"
            } else if name.contains("mysqld") {
                "mysql"
            } else {
                return None;
            };
            let client_dir = larafim_server::port::process_path(owner.pid)
                .and_then(|p| p.parent().map(Path::to_path_buf))
                .filter(|d| has_client(d));
            Some(DetectedDb {
                port,
                kind: kind.into(),
                process: owner.name,
                hint: owner.hint,
                client_dir,
                known: known.contains(&port),
            })
        })
        .collect()
}
