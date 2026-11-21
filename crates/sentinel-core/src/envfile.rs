//! Edição segura do `.env` de um projeto: mostra o que muda, guarda uma
//! cópia antes da primeira alteração e mexe só nas chaves pedidas,
//! preservando comentários, ordem e o resto do arquivo.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};

pub const BACKUP_NAME: &str = ".env.sentinel.bak";

/// Uma mudança: `value: None` = comentar a linha (desativar a chave).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvChange {
    pub key: String,
    pub value: Option<String>,
}

impl EnvChange {
    pub fn set(key: &str, value: &str) -> Self {
        Self { key: key.into(), value: Some(value.into()) }
    }
    pub fn unset(key: &str) -> Self {
        Self { key: key.into(), value: None }
    }
}

/// Linha antes → depois, para a pessoa revisar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffLine {
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

fn key_of(line: &str) -> Option<&str> {
    let t = line.trim_start();
    let t = t.strip_prefix("export ").unwrap_or(t);
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    (!k.is_empty() && !k.starts_with('#') && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then_some(k)
}

/// Valor atual de uma chave (ativa, não comentada).
pub fn get(text: &str, key: &str) -> Option<String> {
    text.lines().find(|l| key_of(l) == Some(key)).map(|l| {
        let v = l.split_once('=').map(|(_, v)| v.trim()).unwrap_or_default();
        v.trim_matches('"').to_string()
    })
}

fn quote(v: &str) -> String {
    if v.is_empty() || v.chars().all(|c| c.is_ascii_alphanumeric() || "._-/:@".contains(c)) {
        v.to_string()
    } else {
        format!("\"{}\"", v.replace('"', "\\\""))
    }
}

/// Aplica as mudanças e devolve (texto novo, o que mudou).
pub fn apply(text: &str, changes: &[EnvChange]) -> (String, Vec<DiffLine>) {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let mut diff = Vec::new();

    for ch in changes {
        let before = get(&lines.join("\n"), &ch.key);
        if before == ch.value {
            continue;
        }
        let idx = lines.iter().position(|l| key_of(l) == Some(ch.key.as_str()));
        match (&ch.value, idx) {
            (Some(v), Some(i)) => lines[i] = format!("{}={}", ch.key, quote(v)),
            (Some(v), None) => {
                // Reaproveita uma linha comentada da mesma chave, se houver.
                let commented = lines.iter().position(|l| {
                    l.trim_start().strip_prefix('#').map(str::trim_start).and_then(key_of) == Some(ch.key.as_str())
                });
                match commented {
                    Some(i) => lines[i] = format!("{}={}", ch.key, quote(v)),
                    None => lines.push(format!("{}={}", ch.key, quote(v))),
                }
            }
            (None, Some(i)) => lines[i] = format!("# {}", lines[i]),
            (None, None) => continue,
        }
        diff.push(DiffLine { key: ch.key.clone(), before, after: ch.value.clone() });
    }
    let mut out = lines.join(newline);
    if text.ends_with('\n') || text.is_empty() {
        out.push_str(newline);
    }
    (out, diff)
}

pub fn env_path(project: &Path) -> PathBuf {
    project.join(".env")
}

/// Só calcula o que mudaria (para a tela de confirmação).
pub fn preview(project: &Path, changes: &[EnvChange]) -> Result<Vec<DiffLine>> {
    let text = std::fs::read_to_string(env_path(project)).unwrap_or_default();
    Ok(apply(&text, changes).1)
}

/// Grava as mudanças. Antes da primeira alteração feita pelo Sentinel,
/// guarda o `.env` original em `.env.sentinel.bak`.
pub fn write(project: &Path, changes: &[EnvChange]) -> Result<Vec<DiffLine>> {
    let path = env_path(project);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let (new_text, diff) = apply(&text, changes);
    if diff.is_empty() {
        return Ok(diff);
    }
    let backup = project.join(BACKUP_NAME);
    if !backup.exists() && path.exists() {
        std::fs::copy(&path, &backup).map_err(|e| Error::io(&backup, e))?;
    }
    std::fs::write(&path, new_text).map_err(|e| Error::io(&path, e))?;
    Ok(diff)
}

/// Volta o `.env` para a cópia feita antes da primeira alteração.
pub fn undo(project: &Path) -> Result<()> {
    let backup = project.join(BACKUP_NAME);
    if !backup.is_file() {
        return Err(Error::Invalid("não há cópia do .env para restaurar".into()));
    }
    std::fs::copy(&backup, env_path(project)).map_err(|e| Error::io(&backup, e))?;
    std::fs::remove_file(&backup).map_err(|e| Error::io(&backup, e))
}

pub fn has_backup(project: &Path) -> bool {
    project.join(BACKUP_NAME).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENV: &str = "APP_NAME=Loja\r\n# banco\r\nDB_CONNECTION=sqlite\r\n# DB_HOST=127.0.0.1\r\nDB_DATABASE=\"laravel\"\r\nMAIL_MAILER=log\r\n";

    #[test]
    fn apply_keeps_rest_and_reports_diff() {
        let (out, diff) = apply(
            ENV,
            &[
                EnvChange::set("DB_CONNECTION", "mariadb"),
                EnvChange::set("DB_HOST", "127.0.0.1"),
                EnvChange::set("DB_PORT", "3307"),
                EnvChange::set("DB_DATABASE", "loja_virtual"),
                EnvChange::set("DB_PASSWORD", ""),
                EnvChange::set("MAIL_MAILER", "log"), // igual: não muda
            ],
        );
        assert!(out.starts_with("APP_NAME=Loja\r\n# banco\r\nDB_CONNECTION=mariadb\r\nDB_HOST=127.0.0.1\r\n"));
        assert!(out.contains("DB_DATABASE=loja_virtual\r\n"));
        assert!(out.ends_with("DB_PORT=3307\r\nDB_PASSWORD=\r\n"));
        let keys: Vec<_> = diff.iter().map(|d| d.key.as_str()).collect();
        assert_eq!(keys, ["DB_CONNECTION", "DB_HOST", "DB_PORT", "DB_DATABASE", "DB_PASSWORD"]);
        assert_eq!(diff[0].before.as_deref(), Some("sqlite"));
        assert_eq!(diff[3].before.as_deref(), Some("laravel"));
    }

    #[test]
    fn unset_comments_line_and_quotes_when_needed() {
        let (out, diff) = apply(ENV, &[EnvChange::unset("DB_DATABASE"), EnvChange::set("APP_NAME", "Minha Loja")]);
        assert!(out.contains("# DB_DATABASE=\"laravel\""));
        assert!(out.contains("APP_NAME=\"Minha Loja\""));
        assert_eq!(diff.len(), 2);
        assert_eq!(get(&out, "DB_DATABASE"), None);
        assert_eq!(get(&out, "APP_NAME").as_deref(), Some("Minha Loja"));
    }

    #[test]
    fn write_backup_and_undo() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".env"), ENV).unwrap();
        write(tmp.path(), &[EnvChange::set("DB_CONNECTION", "mariadb")]).unwrap();
        assert!(has_backup(tmp.path()));
        // Segunda alteração não sobrescreve a cópia original.
        write(tmp.path(), &[EnvChange::set("DB_CONNECTION", "sqlite"), EnvChange::set("X", "1")]).unwrap();
        assert_eq!(std::fs::read_to_string(tmp.path().join(BACKUP_NAME)).unwrap(), ENV);
        undo(tmp.path()).unwrap();
        assert_eq!(std::fs::read_to_string(tmp.path().join(".env")).unwrap(), ENV);
        assert!(!has_backup(tmp.path()));
    }
}
