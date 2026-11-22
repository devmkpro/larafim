# Publicar uma versão

## CI

[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) roda em cada push na `main` e em cada pull
request, no Windows:

1. `npm ci`, traduções completas (`i18n.mjs check --strict`) e tipos da interface (`npm run check`);
2. compila a interface e o `larafim-shim`;
3. `cargo clippy` sem avisos;
4. `cargo test --workspace` (inclui os testes de segurança).

Um segundo job audita as dependências contra vulnerabilidades conhecidas (`cargo audit` e
`npm audit --audit-level=high`).

O [Dependabot](../../.github/dependabot.yml) abre pull requests semanais com atualizações de Rust, npm e
das próprias Actions.

## Release

[`.github/workflows/release.yml`](../../.github/workflows/release.yml) roda quando uma tag `vX.Y.Z` é
enviada:

1. confere se a tag bate com a `version` de `app/src-tauri/tauri.conf.json`;
2. roda os testes de novo;
3. `npx tauri build`, que gera o instalador NSIS;
4. gera o `SHA256SUMS.txt`;
5. cria a **atestação de origem** (*build provenance*) do instalador, que qualquer pessoa confere com
   `gh attestation verify`;
6. cria um **rascunho** de release com o instalador, o `SHA256SUMS.txt` e as notas (seção do `CHANGELOG.md` + como instalar + lista de PRs).

### Passo a passo

1. Atualize a versão em `app/src-tauri/tauri.conf.json`, `app/package.json` e no
   `[workspace.package]` do `Cargo.toml` (as três iguais).
2. Escreva a seção `## vX.Y.Z` no [`CHANGELOG.md`](../../CHANGELOG.md): destaques, correções e o que
   a pessoa precisa saber. Esse texto vira o começo da release. O workflow acrescenta "Instalar" (com o
   SHA-256 do instalador) e a lista de PRs agrupada por categoria
   ([`.github/release.yml`](../../.github/release.yml)). Sem essa seção, o workflow falha.
3. Faça o commit e espere o CI ficar verde.
4. Crie e envie a tag:

   ```powershell
   git tag v0.2.0
   git push origin v0.2.0
   ```

5. Quando o workflow terminar, abra a release em rascunho no GitHub, revise as notas e publique.

Dá para recompilar uma tag existente em *Actions › Release › Run workflow*.

## macOS e Linux

Não há builds para macOS e Linux. O app depende de partes do Windows (veja
[Arquitetura › Outras plataformas](arquitetura.md#outras-plataformas)). Um instalador dessas plataformas
abriria, mas não conseguiria baixar nem rodar PHP, bancos ou domínios. Por isso o workflow só publica o
instalador de Windows.

## Assinatura de código (pendente)

O instalador ainda não é assinado, por isso o SmartScreen avisa na primeira vez. Opções para resolver:

- **[SignPath Foundation](https://signpath.org)**: assinatura gratuita para projetos open source.
- **Azure Trusted Signing**: pago, mais barato que um certificado tradicional.
- Certificado OV/EV comprado.

Com um certificado, a assinatura entra no `tauri.conf.json` (`bundle.windows.signCommand` ou
`certificateThumbprint`), e as credenciais ficam em *secrets* do repositório, **nunca** no código.

## Atualização automática (pendente)

O *updater* do Tauri exige um par de chaves de assinatura de atualizações. A chave privada fica num
*secret* (`TAURI_SIGNING_PRIVATE_KEY`), e o arquivo `latest.json` é publicado na release. Fica para uma
versão futura.
