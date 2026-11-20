//! Ícone na bandeja do Windows: o lugar das ações do dia a dia
//! (abrir um projeto online, desligar tudo, configurações).

use tauri::{
    AppHandle, Wry,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_opener::OpenerExt;

use crate::server::OnlineSite;

const TRAY_ID: &str = "sentinel";
const OPEN: &str = "open";
const SETTINGS: &str = "settings";
const STOP_ALL: &str = "stop-all";
const QUIT: &str = "quit";
const URL_PREFIX: &str = "url:";

fn build_menu(app: &AppHandle, online: &[OnlineSite]) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, OPEN, "Abrir Sentinel", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    if online.is_empty() {
        menu.append(&MenuItem::with_id(app, "none", "Nenhum projeto online", false, None::<&str>)?)?;
    } else {
        for site in online {
            let label = site.url.trim_start_matches("http://");
            menu.append(&MenuItem::with_id(app, format!("{URL_PREFIX}{}", site.url), label, true, None::<&str>)?)?;
        }
        menu.append(&MenuItem::with_id(app, STOP_ALL, "Desligar todos os projetos", true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, SETTINGS, "Configurações…", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, QUIT, "Sair do Sentinel", true, None::<&str>)?)?;
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
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main_window(tray.app_handle());
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
    let tip = match online.len() {
        0 => "Sentinel".to_string(),
        1 => "Sentinel — 1 projeto online".to_string(),
        n => format!("Sentinel — {n} projetos online"),
    };
    let _ = tray.set_tooltip(Some(tip));
}
