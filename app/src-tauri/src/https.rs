//! HTTPS local (fase 9): CA do Sentinel (restrita ao domínio local), chave
//! guardada com DPAPI, e o certificado dela no repositório "Autoridades
//! raiz confiáveis" do usuário — só quando a pessoa pede (o Windows mostra
//! um aviso de segurança para ela confirmar).

use sentinel_core::t;
use std::path::PathBuf;
use std::sync::Arc;

use sentinel_server::tls::{self, CaPem, SniResolver};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

fn ca_dir(app: &AppHandle) -> PathBuf {
    app.state::<AppState>().paths.data_dir().join("ca")
}

/// Uma CA por domínio local (a restrição de nome vale só para ele).
fn ca_files(app: &AppHandle, tld: &str) -> (PathBuf, PathBuf) {
    let dir = ca_dir(app);
    (dir.join(format!("sentinel-ca-{tld}.crt")), dir.join(format!("sentinel-ca-{tld}.key.dpapi")))
}

fn computer_name() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "este PC".into())
}

/// Lê a CA ou cria na primeira vez.
pub fn load_or_create_ca(app: &AppHandle, tld: &str) -> Result<CaPem, String> {
    let (crt, key) = ca_files(app, tld);
    if let (Ok(cert), Ok(enc)) = (std::fs::read_to_string(&crt), std::fs::read_to_string(&key)) {
        return Ok(CaPem { cert, key: crate::secret::unprotect(enc.trim())? });
    }
    let ca = tls::generate_ca(tld, &computer_name()).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(ca_dir(app)).map_err(|e| e.to_string())?;
    std::fs::write(&key, crate::secret::protect(&ca.key)?).map_err(|e| e.to_string())?;
    std::fs::write(&crt, &ca.cert).map_err(|e| e.to_string())?;
    tracing::info!(tld, "CA local criada");
    Ok(ca)
}

/// Configuração TLS do gateway para o domínio local atual.
pub fn server_config(app: &AppHandle) -> Result<Arc<tls::ServerConfig>, String> {
    let tld = app.state::<AppState>().config.lock().unwrap().tld.clone();
    let ca = load_or_create_ca(app, &tld)?;
    let resolver = SniResolver::new(&ca, &tld).map_err(|e| e.to_string())?;
    Ok(tls::server_config(Arc::new(resolver)))
}

// ---------- Repositório de certificados do Windows (usuário atual) ----------

#[cfg(windows)]
mod store {
    use windows_sys::Win32::Security::Cryptography::*;

    fn open_root() -> Result<HCERTSTORE, String> {
        let name: Vec<u16> = "Root".encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: nome terminado em zero; o handle é fechado por quem chama.
        let store = unsafe { CertOpenStore(CERT_STORE_PROV_SYSTEM_W, 0, 0, CERT_SYSTEM_STORE_CURRENT_USER, name.as_ptr().cast()) };
        if store.is_null() { Err(sentinel_core::t!("não foi possível abrir os certificados do usuário")) } else { Ok(store) }
    }

    /// Procura o certificado (mesmos bytes) e chama `f` com o contexto.
    fn find<T>(der: &[u8], f: impl FnOnce(HCERTSTORE, *const CERT_CONTEXT) -> T) -> Result<Option<T>, String> {
        let store = open_root()?;
        let mut ctx: *const CERT_CONTEXT = std::ptr::null();
        let mut found = None;
        loop {
            // SAFETY: enumeração padrão; o contexto anterior é liberado pela própria API.
            ctx = unsafe { CertEnumCertificatesInStore(store, ctx) };
            if ctx.is_null() {
                break;
            }
            // SAFETY: ctx válido enquanto a enumeração não avança.
            let bytes = unsafe { std::slice::from_raw_parts((*ctx).pbCertEncoded, (*ctx).cbCertEncoded as usize) };
            if bytes == der {
                found = Some(f(store, ctx));
                // SAFETY: encerra a enumeração liberando o contexto atual.
                unsafe { CertFreeCertificateContext(ctx) };
                break;
            }
        }
        // SAFETY: handle aberto acima.
        unsafe { CertCloseStore(store, 0) };
        Ok(found)
    }

    pub fn contains(der: &[u8]) -> bool {
        matches!(find(der, |_, _| ()), Ok(Some(())))
    }

    /// O Windows mostra o aviso de segurança; se a pessoa recusar, falha.
    pub fn add(der: &[u8]) -> Result<(), String> {
        let store = open_root()?;
        // SAFETY: buffer DER válido.
        let ok = unsafe {
            CertAddEncodedCertificateToStore(store, X509_ASN_ENCODING, der.as_ptr(), der.len() as u32, CERT_STORE_ADD_REPLACE_EXISTING, std::ptr::null_mut())
        };
        // SAFETY: handle aberto acima.
        unsafe { CertCloseStore(store, 0) };
        if ok != 0 { Ok(()) } else { Err(sentinel_core::t!("o certificado não foi instalado (o aviso do Windows foi recusado?)")) }
    }

    pub fn remove(der: &[u8]) -> Result<(), String> {
        let r = find(der, |_, ctx| {
            // SAFETY: a exclusão consome uma cópia do contexto.
            unsafe { CertDeleteCertificateFromStore(CertDuplicateCertificateContext(ctx)) != 0 }
        })?;
        match r {
            Some(true) | None => Ok(()),
            Some(false) => Err(sentinel_core::t!("o certificado não foi removido (o aviso do Windows foi recusado?)")),
        }
    }
}

#[cfg(not(windows))]
mod store {
    pub fn contains(_: &[u8]) -> bool {
        false
    }
    pub fn add(_: &[u8]) -> Result<(), String> {
        Err(t!("só no Windows"))
    }
    pub fn remove(_: &[u8]) -> Result<(), String> {
        Ok(())
    }
}

fn current_ca_der(app: &AppHandle) -> Result<Vec<u8>, String> {
    let tld = app.state::<AppState>().config.lock().unwrap().tld.clone();
    let ca = load_or_create_ca(app, &tld)?;
    tls::ca_der(&ca.cert).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct HttpsStatus {
    enabled: bool,
    port: u16,
    /// A CA está nas "Autoridades raiz confiáveis" do usuário.
    trusted: bool,
    ca_file: Option<String>,
}

#[tauri::command]
pub fn https_status(app: AppHandle) -> HttpsStatus {
    let (enabled, port, tld) = {
        let c = app.state::<AppState>().config.lock().unwrap().clone();
        (c.https, c.https_port, c.tld.clone())
    };
    let (crt, _) = ca_files(&app, &tld);
    let trusted = std::fs::read_to_string(&crt)
        .ok()
        .and_then(|pem| tls::ca_der(&pem).ok())
        .is_some_and(|der| store::contains(&der));
    HttpsStatus { enabled, port, trusted, ca_file: crt.is_file().then(|| crt.display().to_string()) }
}

/// Liga/desliga o HTTPS e troca a porta. Cria a CA se precisar.
#[tauri::command]
pub async fn https_configure(app: AppHandle, enabled: bool, port: u16) -> CmdResult<()> {
    if port == 0 {
        return Err(t!("porta inválida"));
    }
    {
        let c = app.state::<AppState>().config.lock().unwrap().clone();
        if port == c.http_port {
            return Err(t!("a porta {port} já é a do HTTP", port = port));
        }
    }
    if enabled {
        let tld = app.state::<AppState>().config.lock().unwrap().tld.clone();
        load_or_create_ca(&app, &tld)?;
    }
    {
        let state = app.state::<AppState>();
        let mut c = state.config.lock().unwrap();
        c.https = enabled;
        c.https_port = port;
        c.save(&state.paths.config_file()).map_err(|e| e.to_string())?;
        let _ = app.emit("config-changed", &*c);
    }
    crate::server::refresh(&app).await.map_err(|e| e.message)?;
    Ok(())
}

/// Instala a CA nas raízes confiáveis do usuário (o Windows pede confirmação).
#[tauri::command]
pub async fn https_trust(app: AppHandle) -> CmdResult<()> {
    let der = current_ca_der(&app)?;
    tauri::async_runtime::spawn_blocking(move || store::add(&der)).await.map_err(|e| e.to_string())??;
    tracing::info!("CA local instalada no repositório do usuário");
    Ok(())
}

#[tauri::command]
pub async fn https_untrust(app: AppHandle) -> CmdResult<()> {
    let der = current_ca_der(&app)?;
    tauri::async_runtime::spawn_blocking(move || store::remove(&der)).await.map_err(|e| e.to_string())??;
    tracing::info!("CA local removida do repositório do usuário");
    Ok(())
}

/// Abre a pasta da CA (para importar em navegadores com repositório próprio, como o Firefox).
#[tauri::command]
pub fn https_open_ca_folder(app: AppHandle) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let dir = ca_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener().open_path(dir.display().to_string(), None::<&str>).map_err(|e| e.to_string())
}
