use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}", crate::i18n::tr("não foi possível localizar a pasta de dados do usuário (%LOCALAPPDATA%)"))]
    NoDataDir,

    #[error("{}", crate::i18n::tf("erro de arquivo em {path}: {source}", &[("path", &.path.display() as &dyn std::fmt::Display), ("source", &.source as &dyn std::fmt::Display)]))]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{}", crate::i18n::tf("arquivo de configuração inválido em {path}: {source}", &[("path", &.path.display() as &dyn std::fmt::Display), ("source", &.source as &dyn std::fmt::Display)]))]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("{}", crate::i18n::tf("não foi possível gravar a configuração: {0}", &[("0", &.0 as &dyn std::fmt::Display)]))]
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
