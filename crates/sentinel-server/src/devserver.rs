//! Servidores de desenvolvimento do Node (`npm run dev`, `pnpm dev`…).
//!
//! Cada servidor roda num Job Object próprio: parar encerra a árvore inteira
//! (cmd → npm → node → esbuild…), e se o Sentinel cair o Windows encerra tudo.
//! A porta é descoberta lendo a saída (`Local: http://localhost:5173/`).

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Porta compartilhada com o gateway (proxy). `None` = ainda não descoberta.
pub type PortCell = Arc<Mutex<Option<u16>>>;

const MAX_LOG_LINES: usize = 400;

#[derive(Debug, Clone)]
pub struct DevSpec {
    /// Identificador (caminho do projeto).
    pub key: String,
    pub cwd: PathBuf,
    pub command: String,
    /// Pasta do Node escolhido (vai na frente do PATH).
    pub node_dir: PathBuf,
    /// Variáveis extras (PORT, APP_URL…).
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DevState {
    Starting,
    Running,
    Exited,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevStatus {
    pub key: String,
    pub state: DevState,
    pub port: Option<u16>,
    pub exit_code: Option<i32>,
    pub command: String,
}

#[derive(Debug, Clone)]
pub enum DevEvent {
    Log { key: String, line: String },
    Status(DevStatus),
}

type Listener = Arc<dyn Fn(DevEvent) + Send + Sync>;

struct Entry {
    status: Arc<Mutex<DevStatus>>,
    port: PortCell,
    logs: Arc<Mutex<VecDeque<String>>>,
    job: job::Job,
}

pub struct DevServers {
    map: Mutex<HashMap<String, Entry>>,
    listener: Listener,
}

/// Tira códigos de cor ANSI da saída.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Primeira URL local da linha: `http://localhost:5173`, `127.0.0.1:3000`, `[::1]:5173`.
pub fn find_local_port(line: &str) -> Option<u16> {
    for host in ["http://localhost:", "http://127.0.0.1:", "http://[::1]:", "http://0.0.0.0:"] {
        if let Some(pos) = line.find(host) {
            let digits: String = line[pos + host.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(p) = digits.parse() {
                return Some(p);
            }
        }
    }
    None
}

impl DevServers {
    pub fn new(listener: impl Fn(DevEvent) + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self { map: Mutex::new(HashMap::new()), listener: Arc::new(listener) })
    }

    /// Porta compartilhada para o proxy (cria vazia se o servidor ainda não subiu).
    pub fn port_cell(&self, key: &str) -> Option<PortCell> {
        self.map.lock().unwrap().get(key).map(|e| e.port.clone())
    }

    pub fn status(&self, key: &str) -> Option<DevStatus> {
        self.map.lock().unwrap().get(key).map(|e| e.status.lock().unwrap().clone())
    }

    pub fn all(&self) -> Vec<DevStatus> {
        self.map.lock().unwrap().values().map(|e| e.status.lock().unwrap().clone()).collect()
    }

    pub fn logs(&self, key: &str) -> Vec<String> {
        self.map.lock().unwrap().get(key).map(|e| e.logs.lock().unwrap().iter().cloned().collect()).unwrap_or_default()
    }

    pub fn is_running(&self, key: &str) -> bool {
        self.status(key).is_some_and(|s| s.state != DevState::Exited)
    }

    /// Sobe o servidor (para o anterior da mesma chave, se houver).
    pub fn start(self: &Arc<Self>, spec: DevSpec) -> std::io::Result<()> {
        self.stop(&spec.key);

        let path = format!("{};{}", spec.node_dir.display(), std::env::var("PATH").unwrap_or_default());
        let mut cmd = Command::new("cmd.exe");
        // Linha de comando passada sem conversão: o Rust escaparia aspas
        // internas como \" e o cmd.exe não entende isso ("pasta" virava \pasta\").
        // Com /S, o cmd tira só as aspas de fora e roda o resto como está.
        #[cfg(windows)]
        cmd.raw_arg(format!("/D /S /C \"{}\"", spec.command));
        #[cfg(not(windows))]
        cmd.arg("-c").arg(&spec.command);
        cmd.current_dir(&spec.cwd)
            .env("PATH", path)
            .env("BROWSER", "none")
            .env("FORCE_COLOR", "0")
            .env("NO_UPDATE_NOTIFIER", "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let job = job::Job::new();
        let mut child = cmd.spawn()?;
        if let Some(h) = child.raw_handle() {
            job.assign(h);
        }

        let status = Arc::new(Mutex::new(DevStatus {
            key: spec.key.clone(),
            state: DevState::Starting,
            port: None,
            exit_code: None,
            command: spec.command.clone(),
        }));
        let port: PortCell = Arc::new(Mutex::new(None));
        let logs = Arc::new(Mutex::new(VecDeque::new()));
        tracing::info!(key = spec.key, command = spec.command, "servidor do Node iniciando");
        (self.listener)(DevEvent::Status(status.lock().unwrap().clone()));

        // Lê stdout e stderr linha a linha.
        let readers: Vec<Box<dyn tokio::io::AsyncRead + Send + Unpin>> = vec![
            Box::new(child.stdout.take().expect("stdout")),
            Box::new(child.stderr.take().expect("stderr")),
        ];
        for reader in readers {
            let (status, port, logs, listener, key) =
                (status.clone(), port.clone(), logs.clone(), self.listener.clone(), spec.key.clone());
            tokio::spawn(async move {
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(raw)) = lines.next_line().await {
                    let line = strip_ansi(&raw);
                    {
                        let mut l = logs.lock().unwrap();
                        if l.len() >= MAX_LOG_LINES {
                            l.pop_front();
                        }
                        l.push_back(line.clone());
                    }
                    let discovered = {
                        let mut p = port.lock().unwrap();
                        match (p.is_none(), find_local_port(&line)) {
                            (true, Some(found)) => {
                                *p = Some(found);
                                Some(found)
                            }
                            _ => None,
                        }
                    };
                    if let Some(found) = discovered {
                        let snapshot = {
                            let mut s = status.lock().unwrap();
                            s.state = DevState::Running;
                            s.port = Some(found);
                            s.clone()
                        };
                        tracing::info!(key, port = found, "servidor do Node pronto");
                        listener(DevEvent::Status(snapshot));
                    }
                    listener(DevEvent::Log { key: key.clone(), line });
                }
            });
        }

        // Se em 4 s o comando não imprimiu uma URL (ex.: Vite do Laravel sem
        // log, scripts próprios), considera rodando — e, se havia PORT
        // definida, usa ela para o proxy.
        {
            let expected: Option<u16> = spec.env.iter().find(|(k, _)| k == "PORT").and_then(|(_, v)| v.parse().ok());
            let (status, port, listener) = (status.clone(), port.clone(), self.listener.clone());
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                let snapshot = {
                    let mut s = status.lock().unwrap();
                    if s.state != DevState::Starting {
                        return;
                    }
                    s.state = DevState::Running;
                    if let Some(p) = expected {
                        let mut cell = port.lock().unwrap();
                        if cell.is_none() {
                            *cell = Some(p);
                            s.port = Some(p);
                        }
                    }
                    s.clone()
                };
                listener(DevEvent::Status(snapshot));
            });
        }

        // Espera o processo terminar.
        {
            let (status, port, listener, key) = (status.clone(), port.clone(), self.listener.clone(), spec.key.clone());
            tokio::spawn(async move {
                let code = child.wait().await.ok().and_then(|s| s.code());
                *port.lock().unwrap() = None;
                let snapshot = {
                    let mut s = status.lock().unwrap();
                    s.state = DevState::Exited;
                    s.port = None;
                    s.exit_code = code;
                    s.clone()
                };
                tracing::info!(key, ?code, "servidor do Node terminou");
                listener(DevEvent::Status(snapshot));
            });
        }

        self.map.lock().unwrap().insert(spec.key, Entry { status, port, logs, job });
        Ok(())
    }

    /// Para o servidor e toda a árvore de processos dele.
    pub fn stop(&self, key: &str) {
        if let Some(entry) = self.map.lock().unwrap().remove(key) {
            entry.job.terminate();
            *entry.port.lock().unwrap() = None;
            let snapshot = {
                let mut s = entry.status.lock().unwrap();
                s.state = DevState::Exited;
                s.port = None;
                s.clone()
            };
            (self.listener)(DevEvent::Status(snapshot));
        }
    }

    pub fn stop_all(&self) {
        let keys: Vec<String> = self.map.lock().unwrap().keys().cloned().collect();
        for k in keys {
            self.stop(&k);
        }
    }
}

/// Job Object próprio de cada servidor.
mod job {
    #[cfg(windows)]
    pub struct Job(windows_sys::Win32::Foundation::HANDLE);

    #[cfg(windows)]
    // SAFETY: o handle pode ser usado de qualquer thread.
    unsafe impl Send for Job {}

    #[cfg(windows)]
    impl Job {
        pub fn new() -> Self {
            use windows_sys::Win32::System::JobObjects::*;
            // SAFETY: chamadas Win32 com ponteiros válidos.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const core::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                Self(h)
            }
        }

        pub fn assign(&self, process: std::os::windows::io::RawHandle) {
            // SAFETY: handle de processo filho válido.
            unsafe {
                windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(self.0, process as _);
            }
        }

        pub fn terminate(&self) {
            // SAFETY: handle de job válido até o Drop.
            unsafe {
                windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
            }
        }
    }

    #[cfg(windows)]
    impl Drop for Job {
        fn drop(&mut self) {
            // Fechar o handle também encerra os processos (KILL_ON_JOB_CLOSE).
            // SAFETY: handle aberto por nós, fechado uma vez.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.0);
            }
        }
    }

    #[cfg(not(windows))]
    pub struct Job;
    #[cfg(not(windows))]
    impl Job {
        pub fn new() -> Self {
            Self
        }
        pub fn assign(&self, _p: i32) {}
        pub fn terminate(&self) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Aspas e espaços no comando chegam intactos ao programa.
    #[tokio::test(flavor = "multi_thread")]
    async fn quoted_command_arrives_intact() {
        let servers = DevServers::new(|_| {});
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("pasta com espaço");
        std::fs::create_dir_all(&dir).unwrap();
        servers
            .start(DevSpec {
                key: "q".into(),
                cwd: tmp.path().to_path_buf(),
                command: r#"mkdir "pasta com espaço\nova" && echo feito"#.into(),
                node_dir: tmp.path().to_path_buf(),
                env: vec![],
            })
            .unwrap();
        for _ in 0..100 {
            if servers.status("q").is_some_and(|s| s.state == DevState::Exited) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(dir.join("nova").is_dir(), "{:?}", servers.logs("q"));
    }

    #[test]
    fn ports_and_ansi() {
        assert_eq!(find_local_port("  ➜  Local:   http://localhost:5173/"), Some(5173));
        assert_eq!(find_local_port("- Local:        http://localhost:3000"), Some(3000));
        assert_eq!(find_local_port("Listening on http://127.0.0.1:8000"), Some(8000));
        assert_eq!(find_local_port("http://[::1]:5174/"), Some(5174));
        assert_eq!(find_local_port("VITE v6 ready in 300 ms"), None);
        assert_eq!(strip_ansi("\u{1b}[32m➜\u{1b}[39m  Local: \u{1b}[36mhttp://localhost:\u{1b}[1m5173\u{1b}[22m/\u{1b}[39m"), "➜  Local: http://localhost:5173/");
    }

    /// Sobe um "servidor" de verdade via cmd, lê a porta e para a árvore.
    #[tokio::test(flavor = "multi_thread")]
    async fn start_detect_and_stop() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev = events.clone();
        let servers = DevServers::new(move |e| ev.lock().unwrap().push(e));
        let tmp = tempfile::tempdir().unwrap();
        servers
            .start(DevSpec {
                key: "x".into(),
                cwd: tmp.path().to_path_buf(),
                command: "echo   Local: http://localhost:%PORT%/ && ping -n 30 127.0.0.1 >nul".into(),
                node_dir: tmp.path().to_path_buf(),
                env: vec![("PORT".into(), "4321".into())],
            })
            .unwrap();
        for _ in 0..100 {
            if servers.status("x").unwrap().port.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        let s = servers.status("x").unwrap();
        assert_eq!((s.state, s.port), (DevState::Running, Some(4321)));
        assert!(servers.logs("x").iter().any(|l| l.contains("4321")));
        assert_eq!(*servers.port_cell("x").unwrap().lock().unwrap(), Some(4321));
        servers.stop("x");
        assert!(servers.status("x").is_none());
        assert!(!servers.is_running("x"));
    }
}
