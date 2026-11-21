<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { kindLabel, projectUrl } from "../lib/api";
  import Glyph from "../lib/Glyph.svelte";
  import { php, type PhpMatch } from "../lib/php";
  import { projectsApi, slugDomain, type FolderProposal, type ProjectsOverview } from "../lib/projects";
  import { app } from "../lib/state.svelte";
  import Toggle from "../lib/Toggle.svelte";

  // Diálogo no estilo ContentDialog do Windows 11 para revisar e adicionar
  // um projeto (pasta avulsa ou projeto novo de uma pasta raiz).
  let {
    proposal,
    onclose,
    onsaved,
  }: {
    proposal: FolderProposal;
    onclose: () => void;
    onsaved: (data: ProjectsOverview, path: string) => void;
  } = $props();

  // Cópia editável da proposta (o diálogo é recriado a cada abertura).
  // svelte-ignore state_referenced_locally
  const initial = proposal.config;
  let domain = $state(initial.domain);
  let phpPin = $state(initial.php ?? "");
  let docroot = $state(initial.docroot);
  let node = $state(initial.node);
  let nodeCommand = $state(initial.node_command ?? "npm run dev");
  let match = $state<PhpMatch | null>(null);
  let error = $state<string | null>(null);
  let saving = $state(false);
  let dialog: HTMLDivElement;

  const name = $derived(proposal.config.path.split(/[\\/]/).pop() ?? "");
  const cleanDomain = $derived(slugDomain(domain));

  $effect(() => {
    if (proposal.view_kind !== "node") {
      php.match(proposal.php_request, null).then((m) => (match = m)).catch(() => {});
    }
    dialog?.querySelector<HTMLInputElement>("input")?.focus();
  });

  async function submit() {
    if (!cleanDomain) {
      error = t("Informe um domínio.");
      return;
    }
    saving = true;
    try {
      const data = await projectsApi.save({
        path: proposal.config.path,
        domain: cleanDomain,
        php: phpPin || null,
        docroot,
        node,
        node_command: node ? nodeCommand.trim() : proposal.config.node_command,
        node_version: null,
        database: null,
        db_version: null,
        db_port: null,
        online: false,
      });
      onsaved(data, proposal.config.path);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }

  const docrootLabel = (d: string) => (d ? `${d}\\` : t("Raiz do projeto"));
</script>

<svelte:window onkeydown={onKey} />

<div class="smoke" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-title" bind:this={dialog}>
    <div class="content">
      <h2 id="dlg-title">{t("Adicionar projeto")}</h2>

      <div class="folder">
        <Glyph name="folder" />
        <div>
          <strong>{name}</strong>
          <div class="caption mono" title={proposal.config.path}>{proposal.config.path}</div>
        </div>
        <span class="kind">{kindLabel[proposal.view_kind]}{proposal.view_kind !== "node" && proposal.has_node ? " + Node" : ""}</span>
      </div>

      {#if !proposal.recognized}
        <div class="notice warn">
          <Glyph name="info" size={14} />
          <span>{t("Não achei composer.json, index.php nem package.json. Vai ser tratado como site PHP simples.")}</span>
        </div>
      {/if}

      <form
        class="form"
        onsubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <label class="row">
          <span>{t("Domínio")}</span>
          <span class="domain">
            <input class="input" bind:value={domain} spellcheck="false" />
            <span class="caption">.{app.config?.tld}</span>
          </span>
        </label>
        {#if app.config && cleanDomain}
          <p class="caption preview">{t("Endereço:")} <span class="mono">{projectUrl(app.config, cleanDomain)}</span></p>
        {/if}

        {#if proposal.view_kind !== "node"}
          <div class="row">
            <span>{t("Versão do PHP")}</span>
            <div class="select">
              <select bind:value={phpPin}>
                <option value="">{match?.installed ? t("Automática ({v})", { v: match.installed }) : t("Automática")}</option>
                {#each match?.available ?? [] as minor (minor)}
                  <option value={minor}>PHP {minor}</option>
                {/each}
              </select>
            </div>
          </div>
          {#if proposal.php_request}
            <p class="caption preview">{t("Pedida pelo projeto: {v}", { v: proposal.php_request })}</p>
          {/if}

          <div class="row">
            <span>{t("Pasta pública")}</span>
            <div class="select">
              <select bind:value={docroot}>
                {#each proposal.docroots as d (d)}
                  <option value={d}>{docrootLabel(d)}</option>
                {/each}
              </select>
            </div>
          </div>
        {/if}

        {#if proposal.has_node}
          <div class="row">
            <span>{t("Servidor do Node")}</span>
            <Toggle label={t("Rodar o servidor de desenvolvimento do Node")} checked={node} onchange={(v) => (node = v)} />
          </div>
          {#if node}
            <label class="row">
              <span>{t("Comando")}</span>
              <input class="input mono-input" bind:value={nodeCommand} spellcheck="false" />
            </label>
          {/if}
        {/if}

        {#if error}
          <div class="notice err" role="alert">
            <Glyph name="info" size={14} />
            <span>{error.charAt(0).toUpperCase() + error.slice(1)}</span>
          </div>
        {/if}

        <button type="submit" hidden aria-hidden="true"></button>
      </form>
    </div>

    <div class="footer">
      <button class="btn accent" onclick={submit} disabled={saving}>{t("Adicionar")}</button>
      <button class="btn" onclick={onclose}>{t("Cancelar")}</button>
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
    animation: fade 0.12s ease-out;
  }
  .dialog {
    width: 480px;
    max-width: calc(100vw - 48px);
    max-height: calc(100vh - 80px);
    display: flex;
    flex-direction: column;
    border-radius: 8px;
    border: 1px solid var(--stroke);
    background: var(--card);
    box-shadow: var(--overlay-shadow), 0 32px 64px rgb(0 0 0 / 0.19);
    overflow: hidden;
    animation: pop 0.16s cubic-bezier(0.1, 0.9, 0.2, 1);
  }
  .content {
    padding: 24px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h2 {
    margin: 0;
    font: 600 20px/28px var(--font-display);
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: var(--r-card);
    background: var(--layer);
    border: 1px solid var(--stroke);
  }
  .folder > div {
    flex: 1;
    min-width: 0;
  }
  .folder .mono {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    font-size: 12px;
    color: var(--text-2);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 32px;
  }
  .row .select {
    width: 200px;
  }
  .domain {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .domain input {
    width: 160px;
    text-align: right;
  }
  .mono-input {
    width: 200px;
    font-family: var(--mono);
    font-size: 12px;
  }
  .preview {
    margin: -4px 0 4px;
    text-align: right;
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--r-card);
    border: 1px solid var(--stroke);
    font-size: 12px;
    line-height: 16px;
  }
  .notice.warn {
    background: color-mix(in srgb, var(--warn) 12%, var(--card));
  }
  .notice.warn :global(.glyph) {
    color: var(--warn);
  }
  .notice.err {
    background: color-mix(in srgb, var(--err) 12%, var(--card));
  }
  .notice.err :global(.glyph) {
    color: var(--err);
  }
  .footer {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 24px;
    background: var(--win-bg);
    border-top: 1px solid var(--stroke);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(1.05);
    }
  }
</style>
