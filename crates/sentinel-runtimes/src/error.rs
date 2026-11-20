use std::path::PathBuf;

/// Erros com mensagens prontas para mostrar na interface.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sem conexão ou servidor indisponível: {0}")]
    Http(#[source] reqwest::Error),

    #[error("o servidor respondeu {status} para {url}")]
    HttpStatus { status: u16, url: String },

    #[error("o catálogo de versões do PHP veio num formato inesperado: {0}")]
    Catalog(String),

    #[error("a versão {0} não está no catálogo oficial")]
    UnknownVersion(String),

    #[error("o arquivo baixado está corrompido (SHA-256 não confere). Tente de novo.")]
    ChecksumMismatch,

    #[error("download cancelado")]
    Cancelled,

    #[error("erro de arquivo em {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("não foi possível extrair o arquivo: {0}")]
    Zip(String),

    #[error(
        "o PHP precisa do Microsoft Visual C++ Redistributable (x64) mais recente. \
         Instale-o e tente de novo."
    )]
    MissingVcRedist,

    #[error("o PHP instalado não respondeu como esperado: {0}")]
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
