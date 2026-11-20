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
        if seg == ".." || seg.contains(['\\', ':', '\0']) || hidden {
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
}
