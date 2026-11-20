//! Núcleo do Sentinel: onde ficam os dados, configuração do app e logs.

pub mod config;
pub mod error;
pub mod logging;
pub mod paths;
pub mod projects;
pub mod store;

pub use config::{AppConfig, Theme};
pub use projects::{DetectedProject, ProjectKind};
pub use store::{ProjectConfig, ProjectStatus, ProjectView, ProjectsFile};
pub use error::{Error, Result};
pub use paths::Paths;
