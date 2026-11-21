# Política de segurança

## Versões com correções

Só a versão mais recente publicada em [Releases](https://github.com/josevictoremiliano/sentinel/releases)
recebe correções de segurança.

## Como reportar uma falha

**Não abra uma issue pública** para falhas de segurança.

Use o reporte privado do GitHub: aba **Security › Report a vulnerability** neste repositório
([link direto](https://github.com/josevictoremiliano/sentinel/security/advisories/new)).

Inclua, se possível:

- o que a falha permite (ler arquivos, executar código, escalar para administrador…);
- passos para reproduzir ou uma prova de conceito;
- a versão do Sentinel e do Windows.

O que esperar:

- confirmação de recebimento em até **7 dias**;
- uma avaliação inicial e um prazo para a correção em até **14 dias**;
- crédito no aviso de segurança, se você quiser.

## Escopo

Interessam principalmente:

- execução de código ou escrita de arquivos a partir de **catálogos, downloads ou arquivos de
  projetos** (ex.: zip ou `composer.json` malicioso);
- qualquer forma de abusar do **ajudante elevado** que grava o arquivo `hosts`;
- leitura de arquivos fora da pasta pública pelo **servidor local** (`*.offline`);
- injeção no `.env`, em SQL ou em comandos;
- XSS na interface do app ou nas páginas do servidor local;
- vazamento de senhas guardadas (DPAPI) ou da chave da CA local.

Fora do escopo: o código dos **seus projetos** (o Sentinel executa o que o projeto define, como
`npm run dev`), os bancos locais sem senha acessíveis por `127.0.0.1` (decisão de projeto, documentada)
e ataques que exigem um programa malicioso já rodando como o seu usuário.

O modelo de segurança completo, com as proteções e os testes, está em
[docs/seguranca.md](docs/seguranca.md).
