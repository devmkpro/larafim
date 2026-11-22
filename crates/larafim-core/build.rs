//! Inclui todos os idiomas de `locales/*.json` no binário. Para um idioma
//! novo basta criar o arquivo: nada a registrar aqui.

use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../locales");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries: Vec<(String, PathBuf)> = std::fs::read_dir(&dir)
        .expect("pasta locales/")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| (p.file_stem().unwrap().to_string_lossy().to_string(), p.canonicalize().unwrap()))
        .collect();
    entries.sort();
    let mut out = String::from("/// (código, conteúdo JSON) de cada idioma em `locales/`.\npub static LOCALES: &[(&str, &str)] = &[\n");
    for (code, path) in &entries {
        println!("cargo:rerun-if-changed={}", path.display());
        out.push_str(&format!("    ({code:?}, include_str!({:?})),\n", path.display().to_string()));
    }
    out.push_str("];\n");
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("locales.rs"), out).unwrap();
}
