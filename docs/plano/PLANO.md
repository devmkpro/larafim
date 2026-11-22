# Larafim — Plano do Projeto

> Gerenciador local de ambientes de desenvolvimento (PHP, Node, MariaDB) para Windows, escrito em Rust.
> Inspiração: Laragon e Laravel Herd — sem precisar baixar, descompactar e trocar versão de PHP na mão.
> **Princípio nº 1: leveza.** O Larafim não pode pesar no PC de quem usa. O foco é o projeto que a pessoa está desenvolvendo, não o Larafim.

---

## 1. Problema

- Projetos diferentes exigem versões diferentes de PHP (8.3, 8.4, 8.5...).
- No Laragon, instalar uma versão nova é manual: baixar no site do PHP, descompactar, mover para a pasta certa, trocar no menu.
- O Laragon roda **uma versão de PHP por vez** — não dá para ter o projeto A em 8.3 e o projeto B em 8.5 ao mesmo tempo.
- Conflitos de URL/porta com o Laragon (e outros serviços que ocupam a porta 80).
- Terminal sem saber qual PHP/Node usar para cada projeto.

## 2. Objetivo

Um app desktop leve que:

1. Lê uma (ou mais) pasta raiz de projetos e **lista** os projetos encontrados (sem subir nada automaticamente).
2. Para cada projeto, mostra tipo (PHP/Laravel/Node/misto), versão de PHP/Node detectada, a URL `http://nome-do-projeto.offline` e um toggle **Online/Offline**.
3. **Online** = o projeto fica acessível em `http://nome-do-projeto.offline`, cada um com **sua própria versão de PHP**, simultaneamente.
4. Instala/atualiza versões de PHP sozinho (download, verificação, extração, `php.ini`), avisando quando há atualização.
5. Oferece MariaDB local e um terminal já configurado com o PHP/Node do projeto.

## 3. Decisões tomadas

| Tema | Decisão | Por quê |
|---|---|---|
| Interface | **Svelte 5** (no Tauri 2) | Compila para JS puro, sem runtime de Virtual DOM: bundle muito menor e menos memória que React. O maior custo de RAM de um app Tauri é o WebView2; o Svelte ajuda a não somar nada em cima disso. |
| Banco | **MariaDB 11.8 LTS**, em modo leve e sob demanda | Mais leve parado que o MySQL (~80–150 MB contra ~350–450 MB do MySQL com as opções padrão) e compatível com ele: mesmo SQL, mesmos clientes, `pdo_mysql` e `mysqli` funcionam igual. No Laravel dá para usar `DB_CONNECTION=mariadb` (Laravel 11+) ou `mysql`. **Só sobe quando for usado.** |
| Bancos extras | **PostgreSQL** e storage S3 (**RustFS**) como download opcional | Não vêm instalados; a pessoa baixa pelo Larafim se precisar e liga por projeto (seção 5). |
| Domínio | **`.offline`** | Diferencia o Larafim de Herd/Valet/Laragon (todos usam `.test`), evitando choque entre programas. Verificado: `.offline` **não** é TLD público (lista da IANA de 20/11/2026), então não colide com sites reais. |
| Config. do projeto | **Fica no Larafim, não no repositório** | Ninguém quer um arquivo do Larafim aparecendo no GitHub. Detalhes na seção 4.1. |

### `my.ini` do MariaDB em modo leve (padrão do Larafim)
```ini
performance_schema = OFF
innodb_buffer_pool_size = 64M
innodb_log_buffer_size = 8M
max_connections = 30
table_open_cache = 256
key_buffer_size = 8M
aria_pagecache_buffer_size = 16M
skip-name-resolve
```
Ajustável na tela do banco ("Modo leve" / "Modo padrão"). Os números de memória acima são estimativas e serão medidos na fase 6.

## 4. Funcionalidades

### 4.1 Projetos
- **Pastas raiz**: o usuário escolhe uma ou mais pastas (ex.: `D:\www`). Cada subpasta é um candidato a projeto.
- **Botão Atualizar**: reescaneia as pastas (sob demanda — nada de varredura pesada constante).
- **Botão Adicionar projeto**: aponta uma pasta avulsa, fora da raiz.
- **Detecção automática** (só leitura de arquivos, barata):
  | Sinal | Conclusão |
  |---|---|
  | `artisan` + `composer.json` | Laravel → docroot `public/` |
  | `composer.json` / `index.php` | PHP genérico → docroot `public/` se existir, senão raiz |
  | `composer.json` → `require.php` (ex.: `^8.3`) | versão de PHP sugerida |
  | `.php-version` | versão de PHP fixada (tem prioridade) |
  | `package.json` | tem Node → script `dev` disponível? |
  | `.nvmrc` / `engines.node` | versão de Node |
- **Tela de importação**: mostra em destaque a URL **`http://nome-do-projeto.offline`** (clicável e copiável) e permite confirmar ou alterar: versão do PHP, docroot, domínio, se usa Node, comando do Node e banco de dados.
- **Online/Offline** por projeto (checkbox). Estado salvo; ao abrir o Larafim, só os marcados como Online sobem.
- **Onde a configuração fica salva**:
  1. **Padrão:** em `%LOCALAPPDATA%\Larafim\projects.toml`. Nada é escrito dentro do projeto.
  2. **Opcional:** botão "Salvar configuração no projeto" grava um `.larafim.toml` na pasta do projeto, útil para levar a configuração para outra máquina. Ao fazer isso, o Larafim adiciona o arquivo em **`.git/info/exclude`** (exclusão local, que não é commitada), então ele **não sobe para o GitHub** e o `.gitignore` do projeto também não é alterado. Quem quiser compartilhar com o time pode marcar "versionar este arquivo".
  3. **Remover do Larafim** nunca apaga arquivos. Projeto de pasta raiz removido (ou ignorado) vai para a lista "Projetos ignorados" em Configurações › Pastas de projetos, de onde pode voltar.
4. O Larafim **lê** arquivos que o projeto já tenha (`.php-version`, `.nvmrc`, `composer.json`), mas nunca os cria nem altera sozinho.

### 4.2 Versões de PHP
- Fonte oficial Windows: `https://downloads.php.net/~windows/releases/releases.json` (o antigo `windows.php.net/downloads/releases/` redireciona para lá). Traz a última versão de cada minor (hoje do 7.4 ao 8.6), link do zip e **SHA-256**. Versões antigas em `/releases/archives/`.
- Certificados raiz (`cacert.pem` do curl.se) baixados uma vez e ligados em `curl.cainfo`/`openssl.cafile`, para o PHP conseguir usar HTTPS no Windows.
- Se faltar o Visual C++ Redistributable, a instalação detecta (o `php -v` falha com DLL ausente) e oferece o link oficial.
- Builds **NTS x64** (as indicadas para FastCGI).
- Instalação: baixar → verificar SHA-256 → extrair para `runtimes\php\8.4.x\` → gerar `php.ini` a partir do `php.ini-development` com extensões comuns ativadas (`curl, fileinfo, gd, intl, mbstring, mysqli, openssl, pdo_mysql, pdo_sqlite, sodium, zip`).
- **Projetos fixam o minor (8.4)**, não o patch. Atualizar 8.4.10 → 8.4.12 troca para todos os projetos 8.4 automaticamente, mantendo a versão anterior para rollback e copiando as customizações de `php.ini`.
- **Aviso de atualização**: ao abrir (e a cada X horas) compara instalado vs. `releases.json` → badge "atualização disponível" + botão "Atualizar".
- Tela de extensões: liga/desliga extensões e edita `php.ini` por versão.
- **Composer** incluso (baixa `composer.phar` + verificação de assinatura).

### 4.3 Servidor web e domínios
- **Gateway HTTP próprio em Rust** (ouvindo em `127.0.0.1:80`) que olha o `Host` e encaminha:
  - projeto PHP → arquivos estáticos servidos direto; `.php` e rotas "front controller" (`/rota` → `public/index.php`) via **FastCGI** para o `php-cgi.exe` da versão do projeto.
  - projeto Node → proxy reverso para a porta do dev server.
- **Pools de `php-cgi` por versão** (não por projeto): todos os projetos 8.3 compartilham o pool 8.3. No Windows o `php-cgi` atende uma requisição por vez, então cada pool tem N processos (ex.: 4) com round-robin. Pools sobem **sob demanda** (primeira requisição) e desligam após inatividade → PC leve.
- **TLD `.offline`** (configurável nas preferências).
- **Resolução de nomes**: bloco gerenciado no arquivo `hosts` (`# BEGIN LARAFIM` / `# END LARAFIM`), escrito por um pequeno helper que pede elevação (UAC) só quando a lista de domínios muda. Fase posterior: DNS local + regra NRPT para `*.offline` (pede admin uma única vez e nunca mais mexe no `hosts`).
- **Cuidado com o navegador**: como `.offline` não é um TLD conhecido, digitar só `loja.offline` na barra pode virar uma busca no Google. Por isso a UI sempre mostra e abre a URL completa (`http://loja.offline`). Depois do primeiro acesso, o navegador passa a reconhecer o endereço.
- **Implementado na fase 3:** gateway próprio (hyper) + cliente FastCGI próprio; pools de `php-cgi` com 3 processos no máximo, criados conforme a demanda e desligados após 10 min parados; bloqueio de `..`, caminhos do Windows e arquivos ocultos (`.env`, `.git`); Job Object do Windows para nenhum `php-cgi` ficar órfão; o `hosts` recebe todos os projetos importados (ligar/desligar não pede UAC de novo); ajudante elevado valida cada linha antes de gravar e guarda `hosts.larafim-backup`.
- **Conflito de portas**: ao iniciar, checa se 80/443/3306 estão ocupadas e mostra *quem* ocupa (Laragon, IIS, Apache...). Oferece porta alternativa.
- **HTTPS (fase posterior)**: CA local gerada pelo Larafim, instalada no repositório de certificados do usuário; certificado por domínio (`https://loja.offline`).

### 4.4 Node
- Projeto com `package.json` e Node marcado: o Larafim roda o script (`npm run dev` / `pnpm dev` / `yarn dev`, detectado pelo lockfile) com `PORT` definido, guarda logs e encaminha o domínio para essa porta.
- Projeto misto (Laravel + Vite): PHP no domínio principal, Vite rodando ao lado (botão "iniciar Vite").
- Versões de Node: baixadas de `https://nodejs.org/dist/index.json` (zip Windows + SHASUMS), mesma lógica do PHP.
- **Implementado na fase 5:** linhas LTS (18+) e a Current mais nova; versão por projeto (fixada, `.nvmrc`, `.node-version`, `engines`); atalhos `node`, `npm`, `npx`. Cada servidor de dev roda num Job Object próprio (parar mata a árvore cmd → npm → node). A porta é lida da saída (`Local: http://localhost:5173`) ou vem de `PORT`; o gateway faz proxy com WebSocket (HMR) e troca o Host para `localhost` (o Vite bloqueia domínios desconhecidos). Projetos Laravel + Vite recebem `APP_URL` com o endereço `.offline` (o plugin do Laravel libera o CORS). Saída ao vivo no painel do projeto.

### 4.5 MariaDB
- Download do zip oficial do **MariaDB 11.8 LTS** para Windows (lista de versões e hashes pela API de downloads da MariaDB), inicialização com `mariadb-install-db`, dados em `data\mariadb\`, `my.ini` em modo leve (seção 3).
- **Sob demanda**: sobe quando um projeto que usa banco fica Online ou quando o usuário liga manualmente; opção de desligar após inatividade.
- Start/stop pela UI e tray; porta 3306 ou alternativa se ocupada (ex.: MySQL do Laragon já usando).
- Botão "criar banco para este projeto" (nome do banco = nome do projeto).
- **Backup**: exportar e importar `.sql` por projeto (`mariadb-dump` / `mariadb`), com backups salvos em `backups\<projeto>\` e data no nome.
- Painel "Configuração para o projeto" com as variáveis `DB_*` (seção 4.6).

- **Implementado na fase 6:** MariaDB LTS (12.3, 11.8, 11.4, 10.11) pela API oficial com SHA-256; dados em `data\mariadb\<linha>`; `my.ini` leve só em 127.0.0.1; liga sob demanda (projeto online que usa banco, ou botão) e desliga com `shutdown`. Seletor de banco no projeto (SQLite / MariaDB do Larafim / bancos existentes) com prévia do `.env`, cópia `.env.larafim.bak` e desfazer. Backups por projeto em `backups\<domínio>` e restauração. Adminer 6.3.1 (SHA-256 fixo) em `db.<domínio>` com login automático no MariaDB do Larafim.
- **MariaDB compartilhado ou próprio** (pedido do usuário): no projeto, a pessoa escolhe entre o MariaDB compartilhado (um servidor para todos, mais leve) e o **MariaDB próprio do projeto** (servidor só dele, com versão e porta próprias — porta livre sugerida a partir da 3310 — e dados em `data\mariadb-projetos`). Vários rodam ao mesmo tempo; o Adminer entra sozinho em todos os servidores do Larafim e pede senha nos demais. Medido: ~140–165 MB de memória por servidor ligado.
- **Bancos que já existem no PC** (pedido do usuário): conexões MySQL/MariaDB cadastradas (senha protegida pela DPAPI do Windows), detecção automática de servidores nas portas 3306–3310 (IPv4 e IPv6, inclusive serviços do Windows), teste de conexão, uso nos projetos com o mesmo fluxo (criar banco, `.env`, backup). Usa os programas do próprio servidor ou os do MariaDB do Larafim (com `--plugin-dir`, para o `caching_sha2_password` do MySQL 8).
- **Terminal integrado (xterm.js):** adiado; por enquanto o botão de terminal abre o Windows Terminal já configurado.

### 4.6 Serviços por projeto e painel "Configuração para o projeto"
**Projetos Laravel (com `.env`)** — no painel do projeto há uma seção **Serviços** com um seletor por categoria:

| Categoria | Opções embutidas do Laravel | Serviços do Larafim |
|---|---|---|
| Banco | SQLite | MariaDB, PostgreSQL |
| E-mail | log | Mailpit |
| Storage | local | S3 local |
| Fila / Cache | sync, database, file | (Redis, futuro) |

Ao escolher um serviço do Larafim: se não estiver instalado, oferece instalar → liga o serviço → (banco) pergunta qual banco usar ou cria um novo → mostra o que vai mudar no `.env` → aplica **só** as chaves daquela categoria. Antes da primeira mudança, guarda os valores originais em `.env.larafim.bak`. Se o `.env` tiver uma configuração que o Larafim não reconhece, o seletor mostra "Não gerenciado pelo Larafim" e não mexe em nada.

**Qualquer projeto (inclusive não Laravel)** — cada serviço tem também o painel com as variáveis prontas, já preenchidas com a porta real e o nome do projeto:

- **Copiar**: copia o bloco para a área de transferência, para colar onde quiser.
- **Aplicar no `.env`**: escolhe um ou vários projetos; o Larafim mostra antes o que vai mudar (linha antiga → linha nova), faz a cópia `.env.larafim.bak` e altera **só** as chaves daquele serviço. Nada é aplicado sem confirmação.
- **Desfazer**: restaura o `.env` a partir da cópia.

Exemplo do Mailpit:
```env
MAIL_MAILER=smtp
MAIL_HOST=127.0.0.1
MAIL_PORT=1025
MAIL_USERNAME=null
MAIL_PASSWORD=null
```
Exemplo do MariaDB (projeto `loja`):
```env
DB_CONNECTION=mariadb
DB_HOST=127.0.0.1
DB_PORT=3306
DB_DATABASE=loja
DB_USERNAME=root
DB_PASSWORD=
```

### 4.7 Terminal e CLI
- **Shims** (`php.exe`, `composer.bat`, `node.exe`, `npm.cmd`) em `bin\`, adicionados ao `PATH` do usuário. O shim olha a pasta atual, descobre o projeto e executa a versão certa. → Em qualquer terminal, dentro do projeto 8.3, `php -v` mostra 8.3.
- **Implementado na fase 4:** atalho próprio (`larafim-shim`, ~450 KB, console) copiado como `php.exe` e `composer.exe`. Ordem de escolha: `LARAFIM_PHP` → projeto do Larafim (versão fixada ou a mais nova compatível) → `.php-version`/`composer.json` da pasta → versão padrão das Configurações → mais nova instalada. Se nada servir, explica e sai com código 1 (nunca usa uma versão errada em silêncio). Configurações › Terminal: instalar/reparar, ligar no PATH do usuário (sem admin), alerta de outro `php.exe` no PATH do sistema, versão padrão e "Testar uma pasta". Botão de terminal no projeto abre o Windows Terminal (ou cmd) com os atalhos na frente do PATH.
- **Instalador (fase 9):** o `larafim-shim.exe` vai nos recursos do pacote NSIS, ao lado do executável.
- **Terminal integrado** na UI (xterm.js + ConPTY), aberto já na pasta do projeto.
- Botão "Abrir no Windows Terminal" / "Abrir no VS Code".

### 4.8 App
- Ícone na bandeja (tray): status, start/stop geral, projetos online.
- Iniciar com o Windows (opcional) — inicia minimizado na bandeja.
- Logs por projeto e por serviço (gateway, php-cgi, mariadb, node).
- **Monitor de recursos**: RAM e CPU de cada serviço que o Larafim subiu, para a pessoa ver que o app está leve (e o que desligar se não estiver).
- **Idiomas (i18n)**: português (Brasil) e inglês. O instalador pergunta o idioma e grava a escolha (`HKCU\Software\Larafim\Language`); na primeira abertura o app usa essa escolha ou, sem ela, o idioma da interface do Windows (não o formato regional). Troca em Configurações › Geral, na hora, sem reiniciar: interface, mensagens de erro do backend, bandeja, título das janelas, páginas do gateway e o shim do terminal. A chave de cada texto é o próprio texto em português (estilo gettext); cada idioma é um JSON em `locales/`, lido pela interface e pelo Rust. Para um idioma novo basta criar o arquivo (ver `locales/README.md`); `node scripts/i18n.mjs check` mostra o que falta traduzir.

### 4.9 Estrutura de telas
Base detalhada em `01_MAPA_DE_TELAS.md` a `05_COMPONENTES_ESTADOS_E_ACEITE.md`, com estes ajustes (referência de organização: Laravel Herd 2.0, com visual próprio):

- **Janela principal = Projetos**, sem tela "Início" separada. Barra superior com: Novo, busca, Atualizar, ordenar e atalhos para Serviços/Ferramentas/Configurações. Barra de status embaixo: gateway, serviços ligados e avisos.
- **Lista + painel lateral (inspector)**: lista de projetos à esquerda (modo lista por padrão; miniaturas como opção, porque gastam recursos). À direita, o projeto selecionado: URL, pasta, Online/Offline, **Ambiente** (PHP, Node, HTTPS), **Serviços** (seletores da seção 4.6), atalhos (abrir no navegador, pasta, VS Code, terminal, logs).
- **Importar com um clique** para projetos detectados (revisão opcional); assistente completo só para pasta avulsa.
- **Primeiro acesso em 3 passos**: pastas (diagnóstico de portas roda em segundo plano e só aparece se houver problema) → PHP sugerido → pronto. O pedido de UAC acontece quando o primeiro projeto fica Online.
- **Configurações** em tela própria com menu lateral: Geral, PHP, Node, Serviços, Rede e domínios, Atalhos, Sobre.
- **Ferramentas** (Mail, Logs, Dumps, Workers) abrem só quando usadas.
- **Bandeja** com as ações do dia a dia: projetos online, abrir projeto, ligar/desligar serviços, versão do PHP, sair.
- **Ctrl+K**: busca rápida de projetos e ações.
- **Temas**: escuro, claro ou seguir o Windows (Configurações → Geral).
- **Visual de app nativo do Windows 11, não de site**: barra de título própria com os botões do sistema; ícones da fonte Segoe Fluent Icons; cores, controles (botões, seletores, interruptores) e cursor no padrão Fluent; lista densa com destaque de seleção do Windows; Configurações em janela separada, no estilo do app Configurações do Windows (linhas com ícone, título, descrição e controle à direita). Nada de títulos gigantes, cartões flutuando com sombra ou espaçamento de página web.

## 5. Ferramentas e serviços

Ideias tiradas de Laravel Herd, Laragon, ServBay e DBngin. Regras para todos:
- **Desligados por padrão** e só sobem sob demanda (princípio de leveza).
- Serviços que precisam de binário extra aparecem num **catálogo de downloads**: só são baixados quando a pessoa clica em "Instalar".
- Todo serviço tem o painel **"Configuração para o projeto"** (seção 4.6): copiar as variáveis ou aplicar no `.env` de um ou vários projetos.

### 5.1 Confirmadas — fazem parte do produto
| Ferramenta | O que faz | Como |
|---|---|---|
| **Mailpit** | Todo e-mail enviado pelo projeto cai numa caixa local; nada sai para a internet | Binário único. SMTP em `127.0.0.1:1025`, caixa de entrada em `http://mail.offline`. Variáveis `MAIL_*` para copiar ou aplicar no `.env` |
| **Adminer** | Abrir e consultar o MariaDB (e o PostgreSQL, se instalado) pelo navegador | Um único arquivo PHP servido em `http://db.offline`, com login já preenchido |
| **Backup de banco** | Exportar/importar `.sql` por projeto, com histórico | `mariadb-dump` / `mariadb` (seção 4.5) |
| **Xdebug** | Liga/desliga por versão de PHP com um clique | Baixa a DLL certa de xdebug.org (versão do PHP, NTS, x64) e edita o `php.ini`. Mostra a configuração para o VS Code / PhpStorm |
| **Visualizador de logs** | Acompanha `storage/logs/laravel.log` e o log de erros do PHP em tempo real | Leitura incremental do arquivo (tail), sem processo extra |
| **Workers do Laravel** | Rodar `queue:work`, `schedule:work` e `reverb:start` como processos gerenciados, com logs e reinício se cair | Mesmo supervisor de processos usado para o Node |
| **Novo projeto Laravel** | Assistente: nome, versão do PHP, banco (cria o banco no MariaDB), starter kit | `composer create-project laravel/laravel` (ou `laravel new`); o projeto já entra na lista, com `.env` configurado e URL `.offline` |
| **Terminal** | Terminal integrado e shims de CLI | Seção 4.7 |

### 5.2 Catálogo de downloads opcionais (liga por projeto)
| Serviço | Para quê | Variáveis entregues ao projeto |
|---|---|---|
| **PostgreSQL** | Banco alternativo para projetos que usam Postgres | `DB_CONNECTION=pgsql`, `DB_HOST=127.0.0.1`, `DB_PORT=5432`, `DB_DATABASE=<projeto>`, `DB_USERNAME=postgres`, `DB_PASSWORD=` |
| **Storage S3 local (RustFS)** | "Fake S3": testar upload/download de arquivos como se fosse a AWS, sem custo | `FILESYSTEM_DISK=s3`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_DEFAULT_REGION=us-east-1`, `AWS_BUCKET=<projeto>`, `AWS_ENDPOINT=http://127.0.0.1:9000`, `AWS_USE_PATH_STYLE_ENDPOINT=true` |

Ao ligar o storage S3 num projeto, o Larafim cria automaticamente o bucket com o nome do projeto. Painel do storage em `http://s3.offline`.

**Fontes escolhidas na fase 8** (mesmo padrão do Mailpit: release no GitHub, conferida pelo SHA-256 do campo `digest`):

- **PostgreSQL:** builds do projeto `theseus-rs/postgresql-binaries` (`postgresql-<versão>-x86_64-pc-windows-msvc.zip`, ~50 MB, com proveniência assinada). O zip da EDB, para comparação, tem ~337 MB e não publica checksum. Versões oferecidas: as majors suportadas (18, 17, 16, 15), padrão a mais nova. Dados em `data\postgres-<major>\`, `initdb -U postgres -A trust -E UTF8` na primeira execução, porta 5432 (ou a próxima livre), `postgresql.conf` leve (`shared_buffers=32MB`, `max_connections=20`), ligado sob demanda como o MariaDB. Criar banco com `psql`, backup com `pg_dump`/restauração com `psql`. Adminer já fala PostgreSQL (precisa da extensão `pdo_pgsql`/`pgsql` ligada no PHP).
- **Storage S3:** **RustFS** (`rustfs/rustfs`, Apache 2.0, 1.0 GA), zip oficial `rustfs-windows-x86_64-v<versão>.zip` (~118 MB). Substitui o MinIO, que deixou de publicar binários. O executável descompactado tem ~372 MB em disco; ligado usa ~76 MB de memória. Os parâmetros (endereço, painel) vão por variáveis `RUSTFS_*`, porque na linha de comando `--console-enable true` faz o `true` virar uma segunda pasta de dados. O Laravel precisa de `league/flysystem-aws-s3-v3` para o disco `s3`; o Larafim avisa quando o pacote falta. API em `127.0.0.1:9000`, painel em `s3.<tld>` pelo gateway, path-style. Chaves locais geradas pelo Larafim e guardadas com DPAPI; bucket criado com o nome do projeto.

### 5.3 Ideias para depois
| Ideia | O que faz | Como |
|---|---|---|
| **Compartilhar projeto** | URL pública temporária para mostrar o projeto a um cliente | Cloudflare Quick Tunnel (`cloudflared`), sem conta |
| **Dumps** | Captura `dump()`/`dd()` numa janela do Larafim | Servidor do VarDumper (Symfony) ou `auto_prepend_file` |
| **Redis** | Cache/filas | Sem versão oficial para Windows: avaliar Memurai ou um port de Redis/Valkey |
| **Meilisearch** | Busca (Laravel Scout) | Binário único |
| **Modo global** | Trocar a versão do PHP de todos os projetos de uma vez | |
| **Workspaces por branch** | Cada branch do Git com cópia própria do projeto, domínio (`feature-x.loja.offline`) e banco | `git worktree` + banco clonado |

## 6. Arquitetura

### 6.1 Stack
| Parte | Escolha | Motivo |
|---|---|---|
| Desktop/UI | **Tauri 2 + Svelte 5** | Binário pequeno, Rust no backend, tray nativo, frontend mais leve |
| Async | `tokio` | processos, rede, downloads |
| Gateway HTTP | `hyper` / `axum` | proxy por `Host` |
| FastCGI | `fastcgi-client` | falar com `php-cgi` |
| Downloads | `reqwest` + `sha2` + `zip` | baixar e validar runtimes |
| Versões | `semver` | casar `^8.3` com versões instaladas |
| Config | `serde` + `toml` | legível e editável |
| Terminal | `portable-pty` + xterm.js | ConPTY no Windows |
| Logs | `tracing` | |

### 6.2 Estrutura do código (workspace Cargo)
```
F:\larafim\larafim\     ← código (pasta separada de F:\larafim\docs)
├─ crates/
│  ├─ larafim-core/      domínio: projetos, detecção, config
│  ├─ larafim-runtimes/  PHP/Node/MariaDB/extras: catálogo, download, instalação, update
│  ├─ larafim-server/    gateway HTTP, pools php-cgi, proxy Node
│  ├─ larafim-services/  supervisor de processos: MariaDB, Mailpit, Postgres, S3, workers, Node
│  ├─ larafim-envfile/   leitura/escrita segura de .env (diff, backup, desfazer)
│  ├─ larafim-system/    hosts, portas, PATH, elevação UAC, certificados
│  └─ larafim-shim/      binário shim (php.exe, node.exe...)
└─ app/                   Tauri (src-tauri + frontend Svelte)
```

### 6.3 Dados em disco
```
%LOCALAPPDATA%\Larafim\
├─ config.toml            pastas raiz, TLD, portas, preferências
├─ projects.toml          projetos e configurações (versão, docroot, domínio, online)
├─ runtimes\php\8.3.x\ 8.4.x\ ...
├─ runtimes\node\22.x\ ...
├─ runtimes\mariadb\11.8.x\
├─ runtimes\postgres\ minio\     (só se instalados)
├─ services\mailpit\ adminer\ ...
├─ data\mariadb\ postgres\ s3\
├─ backups\<projeto>\
├─ bin\                   shims no PATH
└─ logs\
```

### 6.4 Fluxo de uma requisição
```
navegador → http://loja.offline
  → hosts: loja.offline = 127.0.0.1
  → Gateway Larafim :80 lê Host "loja.offline" → projeto "loja" (PHP 8.3, docroot public/)
  → arquivo estático existe? serve direto
  → senão: FastCGI → pool php-cgi 8.3 (sobe se estiver parado) → resposta
```

## 7. Metas de desempenho
| Situação | Meta |
|---|---|
| Larafim aberto, nenhum projeto online | < 80 MB de RAM no total (app + WebView2) |
| Larafim fechado na bandeja | < 30 MB (WebView descarregado, só o núcleo Rust) |
| Gateway parado esperando requisição | ~0% de CPU |
| Pool de PHP | só existe enquanto há projeto daquela versão sendo acessado |

Medir essas metas no fim de cada fase.

**Medição da fase 0 (20/11/2026, build de release, Windows 11):**
| Situação | Medido | Meta |
|---|---|---|
| Executável | 4,3 MB | — |
| Janela aberta (Larafim + processos do WebView2) | ~103 MB de memória privada | < 80 MB ❌ |
| Só na bandeja (janela descarregada) | 9,5 MB | < 30 MB ✅ |

O WebView2 sozinho usa ~90 MB mesmo com uma página simples. Já aplicado: GPU desligada e um único processo de renderização (caiu de 187 MB para 103 MB). Como o Larafim passa a maior parte do tempo na bandeja, o custo real no dia a dia é o de 9,5 MB. A meta de janela aberta fica para revisar quando as telas reais existirem.

## 8. Roadmap

| Fase | Entrega | Resultado |
|---|---|---|
| **0. Fundação** ✅ | Workspace Cargo, app Tauri + Svelte com tray, config TOML, logs | Janela abre, salva preferências |
| **1. Runtimes PHP** ✅ | Catálogo `releases.json`, instalar/remover/atualizar, `php.ini`, Composer, aviso de update | Nunca mais baixar PHP na mão |
| **2. Projetos** ✅ | Pastas raiz, scan, detecção, lista com Atualizar/Adicionar, tela de importação com URL `.offline` | Lista de projetos com versão sugerida |
| **3. Servidor** ⭐ MVP ✅ | Gateway + pools php-cgi + `hosts` + toggle Online/Offline + detecção de conflito de porta | `projeto.offline` rodando, várias versões ao mesmo tempo |
| **4. CLI** ✅ | Shims `php`/`composer` por pasta | Terminal usa a versão certa sozinho |
| **5. Node** ✅ | Runtimes Node, dev server por projeto, proxy, Vite em projeto misto | Projetos Node e Laravel+Vite |
| **6. Banco** ✅ (terminal integrado adiado) | MariaDB leve e sob demanda, Adminer, criar banco, backup, painel de `.env`, terminal integrado | Ambiente completo |
| **7. Ferramentas** ✅ | Mailpit (caixa local em `mail.<tld>`, aplica no `.env` com prévia), Xdebug por versão do PHP (só builds estáveis, `launch.json` pronto), visualizador de logs, workers do Laravel (fila, agendador, Reverb; voltam sozinhos ao abrir o app), novo projeto Laravel | Paridade com o essencial do Herd |
| **8. Catálogo opcional** ✅ | PostgreSQL (download de ~50 MB por versão, servidor sob demanda, banco por projeto, backup/restauração, `.env`, extensões `pgsql`/`pdo_pgsql` ligadas no PHP do projeto, Adminer com login automático) e storage S3 local com RustFS (bucket por projeto criado por requisição S3 assinada, `.env` com `AWS_*`, painel em `s3.<tld>/rustfs/console/`) | Projetos com Postgres e upload de arquivos testáveis |
| **9. Polimento** ✅ (DNS/NRPT, auto-update e compartilhar adiados) | Iniciar com o Windows (HKCU\Run, abre só na bandeja com `--tray`), HTTPS local (CA própria restrita ao domínio local por *name constraints*, chave com DPAPI, certificado por domínio via SNI, `HTTPS=on` para o PHP e `X-Forwarded-Proto` para o Node, botão "Confiar…" que instala a CA só no usuário), instalador NSIS por usuário com o `larafim-shim.exe` | Pronto para uso diário |

## 9. Riscos e cuidados
- **Porta 80 ocupada** (Laragon, IIS, Skype antigo): detectar e explicar; não brigar com o Laragon — rodar lado a lado com porta alternativa se preciso.
- **HTTPS e ferramentas do Windows**: navegadores (Edge, Chrome) não checam revogação de certificados locais, mas programas que usam o Schannel (o `curl` do Windows, PowerShell, .NET) checam e falham com "revocation status unknown"; nesses, usar `--ssl-no-revoke` (ou equivalente). O Firefox tem lista própria de autoridades: importar o `.crt` de `data\ca\` ou ligar `security.enterprise_roots.enabled`.
- **Adiados da fase 9**: *DNS/NRPT* — o arquivo `hosts` já resolve os domínios; NRPT exige um servidor DNS local e admin, só vale a pena para subdomínios curinga. *Compartilhar projeto* — chegou a ser feito (túnel rápido da Cloudflare) e foi removido a pedido do usuário por enquanto; volta numa fase futura. *Auto-update do app* — precisa de chave de assinatura do updater do Tauri e de um endereço público para publicar as versões (ainda não há repositório/site).
- **Permissão de admin** para `hosts`/certificados: concentrar num helper elevado pequeno, pedir só quando necessário.
- **Antivírus** pode segurar `php-cgi.exe`/downloads: verificar hash e usar só fontes oficiais.
- **Concorrência do php-cgi no Windows**: resolvido com pool de processos; medir na fase 3.
- **`.offline` não é reservado** (ao contrário de `.test`): hoje não existe, mas em tese poderia ser registrado no futuro. Como o `hosts` tem prioridade sobre o DNS público, os projetos continuariam funcionando; o TLD é configurável de qualquer forma.
- **Distribuição do MinIO mudou**: em 2025 a MinIO tirou o painel web da edição gratuita e passou a não publicar binários prontos dela. Verificar na fase 8; se não houver binário oficial para Windows, usar outro storage compatível com S3 (ex.: RustFS, que o próprio Herd já oferece ao lado do MinIO, ou Garage). As variáveis `AWS_*` entregues ao projeto continuam as mesmas, então para o projeto não muda nada.
- **Alterar o `.env` do usuário** é delicado: sempre mostrar o que muda, pedir confirmação, guardar cópia e permitir desfazer.
- **Windows primeiro**: a arquitetura não impede macOS/Linux depois, mas o MVP é só Windows.
