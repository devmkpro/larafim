//! Senhas guardadas com a DPAPI do Windows: só o seu usuário, neste PC,
//! consegue abrir. No arquivo de configuração fica apenas o texto cifrado.

use sentinel_core::t;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptProtectData, CryptUnprotectData};

const UI_FORBIDDEN: u32 = 0x1; // CRYPTPROTECT_UI_FORBIDDEN

pub fn protect(plain: &str) -> Result<String, String> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    let mut data = plain.as_bytes().to_vec();
    let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_mut_ptr() };
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    // SAFETY: blobs válidos; a saída é liberada com LocalFree.
    unsafe {
        if CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), UI_FORBIDDEN, &mut output) == 0 {
            return Err(t!("não foi possível proteger a senha"));
        }
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as _);
        Ok(STANDARD.encode(bytes))
    }
}

pub fn unprotect(encoded: &str) -> Result<String, String> {
    if encoded.is_empty() {
        return Ok(String::new());
    }
    let mut data = STANDARD.decode(encoded).map_err(|_| t!("senha guardada ilegível"))?;
    let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_mut_ptr() };
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    // SAFETY: idem.
    unsafe {
        if CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), UI_FORBIDDEN, &mut output) == 0 {
            return Err(t!("não foi possível abrir a senha guardada (foi salva por outro usuário?)"));
        }
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as _);
        String::from_utf8(bytes).map_err(|_| t!("senha guardada ilegível"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn roundtrip() {
        let enc = super::protect("s3nh@ ção").unwrap();
        assert!(!enc.contains("s3nh"));
        assert_eq!(super::unprotect(&enc).unwrap(), "s3nh@ ção");
        assert_eq!(super::protect("").unwrap(), "");
        assert_eq!(super::unprotect("").unwrap(), "");
    }
}
