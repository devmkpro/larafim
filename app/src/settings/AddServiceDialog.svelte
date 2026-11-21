<script lang="ts" module>
  export type AddChoice =
    | { kind: "mariadb" | "postgres"; major: string; port: number }
    | { kind: "existing" | "mailpit" | "s3" };
</script>

<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import Glyph, { type GlyphName } from "../lib/Glyph.svelte";

  type Row = { major: string; latest: string | null; installed: string | null };

  // Escolher um serviço para instalar: abas por categoria, um cartão por serviço.
  let {
    maria,
    pg,
    mariaPort,
    pgPort,
    mailInstalled,
    s3Installed,
    onclose,
    onadd,
  }: {
    maria: Row[];
    pg: Row[];
    mariaPort: number;
    pgPort: number;
    mailInstalled: boolean;
    s3Installed: boolean;
    onclose: () => void;
    onadd: (choice: AddChoice) => void;
  } = $props();

  type Tab = "db" | "mail" | "storage";
  type Kind = AddChoice["kind"];
  interface Card {
    kind: Kind;
    tab: Tab;
    name: string;
    sub: string;
    icon: GlyphName;
    done: boolean;
  }

  const free = (rows: Row[]) => rows.filter((r) => !r.installed && r.latest);

  const cards = $derived<Card[]>([
    { kind: "mariadb", tab: "db", name: "MariaDB", sub: free(maria)[0]?.latest ?? t("Instalado"), icon: "database", done: !free(maria).length },
    { kind: "postgres", tab: "db", name: "PostgreSQL", sub: free(pg)[0]?.latest ?? t("Instalado"), icon: "database", done: !free(pg).length },
    { kind: "existing", tab: "db", name: t("Banco que já tenho"), sub: t("MySQL ou MariaDB"), icon: "link", done: false },
    { kind: "mailpit", tab: "mail", name: "Mailpit", sub: mailInstalled ? t("Instalado") : t("Caixa de e-mails local"), icon: "mail", done: mailInstalled },
    { kind: "s3", tab: "storage", name: t("Storage S3"), sub: s3Installed ? t("Instalado") : "RustFS", icon: "package", done: s3Installed },
  ]);

  const tabs: { id: Tab; label: () => string }[] = [
    { id: "db", label: () => t("Banco de dados") },
    { id: "mail", label: () => t("E-mail") },
    { id: "storage", label: () => t("Storage") },
  ];

  let tab = $state<Tab>("db");
  let kind = $state<Kind | null>(null);
  let major = $state("");
  let port = $state(0);

  const visible = $derived(cards.filter((c) => c.tab === tab));
  const versions = $derived(kind === "mariadb" ? free(maria) : kind === "postgres" ? free(pg) : []);

  function pick(c: Card) {
    if (c.done) return;
    kind = c.kind;
    if (c.kind === "mariadb" || c.kind === "postgres") {
      major = versions[0]?.major ?? "";
      port = c.kind === "mariadb" ? mariaPort : pgPort;
    }
  }

  function switchTab(id: Tab) {
    tab = id;
    const first = cards.find((c) => c.tab === id && !c.done);
    if (first) pick(first);
    else kind = null;
  }

  $effect.pre(() => {
    // Abre já com o primeiro disponível marcado.
    if (kind === null) {
      const first = cards.find((c) => c.tab === tab && !c.done);
      if (first) pick(first);
    }
  });

  const portOk = $derived(Number.isInteger(Number(port)) && Number(port) >= 1024 && Number(port) <= 65535);
  const canAdd = $derived(
    kind !== null && (kind === "mariadb" || kind === "postgres" ? !!major && portOk : true),
  );

  function add() {
    if (!kind || !canAdd) return;
    if (kind === "mariadb" || kind === "postgres") onadd({ kind, major, port: Number(port) });
    else onadd({ kind });
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="smoke" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="add-svc-title">
    <div class="content">
      <h2 id="add-svc-title">{t("Adicionar serviço")}</h2>
      <p class="caption">{t("O Sentinel baixa e deixa pronto; liga sozinho quando algum projeto precisa.")}</p>

      <div class="tabs" role="tablist">
        {#each tabs as tb (tb.id)}
          <button role="tab" class="tab" class:active={tab === tb.id} aria-selected={tab === tb.id} onclick={() => switchTab(tb.id)}>
            {tb.label()}
          </button>
        {/each}
      </div>

      <div class="cards">
        {#each visible as c (c.kind)}
          <button class="svc-card" class:selected={kind === c.kind} disabled={c.done} onclick={() => pick(c)}>
            {#if kind === c.kind}<span class="tick"><Glyph name="check" size={10} /></span>{/if}
            <Glyph name={c.icon} size={28} />
            <strong>{c.name}</strong>
            <span class="caption">{c.sub}</span>
          </button>
        {/each}
      </div>

      {#if kind === "mariadb" || kind === "postgres"}
        <div class="row">
          <span>{t("Versão")}</span>
          <div class="select">
            <select bind:value={major}>
              {#each versions as v (v.major)}<option value={v.major}>{v.latest}</option>{/each}
            </select>
          </div>
        </div>
        <div class="row">
          <span>{t("Porta")}</span>
          <input class="input" type="number" min="1024" max="65535" bind:value={port} />
        </div>
        <p class="caption hint">
          {kind === "mariadb"
            ? t("Se a 3306 já tiver outro banco (ex.: MySQL do Laragon), use 3307.")
            : t("A porta vale para todas as versões do PostgreSQL.")}
        </p>
      {:else if kind === "existing"}
        <p class="caption hint">{t("Use um MySQL ou MariaDB que já roda no PC (Laragon, XAMPP…) nos projetos, sem instalar outro.")}</p>
      {:else if kind === "mailpit"}
        <p class="caption hint">{t("Caixa de e-mails local: tudo que os projetos enviam fica aqui, nada sai para a internet")}</p>
      {:else if kind === "s3"}
        <p class="caption hint">{t("Testar upload de arquivos como na AWS, sem custo (RustFS, ~118 MB)")}</p>
      {/if}
    </div>
    <div class="footer">
      <span class="grow"></span>
      <button class="btn" onclick={onclose}>{t("Cancelar")}</button>
      <button class="btn accent" disabled={!canAdd} onclick={add}>
        {kind === "existing" ? t("Continuar") : t("Adicionar serviço")}
      </button>
    </div>
  </div>
</div>

<style>
  .smoke {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.3);
  }
  .dialog {
    width: 600px;
    max-width: calc(100vw - 48px);
    border-radius: 8px;
    border: 1px solid var(--stroke);
    background: var(--card);
    box-shadow: var(--overlay-shadow), 0 32px 64px rgb(0 0 0 / 0.19);
    overflow: hidden;
  }
  .content {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font: 600 20px/28px var(--font-display);
  }
  p {
    margin: 0;
  }
  .content > .caption:first-of-type {
    margin: -8px 0 4px;
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-ctrl);
    background: var(--layer);
    border: 1px solid var(--stroke);
  }
  .tab {
    flex: 1;
    height: 30px;
    border: 1px solid transparent;
    border-radius: calc(var(--r-ctrl) - 1px);
    background: transparent;
    color: var(--text-2);
  }
  .tab:hover {
    background: var(--subtle-hover);
  }
  .tab.active {
    background: var(--card);
    border-color: var(--stroke);
    color: var(--text);
    font-weight: 600;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .svc-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 18px 8px 14px;
    border-radius: var(--r-card);
    border: 1px solid var(--stroke);
    background: var(--layer);
    color: var(--text-2);
  }
  .svc-card strong {
    color: var(--text);
    font-weight: 600;
  }
  .svc-card:hover:not(:disabled) {
    background: var(--subtle-hover);
  }
  .svc-card.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .svc-card:disabled {
    opacity: 0.5;
  }
  .tick {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--accent);
    color: var(--accent-fg);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 32px;
  }
  .row > span {
    width: 80px;
  }
  .row > .input,
  .row > .select {
    width: 200px;
  }
  .hint {
    margin-left: 92px;
  }
  .row ~ .hint {
    margin-top: -4px;
  }
  .content > .hint:not(.row ~ .hint) {
    margin-left: 0;
  }
  .footer {
    display: flex;
    gap: 8px;
    padding: 20px 24px;
    border-top: 1px solid var(--stroke);
    background: var(--layer);
  }
  .grow {
    flex: 1;
  }
</style>
