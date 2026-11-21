//! Serviço PostgreSQL: um servidor local (127.0.0.1) iniciado sob demanda,
//! com configuração leve. Dados em `data\postgres-<major>`. Usuário
//! `postgres` sem senha (autenticação `trust`, só na máquina).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

pub use crate::dbclient::valid_db;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgSetup {
    pub version: String,
    pub install_dir: PathBuf,
    pub data_dir: PathBuf,
    pub port: u16,
}

impl PgSetup {
    fn bin(&self, tool: &str) -> PathBuf {
        self.install_dir.join("bin").join(format!("{tool}.exe"))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PgStatus {
    pub running: bool,
    pub version: Option<String>,
    pub port: Option<u16>,
    pub pid: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum PgError {
    #[error("{0}")]
    Failed(String),
    #[error("a porta {0} já está em uso")]
    PortBusy(u16),
}

fn hidden(program: &Path) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    c.creation_flags(0x0800_0000);
    c.stdin(Stdio::null());
    c.env("PGCLIENTENCODING", "UTF8");
    c
}

/// Parâmetros leves passados na linha de comando (`-c chave=valor`).
pub fn light_settings(port: u16) -> Vec<String> {
    [
        "listen_addresses=127.0.0.1".to_string(),
        format!("port={port}"),
        "shared_buffers=32MB".into(),
        "max_connections=20".into(),
        "work_mem=4MB".into(),
        "maintenance_work_mem=32MB".into(),
        "max_wal_size=256MB".into(),
        "logging_collector=off".into(),
    ]
    .into_iter()
    .flat_map(|s| ["-c".to_string(), s])
    .collect()
}

fn output_error(out: &std::process::Output) -> String {
    let text = format!("{}{}", String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines.iter().rev().take(4).rev().copied().collect::<Vec<_>>().join(" | ")
}

pub struct PgService {
    inner: Mutex<Option<(PgSetup, Child)>>,
}

impl Default for PgService {
    fn default() -> Self {
        Self { inner: Mutex::new(None) }
    }
}

impl PgService {
    pub async fn status(&self) -> PgStatus {
        let mut g = self.inner.lock().await;
        if let Some((_, child)) = g.as_mut()
            && !matches!(child.try_wait(), Ok(None))
        {
            *g = None;
        }
        match g.as_ref() {
            Some((s, child)) => PgStatus { running: true, version: Some(s.version.clone()), port: Some(s.port), pid: child.id() },
            None => PgStatus { running: false, version: None, port: None, pid: None },
        }
    }

    async fn init_if_needed(setup: &PgSetup) -> Result<(), PgError> {
        if setup.data_dir.join("PG_VERSION").is_file() {
            return Ok(());
        }
        if let Some(parent) = setup.data_dir.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PgError::Failed(e.to_string()))?;
        }
        tracing::info!(dir = %setup.data_dir.display(), "inicializando dados do PostgreSQL");
        let out = hidden(&setup.bin("initdb"))
            .args(["-U", "postgres", "-A", "trust", "-E", "UTF8", "--no-locale", "-D"])
            .arg(&setup.data_dir)
            .output()
            .await
            .map_err(|e| PgError::Failed(format!("initdb: {e}")))?;
        if !out.status.success() || !setup.data_dir.join("PG_VERSION").is_file() {
            let _ = std::fs::remove_dir_all(&setup.data_dir);
            return Err(PgError::Failed(format!("não foi possível criar os dados do PostgreSQL: {}", output_error(&out))));
        }
        Ok(())
    }

    pub async fn start(&self, setup: &PgSetup) -> Result<(), PgError> {
        let mut g = self.inner.lock().await;
        if let Some((current, child)) = g.as_mut() {
            if matches!(child.try_wait(), Ok(None)) && current == setup {
                return Ok(());
            }
            drop(g);
            self.stop().await;
            g = self.inner.lock().await;
        }
        if crate::port::check_free(setup.port).is_err() {
            return Err(PgError::PortBusy(setup.port));
        }
        Self::init_if_needed(setup).await?;
        // Um desligamento à força deixa o pid antigo; o postgres recusa subir com ele.
        let pid_file = setup.data_dir.join("postmaster.pid");
        if pid_file.is_file() {
            let _ = std::fs::remove_file(&pid_file);
        }

        let log = setup.data_dir.join("sentinel-postgres.log");
        let log_file = std::fs::File::create(&log).map_err(|e| PgError::Failed(e.to_string()))?;
        let mut child = hidden(&setup.bin("postgres"))
            .arg("-D")
            .arg(&setup.data_dir)
            .args(light_settings(setup.port))
            .stdout(Stdio::null())
            .stderr(Stdio::from(log_file))
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| PgError::Failed(format!("postgres: {e}")))?;
        crate::job::track(&child);

        let started = Instant::now();
        loop {
            if tokio::net::TcpStream::connect(("127.0.0.1", setup.port)).await.is_ok() {
                break;
            }
            if !matches!(child.try_wait(), Ok(None)) || started.elapsed() > Duration::from_secs(40) {
                let _ = child.kill().await;
                let text = std::fs::read_to_string(&log).unwrap_or_default();
                let tail: Vec<&str> = text.lines().rev().take(6).collect();
                let tail = tail.into_iter().rev().collect::<Vec<_>>().join(" | ");
                let hint = if text.contains("administrative permissions") {
                    " (o PostgreSQL não roda como administrador: abra o Sentinel sem \"Executar como administrador\")"
                } else {
                    ""
                };
                return Err(PgError::Failed(format!("o PostgreSQL não iniciou: {tail}{hint}")));
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        // Porta aberta não quer dizer pronto para consultas (recuperação etc.).
        let ready = PgClient::for_setup(setup);
        for _ in 0..60 {
            if ready.sql("postgres", "SELECT 1").await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        tracing::info!(version = setup.version, port = setup.port, "PostgreSQL rodando");
        *g = Some((setup.clone(), child));
        Ok(())
    }

    /// Desligamento rápido e limpo (`pg_ctl stop -m fast`), à força se não responder.
    pub async fn stop(&self) {
        let Some((setup, mut child)) = self.inner.lock().await.take() else { return };
        let _ = hidden(&setup.bin("pg_ctl"))
            .args(["stop", "-m", "fast", "-w", "-t", "15", "-D"])
            .arg(&setup.data_dir)
            .output()
            .await;
        if tokio::time::timeout(Duration::from_secs(20), child.wait()).await.is_err() {
            tracing::warn!("PostgreSQL não desligou a tempo; encerrando à força");
            let _ = child.kill().await;
        }
        tracing::info!("PostgreSQL parado");
    }

    pub async fn client(&self) -> Result<PgClient, PgError> {
        let g = self.inner.lock().await;
        let (s, _) = g.as_ref().ok_or_else(|| PgError::Failed("o PostgreSQL não está rodando".into()))?;
        Ok(PgClient::for_setup(s))
    }
}

/// `psql`/`pg_dump` ligados ao servidor do Sentinel.
#[derive(Debug, Clone)]
pub struct PgClient {
    pub bin_dir: PathBuf,
    pub port: u16,
}

impl PgClient {
    fn for_setup(s: &PgSetup) -> Self {
        Self { bin_dir: s.install_dir.join("bin"), port: s.port }
    }

    fn command(&self, tool: &str) -> Command {
        let mut c = hidden(&self.bin_dir.join(format!("{tool}.exe")));
        c.args(["-h", "127.0.0.1", "-p", &self.port.to_string(), "-U", "postgres"]);
        c
    }

    /// Roda SQL e devolve a saída sem cabeçalhos (uma linha por registro).
    pub async fn sql(&self, db: &str, statement: &str) -> Result<String, String> {
        let out = self
            .command("psql")
            .args(["-X", "-q", "-t", "-A", "-v", "ON_ERROR_STOP=1", "-d", db, "-c", statement])
            .output()
            .await
            .map_err(|e| format!("psql: {e}"))?;
        if !out.status.success() {
            return Err(output_error(&out));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    pub async fn databases(&self) -> Result<Vec<String>, String> {
        let out = self
            .sql("postgres", "SELECT datname FROM pg_database WHERE NOT datistemplate AND datname <> 'postgres' ORDER BY 1")
            .await?;
        Ok(out.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect())
    }

    /// Cria o banco se ainda não existir.
    pub async fn create_database(&self, name: &str) -> Result<(), String> {
        if !valid_db(name) {
            return Err(format!("nome de banco inválido: {name}"));
        }
        if self.databases().await?.iter().any(|d| d == name) {
            return Ok(());
        }
        self.sql("postgres", &format!("CREATE DATABASE \"{name}\" ENCODING 'UTF8'")).await.map(|_| ())
    }

    pub async fn dump(&self, db: &str, file: &Path) -> Result<(), String> {
        if !valid_db(db) {
            return Err(format!("nome de banco inválido: {db}"));
        }
        let out = self
            .command("pg_dump")
            .args(["--no-owner", "--no-privileges", "--encoding=UTF8", "-d", db, "-f"])
            .arg(file)
            .output()
            .await
            .map_err(|e| format!("pg_dump: {e}"))?;
        if !out.status.success() {
            let _ = std::fs::remove_file(file);
            return Err(output_error(&out));
        }
        Ok(())
    }

    /// Restaura um `.sql` (de `pg_dump`) num banco, criando-o se preciso.
    pub async fn restore(&self, db: &str, file: &Path) -> Result<(), String> {
        self.create_database(db).await?;
        let out = self
            .command("psql")
            .args(["-X", "-q", "-v", "ON_ERROR_STOP=1", "-d", db, "-f"])
            .arg(file)
            .output()
            .await
            .map_err(|e| format!("psql: {e}"))?;
        if !out.status.success() {
            return Err(output_error(&out));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_light_and_local() {
        let s = light_settings(5433);
        assert!(s.contains(&"listen_addresses=127.0.0.1".to_string()));
        assert!(s.contains(&"port=5433".to_string()));
        assert!(s.contains(&"shared_buffers=32MB".to_string()));
        assert_eq!(s.iter().filter(|a| *a == "-c").count(), s.len() / 2);
    }

    /// PostgreSQL de verdade: `SENTINEL_TEST_POSTGRES=<pasta instalada> cargo test -p sentinel-server -- --ignored real_postgres`
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn real_postgres() {
        let install = PathBuf::from(std::env::var("SENTINEL_TEST_POSTGRES").expect("defina SENTINEL_TEST_POSTGRES"));
        let tmp = tempfile::tempdir().unwrap();
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let setup = PgSetup {
            version: install.file_name().unwrap().to_string_lossy().to_string(),
            install_dir: install,
            data_dir: tmp.path().join("data"),
            port,
        };
        let svc = PgService::default();
        svc.start(&setup).await.unwrap();
        let c = svc.client().await.unwrap();
        c.create_database("loja_teste").await.unwrap();
        c.sql("loja_teste", "CREATE TABLE produtos (id int primary key, nome text); INSERT INTO produtos VALUES (1, 'Caneca ☕')")
            .await
            .unwrap();
        let dump = tmp.path().join("b.sql");
        c.dump("loja_teste", &dump).await.unwrap();
        c.restore("loja_copia", &dump).await.unwrap();
        let out = c.sql("loja_copia", "SELECT nome FROM produtos").await.unwrap();
        assert!(out.contains("Caneca ☕"), "{out}");
        svc.stop().await;
        assert!(!svc.status().await.running);
        svc.start(&setup).await.unwrap();
        assert!(svc.client().await.unwrap().databases().await.unwrap().contains(&"loja_copia".to_string()));
        svc.stop().await;
    }
}
