//! App desktop do Sentinel (Tauri 2).
//!
//! Leveza: janelas são **destruídas** ao fechar, liberando o WebView da
//! memória. Com "continuar na bandeja" ligado, o processo segue vivo só com o
//! ícone da bandeja e recria a janela quando a pessoa pede.

mod commands;
mod php;
mod projects;
mod server;
mod terminal;
mod tray;

use std::sync::Mutex;

use sentinel_core::{AppConfig, Paths, ProjectsFile, Theme};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

pub const MAIN_WINDOW: &str = "main";
pub const SETTINGS_WINDOW: &str = "settings";

/// Argumentos do WebView2. Os 3 primeiros são os padrões do Tauri
/// (substituídos ao definir args). Sem GPU e com um só processo de
/// renderização a interface gasta bem menos memória.
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
    --disable-gpu --disable-gpu-compositing --renderer-process-limit=1 \
    --disable-background-networking --disable-component-update --no-pings";

/// Estado compartilhado do app.
pub struct AppState {
    pub paths: Paths,
    pub config: Mutex<AppConfig>,
    pub projects: Mutex<ProjectsFile>,
}

pub fn run() {
    let paths = Paths::default_root().expect("pasta de dados indisponível");
    paths.ensure().expect("não foi possível criar a pasta de dados do Sentinel");
    let _log_guard = sentinel_core::logging::init(&paths.logs_dir());

    let config = AppConfig::load(&paths.config_file()).unwrap_or_else(|e| {
        tracing::error!("configuração ilegível, usando padrão: {e}");
        AppConfig::default()
    });
    let projects = ProjectsFile::load(&paths.projects_file()).unwrap_or_else(|e| {
        tracing::error!("projects.toml ilegível, começando vazio: {e}");
        ProjectsFile::default()
    });
    tracing::info!(version = env!("CARGO_PKG_VERSION"), root = %paths.root().display(), "Sentinel iniciando");

    let app = tauri::Builder::default()
        // Abrir o Sentinel de novo só traz a janela existente para frente.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { paths, config: Mutex::new(config), projects: Mutex::new(projects) })
        .manage(php::PhpState::default())
        .manage(server::ServerState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::app_info,
            commands::open_data_dir,
            commands::open_folder,
            commands::open_settings,
            projects::list_projects,
            projects::import_project,
            projects::import_all,
            projects::save_project,
            projects::remove_project,
            projects::ignore_project,
            projects::unignore_project,
            projects::inspect_folder,
            projects::project_docroots,
            php::php_overview,
            php::php_install,
            php::php_cancel,
            php::php_remove,
            php::php_extensions,
            php::php_set_extension,
            php::php_open_ini,
            php::php_open_folder,
            php::php_match,
            php::open_vc_redist,
            server::set_project_online,
            server::server_status,
            server::stop_all_projects,
            server::fix_hosts,
            server::open_url,
            terminal::terminal_status,
            terminal::install_shims,
            terminal::set_shims_in_path,
            terminal::php_for_folder,
            terminal::open_terminal,
        ])
        .setup(|app| {
            tray::create(app.handle())?;
            php::start_update_checker(app.handle().clone());
            server::start(app.handle());
            terminal::refresh_on_start(app.handle());
            show_main_window(app.handle());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Sentinel");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            server::shutdown(app);
            return;
        }
        if let RunEvent::ExitRequested { code, api, .. } = event {
            // `code == None`: a última janela foi fechada (não foi um "Sair").
            if code.is_none() && keep_in_tray(app) {
                api.prevent_exit();
                tracing::info!("janelas fechadas; Sentinel continua na bandeja");
            }
        }
    });
}

fn keep_in_tray(app: &AppHandle) -> bool {
    app.state::<AppState>().config.lock().map(|c| c.keep_in_tray).unwrap_or(true)
}

fn native_theme(app: &AppHandle) -> Option<tauri::Theme> {
    match app.state::<AppState>().config.lock().map(|c| c.theme) {
        Ok(Theme::Dark) => Some(tauri::Theme::Dark),
        Ok(Theme::Light) => Some(tauri::Theme::Light),
        _ => None,
    }
}

/// Traz uma janela para frente se ela existir. Devolve `false` se não existir.
fn focus_existing(app: &AppHandle, label: &str) -> bool {
    match app.get_webview_window(label) {
        Some(window) => {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
            true
        }
        None => false,
    }
}

/// Janela sem moldura do Windows: a barra de título é desenhada pela interface.
fn window_builder<'a>(
    app: &'a AppHandle,
    label: &'a str,
    title: &str,
    size: (f64, f64),
    min: (f64, f64),
) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .title(title)
        .inner_size(size.0, size.1)
        .min_inner_size(min.0, min.1)
        .decorations(false)
        .shadow(true)
        .center()
        .theme(native_theme(app))
        .additional_browser_args(BROWSER_ARGS)
}

/// Mostra a janela principal, criando-a se tiver sido descarregada.
pub fn show_main_window(app: &AppHandle) {
    if focus_existing(app, MAIN_WINDOW) {
        return;
    }
    let result = window_builder(app, MAIN_WINDOW, "Sentinel", (1120.0, 720.0), (860.0, 520.0)).build();
    if let Err(e) = result {
        tracing::error!("não foi possível abrir a janela principal: {e}");
    }
}

/// Abre (ou traz para frente) a janela de Configurações numa seção.
pub fn show_settings_window(app: &AppHandle, section: &str) {
    if focus_existing(app, SETTINGS_WINDOW) {
        let _ = app.emit_to(SETTINGS_WINDOW, "settings:navigate", section);
        return;
    }
    // A seção inicial é entregue antes da página carregar.
    let init = format!(
        "window.__SENTINEL_SECTION__ = {};",
        serde_json::to_string(section).unwrap_or_else(|_| "\"general\"".into())
    );
    let result = window_builder(app, SETTINGS_WINDOW, "Configurações — Sentinel", (860.0, 620.0), (680.0, 460.0))
        .initialization_script(&init)
        .build();
    if let Err(e) = result {
        tracing::error!("não foi possível abrir as configurações: {e}");
    }
}
