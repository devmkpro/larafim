# Novidades

As versões do Sentinel e o que mudou em cada uma. O instalador de cada versão fica em
[Releases](https://github.com/josevictoremiliano/sentinel/releases).

## v0.1.0

Primeira versão pública. 🎉

O Sentinel monta o ambiente de desenvolvimento local para projetos **PHP, Laravel e Node** no
Windows: cada projeto no seu endereço (`http://loja.offline`), cada um com a sua versão de PHP, tudo
ao mesmo tempo.

### Destaques

- **Várias versões de PHP ao mesmo tempo**, baixadas do site oficial com o SHA-256 conferido, com
  aviso de atualização, extensões e `php.ini` por versão, e Composer incluso.
- **Um domínio por projeto** (`.offline`), servido por um gateway próprio em Rust: estáticos direto,
  PHP por FastCGI e projetos Node por proxy com *hot reload*. HTTPS local opcional, com uma CA restrita
  ao domínio local.
- **Serviços sob demanda:** MariaDB (compartilhado ou próprio do projeto), PostgreSQL, bancos que você
  já tem (Laragon, XAMPP…), Mailpit, storage S3 (RustFS) e Adminer.
- **`.env` com segurança:** prévia do que muda, cópia automática e desfazer.
- **Terminal certo:** `php`, `composer`, `node` e `npm` usam a versão do projeto da pasta atual.
- **Ferramentas:** visualizador de logs do Laravel (níveis, filtros e stack trace com o `vendor/`
  recolhido), Xdebug com um clique, workers (fila, agendador, Reverb) e novo projeto Laravel.
- **Bandeja:** painel de atalhos no clique esquerdo; inicia com o Windows, se você quiser.
- **Português e inglês:** o idioma é escolhido no instalador e pode ser trocado depois, sem reiniciar.
- **Leve:** cerca de 10 MB de memória na bandeja.

### Segurança

Downloads só de fontes oficiais e com SHA-256; extração de zip protegida (caminhos para fora da pasta,
links simbólicos, zip bomb); validação do arquivo `hosts`, do `.env` e dos caminhos do servidor local;
CSP estrita na interface. Detalhes em [docs/seguranca.md](https://github.com/josevictoremiliano/sentinel/blob/main/docs/seguranca.md).

### Requisitos e limitações

- Windows 10 ou 11 (x64). Só Windows, por enquanto.
- O instalador **ainda não tem assinatura digital**: o SmartScreen pode avisar
  (*Mais informações › Executar assim mesmo*). Confira o download antes, como explicado abaixo.
- Na primeira vez que um projeto fica online, o Windows pede administrador (UAC) para registrar os
  domínios no arquivo `hosts`.
