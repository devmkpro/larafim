# Documentação do Sentinel

## Usar o Sentinel

| Guia | Conteúdo |
|---|---|
| [Instalação](guia/instalacao.md) | Baixar, conferir o download, instalar, atualizar e desinstalar |
| [Primeiros passos](guia/primeiros-passos.md) | Do zero ao primeiro projeto no navegador |
| [Projetos](guia/projetos.md) | Pastas, detecção, importação, domínios, ligar e desligar |
| [PHP e Node](guia/php-e-node.md) | Versões, extensões, `php.ini`, Composer, Node por projeto |
| [Serviços e bancos](guia/servicos.md) | MariaDB, PostgreSQL, bancos que você já tem, Mailpit, S3, Adminer, `.env` |
| [Ferramentas](guia/ferramentas.md) | Logs, Xdebug, workers do Laravel, novo projeto Laravel, painel da bandeja |
| [HTTPS e domínios](guia/https-e-dominios.md) | Domínio `.offline`, arquivo `hosts`, certificado local |
| [Terminal](guia/terminal.md) | `php` e `composer` com a versão certa em qualquer terminal |
| [Problemas comuns](guia/problemas-comuns.md) | Porta ocupada, SmartScreen, antivírus, Firefox… |

## Desenvolver

| Guia | Conteúdo |
|---|---|
| [Arquitetura](desenvolvimento/arquitetura.md) | Crates, janelas, dados em disco, fluxo de uma requisição |
| [Compilar e testar](desenvolvimento/compilar-e-testar.md) | Requisitos, comandos, variáveis de ambiente, testes |
| [Traduções](desenvolvimento/traducoes.md) | Como funcionam os idiomas e como acrescentar um |
| [Publicar uma versão](desenvolvimento/publicar-versao.md) | CI, release automática, assinatura |

## Segurança

- [Modelo de segurança](seguranca.md): o que o Sentinel protege, como e quais testes cobrem cada ponto.
- [Reportar uma falha](../SECURITY.md).

## Histórico

[`plano/`](plano/PLANO.md) guarda o planejamento original (decisões, roadmap por fases e as
especificações de telas). É útil para entender o *porquê* das escolhas; o comportamento atual está
nos guias acima.
