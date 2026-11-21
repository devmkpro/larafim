## O que muda

<!-- O que e por quê. Link para a issue, se houver. -->

## Checklist

- [ ] `cargo clippy --workspace --all-targets -- -D warnings` sem avisos
- [ ] `cargo test --workspace` passando
- [ ] `npm run check` (em `app/`) sem erros
- [ ] `node scripts/i18n.mjs check` sem textos faltando
- [ ] Documentação em `docs/` atualizada, se o comportamento mudou

### Segurança

- [ ] A mudança lê dado de fora (download, catálogo, arquivo de projeto, requisição, argumento da interface)?
      Se sim: é validado antes de virar caminho/arquivo/SQL/comando/HTML, e há teste com entrada maliciosa.
- [ ] Não afrouxa CSP, *capabilities* do Tauri, hosts de download confiáveis nem a validação do `hosts`.
- [ ] Nenhum segredo, token ou dado pessoal no código, nos testes ou nos logs.
