//! Terminal e linha de comando (fase 4): atalhos `php`/`composer` em
//! `%LOCALAPPDATA%\Sentinel\bin`, PATH do usuário e abrir terminal no projeto.

use std::path::{Path, PathBuf};

use sentinel_runtimes::php::select::{Selection, select};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

const SHIM_FILE: &str = "sentinel-shim.exe";
const TOOLS: [&str; 5] = ["php.exe", "composer.exe", "node.exe", "npm.exe", "npx.exe"];

/// Onde está o executável do atalho que vem com o Sentinel.
fn shim_source(app: &AppHandle) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join(SHIM_FILE));
    }
    if let Ok(res) = app.path().resource_dir() {
        candidates.push(res.join(SHIM_FILE));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::read(a), std::fs::read(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Copia o atalho como php.exe/composer.exe. `force` = mesmo se iguais.
fn install(app: &AppHandle, bin: &Path) -> CmdResult<()> {
    let source = shim_source(app).ok_or("o atalho (sentinel-shim.exe) não foi encontrado junto do Sentinel")?;
    std::fs::create_dir_all(bin).map_err(|e| e.to_string())?;
    for tool in TOOLS {
        let dest = bin.join(tool);
        if dest.is_file() && same_file(&source, &dest) {
            continue;
        }
        // Se estiver em uso (um `php` rodando agora), tenta trocar o nome antes.
        if dest.is_file() && std::fs::remove_file(&dest).is_err() {
            let old = dest.with_extension("old");
            let _ = std::fs::remove_file(&old);
            std::fs::rename(&dest, &old).map_err(|e| format!("{tool} está em uso: {e}"))?;
        }
        std::fs::copy(&source, &dest).map_err(|e| format!("não foi possível criar {tool}: {e}"))?;
    }
    tracing::info!(bin = %bin.display(), "atalhos php/composer instalados");
    Ok(())
}

// ---------- PATH do usuário (HKCU\Environment) ----------

fn norm(p: &str) -> String {
    p.trim().trim_end_matches('\\').to_lowercase()
}

/// Coloca `dir` no começo da lista do PATH (sem duplicar).
pub fn path_with(value: &str, dir: &str) -> String {
    let mut parts: Vec<&str> = value.split(';').filter(|p| !p.trim().is_empty() && norm(p) != norm(dir)).collect();
    parts.insert(0, dir);
    parts.join(";")
}

/// Tira `dir` da lista do PATH.
pub fn path_without(value: &str, dir: &str) -> String {
    value
        .split(';')
        .filter(|p| !p.trim().is_empty() && norm(p) != norm(dir))
        .collect::<Vec<_>>()
        .join(";")
}

fn path_contains(value: &str, dir: &str) -> bool {
    value.split(';').any(|p| norm(p) == norm(dir))
}

/// Expande `%VARIAVEL%` (os valores do registro vêm sem expandir).
fn expand(entry: &str) -> String {
    let mut out = String::new();
    let mut rest = entry;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                out.push_str(&std::env::var(name).unwrap_or_else(|_| format!("%{name}%")));
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(windows)]
mod reg {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, RegType};

    pub fn user_path() -> String {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("Environment")
            .and_then(|k| k.get_value::<String, _>("Path"))
            .unwrap_or_default()
    }

    pub fn system_path() -> String {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment", KEY_READ)
            .and_then(|k| k.get_value::<String, _>("Path"))
            .unwrap_or_default()
    }

    /// Grava o PATH do usuário mantendo o tipo REG_EXPAND_SZ e avisa o Windows.
    pub fn set_user_path(value: &str) -> std::io::Result<()> {
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
        let mut data: Vec<u8> = value.encode_utf16().chain(std::iter::once(0)).flat_map(|c| c.to_le_bytes()).collect();
        if data.is_empty() {
            data = vec![0, 0];
        }
        key.set_raw_value("Path", &winreg::RegValue { bytes: data.into(), vtype: RegType::REG_EXPAND_SZ })?;
        broadcast();
        Ok(())
    }

    /// Avisa os programas abertos (Explorer etc.) que o ambiente mudou:
    /// terminais novos já enxergam o PATH atualizado.
    fn broadcast() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE};
        let env: Vec<u16> = "Environment".encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: string terminada em zero viva durante a chamada.
        unsafe {
            SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, env.as_ptr() as isize, SMTO_ABORTIFHUNG, 3000, std::ptr::null_mut());
        }
    }
}

#[derive(Serialize)]
pub struct TerminalStatus {
    bin_dir: PathBuf,
    /// php.exe e composer.exe existem em bin.
    installed: bool,
    /// O atalho que vem com o Sentinel foi encontrado (para instalar/reparar).
    source_found: bool,
    /// bin está no PATH do usuário.
    in_path: bool,
    /// Outros php.exe que o Windows encontra ANTES do Sentinel.
    conflicts: Vec<String>,
    default_php: Option<String>,
}

fn status(app: &AppHandle, state: &AppState) -> TerminalStatus {
    let bin = state.paths.bin_dir();
    let bin_s = bin.display().to_string();
    let user = reg::user_path();
    let in_path = path_contains(&user, &bin_s);

    // Ordem real do Windows: PATH do sistema, depois o do usuário.
    let mut conflicts = Vec::new();
    let effective = format!("{};{}", reg::system_path(), if in_path { user.clone() } else { path_with(&user, &bin_s) });
    for entry in effective.split(';').filter(|e| !e.trim().is_empty()) {
        let dir = expand(entry);
        if norm(&dir) == norm(&bin_s) {
            break;
        }
        let php = Path::new(&dir).join("php.exe");
        if php.is_file() {
            conflicts.push(php.display().to_string());
        }
    }

    TerminalStatus {
        installed: TOOLS.iter().all(|t| bin.join(t).is_file()),
        source_found: shim_source(app).is_some(),
        in_path,
        conflicts,
        default_php: state.config.lock().map(|c| c.default_php.clone()).unwrap_or(None),
        bin_dir: bin,
    }
}

#[tauri::command]
pub fn terminal_status(app: AppHandle, state: State<'_, AppState>) -> TerminalStatus {
    status(&app, &state)
}

#[tauri::command]
pub fn install_shims(app: AppHandle, state: State<'_, AppState>) -> CmdResult<TerminalStatus> {
    install(&app, &state.paths.bin_dir())?;
    Ok(status(&app, &state))
}

/// Liga/desliga os atalhos no PATH do usuário (não precisa de administrador).
#[tauri::command]
pub fn set_shims_in_path(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> CmdResult<TerminalStatus> {
    let bin = state.paths.bin_dir();
    if enabled {
        install(&app, &bin)?;
    }
    let bin_s = bin.display().to_string();
    let current = reg::user_path();
    let updated = if enabled { path_with(&current, &bin_s) } else { path_without(&current, &bin_s) };
    if updated != current {
        reg::set_user_path(&updated).map_err(|e| format!("não foi possível alterar o PATH: {e}"))?;
        tracing::info!(enabled, "PATH do usuário alterado");
    }
    Ok(status(&app, &state))
}

/// Qual PHP o terminal usaria numa pasta (mesma regra dos atalhos).
#[tauri::command]
pub fn php_for_folder(state: State<'_, AppState>, path: PathBuf) -> Selection {
    select(&state.paths, &path)
}

/// Abre o Windows Terminal (ou o Prompt de Comando) na pasta, já com os
/// atalhos do Sentinel no começo do PATH — funciona mesmo sem mexer no PATH.
#[tauri::command]
pub fn open_terminal(app: AppHandle, state: State<'_, AppState>, path: PathBuf) -> CmdResult<()> {
    if !path.is_dir() {
        return Err(format!("a pasta {} não existe", path.display()));
    }
    let bin = state.paths.bin_dir();
    install(&app, &bin)?;
    let env_path = format!("{};{}", bin.display(), std::env::var("PATH").unwrap_or_default());

    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    const DETACHED: u32 = 0x0000_0008;

    let wt = std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join(r"Microsoft\WindowsApps\wt.exe"));
    let result = match wt.filter(|w| w.is_file()) {
        Some(wt) => std::process::Command::new(wt)
            .arg("-d")
            .arg(&path)
            .env("PATH", &env_path)
            .creation_flags(DETACHED)
            .spawn(),
        None => std::process::Command::new("cmd.exe")
            .arg("/K")
            .arg("echo Terminal do Sentinel: php e composer usam a versao do projeto.")
            .current_dir(&path)
            .env("PATH", &env_path)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn(),
    };
    result.map(|_| ()).map_err(|e| format!("não foi possível abrir o terminal: {e}"))
}

/// Ao abrir o Sentinel: se os atalhos já existem, atualiza para a versão
/// que veio com este Sentinel (sem perguntar; não mexe no PATH).
pub fn refresh_on_start(app: &AppHandle) {
    let state = app.state::<AppState>();
    let bin = state.paths.bin_dir();
    if TOOLS.iter().any(|t| bin.join(t).is_file())
        && let Err(e) = install(app, &bin)
    {
        tracing::warn!("atalhos não atualizados: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_edits() {
        let v = r"C:\laragon\bin\php;%USERPROFILE%\bin;C:\Tools\";
        let added = path_with(v, r"C:\S\bin");
        assert!(added.starts_with(r"C:\S\bin;C:\laragon"));
        let again = path_with(&added, r"c:\s\bin\");
        assert_eq!(again.split(';').count(), added.split(';').count(), "não duplica");
        assert_eq!(path_without(&added, r"C:\S\bin"), v.trim_end_matches(';'));
        assert!(path_contains(&added, r"c:\S\BIN"));
        assert_eq!(path_with("", r"C:\S\bin"), r"C:\S\bin");
    }

    #[test]
    fn expands_variables() {
        // SAFETY: teste de uma thread só, variável exclusiva deste teste.
        unsafe { std::env::set_var("SENTINEL_TESTE_X", r"C:\X") };
        assert_eq!(expand(r"%SENTINEL_TESTE_X%\bin"), r"C:\X\bin");
        assert_eq!(expand("%NAO_EXISTE_123%"), "%NAO_EXISTE_123%");
        assert_eq!(expand("sem%"), "sem%");
    }
}
