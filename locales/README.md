# Idiomas do Sentinel

Cada idioma é um arquivo `<código>.json` nesta pasta. A interface (Svelte) e o
backend (Rust) leem os mesmos arquivos, e o instalador pergunta o idioma a
partir deles.

A chave de cada texto é **o próprio texto em português** (estilo gettext):

```json
{
  "_meta": { "name": "English", "nsis": "English", "langid": 1033 },
  "Adicionar projeto": "Add project",
  "Servidor ligado · porta {port}": "Server on · port {port}"
}
```

- `_meta.name`: nome do idioma nele mesmo (aparece no seletor de Configurações).
- `_meta.nsis` e `_meta.langid`: idioma do instalador NSIS e o código de
  idioma do Windows (ex.: `Spanish` / `3082`). Sem eles, o idioma funciona no
  app mas não aparece na pergunta do instalador.
- `{nome}`: parâmetros. A tradução precisa ter os mesmos.
- O que faltar aparece em português. O `pt-BR.json` só tem o `_meta`.

## Acrescentar um idioma

1. Crie `locales/<código>.json` com o `_meta` (ex.: `es.json`).
2. `node scripts/i18n.mjs fill es` acrescenta todas as chaves com valor vazio.
3. Traduza os valores.
4. `node scripts/i18n.mjs check` mostra o que falta, o que sobrou e
   parâmetros diferentes.

Nada mais precisa ser registrado: o Rust inclui todos os JSON no build
(`crates/sentinel-core/build.rs`), a interface carrega todos sozinha, e o
build do instalador roda `node scripts/i18n.mjs sync`, que atualiza a lista
de idiomas do NSIS e o gancho que grava a escolha.

## No código

- Interface: `t("Texto")`, `t("Texto {x}", { x })`; plural com
  `plural(n, "{n} item", "{n} itens")`.
- Rust: `t!("Texto")`, `t!("Texto {x}", x = valor)`; em `#[error(...)]`,
  `sentinel_core::i18n::tf("Texto {0}", &[("0", &.0 as &dyn Display)])`.
- Logs (`tracing`) ficam em português: são para quem desenvolve.
