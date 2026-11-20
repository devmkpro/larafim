use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Segue o tema do Windows.
    #[default]
    System,
    Light,
    Dark,
}

/// Preferências globais do Sentinel (`config.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme: Theme,
    /// Sufixo dos domínios locais, sem ponto. Ex.: `offline` → `loja.offline`.
    pub tld: String,
    pub http_port: u16,
    /// Pastas raiz onde cada subpasta é candidata a projeto.
    pub roots: Vec<PathBuf>,
    pub start_with_windows: bool,
    /// Ao fechar a janela, o Sentinel continua na bandeja.
    pub keep_in_tray: bool,
    pub onboarding_done: bool,
    /// Minor do PHP usado no terminal fora de projetos ("8.4"). Vazio = mais novo instalado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_php: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            tld: "offline".into(),
            http_port: 80,
            roots: Vec::new(),
            start_with_windows: false,
            keep_in_tray: true,
            onboarding_done: false,
            default_php: None,
        }
    }
}

impl AppConfig {
    /// Lê o arquivo; se não existir, devolve o padrão.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|source| Error::ConfigParse {
                path: path.to_path_buf(),
                source,
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    /// Grava de forma atômica (arquivo temporário + renomear), para nunca
    /// deixar uma configuração pela metade se o app fechar no meio.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let text = toml::to_string_pretty(self)?;
        write_atomic(path, text.as_bytes())
    }

    pub fn validate(&self) -> Result<()> {
        let tld = self.tld.trim();
        let valid_tld = !tld.is_empty()
            && tld.len() <= 63
            && tld.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !tld.starts_with('-')
            && !tld.ends_with('-');
        if !valid_tld {
            return Err(Error::Invalid(format!(
                "domínio \".{}\" inválido: use só letras minúsculas, números e hífen",
                self.tld
            )));
        }
        if self.http_port == 0 {
            return Err(Error::Invalid("a porta HTTP não pode ser 0".into()));
        }
        Ok(())
    }

    /// URL completa de um projeto, com a porta quando não for a 80.
    pub fn project_url(&self, name: &str) -> String {
        if self.http_port == 80 {
            format!("http://{name}.{}", self.tld)
        } else {
            format!("http://{name}.{}:{}", self.tld, self.http_port)
        }
    }
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = AppConfig::load(&tmp.path().join("config.toml")).unwrap();
        assert_eq!(cfg, AppConfig::default());
        assert_eq!(cfg.tld, "offline");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sub").join("config.toml");
        let cfg = AppConfig {
            theme: Theme::Dark,
            roots: vec![PathBuf::from(r"D:\www")],
            http_port: 8080,
            ..Default::default()
        };
        cfg.save(&path).unwrap();
        assert_eq!(AppConfig::load(&path).unwrap(), cfg);
    }

    #[test]
    fn partial_file_fills_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "theme = \"light\"\n").unwrap();
        let cfg = AppConfig::load(&path).unwrap();
        assert_eq!(cfg.theme, Theme::Light);
        assert_eq!(cfg.tld, "offline");
        assert!(cfg.keep_in_tray);
    }

    #[test]
    fn rejects_bad_tld() {
        let cfg = AppConfig { tld: ".Off line".into(), ..Default::default() };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn project_url_includes_port_only_when_needed() {
        let mut cfg = AppConfig::default();
        assert_eq!(cfg.project_url("loja"), "http://loja.offline");
        cfg.http_port = 8080;
        assert_eq!(cfg.project_url("loja"), "http://loja.offline:8080");
    }
}
