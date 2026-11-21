# Como contribuir

Obrigado pelo interesse! Issues e pull requests são bem-vindos.

## Antes de começar

- **Falha de segurança?** Não abra issue pública: veja [SECURITY.md](SECURITY.md).
- **Mudança grande** (nova funcionalidade, troca de dependência importante)? Abra uma issue antes para
  combinar o caminho.
- Leia a [arquitetura](docs/desenvolvimento/arquitetura.md) e
  [compilar e testar](docs/desenvolvimento/compilar-e-testar.md).

## Fluxo

1. Faça um fork e crie um branch a partir da `main`.
2. Faça a mudança, com testes.
3. Rode localmente:

   ```powershell
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   cd app; npm run check; cd ..
   node scripts/i18n.mjs check
   ```

4. Abra o pull request descrevendo **o que** mudou e **por quê**. O CI precisa ficar verde.

## Regras do projeto

- **Leveza primeiro.** Nada roda em segundo plano sem necessidade; serviços sobem sob demanda.
- **Visual nativo do Windows 11.** Siga os componentes existentes (Fluent, Segoe Fluent Icons), sem cara
  de site.
- **Textos traduzíveis.** Todo texto visível passa por `t("…")` (interface) ou `t!("…")` (Rust), com a
  chave em português. Traduza o texto novo em `locales/en.json`
  ([Traduções](docs/desenvolvimento/traducoes.md)).
- **Nunca mexer no projeto do usuário sem pedir.** Mudanças no `.env` sempre com prévia, cópia e desfazer.
- **Segurança.** Dado de fora (download, arquivo de projeto, requisição) é validado antes de virar
  caminho, linha de arquivo, SQL ou comando, e ganha um teste com entrada maliciosa
  ([Segurança](docs/seguranca.md#ao-mexer-no-código)).
- **Dependências novas** só com bom motivo: cada uma aumenta o binário e a superfície de ataque.

## Mensagens de commit

Curtas e no imperativo, de preferência no formato
[Conventional Commits](https://www.conventionalcommits.org/pt-br/) (`feat:`, `fix:`, `docs:`…).

## Licença

Ao contribuir, você concorda que sua contribuição seja distribuída sob a [licença MIT](LICENSE) do projeto.
