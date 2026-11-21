//! Storage S3 local (fase 8): RustFS (Apache 2.0), release oficial do GitHub
//! conferida pelo SHA-256. Também cria buckets com uma requisição S3
//! assinada (AWS Signature V4), sem SDK.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use sentinel_core::Paths;
use sha2::{Digest, Sha256};

use crate::download::{download_verified, fetch_text};
use crate::error::{Error, Result};

const API: &str = "https://api.github.com/repos/rustfs/rustfs/releases/latest";
const URL_PREFIX: &str = "https://github.com/rustfs/rustfs/";

pub fn rustfs_dir(paths: &Paths) -> PathBuf {
    paths.runtimes_dir().join("rustfs")
}

pub fn rustfs_exe(paths: &Paths) -> PathBuf {
    rustfs_dir(paths).join("rustfs.exe")
}

pub fn rustfs_installed(paths: &Paths) -> bool {
    rustfs_exe(paths).is_file()
}

/// (versão, url, sha256) da última versão estável.
pub fn parse_rustfs_release(json: &str) -> Result<(String, String, String)> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| Error::Catalog(e.to_string()))?;
    let tag = v["tag_name"].as_str().unwrap_or_default().to_string();
    let wanted = format!("rustfs-windows-x86_64-v{}.zip", tag.trim_start_matches('v'));
    let asset = v["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"] == wanted.as_str()))
        .ok_or_else(|| Error::Catalog("RustFS sem pacote para Windows nesta versão".into()))?;
    let url = asset["browser_download_url"].as_str().unwrap_or_default().to_string();
    let sha = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).unwrap_or_default().to_lowercase();
    if !url.starts_with(URL_PREFIX) || sha.len() != 64 || !sha.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::Catalog("dados do RustFS incompletos (sem SHA-256)".into()));
    }
    Ok((tag, url, sha))
}

pub async fn install_rustfs(
    client: &reqwest::Client,
    paths: &Paths,
    on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<String> {
    let (tag, url, sha) = parse_rustfs_release(&fetch_text(client, API).await?)?;
    let zip_path = paths.downloads_dir().join(format!("rustfs-{tag}.zip"));
    let never = AtomicBool::new(false);
    download_verified(client, &url, &sha, &zip_path, &never, on_progress).await?;
    let dir = rustfs_dir(paths);
    let (z, d) = (zip_path.clone(), dir.clone());
    tokio::task::spawn_blocking(move || -> Result<()> {
        std::fs::create_dir_all(&d).map_err(|e| Error::io(&d, e))?;
        let f = std::fs::File::open(&z).map_err(|e| Error::io(&z, e))?;
        let mut a = zip::ZipArchive::new(f).map_err(|e| Error::Zip(e.to_string()))?;
        // O executável pode estar na raiz ou numa subpasta do zip.
        let name = (0..a.len())
            .filter_map(|i| a.by_index(i).ok().and_then(|e| e.name().ok().map(|n| n.to_string())))
            .find(|n| n.rsplit('/').next() == Some("rustfs.exe") && !n.contains(".."))
            .ok_or_else(|| Error::Zip("o pacote não contém rustfs.exe".into()))?;
        let mut entry = a.by_name(&name).map_err(|e| Error::Zip(e.to_string()))?;
        let exe = d.join("rustfs.exe");
        let mut out = std::fs::File::create(&exe).map_err(|e| Error::io(&exe, e))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| Error::io(&exe, e))?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Zip(e.to_string()))??;
    let _ = std::fs::remove_file(&zip_path);
    std::fs::write(dir.join("VERSION"), &tag).map_err(|e| Error::io(&dir, e))?;
    tracing::info!(version = tag, "RustFS instalado");
    Ok(tag)
}

/// Nome de bucket válido (S3): 3–63 caracteres, minúsculas, números e `-`.
pub fn bucket_name(domain: &str) -> String {
    let mut s: String = domain
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    s = s.trim_matches('-').to_string();
    s.truncate(63);
    while s.len() < 3 {
        s.push('0');
    }
    s
}

// ---------- AWS Signature V4 (só o necessário para criar bucket) ----------

fn hmac(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(k.map(|b| b ^ 0x36));
    inner.update(data);
    let mut outer = Sha256::new();
    outer.update(k.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

/// Cabeçalho `Authorization` de uma requisição sem corpo.
/// `amz_date` = "AAAAMMDDTHHMMSSZ"; `host` inclui a porta.
pub fn sign_v4(method: &str, host: &str, path: &str, access: &str, secret: &str, amz_date: &str, region: &str) -> String {
    let date = &amz_date[..8];
    let empty_hash = hex::encode(Sha256::digest(b""));
    let canonical = format!(
        "{method}\n{path}\n\nhost:{host}\nx-amz-content-sha256:{empty_hash}\nx-amz-date:{amz_date}\n\nhost;x-amz-content-sha256;x-amz-date\n{empty_hash}"
    );
    let scope = format!("{date}/{region}/s3/aws4_request");
    let to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}", hex::encode(Sha256::digest(canonical.as_bytes())));
    let k_date = hmac(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, b"s3");
    let k_signing = hmac(&k_service, b"aws4_request");
    let signature = hex::encode(hmac(&k_signing, to_sign.as_bytes()));
    format!("AWS4-HMAC-SHA256 Credential={access}/{scope}, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature={signature}")
}

fn amz_now() -> String {
    // Data UTC sem depender de crate de datas.
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Algoritmo de Howard Hinnant (dias desde 1970 → data civil).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Cria o bucket (se já existir e for nosso, tudo bem).
pub async fn create_bucket(client: &reqwest::Client, port: u16, access: &str, secret: &str, bucket: &str) -> Result<()> {
    let host = format!("127.0.0.1:{port}");
    let path = format!("/{bucket}");
    let date = amz_now();
    let auth = sign_v4("PUT", &host, &path, access, secret, &date, "us-east-1");
    let resp = client
        .put(format!("http://{host}{path}"))
        .header("x-amz-date", &date)
        .header("x-amz-content-sha256", hex::encode(Sha256::digest(b"")))
        .header("authorization", auth)
        .send()
        .await
        .map_err(|e| Error::Catalog(format!("storage S3: {e}")))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().await.unwrap_or_default();
    if body.contains("BucketAlreadyOwnedByYou") {
        return Ok(());
    }
    Err(Error::Catalog(format!("o storage S3 recusou criar o bucket {bucket} ({status}): {}", body.chars().take(300).collect::<String>())))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_rfc4231_case2() {
        let mac = hmac(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(hex::encode(mac), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    }

    /// Exemplo de chave de assinatura da documentação da AWS (SigV4).
    #[test]
    fn signing_key_matches_aws_example() {
        let secret = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";
        let k = hmac(format!("AWS4{secret}").as_bytes(), b"20120215");
        let k = hmac(&k, b"us-east-1");
        let k = hmac(&k, b"iam");
        let k = hmac(&k, b"aws4_request");
        assert_eq!(hex::encode(k), "f4780e2d9f65fa895f9c67b32ce1baf0b0d8a43505a000a1a9e090d414db404d");
    }

    #[test]
    fn auth_header_shape() {
        let h = sign_v4("PUT", "127.0.0.1:9000", "/loja", "sentinel", "segredo123", "20261120T120000Z", "us-east-1");
        assert!(h.starts_with("AWS4-HMAC-SHA256 Credential=sentinel/20261120/us-east-1/s3/aws4_request, "));
        assert!(h.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
        assert_eq!(h.rsplit("Signature=").next().unwrap().len(), 64);
    }

    #[test]
    fn date_format() {
        let d = amz_now();
        assert_eq!(d.len(), 16);
        assert!(d.starts_with("20") && d.as_bytes()[8] == b'T' && d.ends_with('Z'));
    }

    #[test]
    fn buckets_and_release() {
        assert_eq!(bucket_name("Loja_Virtual"), "loja-virtual");
        assert_eq!(bucket_name("a"), "a00");
        let a = "A".repeat(64);
        let json = format!(
            r#"{{"tag_name":"1.0.6","assets":[
              {{"name":"rustfs-windows-x86_64-latest.zip","browser_download_url":"https://github.com/rustfs/rustfs/releases/download/1.0.6/rustfs-windows-x86_64-latest.zip","digest":"sha256:{a}"}},
              {{"name":"rustfs-windows-x86_64-v1.0.6.zip","browser_download_url":"https://github.com/rustfs/rustfs/releases/download/1.0.6/rustfs-windows-x86_64-v1.0.6.zip","digest":"sha256:{a}"}}]}}"#
        );
        let (tag, url, sha) = parse_rustfs_release(&json).unwrap();
        assert_eq!(tag, "1.0.6");
        assert!(url.ends_with("v1.0.6.zip"));
        assert_eq!(sha, "a".repeat(64));
    }
}
