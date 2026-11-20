//! Páginas HTML que o próprio Sentinel mostra (projeto offline, não
//! encontrado, erro do PHP). Simples, claras e no tema do sistema.

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn page(title: &str, message: &str, extra: &str) -> String {
    format!(
        r#"<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>{title} — Sentinel</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
:root{{color-scheme:light dark;--bg:#f3f3f3;--card:#fff;--text:#1a1a1a;--muted:#5c5c5c;--accent:#0f7b74;--border:#e5e5e5}}
@media (prefers-color-scheme:dark){{:root{{--bg:#202020;--card:#2b2b2b;--text:#fff;--muted:#cfcfcf;--accent:#4cc2b8;--border:#363636}}}}
body{{margin:0;min-height:100vh;display:grid;place-items:center;background:var(--bg);color:var(--text);font:14px/1.5 "Segoe UI Variable Text","Segoe UI",system-ui,sans-serif}}
main{{max-width:560px;margin:24px;padding:28px 32px;background:var(--card);border:1px solid var(--border);border-radius:8px}}
h1{{margin:0 0 8px;font:600 22px/1.3 "Segoe UI Variable Display","Segoe UI",sans-serif}}
p{{margin:0 0 12px;color:var(--muted)}} code{{font-family:"Cascadia Mono",Consolas,monospace;font-size:13px}}
ul{{margin:8px 0 0;padding-left:20px}} a{{color:var(--accent)}} .brand{{font-size:12px;color:var(--muted);margin-top:20px}}
pre{{white-space:pre-wrap;background:var(--bg);padding:12px;border-radius:6px;font:12px/1.5 "Cascadia Mono",Consolas,monospace;max-height:320px;overflow:auto}}
</style></head><body><main><h1>{title}</h1><p>{message}</p>{extra}<div class="brand">Sentinel</div></main></body></html>"#,
        title = escape(title),
    )
}

/// Domínio que não pertence a nenhum projeto.
pub fn unknown_site(host: &str, online: &[String]) -> String {
    let list = if online.is_empty() {
        "<p>Nenhum projeto está online agora.</p>".to_string()
    } else {
        let items: String = online
            .iter()
            .map(|h| format!(r#"<li><a href="http://{0}">{0}</a></li>"#, escape(h)))
            .collect();
        format!("<p>Projetos online:</p><ul>{items}</ul>")
    };
    page(
        "Projeto não encontrado",
        &format!("Nenhum projeto do Sentinel usa o endereço <code>{}</code>.", escape(host)),
        &list,
    )
}

pub fn offline(name: &str) -> String {
    page(
        "Projeto offline",
        &format!(
            "O projeto <strong>{}</strong> está desligado. Ligue-o no Sentinel (interruptor Online) e recarregue esta página.",
            escape(name)
        ),
        "",
    )
}

pub fn forbidden() -> String {
    page("Acesso negado", "Este arquivo não é servido pelo Sentinel (arquivos ocultos como .env ficam protegidos).", "")
}

pub fn not_found(path: &str) -> String {
    page("Página não encontrada", &format!("Não existe nada em <code>{}</code>.", escape(path)), "")
}

pub fn no_php(name: &str) -> String {
    page(
        "PHP não instalado",
        &format!(
            "O projeto <strong>{}</strong> precisa de uma versão do PHP que não está instalada. Instale em Sentinel › Configurações › PHP.",
            escape(name)
        ),
        "",
    )
}

pub fn php_error(detail: &str) -> String {
    page(
        "O PHP não respondeu",
        "O Sentinel não conseguiu executar o PHP deste projeto. Detalhes:",
        &format!("<pre>{}</pre>", escape(detail)),
    )
}
