use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Liga os logs: arquivo diário em `logs\larafim.log.AAAA-MM-DD` e, em
/// desenvolvimento, também no console. Guarde o `WorkerGuard` enquanto o app
/// estiver vivo, senão as últimas linhas podem se perder.
///
/// Nível padrão `info`; mude com a variável `LARAFIM_LOG` (ex.: `debug`).
pub fn init(logs_dir: &Path) -> WorkerGuard {
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("larafim")
        .filename_suffix("log")
        .max_log_files(7)
        .build(logs_dir)
        .expect("não foi possível criar o arquivo de log");
    let (file_writer, guard) = tracing_appender::non_blocking(appender);

    let filter = EnvFilter::try_from_env("LARAFIM_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(file_writer).with_ansi(false));

    if cfg!(debug_assertions) {
        let _ = registry.with(fmt::layer().with_writer(std::io::stderr)).try_init();
    } else {
        let _ = registry.try_init();
    }
    guard
}
