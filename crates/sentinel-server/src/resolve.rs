//! Decide o que uma URL vira dentro da pasta pública do projeto:
//! arquivo estático, script PHP ou o front controller (`index.php`).

use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    Static(PathBuf),
    Php {
        script: PathBuf,
        /// Caminho do script na URL (`/index.php`, `/admin/x.php`).
        script_name: String,
    },
    /// Caminho com `..`, arquivo oculto (`.env`, `.git`) etc.
    Forbidden,
    NotFound,
}

/// Nomes que o Windows trata de um jeito especial e que serviriam para
/// driblar as regras acima:
/// - `info.php.` / `info.php ` (ponto ou espaço no fim): o Windows abre
///   `info.php`, mas a extensão some — o código seria entregue como texto;
/// - `ENV~1` (nome curto 8.3): alcança `.env` e outros arquivos ocultos;
/// - `x:stream` (fluxo alternativo do NTFS), `\`, caracteres de controle e
///   os proibidos em nomes (`<>"|?*`);
/// - nomes de dispositivo (`CON`, `NUL`, `COM1`…).
fn windows_trick(seg: &str) -> bool {
    let short_name = seg.char_indices().any(|(i, c)| c == '~' && seg[i + 1..].starts_with(|n: char| n.is_ascii_digit()));
    seg.ends_with(['.', ' '])
        || short_name
        || seg.chars().any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
        || sentinel_core::safety::is_reserved_windows_name(seg)
}

/// `url_path` é o caminho da URL sem a query string.
pub fn resolve(docroot: &Path, url_path: &str, php: bool) -> Target {
    let decoded = percent_encoding::percent_decode_str(url_path).decode_utf8_lossy();
    let mut segments = Vec::new();
    for seg in decoded.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        // Nada de subir pastas, caminhos do Windows ou arquivos ocultos.
        let hidden = seg.starts_with('.') && seg != ".well-known";
        if seg == ".." || hidden || windows_trick(seg) {
            return Target::Forbidden;
        }
        segments.push(seg.to_string());
    }

    let mut path = docroot.to_path_buf();
    for seg in &segments {
        path.push(seg);
    }
    let url_of = |segs: &[String]| format!("/{}", segs.join("/"));
    let is_php = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("php"));

    if path.is_file() {
        if is_php(&path) {
            return if php {
                Target::Php { script: path, script_name: url_of(&segments) }
            } else {
                Target::Forbidden // não entrega código PHP como texto
            };
        }
        return Target::Static(path);
    }

    if path.is_dir() {
        if php && path.join("index.php").is_file() {
            let mut segs = segments.clone();
            segs.push("index.php".into());
            return Target::Php { script: path.join("index.php"), script_name: url_of(&segs) };
        }
        if path.join("index.html").is_file() {
            return Target::Static(path.join("index.html"));
        }
    }

    // Front controller (Laravel, Symfony, WordPress…): tudo o que não é
    // arquivo vai para o index.php da pasta pública.
    if php && docroot.join("index.php").is_file() {
        return Target::Php { script: docroot.join("index.php"), script_name: "/index.php".into() };
    }
    Target::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn site() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path();
        fs::create_dir_all(p.join("css")).unwrap();
        fs::create_dir_all(p.join("admin")).unwrap();
        fs::create_dir_all(p.join(".well-known")).unwrap();
        fs::write(p.join("index.php"), "<?php").unwrap();
        fs::write(p.join("info.php"), "<?php").unwrap();
        fs::write(p.join("css").join("app.css"), "").unwrap();
        fs::write(p.join("admin").join("index.php"), "<?php").unwrap();
        fs::write(p.join(".env"), "SEGREDO=1").unwrap();
        fs::write(p.join(".well-known").join("x.txt"), "").unwrap();
        fs::write(p.join("foto de férias.png"), "").unwrap();
        tmp
    }

    #[test]
    fn routes() {
        let s = site();
        let root = s.path();
        assert_eq!(resolve(root, "/css/app.css", true), Target::Static(root.join("css").join("app.css")));
        assert_eq!(
            resolve(root, "/info.php", true),
            Target::Php { script: root.join("info.php"), script_name: "/info.php".into() }
        );
        assert_eq!(
            resolve(root, "/", true),
            Target::Php { script: root.join("index.php"), script_name: "/index.php".into() }
        );
        assert_eq!(
            resolve(root, "/admin/", true),
            Target::Php { script: root.join("admin").join("index.php"), script_name: "/admin/index.php".into() }
        );
        // rota do Laravel → front controller
        assert_eq!(
            resolve(root, "/produtos/42", true),
            Target::Php { script: root.join("index.php"), script_name: "/index.php".into() }
        );
        assert_eq!(resolve(root, "/foto%20de%20f%C3%A9rias.png", true), Target::Static(root.join("foto de férias.png")));
    }

    #[test]
    fn protections() {
        let s = site();
        let root = s.path();
        assert_eq!(resolve(root, "/.env", true), Target::Forbidden);
        assert_eq!(resolve(root, "/../segredo.txt", true), Target::Forbidden);
        assert_eq!(resolve(root, "/%2e%2e/segredo.txt", true), Target::Forbidden);
        assert_eq!(resolve(root, "/css/..%5C..%5Cwin.ini", true), Target::Forbidden);
        assert_eq!(resolve(root, "/C:/Windows/win.ini", true), Target::Forbidden);
        assert_eq!(resolve(root, "/info.php", false), Target::Forbidden, "sem PHP, não entrega o código");
        assert_eq!(resolve(root, "/.well-known/x.txt", true), Target::Static(root.join(".well-known").join("x.txt")));
        assert_eq!(resolve(root, "/nada", false), Target::NotFound);
    }

    /// Truques de nome do Windows que levariam ao código PHP ou ao `.env`.
    #[test]
    fn windows_name_tricks() {
        let s = site();
        let root = s.path();
        for url in [
            "/info.php.",            // o Windows abre info.php, mas sem a extensão .php
            "/info.php%20",          // idem, com espaço
            "/info.php%2e",          // ponto codificado
            "/info.php::$DATA",      // fluxo do NTFS
            "/info.php%3A%3A$DATA",
            "/ENV~1",                // nome curto de .env
            "/env~1",
            "/admin~1/index.php",
            "/CON",
            "/aux.txt",
            "/css/nul",
            "/x%00.png",             // byte nulo
            "/a%0Ab",                // quebra de linha
            "/*.php",
            "/in<fo>.php",
            "/%22quoted%22",
            "/a%7Cb",
        ] {
            assert_eq!(resolve(root, url, true), Target::Forbidden, "{url}");
        }
        // Nome com til que não é nome curto continua valendo.
        fs::write(root.join("versao~beta.txt"), "").unwrap();
        assert_eq!(resolve(root, "/versao~beta.txt", true), Target::Static(root.join("versao~beta.txt")));
    }
}
