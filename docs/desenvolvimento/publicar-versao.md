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
6. publica a release com o instalador, o `SHA256SUMS.txt` e as notas (seção do `CHANGELOG.md` + como instalar + lista de PRs).

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

5. Quando o workflow terminar, confira a release no GitHub (notas e arquivos).
6. Atualize o Larafim no winget (veja abaixo).

Dá para recompilar uma tag existente em *Actions › Release › Run workflow*.

## winget

Os manifestos ficam em [`packaging/winget`](../../packaging/winget), no mesmo formato de pastas do
repositório [`microsoft/winget-pkgs`](https://github.com/microsoft/winget-pkgs). O ID do pacote é
`Larafim.Larafim`.

O manifesto leva o SHA-256 do instalador, então só dá para fazê-lo **depois** que a release sai.

**Primeiro envio:** valide e envie a pasta da versão com o
[`wingetcreate`](https://github.com/microsoft/winget-create). Ele pede um token do GitHub com
`public_repo`, que você passa só na linha de comando, sem salvar em nenhum arquivo.

```powershell
winget validate --manifest .\packaging\winget\manifests\l\Larafim\Larafim\0.1.1
wingetcreate submit --token SEU_TOKEN .\packaging\winget\manifests\l\Larafim\Larafim\0.1.1
```

**Versões seguintes:** com o pacote já no winget, o `wingetcreate` gera o manifesto novo sozinho a
partir da URL do instalador:

```powershell
wingetcreate update Larafim.Larafim --version 0.2.0 --urls https://github.com/josevictoremiliano/larafim/releases/download/v0.2.0/Larafim_0.2.0_x64-setup.exe --submit --token SEU_TOKEN
```

Um robô valida o pull request e um moderador aprova (alguns dias).

## macOS e Linux

Não há builds para macOS e Linux. O app depende de partes do Windows (veja
[Arquitetura › Outras plataformas](arquitetura.md#outras-plataformas)). Um instalador dessas plataformas
abriria, mas não conseguiria baixar nem rodar PHP, bancos ou domínios. Por isso o workflow só publica o
instalador de Windows.

## Assinatura de código (pendente)

O instalador ainda não é assinado, por isso o SmartScreen avisa na primeira vez. O caminho escolhido
é a **[SignPath Foundation](https://signpath.org)**, que assina de graça projetos open source. O
certificado e a chave são deles, e o editor que aparece no Windows é "SignPath Foundation".

1. **Política de assinatura** publicada no README (seção *Política de assinatura de código*). Eles a
   leem ao analisar o pedido. Mantenha a lista de quem aprova assinaturas atualizada.
2. **Pedido** pelo formulário do site da SignPath Foundation. A análise pode levar semanas.
3. **Depois de aprovado:** o token de API da SignPath vai num *secret* do repositório (por exemplo
   `SIGNPATH_API_TOKEN`), **nunca** no código. O ID da organização e os nomes do projeto e da
   política de assinatura podem ficar no workflow.
4. **No `release.yml`:** um passo com `signpath/github-action-submit-signing-request` envia o arquivo,
   espera a aprovação no painel da SignPath e baixa o arquivo assinado. O ideal é assinar o
   `larafim-app.exe` e o `larafim-shim.exe` antes do NSIS empacotar, e depois o instalador.

## Atualização automática (pendente)

O *updater* do Tauri exige um par de chaves de assinatura de atualizações. A chave privada fica num
*secret* (`TAURI_SIGNING_PRIVATE_KEY`), e o arquivo `latest.json` é publicado na release. Fica para uma
versão futura.
