<script lang="ts">
  import { api, kindLabel, projectUrl } from "../lib/api";
  import Glyph from "../lib/Glyph.svelte";
  import { confirm, php, type PhpMatch } from "../lib/php";
  import { projectsApi, slugDomain, toConfig, type Project, type ProjectConfig, type ProjectsOverview } from "../lib/projects";
  import { asServerError, server, type ServerError } from "../lib/server";
  import { terminal } from "../lib/terminal";
  import { app } from "../lib/state.svelte";
  import Toggle from "../lib/Toggle.svelte";

  let {
    project,
    tick = 0,
    onchange,
    onreview,
  }: {
    project: Project | null;
    tick?: number;
    onchange: (data: ProjectsOverview) => void;
    onreview: (path: string) => void;
  } = $props();

  let copied = $state(false);
  let match = $state<PhpMatch | null>(null);
  let docroots = $state<string[]>([]);
  let error = $state<string | null>(null);
  let saving = $state(false);

  // Campos de texto: só gravam ao confirmar (Enter ou sair do campo).
  let domain = $state("");
  let nodeCommand = $state("");

  const url = $derived(project && app.config ? projectUrl(app.config, project.domain) : "");
  const imported = $derived(project?.status === "imported");

  // Ao trocar de projeto (ou ele mudar), recarrega campos e dados auxiliares.
  $effect(() => {
    const p = project;
    void tick;
    error = null;
    match = null;
    if (!p) return;
    domain = p.domain;
    nodeCommand = p.node_command ?? "";
    if (p.kind !== "node" && p.status !== "missing") {
      php
        .match(p.php_request, p.php_pin)
        .then((m) => {
          if (project?.path === p.path) match = m;
        })
        .catch(() => {});
    }
    if (p.status === "imported") {
      projectsApi
        .docroots(p.path)
        .then((d) => {
          if (project?.path === p.path) docroots = d.includes(p.docroot) ? d : [...d, p.docroot];
        })
        .catch(() => (docroots = [p.docroot]));
    }
  });

  async function save(change: Partial<ProjectConfig>) {
    if (!project) return;
    saving = true;
    try {
      onchange(await projectsApi.save({ ...toConfig(project), ...change }));
      error = null;
    } catch (e) {
      error = String(e);
      // Volta os campos de texto ao valor gravado.
      domain = project.domain;
      nodeCommand = project.node_command ?? "";
    } finally {
      saving = false;
    }
  }

  function commitDomain() {
    if (!project) return;
    const value = slugDomain(domain);
    if (!value) {
      domain = project.domain;
      return;
    }
    domain = value;
    if (value !== project.domain) save({ domain: value });
  }

  function commitNodeCommand() {
    if (project && nodeCommand.trim() !== (project.node_command ?? "")) save({ node_command: nodeCommand.trim() });
  }

  async function act(action: Promise<ProjectsOverview>) {
    try {
      onchange(await action);
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  // ---------- Online / offline ----------
  let switching = $state(false);
  let serverError = $state<ServerError | null>(null);
  const altPort = $derived(app.config?.http_port === 8080 ? 8081 : 8080);

  $effect(() => {
    void project?.path;
    serverError = null;
  });

  async function toggleOnline(online: boolean) {
    if (!project) return;
    const path = project.path;
    switching = true;
    serverError = null;
    try {
      await server.setOnline(path, online);
    } catch (e) {
      if (project?.path === path) serverError = asServerError(e);
    } finally {
      switching = false;
    }
  }

  async function useAltPort() {
    await app.update({ http_port: altPort });
    await toggleOnline(true);
  }

  async function removeProject() {
    if (!project) return;
    const ok = await confirm(
      `Tirar "${project.name}" do Sentinel? Os arquivos do projeto NÃO são apagados.` +
        (project.root ? " Ele deixa de aparecer na lista; dá para mostrar de novo em Configurações › Pastas de projetos." : ""),
      "Remover do Sentinel",
      "Remover",
    );
    if (ok) act(projectsApi.remove(project.path));
  }

  async function copyUrl() {
    await navigator.clipboard.writeText(url);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  const docrootLabel = (d: string) => (d ? `${d}\\` : "Raiz do projeto");
  const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
</script>

<aside class="inspector">
  {#if project && app.config}
    <div class="head">
      <div class="title-row">
        <h2 title={url}>{project.domain}.{app.config.tld}</h2>
        <button class="btn subtle icon" title={copied ? "Copiado!" : "Copiar endereço"} onclick={copyUrl}>
          <Glyph name={copied ? "check" : "link"} />
        </button>
        {#if project.status !== "missing"}
          <button
            class="btn subtle icon"
            title="Abrir terminal na pasta (php e composer com a versão do projeto)"
            onclick={() => terminal.openTerminal(project.path).catch((e) => (error = String(e)))}
          >
            <Glyph name="terminal" />
          </button>
          <button class="btn subtle icon" title="Abrir pasta no Explorador" onclick={() => api.openFolder(project.path)}>
            <Glyph name="folderOpen" />
          </button>
        {/if}
      </div>
      <div class="path mono" title={project.path}>{project.path}</div>
    </div>

    {#if error}
      <div class="notice err" role="alert">
        <Glyph name="info" size={14} />
        <span>{capitalize(error)}</span>
      </div>
    {/if}

    {#if project.status === "new"}
      <div class="notice accent">
        <Glyph name="info" size={14} />
        <span>Projeto encontrado na pasta, ainda não importado. Importe para configurar e usar.</span>
      </div>
      <div class="buttons">
        <button class="btn accent" onclick={() => act(projectsApi.import(project.path))}>Importar</button>
        <button class="btn" onclick={() => onreview(project.path)}>Revisar…</button>
        <span class="grow"></span>
        <button class="btn subtle" title="Esconder da lista" onclick={() => act(projectsApi.ignore(project.path))}>
          Ignorar
        </button>
      </div>
    {:else if project.status === "missing"}
      <div class="notice warn">
        <Glyph name="info" size={14} />
        <span>A pasta deste projeto não existe mais (foi movida, renomeada ou apagada).</span>
      </div>
      <div class="buttons">
        <button class="btn" onclick={removeProject}>Remover do Sentinel</button>
      </div>
    {:else}
      <div class="status card" class:is-on={project.online}>
        <span class="dot" class:on={project.online} class:busy={switching}></span>
        <div class="status-text">
          <strong>{switching ? (project.online ? "Desligando…" : "Ligando…") : project.online ? "Online" : "Offline"}</strong>
          <span class="caption">
            {#if project.kind === "node"}
              Projetos só de Node ficam online na fase 5
            {:else if project.online}
              {url.replace("http://", "")}
            {:else}
              Ligue para acessar no navegador
            {/if}
          </span>
        </div>
        <Toggle
          label="Projeto online"
          checked={project.online}
          disabled={switching || project.kind === "node"}
          onchange={(v) => toggleOnline(v)}
        />
      </div>

      {#if serverError}
        <div class="notice err server-error" role="alert">
          <Glyph name="info" size={14} />
          <div class="err-body">
            <span>{capitalize(serverError.message)}</span>
            {#if serverError.code === "port_busy"}
              <span class="caption">
                Feche o outro programa ou use outra porta (os endereços passam a ter
                <span class="mono">:{altPort}</span>).
              </span>
            {/if}
            <div class="err-actions">
              {#if serverError.code === "port_busy"}
                <button class="btn" onclick={useAltPort}>Usar porta {altPort}</button>
                <button class="btn subtle" onclick={() => toggleOnline(true)}>Tentar de novo</button>
              {:else if serverError.code === "uac_denied" || serverError.code === "hosts_failed"}
                <button class="btn" onclick={() => toggleOnline(true)}>Tentar de novo</button>
              {:else if serverError.code === "no_php"}
                <button class="btn" onclick={() => api.openSettings("php")}>
                  {serverError.suggest ? `Instalar PHP ${serverError.suggest}` : "Instalar PHP"}
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/if}

      <button class="btn wide" disabled={!project.online} onclick={() => server.openUrl(url)}>
        <Glyph name="globe" />
        Abrir no navegador
      </button>
    {/if}

    {#if project.status !== "missing"}
      <div class="section">
        <div class="section-title">Ambiente</div>

        <div class="field">
          <span>Tipo</span>
          <span class="value">{kindLabel[project.kind]}{project.kind !== "node" && project.has_node ? " + Node" : ""}</span>
        </div>

        {#if imported}
          <label class="field">
            <span>Domínio</span>
            <span class="domain">
              <input
                class="input"
                bind:value={domain}
                onblur={commitDomain}
                onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()}
                spellcheck="false"
              />
              <span class="caption">.{app.config.tld}</span>
            </span>
          </label>
        {/if}

        {#if project.kind !== "node"}
          <div class="field">
            <span>Versão do PHP</span>
            <div class="select">
              <select
                disabled={!imported || saving}
                value={project.php_pin ?? ""}
                onchange={(e) => save({ php: e.currentTarget.value || null })}
              >
                <option value="">
                  {match?.installed && !project.php_pin ? `Automática (${match.installed})` : "Automática"}
                </option>
                {#each match?.available ?? [] as minor (minor)}
                  <option value={minor}>PHP {minor}</option>
                {/each}
                {#if project.php_pin && !(match?.available ?? []).includes(project.php_pin)}
                  <option value={project.php_pin}>PHP {project.php_pin} (não instalado)</option>
                {/if}
              </select>
            </div>
          </div>
          <p class="caption hint">
            {project.php_request
              ? `Pedida pelo projeto: ${project.php_request}`
              : "O projeto não informa a versão do PHP."}
          </p>
          {#if match && !match.installed}
            <div class="notice warn">
              <Glyph name="info" size={14} />
              <span>
                {project.php_pin
                  ? `O PHP ${project.php_pin} não está instalado.`
                  : "Nenhum PHP instalado atende a este projeto."}
              </span>
              <button class="btn" onclick={() => api.openSettings("php")}>
                {match.suggest ? `Instalar ${match.suggest}` : "Ver versões"}
              </button>
            </div>
          {/if}

          <div class="field">
            <span>Pasta pública</span>
            {#if imported}
              <div class="select">
                <select disabled={saving} value={project.docroot} onchange={(e) => save({ docroot: e.currentTarget.value })}>
                  {#each docroots as d (d)}
                    <option value={d}>{docrootLabel(d)}</option>
                  {/each}
                </select>
              </div>
            {:else}
              <span class="value mono">{docrootLabel(project.docroot)}</span>
            {/if}
          </div>
        {/if}

        {#if project.has_node}
          <div class="field">
            <span>Servidor do Node</span>
            <Toggle
              label="Rodar o servidor de desenvolvimento do Node"
              checked={project.node_enabled}
              disabled={!imported || saving}
              onchange={(v) => save({ node: v, node_command: project.node_command ?? "npm run dev" })}
            />
          </div>
          {#if imported && project.node_enabled}
            <label class="field">
              <span>Comando</span>
              <input
                class="input mono-input"
                bind:value={nodeCommand}
                onblur={commitNodeCommand}
                onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()}
                spellcheck="false"
              />
            </label>
          {/if}
          {#if project.node_request}
            <p class="caption hint">Node pedido: {project.node_request} · versões do Node chegam na fase 5</p>
          {/if}
        {/if}
      </div>
    {/if}

    {#if imported && project.kind !== "node"}
      <div class="section">
        <div class="section-title">Serviços</div>
        <div class="field">
          <span>Banco de dados</span>
          <div class="select"><select disabled><option>Não gerenciado</option></select></div>
        </div>
        <div class="field">
          <span>E-mail</span>
          <div class="select"><select disabled><option>Não gerenciado</option></select></div>
        </div>
        <div class="field">
          <span>Storage</span>
          <div class="select"><select disabled><option>Não gerenciado</option></select></div>
        </div>
        <p class="caption">MariaDB, Mailpit e S3 local entram aqui nas fases 6 a 8.</p>
      </div>
    {/if}

    {#if imported}
      <div class="footer">
        <button class="btn subtle danger" onclick={removeProject}>
          <Glyph name="delete" />
          Remover do Sentinel
        </button>
      </div>
    {/if}
  {:else}
    <div class="none caption">Selecione um projeto para ver os detalhes.</div>
  {/if}
</aside>

<style>
  .inspector {
    width: 340px;
    flex: none;
    overflow: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background: var(--win-bg);
    border-left: 1px solid var(--stroke);
  }
  .inspector > :global(*) {
    flex-shrink: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  h2 {
    flex: 1;
    margin: 0;
    font: 600 16px/22px var(--font-display);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .path {
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }

  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 8px 8px 10px;
    border-radius: var(--r-card);
    border: 1px solid var(--stroke);
    font-size: 12px;
    line-height: 16px;
    user-select: text;
  }
  .notice span {
    flex: 1;
  }
  .notice.accent {
    background: var(--accent-soft);
  }
  .notice.accent :global(.glyph) {
    color: var(--accent-text);
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

  .buttons {
    display: flex;
    gap: 6px;
  }
  .grow {
    flex: 1;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
  }
  .status-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .status-text .caption {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status.is-on {
    border-color: color-mix(in srgb, var(--ok) 35%, var(--stroke));
  }
  .dot.busy {
    background: var(--warn);
    animation: pulse 1s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
  .wide {
    width: 100%;
  }
  .server-error {
    align-items: flex-start;
  }
  .server-error :global(.glyph) {
    margin-top: 1px;
  }
  .err-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .err-actions {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }
  .err-actions:empty {
    display: none;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 4px;
  }
  .section-title {
    margin-bottom: 2px;
  }
  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 32px;
  }
  .field .select {
    width: 170px;
  }
  .domain {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .domain input {
    width: 156px;
    text-align: right;
  }
  .mono-input {
    width: 170px;
    font-family: var(--mono);
    font-size: 12px;
  }
  .value {
    color: var(--text-2);
  }
  .hint {
    margin: -4px 0 2px;
    text-align: right;
  }
  p {
    margin: 2px 0 0;
  }

  .footer {
    margin-top: auto;
    padding-top: 8px;
    border-top: 1px solid var(--stroke);
  }
  .danger {
    color: var(--err);
  }

  .none {
    margin: auto;
    text-align: center;
  }
</style>
