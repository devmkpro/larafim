//! Ícone na bandeja do Windows: o lugar das ações do dia a dia
//! (abrir um projeto online, desligar tudo, configurações).

use tauri::{
    AppHandle, Wry,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use sentinel_core::t;
use tauri_plugin_opener::OpenerExt;

use crate::server::OnlineSite;

const TRAY_ID: &str = "sentinel";
const OPEN: &str = "open";
const SETTINGS: &str = "settings";
const LOGS: &str = "logs";
const STOP_ALL: &str = "stop-all";
const QUIT: &str = "quit";
const URL_PREFIX: &str = "url:";

fn build_menu(app: &AppHandle, online: &[OnlineSite]) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, OPEN, t!("Abrir Sentinel"), true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    if online.is_empty() {
        menu.append(&MenuItem::with_id(app, "none", t!("Nenhum projeto online"), false, None::<&str>)?)?;
    } else {
        for site in online {
            let label = site.url.trim_start_matches("http://").trim_start_matches("https://");
            menu.append(&MenuItem::with_id(app, format!("{URL_PREFIX}{}", site.url), label, true, None::<&str>)?)?;
        }
        menu.append(&MenuItem::with_id(app, STOP_ALL, t!("Desligar todos os projetos"), true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, LOGS, t!("Visualizador de logs"), true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, SETTINGS, t!("Configurações…"), true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, QUIT, t!("Sair do Sentinel"), true, None::<&str>)?)?;
    Ok(menu)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, &[])?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Sentinel")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            match id {
                OPEN => crate::show_main_window(app),
                SETTINGS => crate::show_settings_window(app, "general"),
                LOGS => crate::show_logs_window(app, None),
                STOP_ALL => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move { crate::server::stop_all(&app).await });
                }
                QUIT => {
                    tracing::info!("saindo pelo menu da bandeja");
                    app.exit(0);
                }
                _ => {
                    if let Some(url) = id.strip_prefix(URL_PREFIX) {
                        let _ = app.opener().open_url(url, None::<&str>);
                    }
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, position, .. } = event {
                crate::tray_panel::toggle(tray.app_handle(), position);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Reconstrói o menu com os projetos online e atualiza a dica do ícone.
pub fn update_online(app: &AppHandle, online: &[OnlineSite]) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    if let Ok(menu) = build_menu(app, online) {
        let _ = tray.set_menu(Some(menu));
    }
    *ONLINE.lock().unwrap() = online.len();
    refresh_tooltip(app);
}

/// Quantos projetos estão online e se há versão nova (para a dica do ícone).
static ONLINE: std::sync::Mutex<usize> = std::sync::Mutex::new(0);
static UPDATE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn refresh_tooltip(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let mut tip = match *ONLINE.lock().unwrap() {
        0 => "Sentinel".to_string(),
        1 => t!("Sentinel — 1 projeto online"),
        n => t!("Sentinel — {n} projetos online", n = n),
    };
    if let Some(v) = UPDATE.lock().unwrap().as_deref() {
        tip.push('\n');
        tip.push_str(&t!("Versão {v} disponível", v = v));
    }
    let _ = tray.set_tooltip(Some(tip));
}

/// Mostra na dica do ícone que há versão nova.
pub fn update_tooltip_for_update(app: &AppHandle, status: &crate::updates::UpdateStatus) {
    *UPDATE.lock().unwrap() = status.available.then(|| status.latest.clone()).flatten();
    refresh_tooltip(app);
}
