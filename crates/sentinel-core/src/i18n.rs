//! Tradução (i18n). A chave é o próprio texto em português (estilo gettext):
//! `t!("Projeto {name} criado", name = n)`. Cada idioma é um JSON em
//! `locales/<código>.json` mapeando o texto em português para a tradução;
//! o que faltar aparece em português. O português (`pt-BR`) não precisa de
//! entradas.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

/// Idioma padrão e das chaves.
pub const SOURCE: &str = "pt-BR";

struct Lang {
    code: String,
    name: String,
    nsis_langid: Option<u32>,
    map: HashMap<String, String>,
}

fn all() -> &'static Vec<Lang> {
    static ALL: OnceLock<Vec<Lang>> = OnceLock::new();
    ALL.get_or_init(|| {
        LOCALES
            .iter()
            .map(|(code, json)| {
                let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
                let meta = &v["_meta"];
                let map = v
                    .as_object()
                    .map(|o| {
                        o.iter()
                            .filter(|(k, _)| !k.starts_with('_'))
                            .filter_map(|(k, val)| val.as_str().filter(|s| !s.is_empty()).map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default();
                Lang {
                    code: code.to_string(),
                    name: meta["name"].as_str().unwrap_or(code).to_string(),
                    nsis_langid: meta["langid"].as_u64().map(|n| n as u32),
                    map,
                }
            })
            .collect()
    })
}

static CURRENT: RwLock<Option<usize>> = RwLock::new(None);

/// (código, nome) dos idiomas disponíveis.
pub fn languages() -> Vec<(String, String)> {
    all().iter().map(|l| (l.code.clone(), l.name.clone())).collect()
}

pub fn is_supported(code: &str) -> bool {
    all().iter().any(|l| l.code == code)
}

/// Troca o idioma do processo (mensagens do backend).
pub fn set_language(code: &str) {
    *CURRENT.write().unwrap() = all().iter().position(|l| l.code == code);
}

pub fn language() -> String {
    CURRENT.read().unwrap().map(|i| all()[i].code.clone()).unwrap_or_else(|| SOURCE.to_string())
}

/// Idioma escolhido no instalador (código de idioma do Windows → código do Sentinel).
pub fn from_windows_langid(langid: u32) -> Option<String> {
    all().iter().find(|l| l.nsis_langid == Some(langid)).map(|l| l.code.clone())
}

/// Melhor idioma para um código do sistema ("pt-BR", "en-US", "es-ES"…).
pub fn best_match(system: &str) -> String {
    let lower = system.to_ascii_lowercase();
    if let Some(l) = all().iter().find(|l| l.code.to_ascii_lowercase() == lower) {
        return l.code.clone();
    }
    let prefix = lower.split(['-', '_']).next().unwrap_or_default();
    all()
        .iter()
        .find(|l| l.code.to_ascii_lowercase().split('-').next() == Some(prefix))
        .map(|l| l.code.clone())
        .unwrap_or_else(|| if prefix == "pt" { SOURCE.to_string() } else { "en".to_string() })
}

/// Tradução do texto (ou o próprio texto, se não houver).
pub fn tr(key: &str) -> String {
    let current = *CURRENT.read().unwrap();
    current.and_then(|i| all()[i].map.get(key).cloned()).unwrap_or_else(|| key.to_string())
}

/// Tradução com parâmetros nomeados (`{nome}`), para quem não pode usar
/// a macro (ex.: mensagens de `#[error(...)]`).
pub fn tf(key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut s = tr(key);
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), &value.to_string());
    }
    s
}

/// `t!("texto")` ou `t!("texto com {nome}", nome = valor)`.
#[macro_export]
macro_rules! t {
    ($key:literal) => {
        $crate::i18n::tr($key)
    };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut s = $crate::i18n::tr($key);
        $( s = s.replace(concat!("{", stringify!($name), "}"), &($value).to_string()); )+
        s
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_locale_is_valid_json_with_meta() {
        for (code, json) in LOCALES {
            let v: serde_json::Value = serde_json::from_str(json).unwrap_or_else(|e| panic!("locales/{code}.json inválido: {e}"));
            assert!(v["_meta"]["name"].is_string(), "locales/{code}.json sem _meta.name");
        }
    }

    #[test]
    fn translate_with_fallback_and_params() {
        assert!(is_supported("pt-BR") && is_supported("en"));
        set_language("en");
        assert_eq!(crate::t!("texto que não existe {x}", x = 3), "texto que não existe 3");
        assert_eq!(best_match("en-US"), "en");
        assert_eq!(best_match("pt-PT"), "pt-BR");
        assert_eq!(best_match("ja-JP"), "en");
        assert_eq!(from_windows_langid(1046).as_deref(), Some("pt-BR"));
        assert_eq!(from_windows_langid(1033).as_deref(), Some("en"));
        set_language("pt-BR");
        assert_eq!(language(), "pt-BR");
    }
}
