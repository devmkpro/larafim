# Arquitetura

O Larafim é um app [Tauri 2](https://tauri.app): o núcleo é Rust e a interface é
[Svelte 5](https://svelte.dev), rodando no WebView2 do Windows. **Princípio nº 1: leveza.** Nada sobe
sem necessidade, e na bandeja o app ocupa cerca de 10 MB.

## Estrutura do repositório

```
.
├─ app/                     app desktop
│  ├─ src/                  interface (Svelte 5 + TypeScript)
│  │  ├─ main-window/       janela principal (projetos + painel)
│  │  ├─ settings/          janela de Configurações
│  │  ├─ logs/              visualizador de logs
│  │  ├─ tray/              painel da bandeja
│  │  └─ lib/               componentes e a ponte com o Rust (api.ts, *.ts)
│  └─ src-tauri/            núcleo do app: janelas, bandeja, comandos chamados pela interface
├─ crates/
│  ├─ larafim-core/        caminhos, config.toml, projects.toml, detecção de projetos, .env, i18n, checagens de segurança
│  ├─ larafim-runtimes/    catálogos e instalação: PHP, Node, MariaDB, PostgreSQL, Mailpit, RustFS, Xdebug, Composer, Adminer
│  ├─ larafim-server/      gateway HTTP/HTTPS, FastCGI, pools de php-cgi, proxy do Node, hosts, portas, processos
│  └─ larafim-shim/        atalhos de terminal (php.exe, composer.exe, node.exe, npm.exe, npx.exe)
├─ locales/                 textos traduzidos (um JSON por idioma)
├─ scripts/i18n.mjs         ferramentas de tradução
└─ docs/                    esta documentação
```

### Quem depende de quem

```
larafim-core ─┬─ larafim-runtimes ─┬─ app/src-tauri
               ├─ larafim-server ───┘
               └─ larafim-shim  (usa core + runtimes; vai junto no instalador)
```

Os crates não conhecem o Tauri: tudo que é lógica testável fica neles. O `app/src-tauri` só liga as
peças, guarda o estado e expõe comandos para a interface.

## Janelas

Todas são do mesmo frontend; o *label* da janela decide o que aparece (`App.svelte`).

| Janela | Label | Quando existe |
|---|---|---|
| Principal | `main` | Ao abrir o app; descarregada ao fechar (o app segue na bandeja) |
| Configurações | `settings` | Sob demanda |
| Logs | `logs` | Sob demanda |
| Painel da bandeja | `tray` | Clique no ícone; escondido ao perder o foco e descarregado após 45 s |

Fechar todas as janelas não encerra o app: ele continua na bandeja (opção *manter na bandeja*).

## Dados em disco

```
%LOCALAPPDATA%\Larafim\          (ou LARAFIM_HOME)
├─ config.toml                    preferências (senhas só cifradas com DPAPI)
├─ projects.toml                  projetos importados
├─ runtimes\php\<versão>\  node\  mariadb\  postgres\  rustfs\  mailpit\  composer\  adminer\
├─ data\mariadb\<linha>\  mariadb-projetos\  postgres-<major>\  s3\  mailpit\  ca\
├─ backups\<projeto>\
├─ downloads\                     zips temporários
├─ bin\                           atalhos do terminal
└─ logs\
```

## Fluxo de uma requisição

```
navegador → http://loja.offline/produtos
  → hosts: loja.offline = 127.0.0.1
  → gateway (127.0.0.1:80) lê o Host → projeto "loja" (PHP 8.3, pasta public\)
  → resolve.rs: caminho seguro? arquivo estático? .php? front controller?
  → estático: servido direto
  → PHP: FastCGI → pool php-cgi 8.3 (sobe na 1ª requisição, desliga após 10 min parado)
  → projeto Node: proxy para a porta do servidor de dev (com WebSocket)
```

HTTPS usa o mesmo caminho, com TLS feito pelo gateway (rustls) e um certificado por domínio, emitido
na hora pela CA local.

## Processos

- `php-cgi`, servidores de dev do Node, workers e serviços rodam dentro de **Job Objects** do
  Windows: se o Larafim fechar ou travar, o Windows encerra toda a árvore de processos. Nada fica
  órfão.
- MariaDB e PostgreSQL desligam com o comando de *shutdown* de cada um, sem perder dados.

## Elevação (administrador)

O Larafim roda **sem administrador**. A única exceção é gravar o arquivo `hosts`: o próprio
executável é aberto elevado (UAC) com um argumento especial, lê a lista pedida, **revalida tudo** e
grava só o bloco do Larafim. Veja [Segurança](../seguranca.md#arquivo-hosts).

## Interface

- Visual de app nativo do Windows 11 (barra de título própria, Segoe Fluent Icons, controles Fluent),
  temas claro/escuro/sistema.
- A ponte com o Rust fica em `app/src/lib/*.ts`. Cada arquivo tem também **dados de exemplo**: com
  `npm run dev`, a interface roda no navegador sem o Rust, útil para trabalhar no visual.
- Todos os textos passam por `t()` (veja [Traduções](traducoes.md)).

## Outras plataformas

Hoje o Larafim é só para Windows. Partes que precisariam de versão para macOS/Linux:

| Parte | No Windows | Equivalente |
|---|---|---|
| Domínios | arquivo `hosts` + UAC | `/etc/hosts` + sudo, ou resolvedor por domínio (`/etc/resolver`, systemd-resolved) |
| Senhas guardadas | DPAPI | Keychain / Secret Service |
| Processos | Job Objects | grupos de processos / cgroups |
| Iniciar com o sistema, idioma do instalador | Registro (HKCU) | LaunchAgent / `.desktop` |
| Runtimes | zips de Windows (PHP, MariaDB, PostgreSQL…) | builds de cada sistema (ex.: static-php-cli) |
| Instalador | NSIS | `.dmg` / AppImage / `.deb` |

O Tauri e os crates em Rust já compilam nessas plataformas. O trabalho está em portar essas partes e
testar.
