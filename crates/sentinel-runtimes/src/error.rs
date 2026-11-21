use std::path::PathBuf;

/// Erros com mensagens prontas para mostrar na interface.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}", sentinel_core::i18n::tf("sem conexão ou servidor indisponível: {0}", &[("0", &.0 as &dyn std::fmt::Display)]))]
    Http(#[source] reqwest::Error),

    #[error("{}", sentinel_core::i18n::tf("o servidor respondeu {status} para {url}", &[("status", &.status as &dyn std::fmt::Display), ("url", &.url as &dyn std::fmt::Display)]))]
    HttpStatus { status: u16, url: String },

    #[error("{}", sentinel_core::i18n::tf("o catálogo de versões do PHP veio num formato inesperado: {0}", &[("0", &.0 as &dyn std::fmt::Display)]))]
    Catalog(String),

    #[error("{}", sentinel_core::i18n::tf("a versão {0} não está no catálogo oficial", &[("0", &.0 as &dyn std::fmt::Display)]))]
    UnknownVersion(String),

    #[error("{}", sentinel_core::i18n::tr("o arquivo baixado está corrompido (SHA-256 não confere). Tente de novo."))]
    ChecksumMismatch,

    #[error("{}", sentinel_core::i18n::tr("download cancelado"))]
    Cancelled,

    #[error("{}", sentinel_core::i18n::tf("erro de arquivo em {path}: {source}", &[("path", &.path.display() as &dyn std::fmt::Display), ("source", &.source as &dyn std::fmt::Display)]))]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{}", sentinel_core::i18n::tf("não foi possível extrair o arquivo: {0}", &[("0", &.0 as &dyn std::fmt::Display)]))]
    Zip(String),

    #[error("{}", sentinel_core::i18n::tr("o PHP precisa do Microsoft Visual C++ Redistributable (x64) mais recente. Instale-o e tente de novo."))]
    MissingVcRedist,

    #[error("{}", sentinel_core::i18n::tf("o PHP instalado não respondeu como esperado: {0}", &[("0", &.0 as &dyn std::fmt::Display)]))]
    Verify(String),

    #[error("{0}")]
    Invalid(String),
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }

    /// Código curto para a interface decidir o que oferecer (ex.: link do VC++).
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingVcRedist => "vc_redist",
            Self::Cancelled => "cancelled",
            Self::Http(_) | Self::HttpStatus { .. } => "network",
            _ => "error",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
