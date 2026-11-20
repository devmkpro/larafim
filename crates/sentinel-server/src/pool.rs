//! Pools de `php-cgi.exe` por versão do PHP. No Windows cada processo
//! atende uma requisição por vez, então cada pool tem alguns processos.
//! Os pools sobem na primeira requisição e são desligados quando ficam
//! parados por um tempo — o PC só paga pelo PHP que está sendo usado.

use std::collections::{HashMap, VecDeque};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Semaphore};

use crate::fcgi::{self, FcgiResponse};

/// Uma instalação do PHP (pasta com `php-cgi.exe` e `php.ini`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhpRuntime {
    pub version: String,
    pub dir: PathBuf,
}

impl PhpRuntime {
    fn cgi(&self) -> PathBuf {
        self.dir.join("php-cgi.exe")
    }
    fn ini(&self) -> PathBuf {
        self.dir.join("php.ini")
    }
}

#[derive(Debug, Clone)]
pub struct PoolSettings {
    pub workers: usize,
    pub idle_timeout: Duration,
    pub request_timeout: Duration,
}

impl Default for PoolSettings {
    fn default() -> Self {
        Self { workers: 3, idle_timeout: Duration::from_secs(10 * 60), request_timeout: Duration::from_secs(120) }
    }
}

struct Worker {
    port: u16,
    child: Option<Child>,
}

impl Worker {
    fn alive(&mut self) -> bool {
        match self.child.as_mut() {
            Some(c) => matches!(c.try_wait(), Ok(None)),
            None => false,
        }
    }

    async fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill().await;
        }
    }
}

struct Pool {
    runtime: PhpRuntime,
    workers: Vec<Mutex<Worker>>,
    idle: std::sync::Mutex<VecDeque<usize>>,
    slots: Semaphore,
    last_used: std::sync::Mutex<Instant>,
}

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("não foi possível iniciar o php-cgi do PHP {version}: {reason}")]
    Spawn { version: String, reason: String },
    #[error("o PHP não respondeu: {0}")]
    Request(String),
}

/// Todos os pools, por versão.
pub struct Pools {
    settings: PoolSettings,
    map: Mutex<HashMap<String, Arc<Pool>>>,
}

impl Pools {
    /// Quem cria deve chamar `reap_idle` periodicamente (ex.: a cada 30 s).
    pub fn new(settings: PoolSettings) -> Arc<Self> {
        Arc::new(Self { settings, map: Mutex::new(HashMap::new()) })
    }

    /// Executa um script no PHP indicado (sobe o pool se preciso).
    pub async fn execute(
        &self,
        runtime: &PhpRuntime,
        params: &[(String, String)],
        stdin: &[u8],
    ) -> Result<FcgiResponse, PoolError> {
        let pool = self.pool_for(runtime).await;
        pool.execute(params, stdin, self.settings.request_timeout).await
    }

    async fn pool_for(&self, runtime: &PhpRuntime) -> Arc<Pool> {
        let mut map = self.map.lock().await;
        if let Some(p) = map.get(&runtime.version)
            && p.runtime == *runtime
        {
            return p.clone();
        }
        // Versão nova (ou atualizada): o pool antigo é desligado.
        if let Some(old) = map.remove(&runtime.version) {
            old.stop().await;
        }
        tracing::info!(version = runtime.version, workers = self.settings.workers, "criando pool do PHP");
        let pool = Arc::new(Pool {
            runtime: runtime.clone(),
            workers: (0..self.settings.workers).map(|_| Mutex::new(Worker { port: 0, child: None })).collect(),
            idle: std::sync::Mutex::new((0..self.settings.workers).collect()),
            slots: Semaphore::new(self.settings.workers),
            last_used: std::sync::Mutex::new(Instant::now()),
        });
        map.insert(runtime.version.clone(), pool.clone());
        pool
    }

    /// Desliga os pools que estão parados há mais que o limite.
    pub async fn reap_idle(&self) {
        let mut map = self.map.lock().await;
        let idle: Vec<String> = map
            .iter()
            .filter(|(_, p)| p.last_used.lock().unwrap().elapsed() > self.settings.idle_timeout && p.slots.available_permits() == p.workers.len())
            .map(|(v, _)| v.clone())
            .collect();
        for version in idle {
            if let Some(p) = map.remove(&version) {
                tracing::info!(version, "pool do PHP parado; desligando");
                p.stop().await;
            }
        }
    }

    /// Versões com processos rodando agora (para o status/monitor).
    pub async fn running(&self) -> Vec<String> {
        self.map.lock().await.keys().cloned().collect()
    }

    pub async fn stop_all(&self) {
        let mut map = self.map.lock().await;
        for (_, p) in map.drain() {
            p.stop().await;
        }
    }
}

impl Pool {
    async fn execute(&self, params: &[(String, String)], stdin: &[u8], timeout: Duration) -> Result<FcgiResponse, PoolError> {
        let _permit = self.slots.acquire().await.map_err(|e| PoolError::Request(e.to_string()))?;
        let idx = self.idle.lock().unwrap().pop_front().unwrap_or(0);
        *self.last_used.lock().unwrap() = Instant::now();

        let result = async {
            let mut worker = self.workers[idx].lock().await;
            // Uma segunda tentativa se o processo tiver caído no meio.
            for attempt in 0..2 {
                if !worker.alive() {
                    self.spawn(&mut worker).await?;
                }
                let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, worker.port));
                match fcgi::execute(addr, params, stdin, timeout).await {
                    Ok(resp) => return Ok(resp),
                    Err(e) if attempt == 0 && e.kind() != std::io::ErrorKind::TimedOut => {
                        tracing::warn!(version = self.runtime.version, "php-cgi não respondeu ({e}); reiniciando");
                        worker.stop().await;
                    }
                    Err(e) => {
                        worker.stop().await;
                        return Err(PoolError::Request(e.to_string()));
                    }
                }
            }
            Err(PoolError::Request("sem resposta".into()))
        }
        .await;

        self.idle.lock().unwrap().push_back(idx);
        result
    }

    async fn spawn(&self, worker: &mut Worker) -> Result<(), PoolError> {
        let fail = |reason: String| PoolError::Spawn { version: self.runtime.version.clone(), reason };
        let port = free_port().map_err(|e| fail(e.to_string()))?;
        let mut cmd = Command::new(self.runtime.cgi());
        cmd.arg("-b")
            .arg(format!("127.0.0.1:{port}"))
            .arg("-c")
            .arg(self.runtime.ini())
            .current_dir(&self.runtime.dir)
            // Sem limite de requisições: no Windows o php-cgi sairia depois de 500.
            .env("PHP_FCGI_MAX_REQUESTS", "0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let child = cmd.spawn().map_err(|e| fail(e.to_string()))?;
        crate::job::track(&child);
        worker.child = Some(child);
        worker.port = port;

        // Espera o processo começar a aceitar conexões.
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                tracing::debug!(version = self.runtime.version, port, "php-cgi pronto");
                return Ok(());
            }
            if !worker.alive() {
                return Err(fail("o processo terminou ao iniciar (php.ini ou extensão com problema?)".into()));
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        worker.stop().await;
        Err(fail("não respondeu em 10 segundos".into()))
    }

    async fn stop(&self) {
        for w in &self.workers {
            w.lock().await.stop().await;
        }
    }
}

/// Porta livre escolhida pelo sistema.
fn free_port() -> std::io::Result<u16> {
    let l = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    Ok(l.local_addr()?.port())
}
