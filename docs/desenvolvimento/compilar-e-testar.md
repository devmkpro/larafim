# Compilar e testar

## Requisitos

- Windows 10/11 x64.
- [Rust](https://rustup.rs) estável, toolchain **MSVC** (o projeto usa a edição 2024).
- Visual Studio Build Tools com "Desenvolvimento para desktop com C++".
- [Node.js](https://nodejs.org) 22 ou mais novo.
- WebView2 (já vem no Windows 11).

## Comandos

Na pasta `app/`:

```powershell
npm ci                       # dependências da interface (exatamente as do package-lock.json)
npm run tauri dev            # app completo com recarga automática
npm run dev                  # só a interface no navegador, com dados de exemplo
npm run check                # checagem de tipos do Svelte/TypeScript
npx tauri build              # instalador em target\release\bundle\nsis\
npx tauri build --no-bundle  # só o executável: target\release\sentinel-app.exe
```

Na raiz:

```powershell
cargo test --workspace                                        # testes do Rust
cargo clippy --workspace --all-targets -- -D warnings         # lint (o CI exige zero avisos)
node scripts/i18n.mjs check                                   # traduções
```

Antes do primeiro `cargo test`/`cargo clippy`, o app precisa de duas coisas que o Tauri embute na
compilação: a interface compilada (`npm run build` em `app/`) e o
`cargo build -p sentinel-shim --release`. O `npm run tauri dev`/`build` já faz isso sozinho.

## Variáveis de ambiente

| Variável | Para quê |
|---|---|
| `SENTINEL_HOME` | Pasta de dados alternativa (ex.: `C:\temp\sentinel-teste`), para testar sem mexer nos seus dados |
| `SENTINEL_LOG` | Nível de log (`info`, `debug`, `trace`) |
| `SENTINEL_SKIP_HOSTS` | `1` = não mexe no arquivo `hosts` (sem UAC). Só para testes automáticos: o navegador não acha os domínios; use `curl -H "Host: projeto.offline" http://127.0.0.1/` |
| `SENTINEL_PHP` / `SENTINEL_NODE` | Força a versão usada pelos atalhos do terminal |
| `SENTINEL_SHIM_DEBUG` | `1` = o atalho explica qual versão escolheu e por quê |

## Testes

`cargo test --workspace` roda tudo que não depende de rede nem de programas instalados, inclusive os
**testes de segurança** (injeção no `.env`, no `hosts`, no servidor local, zips maliciosos, catálogos
adulterados, XSS nas páginas do gateway). A lista completa está em
[Segurança](../seguranca.md#testes).

Os testes marcados `#[ignore]` precisam de rede ou de runtimes de verdade:

```powershell
# catálogos reais (rede)
cargo test --workspace -- --ignored real_

# PHP de ponta a ponta: gateway + FastCGI + php-cgi de verdade
$env:SENTINEL_TEST_PHP = "$env:LOCALAPPDATA\Sentinel\runtimes\php\8.4.28"
cargo test -p sentinel-server --test e2e_php -- --ignored
```

`SENTINEL_TEST_MARIADB` e `SENTINEL_TEST_POSTGRES` apontam para instalações reais nos testes ignorados
dos bancos.

## Antes de abrir um pull request

- `cargo clippy --workspace --all-targets -- -D warnings` sem avisos;
- `cargo test --workspace` passando;
- `npm run check` sem erros;
- `node scripts/i18n.mjs check` sem textos faltando (traduza os textos novos em `locales/en.json`);
- mudou algo que lê dado de fora (arquivo de projeto, download, requisição)? Acrescente um teste com
  entrada maliciosa (veja [Segurança](../seguranca.md#ao-mexer-no-código)).

O CI roda tudo isso em cada push e pull request.
