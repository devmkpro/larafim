//! Arquivos de apoio baixados uma vez: certificados raiz (para o PHP falar
//! HTTPS) e o Composer.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sentinel_core::Paths;

use crate::download::{download_verified, fetch_text};
use crate::error::Result;

const CACERT_URL: &str = "https://curl.se/ca/cacert.pem";
const CACERT_SHA_URL: &str = "https://curl.se/ca/cacert.pem.sha256";
const COMPOSER_URL: &str = "https://getcomposer.org/download/latest-stable/composer.phar";
const COMPOSER_SHA_URL: &str = "https://getcomposer.org/download/latest-stable/composer.phar.sha256";

/// Onde fica o pacote de certificados usado por `curl.cainfo`/`openssl.cafile`.
pub fn cacert_path(paths: &Paths) -> PathBuf {
    paths.runtimes_dir().join("cacert.pem")
}

pub fn composer_path(paths: &Paths) -> PathBuf {
    paths.runtimes_dir().join("composer").join("composer.phar")
}

/// O arquivo de hash vem como `<hash>  <nome>` ou só `<hash>`.
fn first_token(text: &str) -> String {
    text.split_whitespace().next().unwrap_or_default().to_string()
}

async fn ensure_file(client: &reqwest::Client, url: &str, sha_url: &str, dest: &Path) -> Result<()> {
    if dest.is_file() {
        return Ok(());
    }
    let sha = first_token(&fetch_text(client, sha_url).await?);
    let never = AtomicBool::new(false);
    download_verified(client, url, &sha, dest, &never, &|_, _| {}).await
}

pub async fn ensure_cacert(client: &reqwest::Client, paths: &Paths) -> Result<PathBuf> {
    let dest = cacert_path(paths);
    ensure_file(client, CACERT_URL, CACERT_SHA_URL, &dest).await?;
    Ok(dest)
}

pub async fn ensure_composer(client: &reqwest::Client, paths: &Paths) -> Result<PathBuf> {
    let dest = composer_path(paths);
    ensure_file(client, COMPOSER_URL, COMPOSER_SHA_URL, &dest).await?;
    Ok(dest)
}

// ---------- Adminer (gerenciar bancos pelo navegador) ----------

/// Versão fixada e conferida (SHA-256 publicado no GitHub do Adminer).
const ADMINER_VERSION: &str = "6.3.1";
const ADMINER_URL: &str = "https://github.com/vrana/adminer/releases/download/v6.3.1/adminer-6.3.1.php";
const ADMINER_SHA256: &str = "a9aed04e4e68c4e46283804a69ee70cad4fbf23203b46470b760868b2fd9bd76";

pub fn adminer_dir(paths: &Paths) -> PathBuf {
    paths.runtimes_dir().join("adminer")
}

pub fn adminer_installed(paths: &Paths) -> bool {
    adminer_dir(paths).join("adminer.php").is_file()
}

/// Página de entrada. Nos bancos do Sentinel (MariaDB compartilhado e
/// próprios de projetos com root sem senha; PostgreSQL com postgres sem
/// senha; todos só em 127.0.0.1) o login é automático; em qualquer outro
/// servidor o Adminer pede usuário e senha normalmente.
/// `ports[0]` é o MariaDB compartilhado (padrão).
fn adminer_index(ports: &[u16], pg_ports: &[u16]) -> String {
    let default = ports.first().copied().unwrap_or(3306);
    let quote = |list: &[u16]| list.iter().map(|p| format!("'127.0.0.1:{p}'")).collect::<Vec<_>>().join(", ");
    let (list, pg_list) = (quote(ports), quote(pg_ports));
    format!(
        r#"<?php
// Gerado pelo Sentinel. Adminer {ADMINER_VERSION} com login automático nos bancos do Sentinel.
const SENTINEL_SERVERS = [{list}];
const SENTINEL_PG = [{pg_list}];
// Usuário do Sentinel para este servidor, ou null se não for do Sentinel.
function sentinel_user($driver, $server) {{
    if ($driver === 'pgsql') return in_array($server, SENTINEL_PG, true) ? 'postgres' : null;
    if ($driver === 'server') return in_array($server, SENTINEL_SERVERS, true) ? 'root' : null;
    return null;
}}
function sentinel_target() {{
    return isset($_GET['pgsql']) ? ['pgsql', $_GET['pgsql']] : ['server', $_GET['server'] ?? ''];
}}
function adminer_object() {{
    class SentinelAdminer extends \Adminer\Adminer {{
        function name() {{ return 'Sentinel · Bancos'; }}
        function credentials() {{
            [$driver, $server] = sentinel_target();
            $user = sentinel_user($driver, $server);
            return $user !== null ? [$server, $user, ''] : parent::credentials();
        }}
        function login($login, $password) {{
            [$driver, $server] = sentinel_target();
            return sentinel_user($driver, $server) !== null ? true : parent::login($login, $password);
        }}
        // Login automático (só nos servidores do Sentinel) dispensa o token do formulário.
        function verifyLoginToken() {{
            $auth = $_POST['auth'] ?? [];
            return sentinel_user($auth['driver'] ?? '', $auth['server'] ?? '') === null;
        }}
    }}
    return new SentinelAdminer;
}}
if (!isset($_GET['username']) && empty($_POST['auth'])) {{
    if (isset($_GET['pg']) && ctype_digit($_GET['pg'])) {{
        $sentinelAuth = ['pgsql', '127.0.0.1:' . $_GET['pg']];
    }} else {{
        $sentinelPort = (isset($_GET['port']) && ctype_digit($_GET['port'])) ? $_GET['port'] : '{default}';
        $sentinelAuth = ['server', '127.0.0.1:' . $sentinelPort];
    }}
    $sentinelUser = sentinel_user($sentinelAuth[0], $sentinelAuth[1]);
    if ($sentinelUser !== null) {{
        $_POST['auth'] = ['driver' => $sentinelAuth[0], 'server' => $sentinelAuth[1], 'username' => $sentinelUser, 'password' => '', 'db' => $_GET['db'] ?? ''];
    }}
}}
include __DIR__ . '/adminer.php';
"#
    )
}

/// Baixa o Adminer (uma vez) e (re)escreve a página de entrada.
pub async fn ensure_adminer(client: &reqwest::Client, paths: &Paths, ports: &[u16], pg_ports: &[u16]) -> Result<PathBuf> {
    let dir = adminer_dir(paths);
    let file = dir.join("adminer.php");
    if !file.is_file() || crate::download::sha256_file(&file).ok().as_deref() != Some(ADMINER_SHA256) {
        let never = AtomicBool::new(false);
        download_verified(client, ADMINER_URL, ADMINER_SHA256, &file, &never, &|_, _| {}).await?;
    }
    write_adminer_index(paths, ports, pg_ports)?;
    Ok(dir)
}

pub fn write_adminer_index(paths: &Paths, ports: &[u16], pg_ports: &[u16]) -> Result<()> {
    if !adminer_installed(paths) {
        return Ok(());
    }
    let index = adminer_dir(paths).join("index.php");
    std::fs::write(&index, adminer_index(ports, pg_ports)).map_err(|e| crate::Error::io(&index, e))
}

#[cfg(test)]
mod tests {
    #[test]
    fn adminer_index_lists_sentinel_servers() {
        let php = super::adminer_index(&[3307, 3310], &[5432]);
        assert!(php.contains("const SENTINEL_SERVERS = ['127.0.0.1:3307', '127.0.0.1:3310'];"));
        assert!(php.contains("const SENTINEL_PG = ['127.0.0.1:5432'];"));
        assert!(php.contains(": '3307';"));
        assert!(php.contains("parent::credentials()"));
        assert!(php.contains("include __DIR__ . '/adminer.php';"));
    }

    use super::*;

    #[test]
    fn hash_file_formats() {
        assert_eq!(first_token("abc123  cacert.pem\n"), "abc123");
        assert_eq!(first_token("abc123\n"), "abc123");
    }
}
