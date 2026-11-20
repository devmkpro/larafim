//! PHP para Windows: catálogo oficial, versões instaladas, instalação e php.ini.

pub mod catalog;
pub mod ini;
pub mod install;
pub mod installed;
pub mod select;

pub use catalog::{CatalogEntry, fetch_catalog};
pub use install::{Stage, install, remove_minor};
pub use installed::{InstalledPhp, best_installed, list_installed, matches_constraint};

/// "8.4.28" → (8, 4, 28). Versões do PHP são sempre x.y.z.
pub fn parse_version(v: &str) -> Option<semver::Version> {
    semver::Version::parse(v.trim()).ok()
}

/// "8.4.28" → "8.4".
pub fn minor_of(version: &str) -> Option<String> {
    let v = parse_version(version)?;
    Some(format!("{}.{}", v.major, v.minor))
}
