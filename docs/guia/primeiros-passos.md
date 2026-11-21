# Primeiros passos

Do zero ao primeiro projeto aberto no navegador.

## 1. Escolha a pasta dos projetos

Na primeira abertura, o Sentinel pergunta onde ficam seus projetos (ex.: `D:\www`). Cada subpasta
vira um candidato a projeto. Dá para ter mais de uma pasta e mudar depois em
*Configurações › Pastas de projetos*.

Nada sobe sozinho: o Sentinel só **lê** as pastas para descobrir o que existe.

## 2. Instale o PHP

Em *Configurações › PHP*, clique em **Instalar** na versão que seus projetos usam. O download vem do
site oficial do PHP para Windows, e o SHA-256 é conferido antes de extrair. O Composer é baixado junto.

O Sentinel já sugere a versão certa para cada projeto, pelo `composer.json` ou pelo `.php-version`.

## 3. Importe os projetos

A janela principal lista o que foi encontrado. Projetos novos aparecem com a marca **Novo**: use
**Importar todos** ou importe um por um. Pastas fora da pasta raiz entram por **Adicionar projeto**.

Cada projeto recebe um endereço a partir do nome da pasta: `loja-virtual` vira
`http://loja-virtual.offline`.

## 4. Ligue um projeto

No painel à direita, ligue o interruptor **Online**. Na primeira vez, o Windows pede permissão de
administrador (UAC). O pedido serve para registrar os domínios no arquivo `hosts`, só acontece quando
a lista de domínios muda e o programa elevado só aceita linhas `127.0.0.1 <domínio válido>`.

Pronto: clique no endereço ou em **Abrir no navegador**.

## 5. Banco de dados e e-mail (opcional)

No painel do projeto, em **Serviços**, escolha o banco (SQLite, MariaDB, PostgreSQL ou um MySQL que
você já tenha) e o e-mail (Mailpit). O Sentinel instala o que faltar, mostra o que vai mudar no `.env`
e só aplica depois da sua confirmação. Veja [Serviços e bancos](servicos.md).

## Próximos passos

- [Terminal](terminal.md): `php` e `composer` com a versão do projeto em qualquer terminal.
- [HTTPS e domínios](https-e-dominios.md): `https://` com certificado local.
- [Ferramentas](ferramentas.md): logs, Xdebug, workers e o painel da bandeja.
