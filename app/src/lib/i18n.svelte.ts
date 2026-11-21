// Tradução da interface. A chave é o próprio texto em português (estilo
// gettext): t("Adicionar projeto"), t("Projeto {name} criado", { name }).
// Cada idioma é um JSON em locales/<código>.json (raiz do projeto), carregado
// automaticamente: para um idioma novo basta criar o arquivo.

type Dict = Record<string, unknown> & { _meta?: { name?: string } };

const files = import.meta.glob("../../../locales/*.json", { eager: true, import: "default" }) as Record<string, Dict>;

const dicts: Record<string, Dict> = {};
for (const [path, dict] of Object.entries(files)) {
  dicts[path.split("/").pop()!.replace(/\.json$/, "")] = dict;
}

export const SOURCE = "pt-BR";

/** Idiomas disponíveis, para o seletor. */
export const languages = Object.entries(dicts)
  .map(([code, d]) => ({ code, name: d._meta?.name ?? code }))
  .sort((a, b) => a.name.localeCompare(b.name));

class I18n {
  lang = $state(SOURCE);
}
export const i18n = new I18n();

export function setLanguage(code: string | null | undefined) {
  i18n.lang = code && dicts[code] ? code : SOURCE;
  document.documentElement.lang = i18n.lang;
}

type Params = Record<string, string | number>;

/** Texto traduzido (ou o original, se faltar), com {parâmetros}. */
export function t(key: string, params?: Params): string {
  const value = dicts[i18n.lang]?.[key];
  let s = typeof value === "string" && value ? value : key;
  if (params) s = s.replace(/\{(\w+)\}/g, (m, k: string) => (k in params ? String(params[k]) : m));
  return s;
}

/** Código para datas e números (toLocaleString). */
export function locale(): string {
  return i18n.lang;
}
