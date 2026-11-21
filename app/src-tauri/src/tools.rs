//! Ferramentas (fase 7): Mailpit, logs do projeto, workers do Laravel,
//! Xdebug e criação de projeto Laravel novo.

use sentinel_core::t;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sentinel_core::envfile::{self, DiffLine, EnvChange};
use sentinel_core::store::same_path;
use sentinel_core::{ProjectKind, ProjectView};
use sentinel_runtimes::php::{self as rphp};
use sentinel_runtimes::{http_client, tools};
use sentinel_server::DevSpec;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;
use crate::server::ServerState;

type CmdResult<T> = Result<T, String>;

pub const MAILPIT_KEY: &str = "service:mailpit";
const MAIL_UI_PORT: u16 = 8025;
const MAIL_SMTP_PORT: u16 = 1025;

#[derive(Default)]
pub struct ToolsState {
    /// Criações de projeto em andamento: chave do processo → dados.
    creating: Mutex<HashMap<String, CreateJob>>,
}

#[derive(Clone)]
struct CreateJob {
    path: PathBuf,
    php: String,
    database: String,
}

fn view_of(app: &AppHandle, path: &Path) -> Result<ProjectView, String> {
    crate::server::views(&app.state::<AppState>())
        .into_iter()
        .find(|v| same_path(&v.path, path))
        .ok_or_else(|| t!("projeto não encontrado"))
}

fn notify_projects(app: &AppHandle) {
    let state = app.state::<AppState>();
    let roots = state.config.lock().unwrap().roots.clone();
    let store = state.projects.lock().unwrap();
    let _ = app.emit("projects-changed", serde_json::json!({ "projects": store.views(&roots), "ignored": store.ignored }));
}

pub(crate) fn update_project(app: &AppHandle, path: &Path, f: impl FnOnce(&mut sentinel_core::ProjectConfig)) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut store = state.projects.lock().unwrap();
        let p = store
            .projects
            .iter_mut()
            .find(|p| same_path(&p.path, path))
            .ok_or_else(|| t!("importe o projeto antes"))?;
        f(p);
        store.save(&state.paths.projects_file()).map_err(|e| e.to_string())?;
    }
    notify_projects(app);
    Ok(())
}

// ================= Mailpit =================

#[derive(Serialize)]
pub struct MailpitStatus {
    installed: bool,
    version: Option<String>,
    running: bool,
    ui_url: String,
    smtp: String,
}

pub fn mail_url(app: &AppHandle) -> String {
    let c = app.state::<AppState>().config.lock().unwrap().clone();
    if c.http_port == 80 { format!("http://mail.{}", c.tld) } else { format!("http://mail.{}:{}", c.tld, c.http_port) }
}

pub fn mailpit_running(app: &AppHandle) -> bool {
    app.state::<ServerState>().dev.is_running(MAILPIT_KEY)
}

#[tauri::command]
pub fn mailpit_status(app: AppHandle) -> MailpitStatus {
    let paths = app.state::<AppState>().paths.clone();
    MailpitStatus {
        installed: tools::mailpit_installed(&paths),
        version: std::fs::read_to_string(tools::mailpit_dir(&paths).join("VERSION")).ok().map(|v| v.trim().to_string()),
        running: mailpit_running(&app),
        ui_url: mail_url(&app),
        smtp: format!("127.0.0.1:{MAIL_SMTP_PORT}"),
    }
}

#[tauri::command]
pub async fn mailpit_install(app: AppHandle) -> CmdResult<()> {
    let paths = app.state::<AppState>().paths.clone();
    let client = http_client().map_err(|e| e.to_string())?;
    tools::install_mailpit(&client, &paths).await.map_err(|e| e.to_string())?;
    let _ = app.emit("services-changed", ());
    Ok(())
}

/// Liga o Mailpit (SMTP 1025, caixa de entrada em mail.<domínio>).
pub async fn start_mailpit(app: &AppHandle) -> Result<(), String> {
    if mailpit_running(app) {
        return Ok(());
    }
    let paths = app.state::<AppState>().paths.clone();
    if !tools::mailpit_installed(&paths) {
        let client = http_client().map_err(|e| e.to_string())?;
        tools::install_mailpit(&client, &paths).await.map_err(|e| e.to_string())?;
    }
    for port in [MAIL_UI_PORT, MAIL_SMTP_PORT] {
        if let Err(owner) = sentinel_server::port::check_free(port) {
            let who = owner.map(|o| o.name).unwrap_or_else(|| t!("outro programa"));
            return Err(t!("a porta {port} (Mailpit) está ocupada por {who}", port = port, who = who));
        }
    }
    let data = paths.data_dir().join("mailpit");
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let exe = tools::mailpit_exe(&paths);
    let command = format!(
        "\"{}\" --listen 127.0.0.1:{MAIL_UI_PORT} --smtp 127.0.0.1:{MAIL_SMTP_PORT} --database \"{}\"",
        exe.display(),
        data.join("mailpit.db").display()
    );
    app.state::<ServerState>()
        .dev
        .start(DevSpec {
            key: MAILPIT_KEY.into(),
            cwd: tools::mailpit_dir(&paths),
            command,
            node_dir: tools::mailpit_dir(&paths),
            env: vec![("PORT".into(), MAIL_UI_PORT.to_string())],
        })
        .map_err(|e| e.to_string())?;
    crate::server::after_change(app, false);
    let _ = app.emit("services-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn mailpit_start(app: AppHandle) -> CmdResult<()> {
    start_mailpit(&app).await
}

#[tauri::command]
pub async fn mailpit_stop(app: AppHandle) {
    app.state::<ServerState>().dev.stop(MAILPIT_KEY);
    crate::server::after_change(&app, false);
    let _ = app.emit("services-changed", ());
}

#[tauri::command]
pub async fn open_mailpit(app: AppHandle) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    start_mailpit(&app).await?;
    crate::server::ensure_hosts(&app).await.map_err(|e| e.message)?;
    crate::server::refresh(&app).await.map_err(|e| e.message)?;
    app.opener().open_url(mail_url(&app), None::<&str>).map_err(|e| e.to_string())
}

fn mail_changes(kind: &str) -> Vec<EnvChange> {
    match kind {
        "mailpit" => vec![
            EnvChange::set("MAIL_MAILER", "smtp"),
            EnvChange::set("MAIL_HOST", "127.0.0.1"),
            EnvChange::set("MAIL_PORT", &MAIL_SMTP_PORT.to_string()),
            EnvChange::set("MAIL_USERNAME", "null"),
            EnvChange::set("MAIL_PASSWORD", "null"),
        ],
        _ => Vec::new(),
    }
}

#[tauri::command]
pub fn project_mail_preview(path: PathBuf, kind: String) -> CmdResult<Vec<DiffLine>> {
    let base = std::fs::read_to_string(envfile::env_path(&path)).unwrap_or_default();
    Ok(envfile::apply(&base, &mail_changes(&kind)).1)
}

/// Liga o projeto ao Mailpit (ajusta o .env com cópia) ou desvincula.
#[tauri::command]
pub async fn project_mail_apply(app: AppHandle, path: PathBuf, kind: String) -> CmdResult<()> {
    if kind == "mailpit" {
        start_mailpit(&app).await?;
        if envfile::env_path(&path).exists() {
            envfile::write(&path, &mail_changes(&kind)).map_err(|e| e.to_string())?;
        }
    } else if kind != "none" {
        return Err(t!("serviço de e-mail desconhecido: {kind}", kind = kind));
    }
    update_project(&app, &path, |p| p.mail = (kind == "mailpit").then_some(kind.clone()))
}

// ================= Logs =================

#[derive(Serialize)]
pub struct LogFile {
    name: String,
    size: u64,
    modified: u64,
}

/// Pasta de logs do projeto: `storage\logs` (Laravel) ou `logs`.
fn log_dir(project: &Path) -> PathBuf {
    let laravel = project.join("storage").join("logs");
    if laravel.is_dir() { laravel } else { project.join("logs") }
}

fn safe_log(project: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || name.contains("..") || !name.ends_with(".log") {
        return Err(t!("arquivo de log inválido"));
    }
    Ok(log_dir(project).join(name))
}

#[tauri::command]
pub fn log_files(path: PathBuf) -> Vec<LogFile> {
    let mut list: Vec<LogFile> = std::fs::read_dir(log_dir(&path))
        .map(|r| {
            r.flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "log"))
                .filter_map(|e| {
                    let m = e.metadata().ok()?;
                    Some(LogFile {
                        name: e.file_name().to_string_lossy().to_string(),
                        size: m.len(),
                        modified: m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    list.sort_by_key(|f| std::cmp::Reverse(f.modified));
    list
}

#[derive(Serialize)]
pub struct LogChunk {
    text: String,
    offset: u64,
    /// O arquivo encolheu (foi limpo ou trocado): o texto recomeça.
    reset: bool,
}

/// Lê o que entrou no log desde `offset`. Na primeira leitura (sem
/// offset) traz só o final (até 256 KB), para não pesar.
#[tauri::command]
pub fn log_read(path: PathBuf, name: String, offset: Option<u64>) -> CmdResult<LogChunk> {
    const FIRST: u64 = 256 * 1024;
    let file = safe_log(&path, &name)?;
    let mut f = std::fs::File::open(&file).map_err(|e| e.to_string())?;
    let size = f.metadata().map_err(|e| e.to_string())?.len();
    let (start, reset) = match offset {
        Some(o) if o <= size => (o, false),
        Some(_) => (size.saturating_sub(FIRST), true),
        None => (size.saturating_sub(FIRST), true),
    };
    let len = (size - start).min(4 * 1024 * 1024);
    f.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; len as usize];
    f.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(LogChunk { text: String::from_utf8_lossy(&buf).to_string(), offset: start + len, reset })
}

/// Esvazia o arquivo de log (a pessoa confirma antes).
#[tauri::command]
pub fn log_clear(path: PathBuf, name: String) -> CmdResult<()> {
    let file = safe_log(&path, &name)?;
    std::fs::write(&file, b"").map_err(|e| e.to_string())
}

#[tauri::command]
pub fn log_open(app: AppHandle, path: PathBuf, name: String) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let file = safe_log(&path, &name)?;
    app.opener().open_path(file.display().to_string(), None::<&str>).map_err(|e| e.to_string())
}

// ================= Workers do Laravel =================

pub fn worker_key(path: &Path, worker: &str) -> String {
    format!("{}|{worker}", path.display().to_string().to_lowercase())
}

fn worker_command(worker: &str) -> Option<&'static str> {
    match worker {
        "queue" => Some("artisan queue:work --tries=3"),
        "schedule" => Some("artisan schedule:work"),
        "reverb" => Some("artisan reverb:start"),
        _ => None,
    }
}

/// Sobe um worker com o PHP do projeto.
pub fn start_worker(app: &AppHandle, view: &ProjectView, worker: &str) -> Result<(), String> {
    let args = worker_command(worker).ok_or_else(|| t!("worker desconhecido"))?;
    if view.kind != ProjectKind::Laravel {
        return Err(t!("workers só existem em projetos Laravel"));
    }
    let installed = rphp::list_installed(&app.state::<AppState>().paths.php_dir());
    let (php, _) = crate::server::php_for(view, &installed);
    let php = php.ok_or_else(|| t!("nenhum PHP instalado atende a este projeto"))?;
    let exe = php.dir.join("php.exe");
    app.state::<ServerState>()
        .dev
        .start(DevSpec {
            key: worker_key(&view.path, worker),
            cwd: view.path.clone(),
            command: format!("\"{}\" {args}", exe.display()),
            node_dir: php.dir.clone(),
            env: Vec::new(),
        })
        .map_err(|e| e.to_string())
}

pub fn stop_workers(app: &AppHandle, path: &Path) {
    for w in ["queue", "schedule", "reverb"] {
        app.state::<ServerState>().dev.stop(&worker_key(path, w));
    }
}

/// Liga os workers marcados de um projeto (ao ficar online).
pub fn start_workers(app: &AppHandle, view: &ProjectView) {
    for w in &view.workers {
        if let Err(e) = start_worker(app, view, w) {
            tracing::warn!(project = view.name, worker = w, "worker não iniciou: {e}");
        }
    }
}

// Comandos que iniciam processos precisam ser `async`: o gerenciador de
// processos usa o runtime assíncrono (num comando síncrono o app cairia).

/// Marca/desmarca um worker. Se o projeto estiver online, liga/desliga na hora.
#[tauri::command]
pub async fn worker_set(app: AppHandle, path: PathBuf, worker: String, enabled: bool) -> CmdResult<()> {
    worker_command(&worker).ok_or_else(|| t!("worker desconhecido"))?;
    update_project(&app, &path, |p| {
        p.workers.retain(|w| w != &worker);
        if enabled {
            p.workers.push(worker.clone());
        }
    })?;
    let view = view_of(&app, &path)?;
    if enabled && view.online {
        start_worker(&app, &view, &worker)?;
    } else if !enabled {
        app.state::<ServerState>().dev.stop(&worker_key(&path, &worker));
    }
    Ok(())
}

#[tauri::command]
pub async fn worker_restart(app: AppHandle, path: PathBuf, worker: String) -> CmdResult<()> {
    let view = view_of(&app, &path)?;
    start_worker(&app, &view, &worker)
}

/// Status e saída de qualquer processo gerenciado, pela chave.
#[tauri::command]
pub fn process_info(app: AppHandle, key: String) -> crate::server::DevInfo {
    crate::server::dev_info_for(&app, &key)
}

/// Projeto tem Reverb instalado?
#[tauri::command]
pub fn has_reverb(path: PathBuf) -> bool {
    path.join("config").join("reverb.php").is_file()
}

// ================= Xdebug =================

#[derive(Serialize)]
pub struct XdebugInfo {
    downloaded: bool,
    enabled: bool,
    launch_json: String,
}

fn installed_php(app: &AppHandle, version: &str) -> Result<rphp::InstalledPhp, String> {
    rphp::list_installed(&app.state::<AppState>().paths.php_dir())
        .into_iter()
        .find(|p| p.version == version)
        .ok_or_else(|| t!("o PHP {version} não está instalado", version = version))
}

#[tauri::command]
pub fn xdebug_info(app: AppHandle, version: String) -> CmdResult<XdebugInfo> {
    let php = installed_php(&app, &version)?;
    let s = tools::xdebug_state(&php);
    Ok(XdebugInfo { downloaded: s.downloaded, enabled: s.enabled, launch_json: tools::vscode_launch(None) })
}

/// Liga/desliga o Xdebug numa versão do PHP. Os processos do PHP dessa
/// versão são reiniciados para valer na próxima requisição.
#[tauri::command]
pub async fn xdebug_set(app: AppHandle, version: String, enabled: bool) -> CmdResult<XdebugInfo> {
    let php = installed_php(&app, &version)?;
    if enabled && !tools::xdebug_state(&php).downloaded {
        let client = http_client().map_err(|e| e.to_string())?;
        tools::ensure_xdebug(&client, &php).await.map_err(|e| e.to_string())?;
    }
    tools::set_xdebug(&php, enabled).map_err(|e| e.to_string())?;
    app.state::<ServerState>().restart_php().await;
    tracing::info!(version, enabled, "Xdebug alterado");
    xdebug_info(app, version)
}

// ================= Projeto Laravel novo =================

#[derive(Deserialize)]
pub struct NewLaravel {
    name: String,
    root: PathBuf,
    php: String,
    /// "none", "react", "vue", "livewire"
    kit: String,
    /// "sqlite" ou "mariadb"
    database: String,
}

fn kit_package(kit: &str) -> Option<&'static str> {
    match kit {
        "none" => Some("laravel/laravel"),
        "react" => Some("laravel/react-starter-kit"),
        "vue" => Some("laravel/vue-starter-kit"),
        "livewire" => Some("laravel/livewire-starter-kit"),
        _ => None,
    }
}

/// Cria o projeto com `composer create-project` (e `npm install` + build
/// nos starter kits). A saída aparece ao vivo; no fim o projeto é importado.
#[tauri::command]
pub async fn laravel_create(app: AppHandle, input: NewLaravel) -> CmdResult<String> {
    let name = input.name.trim().to_lowercase();
    let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(t!("use só letras, números, - e _ no nome"));
    }
    let package = kit_package(&input.kit).ok_or_else(|| t!("starter kit desconhecido"))?;
    if !input.root.is_dir() {
        return Err(t!("a pasta escolhida não existe"));
    }
    let target = input.root.join(&name);
    if target.exists() {
        return Err(t!("já existe uma pasta {name} ali", name = name));
    }
    let paths = app.state::<AppState>().paths.clone();
    if !rphp::list_installed(&paths.php_dir()).iter().any(|p| p.minor == input.php) {
        return Err(t!("o PHP {a} não está instalado", a = input.php));
    }
    if !sentinel_runtimes::extras::composer_path(&paths).is_file() {
        return Err(t!("o Composer ainda não foi baixado (instale um PHP pelo Sentinel)"));
    }
    if input.kit != "none" && sentinel_runtimes::node::list_installed(&paths.node_dir()).is_empty() {
        return Err(t!("os starter kits precisam do Node.js: instale em Configurações › Node.js"));
    }
    // Os atalhos (php, composer, npm) escolhem as versões; SENTINEL_PHP fixa o PHP.
    crate::terminal::ensure_shims(&app)?;
    let mut command = format!("composer create-project {package} \"{name}\" --no-interaction --prefer-dist");
    if input.kit != "none" {
        command.push_str(&format!(" && cd \"{name}\" && npm install && npm run build"));
    }
    let key = format!("create:{}", target.display().to_string().to_lowercase());
    app.state::<ToolsState>().creating.lock().unwrap().insert(
        key.clone(),
        CreateJob { path: target.clone(), php: input.php.clone(), database: input.database.clone() },
    );
    app.state::<ServerState>()
        .dev
        .start(DevSpec {
            key: key.clone(),
            cwd: input.root.clone(),
            command,
            node_dir: paths.bin_dir(),
            env: vec![("SENTINEL_PHP".into(), input.php.clone()), ("COMPOSER_NO_INTERACTION".into(), "1".into())],
        })
        .map_err(|e| e.to_string())?;
    tracing::info!(name, kit = input.kit, "criando projeto Laravel");
    Ok(key)
}

#[derive(Serialize, Clone)]
struct Created {
    key: String,
    path: PathBuf,
    ok: bool,
    error: Option<String>,
}

/// Chamado quando um processo termina: se era uma criação, importa o projeto.
pub fn on_process_exit(app: &AppHandle, key: &str, code: Option<i32>) {
    let Some(job) = app.state::<ToolsState>().creating.lock().unwrap().remove(key) else { return };
    let app = app.clone();
    let key = key.to_string();
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            if code != Some(0) || !job.path.join("artisan").is_file() {
                return Err(match code {
                    Some(c) => t!("a criação falhou (código {c}). Veja a saída.", c = c),
                    None => t!("a criação foi interrompida. Veja a saída."),
                });
            }
            {
                let state = app.state::<AppState>();
                let mut store = state.projects.lock().unwrap();
                if store.find(&job.path).is_none() {
                    store.import(&job.path).map_err(|e| e.to_string())?;
                }
                if let Some(p) = store.projects.iter_mut().find(|p| same_path(&p.path, &job.path)) {
                    p.php = Some(job.php.clone());
                }
                store.save(&state.paths.projects_file()).map_err(|e| e.to_string())?;
            }
            notify_projects(&app);
            if job.database == "mariadb" {
                crate::services::apply_db(&app, &job.path, "mariadb").await?;
            }
            Ok(())
        }
        .await;
        let _ = app.emit(
            "laravel:created",
            Created { key, path: job.path.clone(), ok: result.is_ok(), error: result.err() },
        );
    });
}
