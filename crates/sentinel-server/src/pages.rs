//! Páginas HTML que o próprio Sentinel mostra (projeto offline, não
//! encontrado, erro do PHP). Simples, claras e no tema do sistema.
use sentinel_core::t;

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

fn page(title: &str, message: &str, extra: &str) -> String {
    format!(
        r#"<!doctype html><html lang="{lang}"><head><meta charset="utf-8"><title>{title} — Sentinel</title>
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
        lang = sentinel_core::i18n::language(),
    )
}

/// Domínio que não pertence a nenhum projeto.
pub fn unknown_site(host: &str, online: &[String]) -> String {
    let list = if online.is_empty() {
        t!("<p>Nenhum projeto está online agora.</p>")
    } else {
        let items: String = online
            .iter()
            .map(|h| format!(r#"<li><a href="http://{0}">{0}</a></li>"#, escape(h)))
            .collect();
        format!("<p>{}</p><ul>{items}</ul>", t!("Projetos online:"))
    };
    page(
        &t!("Projeto não encontrado"),
        &t!("Nenhum projeto do Sentinel usa o endereço <code>{host}</code>.", host = escape(host)),
        &list,
    )
}

pub fn offline(name: &str) -> String {
    page(
        &t!("Projeto offline"),
        &t!(
            "O projeto <strong>{name}</strong> está desligado. Ligue-o no Sentinel (interruptor Online) e recarregue esta página.",
            name = escape(name)
        ),
        "",
    )
}

pub fn forbidden() -> String {
    page(&t!("Acesso negado"), &t!("Este arquivo não é servido pelo Sentinel (arquivos ocultos como .env ficam protegidos)."), "")
}

pub fn not_found(path: &str) -> String {
    page(&t!("Página não encontrada"), &t!("Não existe nada em <code>{path}</code>.", path = escape(path)), "")
}

pub fn no_php(name: &str) -> String {
    page(
        &t!("PHP não instalado"),
        &t!(
            "O projeto <strong>{name}</strong> precisa de uma versão do PHP que não está instalada. Instale em Sentinel › Configurações › PHP.",
            name = escape(name)
        ),
        "",
    )
}

pub fn php_error(detail: &str) -> String {
    page(
        &t!("O PHP não respondeu"),
        &t!("O Sentinel não conseguiu executar o PHP deste projeto. Detalhes:"),
        &format!("<pre>{}</pre>", escape(detail)),
    )
}

/// Projeto Node ligado, servidor de dev ainda subindo (recarrega sozinha).
pub fn node_starting(name: &str) -> String {
    page(
        &t!("Iniciando…"),
        &t!(
            "O servidor de desenvolvimento do projeto <strong>{name}</strong> está subindo. Esta página recarrega sozinha.",
            name = escape(name)
        ),
        "<script>setTimeout(()=>location.reload(),1500)</script>",
    )
}

pub fn proxy_error(port: u16, detail: &str) -> String {
    page(
        &t!("Servidor do Node não respondeu"),
        &t!("O Sentinel não conseguiu falar com o servidor de desenvolvimento na porta {port}.", port = port),
        &format!("<pre>{}</pre>", escape(detail)),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn pages_follow_the_language() {
        sentinel_core::i18n::set_language("en");
        let html = super::unknown_site("x.offline", &[]);
        assert!(html.contains(r#"<html lang="en">"#));
        assert!(html.contains("Project not found"), "{html}");
        assert!(html.contains("No Sentinel project uses the address <code>x.offline</code>."));
        assert!(super::offline("loja").contains("The project <strong>loja</strong> is off."));
        sentinel_core::i18n::set_language("pt-BR");
        assert!(super::unknown_site("x.offline", &[]).contains("Projeto não encontrado"));
    }

    /// Tudo que vem da requisição (domínio, caminho) ou do PHP (mensagem de
    /// erro) aparece escapado: nada vira HTML/JS na página do Sentinel.
    #[test]
    fn request_data_is_escaped() {
        let evil = r#"<script>alert('x')</script>"><img src=x onerror=alert(1)>"#;
        for html in [
            super::unknown_site(evil, &[]),
            super::not_found(evil),
            super::php_error(evil),
            super::offline(evil),
            super::no_php(evil),
            super::node_starting(evil),
            super::proxy_error(3000, evil),
        ] {
            assert!(!html.contains("<script>alert"), "{html}");
            assert!(!html.contains("<img src=x"), "{html}");
            assert!(!html.contains("'x'"), "{html}");
            assert!(html.contains("&lt;script&gt;"), "{html}");
        }
    }
}
