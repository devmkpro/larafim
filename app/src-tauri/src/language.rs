//! Idioma do app: escolhido no instalador (gravado no registro), depois
//! trocado em Configurações › Geral. Sem escolha, segue o Windows.

use sentinel_core::i18n;

/// Onde o instalador grava o idioma escolhido (HKCU\Software\Sentinel\Language).
const REG_KEY: &str = r"Software\Sentinel";
const REG_VALUE: &str = "Language";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn installer_choice() -> Option<String> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    let (key, value) = (wide(REG_KEY), wide(REG_VALUE));
    let mut buf = vec![0u16; 64];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY: buffers válidos; tamanho em bytes.
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut size)
    };
    if r != 0 {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1);
    let code = String::from_utf16_lossy(&buf[..len]).trim().to_string();
    i18n::is_supported(&code).then_some(code)
}

/// Idioma da interface do Windows (não o formato regional: dá para ter o
/// Windows em português com datas e números no formato en-US).
#[cfg(windows)]
fn system_locale() -> Option<String> {
    use windows_sys::Win32::Globalization::{GetUserDefaultUILanguage, LCIDToLocaleName};
    let mut buf = [0u16; 85];
    // SAFETY: LANGID do idioma da interface; buffer com o tamanho máximo de um nome de localidade.
    let n = unsafe { LCIDToLocaleName(u32::from(GetUserDefaultUILanguage()), buf.as_mut_ptr(), buf.len() as i32, 0) };
    (n > 1).then(|| String::from_utf16_lossy(&buf[..(n - 1) as usize]))
}

#[cfg(not(windows))]
fn installer_choice() -> Option<String> {
    None
}
#[cfg(not(windows))]
fn system_locale() -> Option<String> {
    std::env::var("LANG").ok()
}

/// Idioma a usar: o da configuração; senão o do instalador; senão o do Windows.
/// Devolve também se foi decidido agora (para gravar na configuração).
pub fn resolve(configured: Option<&str>) -> (String, bool) {
    if let Some(code) = configured.filter(|c| i18n::is_supported(c)) {
        return (code.to_string(), false);
    }
    let code = installer_choice().unwrap_or_else(|| i18n::best_match(&system_locale().unwrap_or_default()));
    (code, true)
}
