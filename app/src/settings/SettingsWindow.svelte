<script lang="ts">
  import { api, onEvent, projectUrl, type SettingsSection, type Theme } from "../lib/api";
  import { pickFolder } from "../lib/folders";
  import Glyph, { type GlyphName } from "../lib/Glyph.svelte";
  import { app } from "../lib/state.svelte";
  import TitleBar from "../lib/TitleBar.svelte";
  import Toggle from "../lib/Toggle.svelte";
  import { projectsApi, type ProjectsOverview } from "../lib/projects";
  import PhpSection from "./PhpSection.svelte";
  import TerminalSection from "./TerminalSection.svelte";
  import SettingRow from "./SettingRow.svelte";

  const nav: { id: SettingsSection; label: string; icon: GlyphName }[] = [
    { id: "general", label: "Geral", icon: "settings" },
    { id: "folders", label: "Pastas de projetos", icon: "folder" },
    { id: "network", label: "Rede e domínios", icon: "globe" },
    { id: "php", label: "PHP", icon: "code" },
    { id: "terminal", label: "Terminal", icon: "terminal" },
    { id: "node", label: "Node.js", icon: "package" },
    { id: "services", label: "Serviços", icon: "database" },
    { id: "about", label: "Sobre", icon: "info" },
  ];

  const initial =
    (window as { __SENTINEL_SECTION__?: string }).__SENTINEL_SECTION__ ??
    new URLSearchParams(location.search).get("section") ??
    "general";
  let section = $state<SettingsSection>(nav.some((n) => n.id === initial) ? (initial as SettingsSection) : "general");
  onEvent<SettingsSection>("settings:navigate", (s) => (section = s));

  const current = $derived(nav.find((n) => n.id === section)!);

  // Campos de texto só gravam ao confirmar (Enter ou sair do campo).
  let tld = $state("");
  let port = $state(80);
  $effect(() => {
    if (app.config) {
      tld = app.config.tld;
      port = app.config.http_port;
    }
  });

  function commitTld() {
    const value = tld.trim().replace(/^\./, "").toLowerCase();
    if (app.config && value && value !== app.config.tld) app.update({ tld: value });
    else if (app.config) tld = app.config.tld;
  }

  function commitPort() {
    const value = Number(port);
    if (app.config && Number.isInteger(value) && value > 0 && value < 65536 && value !== app.config.http_port) {
      app.update({ http_port: value });
    } else if (app.config) port = app.config.http_port;
  }

  // Projetos ignorados (Configurações › Pastas de projetos).
  let ignored = $state<string[]>([]);
  $effect(() => {
    if (section !== "folders") return;
    projectsApi.list().then((d) => (ignored = d.ignored)).catch(() => {});
    return onEvent<ProjectsOverview>("projects-changed", (d) => (ignored = d.ignored));
  });

  async function unignore(path: string) {
    try {
      ignored = (await projectsApi.unignore(path)).ignored;
    } catch (e) {
      app.error = String(e);
    }
  }

  async function addRoot() {
    const dir = await pickFolder("Escolha uma pasta de projetos");
    if (!dir || !app.config || app.config.roots.includes(dir)) return;
    await app.update({ roots: [...app.config.roots, dir] });
  }

  const nodeVersions = ["24", "22", "20", "18"];
  const services: { name: string; detail: string; phase: number; icon: GlyphName }[] = [
    { name: "MariaDB", detail: "Banco de dados · porta 3306", phase: 6, icon: "database" },
    { name: "Adminer", detail: "Gerenciar bancos pelo navegador", phase: 6, icon: "globe" },
    { name: "Mailpit", detail: "Caixa de e-mails local · SMTP 1025", phase: 7, icon: "mail" },
    { name: "PostgreSQL", detail: "Download opcional · porta 5432", phase: 8, icon: "database" },
    { name: "Storage S3 local", detail: "Download opcional · porta 9000", phase: 8, icon: "server" },
  ];
</script>

<div class="window">
  <TitleBar title="Configurações" />

  <div class="layout">
    <nav aria-label="Seções">
      {#each nav as item (item.id)}
        <button
          class="nav-item"
          class:active={section === item.id}
          aria-current={section === item.id ? "page" : undefined}
          onclick={() => (section = item.id)}
        >
          <Glyph name={item.icon} />
          {item.label}
        </button>
      {/each}
    </nav>

    <main>
      {#if section !== "php" && section !== "terminal"}<h1>{current.label}</h1>{/if}

      {#if app.error}
        <div class="infobar error"><Glyph name="info" /><span>{app.error}</span></div>
      {/if}

      {#if !app.config}
        <p class="caption">Carregando…</p>
      {:else if section === "general"}
        <SettingRow icon="brush" title="Tema" description="Escolha a aparência do Sentinel">
          <div class="select">
            <select
              value={app.config.theme}
              onchange={(e) => app.update({ theme: e.currentTarget.value as Theme })}
            >
              <option value="system">Seguir o Windows</option>
              <option value="light">Claro</option>
              <option value="dark">Escuro</option>
            </select>
          </div>
        </SettingRow>
        <SettingRow
          icon="pin"
          title="Continuar na bandeja ao fechar"
          description="A janela sai da memória e o Sentinel segue no ícone da bandeja"
        >
          <span class="caption">{app.config.keep_in_tray ? "Ativado" : "Desativado"}</span>
          <Toggle
            label="Continuar na bandeja ao fechar"
            checked={app.config.keep_in_tray}
            onchange={(v) => app.update({ keep_in_tray: v })}
          />
        </SettingRow>
        <SettingRow icon="play" title="Iniciar com o Windows" description="Disponível na fase 9">
          <span class="caption">Desativado</span>
          <Toggle label="Iniciar com o Windows" disabled />
        </SettingRow>
      {:else if section === "folders"}
        <p class="lead caption">Cada subpasta destas pastas aparece como projeto. Nada é ligado sozinho.</p>
        {#each app.config.roots as root (root)}
          <SettingRow icon="folder" title={root}>
            <button class="btn" onclick={() => api.openFolder(root)}>Abrir</button>
            <button
              class="btn subtle icon"
              title="Remover da lista (os arquivos não são apagados)"
              onclick={() => app.update({ roots: app.config!.roots.filter((r) => r !== root) })}
            >
              <Glyph name="delete" />
            </button>
          </SettingRow>
        {/each}
        <div class="actions">
          <button class="btn accent" onclick={addRoot}><Glyph name="add" /> Adicionar pasta</button>
        </div>

        {#if ignored.length}
          <h2 class="subhead">Projetos ignorados</h2>
          <p class="lead caption">Não aparecem na lista de projetos. Os arquivos continuam onde estão.</p>
          {#each ignored as path (path)}
            <SettingRow icon="folder" title={path.split(/[\\/]/).pop() ?? path} description={path}>
              <button class="btn" onclick={() => unignore(path)}>Mostrar de novo</button>
            </SettingRow>
          {/each}
        {/if}
      {:else if section === "network"}
        <SettingRow
          icon="globe"
          title="Domínio dos projetos"
          description={`Exemplo: ${projectUrl(app.config, "loja")}`}
        >
          <span class="caption">.</span>
          <input
            class="input short"
            bind:value={tld}
            onblur={commitTld}
            onkeydown={(e) => e.key === "Enter" && commitTld()}
          />
        </SettingRow>
        <SettingRow
          icon="server"
          title="Porta HTTP"
          description="Use outra porta se a 80 estiver ocupada (Laragon, IIS…)"
        >
          <input
            class="input port"
            type="number"
            min="1"
            max="65535"
            bind:value={port}
            onblur={commitPort}
            onkeydown={(e) => e.key === "Enter" && commitPort()}
          />
        </SettingRow>
      {:else if section === "php"}
        <PhpSection />
      {:else if section === "terminal"}
        <TerminalSection />
      {:else if section === "node"}
        <p class="lead caption">Instale versões do Node.js para os projetos que usam Vite, Next, Nuxt…</p>
        <div class="infobar">
          <Glyph name="info" />
          <span>O Node.js gerenciado pelo Sentinel chega na <strong>fase 5</strong>.</span>
        </div>
        <div class="table card">
          <div class="thead section-title"><span>Versão</span><span>Instalada</span><span></span></div>
          {#each nodeVersions as v (v)}
            <div class="tr">
              <span>{v}</span>
              <span class="caption">Não</span>
              <button class="btn" disabled>Instalar</button>
            </div>
          {/each}
        </div>
      {:else if section === "services"}
        <p class="lead caption">Serviços ficam desligados e só sobem quando algum projeto precisa.</p>
        {#each services as s (s.name)}
          <SettingRow icon={s.icon} title={s.name} description={`${s.detail} · fase ${s.phase}`}>
            <span class="caption">Não instalado</span>
            <button class="btn" disabled>Instalar</button>
          </SettingRow>
        {/each}
      {:else if section === "about"}
        <SettingRow icon="shield" title="Sentinel" description={`Versão ${app.info?.version}`} />
        <SettingRow icon="folder" title="Pasta de dados" description={app.info?.data_dir}>
          <button class="btn" onclick={() => api.openDataDir()}>Abrir</button>
        </SettingRow>
      {/if}
    </main>
  </div>
</div>

<style>
  .window {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .layout {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  nav {
    width: 240px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 8px 16px 8px;
  }
  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    height: 36px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-ctrl);
    background: transparent;
    text-align: left;
  }
  .nav-item:hover {
    background: var(--subtle-hover);
  }
  .nav-item.active {
    background: var(--selected);
  }
  .nav-item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    width: 3px;
    height: 16px;
    border-radius: 2px;
    background: var(--accent);
    transform: translateY(-50%);
  }

  main {
    flex: 1;
    min-width: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 28px 28px 20px;
  }
  /* Blocos não encolhem: a página rola. */
  main > :global(*) {
    flex-shrink: 0;
  }
  h1 {
    margin: 0 0 16px;
    font: 600 28px/36px var(--font-display);
  }
  .lead {
    margin: -8px 0 12px;
  }
  .infobar {
    margin-bottom: 8px;
  }
  .infobar.error :global(.glyph) {
    color: var(--err);
  }
  .actions {
    margin-top: 8px;
  }
  .subhead {
    margin: 28px 0 8px;
    font: 600 14px/20px var(--font);
  }
  .input.short {
    width: 140px;
  }
  .input.port {
    width: 100px;
  }

  .table {
    overflow: hidden;
  }
  .thead,
  .tr {
    display: grid;
    grid-template-columns: 1fr 1fr 96px;
    align-items: center;
    gap: 12px;
    padding: 0 16px;
  }
  .thead {
    height: 36px;
    background: var(--layer);
    border-bottom: 1px solid var(--stroke);
  }
  .tr {
    height: 48px;
  }
  .tr + .tr {
    border-top: 1px solid var(--stroke);
  }
  .tr .btn {
    justify-self: end;
  }
</style>
