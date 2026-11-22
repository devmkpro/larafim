# Larafim — Mapa de telas
Base: PLANO.md. Proposta de UX; MVP até fase 3, demais funcionalidades posteriores.

## Navegação
- Início: dashboard, projetos online, alertas, gateway e atalhos.
- Projetos: lista, descoberta, importação, detalhes e configuração.
- Runtimes: PHP, extensões, php.ini, Composer; Node na fase 5.
- Serviços: MariaDB, PostgreSQL, MinIO, Mailpit e Adminer conforme fases.
- Ferramentas: terminal, Xdebug, logs, workers, criação Laravel.
- Configurações: pastas, domínio, portas, inicialização, certificados e atualizações.

## Catálogo de telas
| ID | Tela | Fase | Forma |
|---|---|---|---|
| ONB-01 | Boas-vindas | 0–3 | Assistente |
| ONB-02 | Diagnóstico inicial | 0–3 | Assistente |
| ONB-03 | Pastas raiz | 2 | Assistente |
| ONB-04 | Instalação PHP inicial | 1–3 | Assistente opcional |
| ONB-05 | Domínio e permissões | 3 | Assistente |
| ONB-06 | Revisão | 3 | Assistente |
| DASH-01 | Visão geral | 3 | Página |
| PROJ-01 | Lista e descoberta | 2 | Página |
| PROJ-02 | Importar projeto | 2 | Assistente |
| PROJ-03 | Detalhes e status | 3 | Página |
| PROJ-04 | Configuração | 3 | Abas |
| PHP-01 | Catálogo PHP | 1 | Página |
| PHP-02 | Instalar/atualizar | 1 | Diálogo/progresso |
| PHP-03 | Versão, extensões e php.ini | 1 | Página |
| NET-01 | Gateway e conflitos | 3 | Página/painel |
| CLI-01 | Shims e PATH | 4 | Página |
| NODE-01 | Runtimes e processos | 5 | Página |
| DB-01 | MariaDB e bancos | 6 | Página |
| DB-02 | Backups SQL | 6 | Página/diálogo |
| TOOL-01 | Terminal | 6 | Painel |
| TOOL-02 | Catálogo e ferramentas | 7 | Página |
| TOOL-03 | Workers e logs | 7 | Abas |
| TOOL-04 | Novo projeto Laravel | 7 | Assistente |
| CFG-01 | Preferências | 0–8 | Página |
| SYS-01 | Diagnóstico e monitor | 3–8 | Página |
| SYS-02 | Tray | 0–8 | Menu nativo |

## Regras globais
Não iniciar projetos descobertos automaticamente. Diferenciar online, offline, iniciando e erro. Carregar terminal e serviços apenas quando solicitados. Mostrar URL completa `http://projeto.offline`. Solicitar elevação apenas para operações que a exigem. Nunca editar `.env` sem prévia, confirmação e backup.
