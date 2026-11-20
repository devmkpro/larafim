use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("não foi possível localizar a pasta de dados do usuário (%LOCALAPPDATA%)")]
    NoDataDir,

    #[error("erro de arquivo em {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("arquivo de configuração inválido em {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("não foi possível gravar a configuração: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    #[error("{0}")]
    Invalid(String),
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
