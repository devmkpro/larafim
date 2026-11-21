use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Pastas usadas pelo Sentinel. Tudo fica dentro de uma raiz
/// (`%LOCALAPPDATA%\Sentinel` por padrão), nada é escrito nos projetos.
#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    /// Raiz padrão: `%LOCALAPPDATA%\Sentinel`.
    /// A variável `SENTINEL_HOME` permite trocar a raiz (útil em testes e desenvolvimento).
    pub fn default_root() -> Result<Self> {
        if let Some(home) = std::env::var_os("SENTINEL_HOME") {
            return Ok(Self::at(home));
        }
        let base = dirs::data_local_dir().ok_or(Error::NoDataDir)?;
        Ok(Self::at(base.join("Sentinel")))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn projects_file(&self) -> PathBuf {
        self.root.join("projects.toml")
    }

    pub fn runtimes_dir(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn php_dir(&self) -> PathBuf {
        self.runtimes_dir().join("php")
    }

    pub fn node_dir(&self) -> PathBuf {
        self.runtimes_dir().join("node")
    }

    pub fn mariadb_dir(&self) -> PathBuf {
        self.runtimes_dir().join("mariadb")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join("downloads")
    }

    /// Cria as pastas básicas (idempotente).
    pub fn ensure(&self) -> Result<()> {
        for dir in [
            self.root.clone(),
            self.runtimes_dir(),
            self.data_dir(),
            self.logs_dir(),
            self.bin_dir(),
            self.downloads_dir(),
        ] {
            std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_creates_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::at(tmp.path().join("Sentinel"));
        paths.ensure().unwrap();
        assert!(paths.logs_dir().is_dir());
        assert!(paths.runtimes_dir().is_dir());
        // segunda chamada não falha
        paths.ensure().unwrap();
    }
}
