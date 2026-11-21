//! Quem está usando uma porta TCP (ex.: 80 ocupada pelo Apache do Laragon).

use sentinel_core::t;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortOwner {
    pub pid: u32,
    /// Nome do executável (`httpd.exe`), ou uma descrição.
    pub name: String,
    /// Dica para a pessoa (Laragon, IIS…), quando reconhecido.
    pub hint: Option<String>,
}

fn hint_for(name: &str, pid: u32) -> Option<String> {
    let n = name.to_lowercase();
    let hint = if pid == 4 {
        t!("serviço do Windows (IIS / HTTP.sys)")
    } else if n.contains("httpd") || n.contains("apache") {
        t!("Apache — provavelmente do Laragon ou XAMPP")
    } else if n.contains("nginx") {
        t!("Nginx — provavelmente do Laragon")
    } else if n.contains("laragon") {
        "Laragon".to_string()
    } else if n.contains("skype") {
        "Skype".to_string()
    } else if n.contains("herd") {
        "Laravel Herd".to_string()
    } else if n.contains("docker") || n.contains("wslrelay") || n.contains("vpnkit") {
        "Docker / WSL".to_string()
    } else if n.contains("mysqld") || n.contains("mariadb") {
        "MySQL/MariaDB".to_string()
    } else {
        return None;
    };
    Some(hint.into())
}

/// Processo escutando na porta (IPv4), se houver.
#[cfg(windows)]
pub fn listener_on(port: u16) -> Option<PortOwner> {
    // IPv4 e IPv6: muitos servidores (MySQL 8, por exemplo) escutam só em "::".
    let pid = listener_pid_v4(port).or_else(|| listener_pid_v6(port))?;
    let name = process_name(pid)
        .or_else(|| snapshot_name(pid))
        .unwrap_or_else(|| if pid == 4 { "System".into() } else { format!("PID {pid}") });
    Some(PortOwner { hint: hint_for(&name, pid), pid, name })
}

/// Nome pelo instantâneo de processos: funciona até para serviços do
/// Windows, cujo caminho não dá para ler sem ser administrador.
#[cfg(windows)]
fn snapshot_name(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    // SAFETY: snapshot fechado no fim; estrutura com dwSize preenchido.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = None;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                if entry.th32ProcessID == pid {
                    let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                    found = Some(String::from_utf16_lossy(&entry.szExeFile[..len]));
                    break;
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        found
    }
}

/// Lê a tabela de portas em escuta (duas chamadas: tamanho, depois dados).
#[cfg(windows)]
fn listener_table(family: u32) -> Option<Vec<u32>> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{GetExtendedTcpTable, TCP_TABLE_OWNER_PID_LISTENER};
    // SAFETY: buffer alinhado a 4 bytes com o tamanho informado pelo sistema.
    unsafe {
        let mut size = 0u32;
        GetExtendedTcpTable(std::ptr::null_mut(), &mut size, 0, family, TCP_TABLE_OWNER_PID_LISTENER, 0);
        let mut buf = vec![0u32; (size as usize).div_ceil(4) + 1];
        (GetExtendedTcpTable(buf.as_mut_ptr().cast(), &mut size, 0, family, TCP_TABLE_OWNER_PID_LISTENER, 0) == 0).then_some(buf)
    }
}

#[cfg(windows)]
fn listener_pid_v4(port: u16) -> Option<u32> {
    use windows_sys::Win32::NetworkManagement::IpHelper::MIB_TCPTABLE_OWNER_PID;
    let buf = listener_table(windows_sys::Win32::Networking::WinSock::AF_INET as u32)?;
    // SAFETY: a leitura respeita `dwNumEntries` da tabela devolvida.
    unsafe {
        let table = &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
        let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
        rows.iter().find(|r| u16::from_be(r.dwLocalPort as u16) == port).map(|r| r.dwOwningPid)
    }
}

#[cfg(windows)]
fn listener_pid_v6(port: u16) -> Option<u32> {
    use windows_sys::Win32::NetworkManagement::IpHelper::MIB_TCP6TABLE_OWNER_PID;
    let buf = listener_table(windows_sys::Win32::Networking::WinSock::AF_INET6 as u32)?;
    // SAFETY: idem.
    unsafe {
        let table = &*(buf.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID);
        let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
        rows.iter().find(|r| u16::from_be(r.dwLocalPort as u16) == port).map(|r| r.dwOwningPid)
    }
}

/// Caminho completo do executável de um processo.
#[cfg(windows)]
pub fn process_path(pid: u32) -> Option<std::path::PathBuf> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
    // SAFETY: handle verificado e fechado; buffer com tamanho informado.
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        (ok != 0).then(|| std::path::PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
    }
}

#[cfg(not(windows))]
pub fn process_path(_pid: u32) -> Option<std::path::PathBuf> {
    None
}

#[cfg(windows)]
fn process_name(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
    // SAFETY: handle verificado e fechado; buffer com tamanho informado.
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit('\\').next().map(String::from)
    }
}

#[cfg(not(windows))]
pub fn listener_on(_port: u16) -> Option<PortOwner> {
    None
}

/// A porta pode ser usada pelo Sentinel? (`Err` traz quem ocupa, se souber.)
pub fn check_free(port: u16) -> Result<(), Option<PortOwner>> {
    if let Some(owner) = listener_on(port) {
        if owner.pid == std::process::id() {
            return Ok(()); // é o próprio Sentinel
        }
        return Err(Some(owner));
    }
    // Reservas do HTTP.sys não aparecem na tabela; o teste de bind pega.
    match std::net::TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => Ok(()),
        Err(_) => Err(Some(PortOwner {
            pid: 4,
            name: "System".into(),
            hint: Some(t!("porta reservada pelo Windows (IIS / HTTP.sys) ou sem permissão")),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_our_own_listener() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let owner = listener_on(port).expect("deveria achar o processo de teste");
        assert_eq!(owner.pid, std::process::id());
        assert!(check_free(port).is_ok(), "o próprio processo não conta como conflito");
        drop(l);
        assert!(check_free(port).is_ok());
    }

    #[test]
    fn hints() {
        assert!(hint_for("httpd.exe", 100).unwrap().contains("Laragon"));
        assert!(hint_for("System", 4).unwrap().contains("IIS"));
        assert!(hint_for("chrome.exe", 100).is_none());
    }
}
