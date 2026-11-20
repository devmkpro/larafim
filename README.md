# Sentinel

Gerenciador local de PHP, Node e serviços para Windows, em Rust + Tauri 2 + Svelte 5.
Planejamento completo em [`../docs/PLANO.md`](../docs/PLANO.md).

## Estrutura

```
crates/sentinel-core/     caminhos, config.toml, projects.toml, detecção de projetos, logs
crates/sentinel-runtimes/ PHP: catálogo oficial, download, php.ini, extensões, Composer
crates/sentinel-server/   gateway HTTP, FastCGI, pools de php-cgi, hosts, portas
app/                    interface Svelte 5 + Vite
app/src-tauri/          app desktop (janela, bandeja, comandos)
```

## Requisitos

- Rust (toolchain MSVC) e Visual Studio Build Tools
- Node.js 20+
- WebView2 (já vem no Windows 11)

## Comandos

```powershell
cd app
npm install
npm run tauri dev        # app com recarga automática
npm run dev              # só a interface no navegador (dados de exemplo)
npm run check            # checagem de tipos do Svelte
npx tauri build --no-bundle   # executável em target\release\sentinel-app.exe
```

```powershell
cargo test --workspace   # testes do Rust (na raiz do workspace)
```

## Dados

Tudo fica em `%LOCALAPPDATA%\Sentinel` (configuração, runtimes, logs). Para testar
sem mexer nos seus dados, defina outra pasta: `$env:SENTINEL_HOME = "C:\temp\sentinel-teste"`.
Nível de log: `$env:SENTINEL_LOG = "debug"`.

Só para testes automáticos: `$env:SENTINEL_SKIP_HOSTS = "1"` faz o Sentinel não mexer no arquivo `hosts` (sem UAC). Os projetos respondem no gateway, mas o navegador não acha o domínio `.offline`; acesse com `curl -H "Host: projeto.offline" http://127.0.0.1/`.

Teste de ponta a ponta com PHP de verdade:

```powershell
$env:SENTINEL_TEST_PHP = "$env:LOCALAPPDATA\Sentinel\runtimes\php\8.4.28"
cargo test -p sentinel-server --test e2e_php -- --ignored
```
