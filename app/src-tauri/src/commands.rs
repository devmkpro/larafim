//! Comandos chamados pela interface (`invoke`). Erros voltam como texto
//! legível para mostrar na tela.

use sentinel_core::t;
use std::path::PathBuf;

use sentinel_core::AppConfig;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::AppState;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> CmdResult<AppConfig> {
    state.config.lock().map(|c| c.clone()).map_err(|e| e.to_string())
}

/// Grava a configuração e avisa todas as janelas (tema, domínio etc.).
#[tauri::command]
pub fn save_config(app: AppHandle, state: State<'_, AppState>, config: AppConfig) -> CmdResult<AppConfig> {
    let mut config = config;
    config.tld = config.tld.trim().trim_start_matches('.').to_lowercase();
    config.roots.dedup();
    let (was, old_language) = {
        let c = state.config.lock().map_err(|e| e.to_string())?;
        (c.start_with_windows, c.language.clone())
    };
    if let Some(lang) = config.language.as_deref()
        && !sentinel_core::i18n::is_supported(lang)
    {
        return Err(sentinel_core::t!("idioma não suportado: {lang}", lang = lang));
    }
    if was != config.start_with_windows {
        crate::autostart::set(config.start_with_windows)?;
    }

    config.save(&state.paths.config_file()).map_err(|e| e.to_string())?;
    let previous = std::mem::replace(&mut *state.config.lock().map_err(|e| e.to_string())?, config.clone());
    // Domínio ou porta mudaram: rotas, gateway e hosts acompanham.
    if previous.tld != config.tld || previous.http_port != config.http_port || previous.roots != config.roots {
        crate::server::after_change(&app, previous.tld != config.tld);
    }
    if old_language != config.language {
        crate::language_changed(&app, config.language.as_deref().unwrap_or(sentinel_core::i18n::SOURCE));
    }
    tracing::info!("configuração salva");
    let _ = app.emit("config-changed", &config);
    Ok(config)
}

#[derive(Serialize)]
pub struct AppInfo {
    version: &'static str,
    data_dir: String,
    logs_dir: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        data_dir: state.paths.root().display().to_string(),
        logs_dir: state.paths.logs_dir().display().to_string(),
    }
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    app.opener()
        .open_path(state.paths.root().display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Abre uma pasta no Explorador. Só aceita pastas que existem (nunca arquivos,
/// para não executar nada).
#[tauri::command]
pub fn open_folder(app: AppHandle, path: PathBuf) -> CmdResult<()> {
    if !path.is_dir() {
        return Err(t!("a pasta {a} não existe", a = path.display()));
    }
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Precisa ser `async`: no Windows, criar janela num comando síncrono trava o
/// WebView (a janela fica em branco).
#[tauri::command]
pub async fn open_logs(app: AppHandle, project: Option<String>) {
    crate::show_logs_window(&app, project.as_deref());
}

/// Abre as Configurações numa seção (também `async`).
#[tauri::command]
pub async fn open_settings(app: AppHandle, section: Option<String>) {
    crate::show_settings_window(&app, section.as_deref().unwrap_or("general"));
}
