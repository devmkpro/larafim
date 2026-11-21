<script lang="ts">
  import { locale, t } from "../lib/i18n.svelte";
  import { onEvent } from "../lib/api";
  import Glyph from "../lib/Glyph.svelte";
  import { confirm } from "../lib/php";
  import ProgressBar from "../lib/ProgressBar.svelte";
  import {
    services,
    type ConnectionInfo,
    type ConnectionInput,
    type DetectedDb,
    type ServicesOverview,
  } from "../lib/services";
  import ConnectionDialog from "./ConnectionDialog.svelte";
  import { toolsApi, type MailpitStatus } from "../lib/tools";
  import { pg, type PgOverview } from "../lib/pg";
  import { s3, type S3Status } from "../lib/s3";

  let s3st = $state<S3Status | null>(null);
  let s3progress = $state<{ received: number; total: number | null } | null>(null);
  async function loadS3() {
    s3st = await s3.status().catch(() => null);
  }

  let pgView = $state<PgOverview | null>(null);
  let pgPort = $state(5432);
  async function loadPg(refresh = false) {
    try {
      pgView = await pg.overview(refresh);
      pgPort = pgView.port;
    } catch (e) {
      error = String(e);
    }
  }
  async function pgInstall(major: string) {
    progress["pg" + major] = "starting";
    try {
      await pg.install(major);
    } catch (e) {
      delete progress["pg" + major];
      error = String(e);
    }
  }
  async function pgRemove(major: string) {
    const ok = await confirm(
      t("Remover os programas do PostgreSQL {v}? Os bancos (dados) ficam guardados e voltam se você reinstalar.", { v: major }),
      t("Remover PostgreSQL"),
      t("Remover"),
    );
    if (ok) run(() => pg.remove(major));
  }
  function commitPgPort() {
    if (!pgView) return;
    const p = Number(pgPort);
    if (!Number.isInteger(p) || p < 1024 || p > 65535) {
      pgPort = pgView.port;
      return;
    }
    if (p !== pgView.port) run(() => pg.configure(pgView!.selected, p));
  }
  const pgInstalled = $derived(pgView?.rows.filter((r) => r.installed) ?? []);

  let mail = $state<MailpitStatus | null>(null);
  async function loadMail() {
    mail = await toolsApi.mailpitStatus().catch(() => null);
  }
  import SettingRow from "./SettingRow.svelte";

  let connections = $state<ConnectionInfo[]>([]);
  let detected = $state<DetectedDb[]>([]);
  let editing = $state<{ input: ConnectionInput; hasPassword: boolean } | null>(null);

  async function loadConnections() {
    connections = await services.connections().catch(() => []);
    detected = await services.detect().catch(() => []);
  }

  function useDetected(d: DetectedDb) {
    editing = {
      hasPassword: false,
      input: {
        id: null,
        name: t("{kind} (porta {port})", { kind: d.kind === "mysql" ? "MySQL" : "MariaDB", port: d.port }),
        kind: d.kind,
        host: "127.0.0.1",
        port: d.port,
        user: "root",
        password: null,
        client_dir: d.client_dir,
      },
    };
  }

  function newConnection() {
    editing = {
      hasPassword: false,
      input: { id: null, name: "", kind: "mysql", host: "127.0.0.1", port: 3306, user: "root", password: null, client_dir: null },
    };
  }

  function edit(c: ConnectionInfo) {
    editing = {
      hasPassword: c.has_password,
      input: { id: c.id, name: c.name, kind: c.kind, host: c.host, port: c.port, user: c.user, password: null, client_dir: c.client_dir },
    };
  }

  async function removeConnection(c: ConnectionInfo) {
    const ok = await confirm(t('Tirar "{name}" do Sentinel? O servidor e os bancos dele não são alterados.', { name: c.name }), t("Remover conexão"), t("Remover"));
    if (ok) run(() => services.removeConnection(c.id));
  }

  let overview = $state<ServicesOverview | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let progress = $state<Record<string, { received: number; total: number | null } | "starting">>({});
  let port = $state(3306);

  async function load(refresh = false) {
    try {
      overview = await services.overview(refresh);
      port = overview.port;
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    load();
    loadConnections();
    loadMail();
    loadPg();
    loadS3();
    const stops = [
      onEvent("services-changed", () => (load(), loadConnections(), loadMail(), loadPg(), loadS3())),
      onEvent<{ received: number; total: number | null }>("s3:progress", (p) => (s3progress = p)),
      onEvent<{ major: string; received: number; total: number | null }>("pg:progress", (p) => {
        progress["pg" + p.major] = { received: p.received, total: p.total };
      }),
      onEvent<{ major: string; ok: boolean; error: string | null }>("pg:done", (d) => {
        delete progress["pg" + d.major];
        if (!d.ok) error = d.error;
        loadPg();
      }),
      onEvent<{ major: string; received: number; total: number | null }>("maria:progress", (p) => {
        progress[p.major] = { received: p.received, total: p.total };
      }),
      onEvent<{ major: string; ok: boolean; error: string | null }>("maria:done", (d) => {
        delete progress[d.major];
        if (!d.ok) error = d.error;
        load();
      }),
    ];
    return () => stops.forEach((s) => s());
  });

  async function run(action: () => Promise<unknown>) {
    busy = true;
    error = null;
    try {
      await action();
      await load();
      await loadPg();
      await loadS3();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function install(major: string) {
    progress[major] = "starting";
    try {
      await services.install(major);
    } catch (e) {
      delete progress[major];
      error = String(e);
    }
  }

  async function remove(major: string) {
    const ok = await confirm(
      t("Remover os programas do MariaDB {v}? Os bancos (dados) ficam guardados e voltam se você reinstalar.", { v: major }),
      t("Remover MariaDB"),
      t("Remover"),
    );
    if (ok) run(() => services.remove(major));
  }

  function commitPort() {
    if (!overview) return;
    const p = Number(port);
    if (!Number.isInteger(p) || p < 1024 || p > 65535) {
      port = overview.port;
      return;
    }
    if (p !== overview.port) run(() => services.configure(overview!.selected, p));
  }

  const installed = $derived(overview?.rows.filter((r) => r.installed) ?? []);
  const mb = (n: number) => (n / 1024 / 1024).toLocaleString(locale(), { maximumFractionDigits: 0 });
</script>

<h1>{t("Serviços")}</h1>
<p class="lead caption">{t("Ficam desligados e só sobem quando algum projeto precisa — ou quando você liga aqui.")}</p>

{#if error}
  <div class="infobar err"><Glyph name="info" /><span>{error}</span></div>
{/if}

{#if overview}
  <div class="card service">
    <div class="svc-head">
      <span class="icon"><Glyph name="database" size={20} /></span>
      <div class="text">
        <strong>MariaDB</strong>
        <span class="caption">
          {#if overview.status.running}
            <span class="dot on inline"></span> {t("Rodando · {v} · porta {port}", { v: overview.status.version ?? "", port: overview.status.port ?? "" })}
          {:else if installed.length}
            {t("Desligado · liga sozinho quando um projeto que usa banco fica online")}
          {:else}
            {t("Não instalado — ou use um banco que você já tem (abaixo)")}
          {/if}
        </span>
      </div>
      {#if installed.length}
        {#if overview.status.running}
          <button class="btn" disabled={busy} onclick={() => run(services.stop)}>{t("Desligar")}</button>
        {:else}
          <button class="btn accent" disabled={busy} onclick={() => run(services.start)}>{t("Ligar")}</button>
        {/if}
      {/if}
    </div>

    {#if installed.length}
      <div class="grid">
        <label class="field">
          <span>{t("Versão em uso")}</span>
          <div class="select">
            <select
              value={overview.selected ?? ""}
              disabled={busy}
              onchange={(e) => run(() => services.configure(e.currentTarget.value || null, overview!.port))}
            >
              <option value="">{t("Mais nova instalada")}</option>
              {#each installed as r (r.major)}<option value={r.major}>MariaDB {r.major}</option>{/each}
            </select>
          </div>
        </label>
        <label class="field">
          <span>{t("Porta")}</span>
          <input
            class="input port"
            type="number"
            min="1024"
            max="65535"
            bind:value={port}
            onblur={commitPort}
            onkeydown={(e) => e.key === "Enter" && commitPort()}
          />
        </label>
      </div>
      <p class="caption note">
        {t("Acesso:")} <span class="mono">127.0.0.1:{overview.port}</span>{t(", usuário")} <span class="mono">root</span>{t(", sem senha (só neste PC). Se a 3306 já tiver outro banco (ex.: MySQL do Laragon), use 3307.")}
      </p>
      {#if overview.status.running && overview.databases.length}
        <div class="dbs">
          <span class="section-title">{t("Bancos")}</span>
          {#each overview.databases as db (db)}
            <button class="chip" title={t("Abrir no Adminer")} onclick={() => run(() => services.openAdminer(db))}>
              <Glyph name="database" size={12} />
              {db}
            </button>
          {/each}
        </div>
      {/if}
    {/if}

    <div class="table">
      {#each overview.rows as row (row.major)}
        {@const p = progress[row.major]}
        <div class="tr">
          <div>
            <strong>MariaDB {row.major}</strong>
            <span class="caption">LTS{row.eol ? " " + t("até {year}", { year: row.eol.slice(0, 4) }) : ""} · {row.latest ?? "—"}</span>
          </div>
          <div>
            {#if row.installed}
              <span class="ok"><Glyph name="check" size={12} /> {row.installed}</span>
              {#if row.update_available}<span class="badge">{row.latest} disponível</span>{/if}
            {:else}<span class="caption">—</span>{/if}
          </div>
          <div class="actions">
            {#if p}
              <div class="progress">
                <span class="caption">{p === "starting" ? t("Preparando…") : (p.total ? t("Baixando {a} de {b} MB", { a: mb(p.received), b: mb(p.total) }) : t("Baixando {a} MB", { a: mb(p.received) }))}</span>
                <ProgressBar value={p === "starting" || !p.total ? null : p.received / p.total} />
              </div>
            {:else if row.installed}
              {#if row.update_available}<button class="btn accent" onclick={() => install(row.major)}>{t("Atualizar")}</button>{/if}
              <button class="btn subtle icon" title={t("Remover")} onclick={() => remove(row.major)}><Glyph name="delete" /></button>
            {:else if row.latest}
              <button class="btn" onclick={() => install(row.major)}>{t("Instalar")}</button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>

  {#if pgView}
    <div class="card service">
      <div class="svc-head">
        <span class="icon"><Glyph name="database" size={20} /></span>
        <div class="text">
          <strong>PostgreSQL</strong>
          <span class="caption">
            {#if pgView.status.running}
              <span class="dot on inline"></span> {t("Rodando · {v} · porta {port}", { v: pgView.status.version ?? "", port: pgView.status.port ?? "" })}
            {:else if pgInstalled.length}
              {t("Desligado · liga sozinho quando um projeto que usa PostgreSQL fica online")}
            {:else}
              {t("Opcional · instale só se algum projeto usa PostgreSQL (~50 MB)")}
            {/if}
          </span>
        </div>
        {#if pgInstalled.length}
          {#if pgView.status.running}
            <button class="btn" disabled={busy} onclick={() => run(pg.stop)}>{t("Desligar")}</button>
          {:else}
            <button class="btn accent" disabled={busy} onclick={() => run(pg.start)}>{t("Ligar")}</button>
          {/if}
        {/if}
      </div>

      {#if pgInstalled.length}
        <div class="grid">
          <label class="field">
            <span>{t("Versão em uso")}</span>
            <div class="select">
              <select
                value={pgView.selected ?? ""}
                disabled={busy}
                onchange={(e) => run(() => pg.configure(e.currentTarget.value || null, pgView!.port))}
              >
                <option value="">{t("Mais nova instalada")}</option>
                {#each pgInstalled as r (r.major)}<option value={r.major}>PostgreSQL {r.major}</option>{/each}
              </select>
            </div>
          </label>
          <label class="field">
            <span>{t("Porta")}</span>
            <input
              class="input port"
              type="number"
              min="1024"
              max="65535"
              bind:value={pgPort}
              onblur={commitPgPort}
              onkeydown={(e) => e.key === "Enter" && commitPgPort()}
            />
          </label>
        </div>
        <p class="caption note">
          {t("Acesso:")} <span class="mono">127.0.0.1:{pgView.port}</span>{t(", usuário")} <span class="mono">postgres</span>{t(", sem senha (só neste PC).")}
        </p>
        {#if pgView.status.running && pgView.databases.length}
          <div class="dbs">
            <span class="section-title">{t("Bancos")}</span>
            {#each pgView.databases as db (db)}
              <button class="chip" title={t("Abrir no Adminer")} onclick={() => run(() => services.openAdminer(db, "postgres"))}>
                <Glyph name="database" size={12} />
                {db}
              </button>
            {/each}
          </div>
        {/if}
      {/if}

      {#if pgView.catalog_error}
        <p class="caption note">{t("Não foi possível ver as versões: {error}", { error: pgView.catalog_error })}</p>
      {/if}
      <div class="table">
        {#each pgView.rows as row (row.major)}
          {@const p = progress["pg" + row.major]}
          <div class="tr">
            <div>
              <strong>PostgreSQL {row.major}</strong>
              <span class="caption">{row.latest ?? "—"}</span>
            </div>
            <div>
              {#if row.installed}
                <span class="ok"><Glyph name="check" size={12} /> {row.installed}</span>
                {#if row.update_available}<span class="badge">{row.latest} disponível</span>{/if}
              {:else}<span class="caption">—</span>{/if}
            </div>
            <div class="actions">
              {#if p}
                <div class="progress">
                  <span class="caption">{p === "starting" ? t("Preparando…") : (p.total ? t("Baixando {a} de {b} MB", { a: mb(p.received), b: mb(p.total) }) : t("Baixando {a} MB", { a: mb(p.received) }))}</span>
                  <ProgressBar value={p === "starting" || !p.total ? null : p.received / p.total} />
                </div>
              {:else if row.installed}
                {#if row.update_available}<button class="btn accent" onclick={() => pgInstall(row.major)}>{t("Atualizar")}</button>{/if}
                <button class="btn subtle icon" title={t("Remover")} onclick={() => pgRemove(row.major)}><Glyph name="delete" /></button>
              {:else if row.latest}
                <button class="btn" onclick={() => pgInstall(row.major)}>{t("Instalar")}</button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if overview.own.length}
    <h2 class="sub">{t("MariaDB próprio de projetos")}</h2>
    <p class="caption sub-lead">{t("Servidores separados, com versão e dados só daquele projeto.")}</p>
    {#each overview.own as o (o.path)}
      <SettingRow
        icon="database"
        title={o.project}
        description={t("MariaDB {v} · porta {port} · {state}", { v: o.version, port: o.port, state: o.running ? t("ligado") : t("desligado") })}
      >
        {#if o.running}
          <button class="btn" disabled={busy} onclick={() => run(() => services.ownStop(o.path))}>{t("Desligar")}</button>
        {:else}
          <button class="btn" disabled={busy} onclick={() => run(() => services.ownStart(o.path))}>{t("Ligar")}</button>
        {/if}
        <button class="btn subtle" disabled={busy} onclick={() => run(() => services.openAdminer(null, null, o.path))}>Adminer</button>
      </SettingRow>
    {/each}
  {/if}

  <h2 class="sub">{t("Bancos que você já tem")}</h2>
  <p class="caption sub-lead">
    {t("Use um MySQL ou MariaDB que já roda no PC (Laragon, XAMPP…) nos projetos, sem instalar outro.")}
  </p>
  {#each detected.filter((d) => !d.known) as d (d.port)}
    <div class="infobar found">
      <Glyph name="database" />
      <span>
        {t("Encontrei um")} <strong>{d.kind === "mysql" ? "MySQL" : "MariaDB"}</strong> {t("rodando na porta {port}", { port: d.port })}
        <span class="caption">({d.process}{d.hint ? ` · ${d.hint}` : ""})</span>
      </span>
      <button class="btn accent" onclick={() => useDetected(d)}>{t("Usar este")}</button>
    </div>
  {/each}
  {#each connections as c (c.id)}
    <SettingRow
      icon="database"
      title={c.name}
      description={`${c.kind === "mysql" ? "MySQL" : "MariaDB"} · ${c.user}@${c.host}:${c.port}${c.used_by.length ? " · " + t("usado por {list}", { list: c.used_by.join(", ") }) : ""}`}
    >
      <button class="btn" onclick={() => run(() => services.openAdminer(null, c.id))}>Adminer</button>
      <button class="btn subtle icon" title={t("Editar")} onclick={() => edit(c)}><Glyph name="settings" /></button>
      <button class="btn subtle icon" title={t("Remover")} onclick={() => removeConnection(c)}><Glyph name="delete" /></button>
    </SettingRow>
  {/each}
  <div class="add"><button class="btn" onclick={newConnection}><Glyph name="add" /> {t("Adicionar banco existente")}</button></div>

  <h2 class="sub">{t("Outros serviços")}</h2>
  <SettingRow icon="globe" title="Adminer" description={t("Gerenciar os bancos pelo navegador em {url}", { url: overview.adminer_url.replace(/^https?:\/\//, "") })}>
    <button class="btn" disabled={busy || !installed.length} onclick={() => run(() => services.openAdminer())}>
      <Glyph name="openExternal" /> {t("Abrir")}
    </button>
  </SettingRow>
  <SettingRow
    icon="package"
    title={t("Storage S3 local")}
    description={s3st?.installed
      ? t("RustFS {v} · API {endpoint} · chave {key} · {state}", { v: s3st.version ?? "", endpoint: s3st.endpoint, key: s3st.access_key, state: s3st.running ? t("ligado") : t("desligado") })
      : t("Testar upload de arquivos como na AWS, sem custo (RustFS, ~118 MB)")}
  >
    {#if s3st?.installing || s3progress}
      <div class="progress">
        <span class="caption">{s3progress ? (s3progress.total ? t("Baixando {a} de {b} MB", { a: mb(s3progress.received), b: mb(s3progress.total) }) : t("Baixando {a} MB", { a: mb(s3progress.received) })) : t("Preparando…")}</span>
        <ProgressBar value={s3progress?.total ? s3progress.received / s3progress.total : null} />
      </div>
    {:else if !s3st?.installed}
      <button class="btn" disabled={busy} onclick={() => run(async () => { try { await s3.install(); } finally { s3progress = null; } })}>{t("Instalar")}</button>
    {:else if s3st.running}
      <button class="btn" disabled={busy} onclick={() => run(s3.stop)}>{t("Desligar")}</button>
    {:else}
      <button class="btn" disabled={busy} onclick={() => run(s3.start)}>{t("Ligar")}</button>
    {/if}
    {#if s3st?.installed}
      <button class="btn subtle" disabled={busy} onclick={() => run(s3.open)}>{t("Abrir")}</button>
    {/if}
  </SettingRow>
  <SettingRow
    icon="mail"
    title="Mailpit"
    description={mail?.installed
      ? t("Caixa de e-mails local · SMTP {smtp} · {state}", { smtp: mail.smtp, state: mail.running ? t("ligado") : t("desligado") }) + (mail.version ? ` · ${mail.version}` : "")
      : t("Caixa de e-mails local: tudo que os projetos enviam fica aqui, nada sai para a internet")}
  >
    {#if !mail?.installed}
      <button class="btn" disabled={busy} onclick={() => run(toolsApi.mailpitInstall)}>{t("Instalar")}</button>
    {:else if mail.running}
      <button class="btn" disabled={busy} onclick={() => run(async () => toolsApi.mailpitStop())}>{t("Desligar")}</button>
    {:else}
      <button class="btn" disabled={busy} onclick={() => run(toolsApi.mailpitStart)}>{t("Ligar")}</button>
    {/if}
    {#if mail?.installed}
      <button class="btn subtle" disabled={busy} onclick={() => run(toolsApi.openMailpit)}>{t("Abrir")}</button>
    {/if}
  </SettingRow>
{:else}
  <div class="loading"><ProgressBar /></div>
{/if}

{#if editing}
  <ConnectionDialog
    initial={editing.input}
    hasPassword={editing.hasPassword}
    onclose={() => (editing = null)}
    onsaved={() => {
      editing = null;
      loadConnections();
    }}
  />
{/if}

<style>
  h1 {
    margin: 0 0 16px;
    font: 600 28px/36px var(--font-display);
  }
  .lead {
    margin: -8px 0 12px;
  }
  .infobar.err {
    margin-bottom: 8px;
    background: color-mix(in srgb, var(--err) 12%, var(--card));
    user-select: text;
  }
  .service + .service {
    margin-top: 12px;
  }
  .service {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 4px;
  }
  .svc-head {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .icon {
    width: 24px;
    display: flex;
    justify-content: center;
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .dot.inline {
    display: inline-block;
    vertical-align: middle;
    margin-right: 2px;
  }
  .grid {
    display: flex;
    gap: 24px;
    padding-left: 38px;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .field .select {
    width: 190px;
  }
  .port {
    width: 96px;
  }
  .note {
    margin: -4px 0 0 38px;
  }
  .dbs {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding-left: 38px;
  }
  .dbs .section-title {
    margin-right: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border-radius: 13px;
    border: 1px solid var(--stroke);
    background: var(--layer);
    font-family: var(--mono);
    font-size: 12px;
  }
  .chip:hover {
    background: var(--subtle-hover);
  }
  .table {
    border-top: 1px solid var(--stroke);
    margin: 0 -16px -14px;
  }
  .tr {
    display: grid;
    grid-template-columns: 1.2fr 1fr minmax(200px, auto);
    align-items: center;
    gap: 12px;
    min-height: 52px;
    padding: 6px 12px 6px 54px;
  }
  .tr + .tr {
    border-top: 1px solid var(--stroke);
  }
  .tr > div:first-child {
    display: flex;
    flex-direction: column;
  }
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .ok :global(.glyph) {
    color: var(--ok);
  }
  .badge {
    margin-left: 8px;
    font-size: 12px;
    padding: 0 8px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--accent-text);
    font-weight: 600;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 4px;
  }
  .progress {
    width: 200px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .loading {
    width: 200px;
  }
  .sub {
    margin: 24px 0 2px;
    font: 600 14px/20px var(--font);
  }
  .sub-lead {
    margin: 0 0 8px;
  }
  .infobar.found {
    align-items: center;
    margin-bottom: 4px;
  }
  .infobar.found span {
    flex: 1;
  }
  .add {
    margin: 4px 0 0;
  }
</style>
