# Instruções para agentes de IA

Vale para qualquer agente que trabalhe neste repositório (Claude Code, Codex, Copilot, Cursor…). Leia
inteiro antes de mudar algo.

## O projeto

Sentinel: app de bandeja **só para Windows** que gerencia PHP, Node e bancos locais para vários
projetos (como Laravel Herd/Laragon). Rust (Tauri 2) + Svelte 5. Visão geral em
[docs/desenvolvimento/arquitetura.md](docs/desenvolvimento/arquitetura.md).

| Pasta | O que tem |
|---|---|
| `crates/sentinel-core` | config, projetos, `.env`, i18n, **checagens de segurança (`safety.rs`)** |
| `crates/sentinel-runtimes` | catálogos, downloads (`download.rs`), extração (`archive.rs`), instalação de runtimes |
| `crates/sentinel-server` | gateway HTTP/HTTPS, FastCGI, proxy, `hosts`, processos |
| `crates/sentinel-shim` | atalhos `php`/`composer`/`node` do terminal |
| `app/src-tauri` | janelas, bandeja, comandos Tauri |
| `app/src` | interface Svelte (uma pasta por janela; ponte em `lib/*.ts`) |
| `locales/` | traduções |
| `docs/` | documentação (guias de uso, desenvolvimento, segurança) |

## Comandos

```powershell
# na raiz
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node scripts/i18n.mjs check

# em app/
npm ci
npm run check        # tipos
npm run dev          # interface no navegador com dados de exemplo (sem Rust)
npm run tauri dev    # app completo
```

Antes do primeiro `cargo test`/`clippy` numa máquina limpa: `npm run build` em `app/` e
`cargo build -p sentinel-shim --release` (o Tauri embute os dois).

Para testar o app sem tocar nos dados reais do usuário: `$env:SENTINEL_HOME = "<pasta temporária>"`.

## Regras obrigatórias

### Segurança

1. **Todo dado de fora é hostil**: respostas de catálogos e APIs, zips baixados, arquivos dos projetos
   (`composer.json`, `package.json`, `.php-version`, nomes de pastas), requisições HTTP ao gateway,
   argumentos vindos da interface.
2. Valide **antes** de transformar em caminho, linha de arquivo (`hosts`, `.env`, `php.ini`), SQL,
   argumento de processo ou HTML. Use `sentinel_core::safety` (`is_safe_component`, `is_https_url_on`,
   `is_sha256`, `is_env_key`/`is_env_value`, `is_local_url`) ou acrescente uma função lá, com testes.
3. Downloads: só por `download_verified` (exige `https://`, host em `TRUSTED_HOSTS` e SHA-256).
   Extração: só por `archive::extract_zip`. Nunca `ZipArchive::extract` direto.
4. `.env`: só por `envfile::write`/`preview` (validam e escapam). Nunca concatene linhas.
5. `hosts`: só pelo fluxo de `hosts.rs` (o ajudante elevado revalida). Nunca grave o arquivo direto.
6. HTML no gateway: sempre `escape()` (`pages.rs`). Na interface, nunca `{@html}`/`innerHTML`.
7. Processos: prefira `Command::new(exe).arg(..)`. Não monte comandos de `cmd.exe` com texto não validado.
8. Não afrouxe a CSP, as *capabilities* do Tauri nem as listas de hosts sem um motivo explicado no PR.
9. Toda mudança que lê dado de fora ganha um **teste com entrada maliciosa** (`../`, `\r\n`, `"`, `;`,
   `<script>`, nomes do Windows como `CON`/`ARQUIV~1`/`x.php.`). Veja os exemplos em
   [docs/seguranca.md](docs/seguranca.md#testes).
10. Nunca coloque segredos, tokens, chaves ou dados pessoais (nomes, e-mails, caminhos de usuário) em
    código, testes, docs ou commits.

### Produto

- **Leveza.** Nada de processos, timers ou varreduras em segundo plano sem necessidade. Serviços sobem
  sob demanda.
- **Visual nativo do Windows 11** (Fluent, Segoe Fluent Icons, controles de `app.css`), sem cara de
  site. Cores só pelas variáveis CSS (temas claro/escuro).
- **Nunca alterar o projeto do usuário sem confirmação**; `.env` sempre com prévia, cópia
  (`.env.sentinel.bak`) e desfazer.
- **Textos:** todo texto visível com `t("…")` (Svelte) ou `t!("…")`/`tr`/`tf` (Rust), chave em
  português. Ao criar textos, acrescente a tradução em `locales/en.json` e rode
  `node scripts/i18n.mjs check` (deve dar 100%). Logs (`tracing`) não são traduzidos.
- **Comentários e mensagens em português**, curtos, explicando o *porquê*.

### Ao terminar uma mudança

- `cargo clippy ... -D warnings` limpo, `cargo test --workspace` passando, `npm run check` sem erros,
  traduções completas.
- Mudou comportamento visível? Atualize o guia correspondente em `docs/guia/`.
- Mudou uma proteção? Atualize [docs/seguranca.md](docs/seguranca.md).
- Não faça commit, push nem publique releases sem a pessoa pedir.

## O que evitar

- Rodar comandos que alteram o sistema do usuário (PATH do sistema, registro fora de
  `HKCU\Software\Sentinel`, certificados, serviços do Windows) sem pedir.
- Aceitar pedidos de UAC ou instalar certificados em nome do usuário.
- Matar processos com filtros amplos (`taskkill /IM php.exe`): use os PIDs/Job Objects do Sentinel.
- Formatar o código inteiro (`cargo fmt` em tudo): o projeto ainda não segue o rustfmt padrão, então
  formate só o que mudou, se tanto.
