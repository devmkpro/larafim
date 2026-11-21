//! Atalho de linha de comando do Sentinel. O mesmo executável é copiado
//! como `php.exe`, `composer.exe`, `node.exe`, `npm.exe` e `npx.exe` para `%LOCALAPPDATA%\Sentinel\bin`.
//! Ao rodar, olha a pasta atual, escolhe a versão do PHP certa e repassa
//! tudo (argumentos, entrada/saída, código de saída) para o PHP real.
//!
//! Variáveis úteis:
//! - `SENTINEL_PHP=8.3` força uma versão na sessão do terminal;
//! - `SENTINEL_SHIM_WHICH=1` só mostra a escolha (JSON) e sai;
//! - `SENTINEL_SHIM_DEBUG=1` mostra a escolha no stderr antes de rodar.

use sentinel_core::t;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use sentinel_core::Paths;
use sentinel_runtimes::php::select::{Selection, select};

/// A pasta do Sentinel: se o atalho está em `<home>\bin`, é `<home>`;
/// senão, a padrão (ou `SENTINEL_HOME`).
fn sentinel_paths() -> Option<Paths> {
    let exe = std::env::current_exe().ok()?;
    if let Some(bin) = exe.parent()
        && bin.file_name().is_some_and(|n| n.eq_ignore_ascii_case("bin"))
        && let Some(home) = bin.parent()
        && home.join("runtimes").is_dir()
    {
        return Some(Paths::at(home));
    }
    Paths::default_root().ok()
}

/// Como o atalho foi chamado: `php`, `composer`…
fn tool_name() -> String {
    std::env::args_os()
        .next()
        .map(PathBuf::from)
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

fn fail(message: &str) -> ExitCode {
    eprintln!("sentinel: {message}");
    ExitCode::from(1)
}

fn which_json(sel: &Selection) -> String {
    serde_json::json!({
        "version": sel.php.as_ref().map(|p| p.version.clone()),
        "exe": sel.php.as_ref().map(|p| p.php_exe().display().to_string()),
        "reason": sel.reason,
        "project": sel.project,
        "request": sel.request,
    })
    .to_string()
}

/// O terminal manda Ctrl+C para todos os processos da janela: o atalho
/// ignora e deixa o PHP decidir, esperando ele terminar.
fn ignore_ctrl_c() {
    #[cfg(windows)]
    // SAFETY: registra "ignorar Ctrl+C" para este processo; sem ponteiros.
    unsafe {
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(None, 1);
    }
}

fn run(program: &Path, args: Vec<std::ffi::OsString>) -> ExitCode {
    ignore_ctrl_c();
    match Command::new(program).args(args).status() {
        Ok(status) => match status.code() {
            Some(code) => ExitCode::from((code & 0xff) as u8),
            None => ExitCode::from(1),
        },
        Err(e) => fail(&t!("não foi possível executar {a}: {e}", a = program.display(), e = e)),
    }
}

/// node / npm / npx: escolhe o Node da pasta e roda.
fn run_node(tool: &str, paths: &Paths, cwd: &Path) -> ExitCode {
    let sel = sentinel_runtimes::node::select(paths, cwd);
    if std::env::var_os("SENTINEL_SHIM_WHICH").is_some() {
        println!(
            "{}",
            serde_json::json!({
                "version": sel.node.as_ref().map(|n| n.version.clone()),
                "reason": sel.reason,
                "project": sel.project,
            })
        );
        return ExitCode::SUCCESS;
    }
    let Some(node) = sel.node else {
        return fail(&t!("{reason}. Instale em Sentinel › Configurações › Node.js.", reason = sel.reason));
    };
    if std::env::var_os("SENTINEL_SHIM_DEBUG").is_some() {
        eprintln!("sentinel: Node {} — {}", node.version, sel.reason);
    }
    // Scripts do npm que chamam "node" acham a mesma versão primeiro.
    let path = format!("{};{}", node.dir.display(), std::env::var("PATH").unwrap_or_default());
    // SAFETY: programa de uma thread só; nada mais lê o ambiente agora.
    unsafe { std::env::set_var("PATH", path) };
    let mut args: Vec<std::ffi::OsString> = Vec::new();
    if tool != "node" {
        args.push(node.npm_cli(tool).into_os_string());
    }
    args.extend(std::env::args_os().skip(1));
    run(&node.node_exe(), args)
}

fn main() -> ExitCode {
    let tool = tool_name();
    let Some(paths) = sentinel_paths() else {
        return fail(&t!("pasta de dados do Sentinel não encontrada"));
    };
    // Mensagens no idioma escolhido no Sentinel.
    if let Ok(cfg) = sentinel_core::AppConfig::load(&paths.config_file())
        && let Some(lang) = cfg.language
    {
        sentinel_core::i18n::set_language(&lang);
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    if matches!(tool.as_str(), "node" | "npm" | "npx") {
        return run_node(&tool, &paths, &cwd);
    }
    let sel = select(&paths, &cwd);

    if std::env::var_os("SENTINEL_SHIM_WHICH").is_some() {
        println!("{}", which_json(&sel));
        return ExitCode::SUCCESS;
    }

    let Some(php) = sel.php.clone() else {
        return fail(&t!("{reason}. Instale em Sentinel › Configurações › PHP.", reason = sel.reason));
    };
    if std::env::var_os("SENTINEL_SHIM_DEBUG").is_some() {
        eprintln!("sentinel: PHP {} — {}", php.version, sel.reason);
    }

    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    match tool.as_str() {
        "composer" => {
            let phar = paths.runtimes_dir().join("composer").join("composer.phar");
            if !phar.is_file() {
                return fail(&t!("Composer não instalado. Instale um PHP pelo Sentinel que ele vem junto."));
            }
            let mut full = vec![phar.into_os_string()];
            full.extend(args);
            run(&php.php_exe(), full)
        }
        // "php", "sentinel-shim" (chamado direto) e qualquer outro nome = PHP.
        _ => run(&php.php_exe(), args),
    }
}
