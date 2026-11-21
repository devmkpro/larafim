//! Serviço MariaDB: um servidor local (127.0.0.1) iniciado sob demanda, com
//! configuração leve. Dados em `data\mariadb\<linha>` (ex.: 11.8).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::dbclient::DbClient;
pub use crate::dbclient::valid_db;

/// O que é preciso para rodar: a instalação e onde ficam os dados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MariaSetup {
    pub version: String,
    pub install_dir: PathBuf,
    pub data_dir: PathBuf,
    pub port: u16,
}

impl MariaSetup {
    fn bin(&self, tool: &str) -> PathBuf {
        self.install_dir.join("bin").join(format!("{tool}.exe"))
    }
    fn ini(&self) -> PathBuf {
        self.data_dir.join("my.ini")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MariaStatus {
    pub running: bool,
    pub version: Option<String>,
    pub port: Option<u16>,
    pub pid: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum MariaError {
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
    c
}

fn fwd(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// `my.ini` leve: só em 127.0.0.1, pouco cache, sem performance_schema.
pub fn render_ini(setup: &MariaSetup) -> String {
    format!(
        "# Gerado pelo Sentinel (modo leve). Reescrito a cada início.\n\
         [mysqld]\n\
         datadir={data}\n\
         port={port}\n\
         bind-address=127.0.0.1\n\
         performance_schema=OFF\n\
         innodb_buffer_pool_size=64M\n\
         innodb_log_buffer_size=8M\n\
         max_connections=30\n\
         table_open_cache=256\n\
         key_buffer_size=8M\n\
         aria_pagecache_buffer_size=16M\n\
         character-set-server=utf8mb4\n\
         collation-server=utf8mb4_unicode_ci\n\
         \n\
         [client]\n\
         port={port}\n\
         plugin-dir={plugins}\n",
        data = fwd(&setup.data_dir),
        port = setup.port,
        plugins = fwd(&setup.install_dir.join("lib").join("plugin")),
    )
}

/// Nome de banco seguro a partir do domínio (`loja-virtual` → `loja_virtual`).
pub fn database_name(domain: &str) -> String {
    let mut s: String = domain
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    s.truncate(64);
    if s.is_empty() { "projeto".into() } else { s }
}


pub struct MariaService {
    inner: Mutex<Option<(MariaSetup, Child)>>,
}

impl Default for MariaService {
    fn default() -> Self {
        Self { inner: Mutex::new(None) }
    }
}

impl MariaService {
    pub async fn status(&self) -> MariaStatus {
        let mut g = self.inner.lock().await;
        if let Some((_, child)) = g.as_mut()
            && !matches!(child.try_wait(), Ok(None))
        {
            *g = None; // o processo morreu sozinho
        }
        match g.as_ref() {
            Some((s, child)) => MariaStatus { running: true, version: Some(s.version.clone()), port: Some(s.port), pid: child.id() },
            None => MariaStatus { running: false, version: None, port: None, pid: None },
        }
    }

    /// Cria a pasta de dados na primeira vez (`mariadb-install-db`).
    async fn init_if_needed(setup: &MariaSetup) -> Result<(), MariaError> {
        if setup.data_dir.join("mysql").is_dir() {
            return Ok(());
        }
        std::fs::create_dir_all(setup.data_dir.parent().unwrap_or(&setup.data_dir))
            .map_err(|e| MariaError::Failed(e.to_string()))?;
        tracing::info!(dir = %setup.data_dir.display(), "inicializando dados do MariaDB");
        let out = hidden(&setup.bin("mariadb-install-db"))
            .arg(format!("--datadir={}", setup.data_dir.display()))
            .arg(format!("--port={}", setup.port))
            .output()
            .await
            .map_err(|e| MariaError::Failed(format!("mariadb-install-db: {e}")))?;
        if !out.status.success() || !setup.data_dir.join("mysql").is_dir() {
            let msg = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            return Err(MariaError::Failed(format!("não foi possível criar os dados do MariaDB: {}", msg.trim())));
        }
        Ok(())
    }

    /// Sobe o servidor (se já estiver rodando com a mesma configuração, nada a fazer).
    pub async fn start(&self, setup: &MariaSetup) -> Result<(), MariaError> {
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
            return Err(MariaError::PortBusy(setup.port));
        }
        Self::init_if_needed(setup).await?;
        std::fs::write(setup.ini(), render_ini(setup)).map_err(|e| MariaError::Failed(e.to_string()))?;

        let log = setup.data_dir.join("sentinel-mariadb.log");
        let log_file = std::fs::File::create(&log).map_err(|e| MariaError::Failed(e.to_string()))?;
        let mut child = hidden(&setup.bin("mariadbd"))
            .arg(format!("--defaults-file={}", setup.ini().display()))
            .arg("--console")
            .stdout(Stdio::null())
            .stderr(Stdio::from(log_file))
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| MariaError::Failed(format!("mariadbd: {e}")))?;
        crate::job::track(&child);

        // Espera aceitar conexões.
        let started = Instant::now();
        loop {
            if tokio::net::TcpStream::connect(("127.0.0.1", setup.port)).await.is_ok() {
                break;
            }
            if !matches!(child.try_wait(), Ok(None)) || started.elapsed() > Duration::from_secs(40) {
                let _ = child.kill().await;
                let tail = std::fs::read_to_string(&log).unwrap_or_default();
                let tail: Vec<&str> = tail.lines().rev().take(6).collect();
                return Err(MariaError::Failed(format!(
                    "o MariaDB não iniciou: {}",
                    tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
                )));
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        tracing::info!(version = setup.version, port = setup.port, "MariaDB rodando");
        *g = Some((setup.clone(), child));
        Ok(())
    }

    /// Desliga do jeito certo (`shutdown`), e à força só se não responder.
    pub async fn stop(&self) {
        let Some((setup, mut child)) = self.inner.lock().await.take() else { return };
        let _ = hidden(&setup.bin("mariadb-admin"))
            .args(["-u", "root", "-h", "127.0.0.1", "-P", &setup.port.to_string(), "shutdown"])
            .output()
            .await;
        if tokio::time::timeout(Duration::from_secs(20), child.wait()).await.is_err() {
            tracing::warn!("MariaDB não desligou a tempo; encerrando à força");
            let _ = child.kill().await;
        }
        tracing::info!("MariaDB parado");
    }

    async fn setup(&self) -> Result<MariaSetup, MariaError> {
        self.inner
            .lock()
            .await
            .as_ref()
            .map(|(s, _)| s.clone())
            .ok_or_else(|| MariaError::Failed("o MariaDB não está rodando".into()))
    }

    /// Cliente de linha de comando ligado a este servidor (root, sem senha).
    pub async fn client(&self) -> Result<DbClient, MariaError> {
        let s = self.setup().await?;
        Ok(DbClient {
            bin_dir: s.install_dir.join("bin"),
            host: "127.0.0.1".into(),
            port: s.port,
            user: "root".into(),
            password: String::new(),
        })
    }

    pub async fn sql(&self, statement: &str) -> Result<String, MariaError> {
        self.client().await?.sql(statement).await.map_err(MariaError::Failed)
    }

    pub async fn create_database(&self, name: &str) -> Result<(), MariaError> {
        self.client().await?.create_database(name).await.map_err(MariaError::Failed)
    }

    pub async fn databases(&self) -> Result<Vec<String>, MariaError> {
        self.client().await?.databases().await.map_err(MariaError::Failed)
    }

    pub async fn dump(&self, db: &str, file: &Path) -> Result<(), MariaError> {
        self.client().await?.dump(db, file).await.map_err(MariaError::Failed)
    }

    pub async fn restore(&self, db: &str, file: &Path) -> Result<(), MariaError> {
        self.client().await?.restore(db, file).await.map_err(MariaError::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_ini() {
        assert_eq!(database_name("loja-virtual"), "loja_virtual");
        assert_eq!(database_name("API.v2"), "api_v2");
        assert!(!valid_db("x`; DROP"));
        let s = MariaSetup {
            version: "11.8.9".into(),
            install_dir: PathBuf::from(r"C:\S\runtimes\mariadb\11.8.9"),
            data_dir: PathBuf::from(r"C:\S\data\mariadb\11.8"),
            port: 3307,
        };
        let ini = render_ini(&s);
        assert!(ini.contains("datadir=C:/S/data/mariadb/11.8\n"));
        assert!(ini.contains("port=3307\n"));
        assert!(ini.contains("bind-address=127.0.0.1"));
    }

    /// MariaDB de verdade: `SENTINEL_TEST_MARIADB=<pasta instalada> cargo test -p sentinel-server -- --ignored real_mariadb`
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn real_mariadb() {
        let install = PathBuf::from(std::env::var("SENTINEL_TEST_MARIADB").expect("defina SENTINEL_TEST_MARIADB"));
        let tmp = tempfile::tempdir().unwrap();
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let setup = MariaSetup {
            version: install.file_name().unwrap().to_string_lossy().to_string(),
            install_dir: install,
            data_dir: tmp.path().join("data"),
            port,
        };
        let svc = MariaService::default();
        svc.start(&setup).await.unwrap();
        assert!(svc.status().await.running);
        svc.create_database("loja_teste").await.unwrap();
        svc.sql("CREATE TABLE loja_teste.produtos (id INT PRIMARY KEY, nome VARCHAR(50)); INSERT INTO loja_teste.produtos VALUES (1,'Caneca ☕')").await.unwrap();
        assert!(svc.databases().await.unwrap().contains(&"loja_teste".to_string()));

        let dump = tmp.path().join("b.sql");
        svc.dump("loja_teste", &dump).await.unwrap();
        let text = std::fs::read_to_string(&dump).unwrap();
        assert!(text.contains("Caneca"), "dump com {} bytes: {}", text.len(), &text[..text.len().min(300)]);
        svc.restore("loja_copia", &dump).await.unwrap();
        let out = svc.sql("SELECT nome FROM loja_copia.produtos").await.unwrap();
        assert!(out.contains("Caneca ☕"), "{out}");

        svc.stop().await;
        assert!(!svc.status().await.running);
        // Segundo início reaproveita os dados.
        svc.start(&setup).await.unwrap();
        assert!(svc.databases().await.unwrap().contains(&"loja_copia".to_string()));
        svc.stop().await;
    }
}
