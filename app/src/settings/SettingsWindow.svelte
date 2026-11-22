<script lang="ts">
  import { languages, t } from "../lib/i18n.svelte";
  import { api, onEvent, projectUrl, type SettingsSection, type Theme } from "../lib/api";
  import { pickFolder } from "../lib/folders";
  import Glyph, { type GlyphName } from "../lib/Glyph.svelte";
  import { app } from "../lib/state.svelte";
  import TitleBar from "../lib/TitleBar.svelte";
  import Toggle from "../lib/Toggle.svelte";
  import { projectsApi, type ProjectsOverview } from "../lib/projects";
  import NodeSection from "./NodeSection.svelte";
  import PhpSection from "./PhpSection.svelte";
  import ServicesSection from "./ServicesSection.svelte";
  import AboutSection from "./AboutSection.svelte";
  import { updates } from "../lib/updates";
  import TerminalSection from "./TerminalSection.svelte";
  import HttpsSettings from "./HttpsSettings.svelte";
  import SettingRow from "./SettingRow.svelte";

  const nav = $derived<{ id: SettingsSection; label: string; icon: GlyphName }[]>([
    { id: "general", label: t("Geral"), icon: "settings" },
    { id: "folders", label: t("Pastas de projetos"), icon: "folder" },
    { id: "network", label: t("Rede e domínios"), icon: "globe" },
    { id: "php", label: "PHP", icon: "code" },
    { id: "terminal", label: "Terminal", icon: "terminal" },
    { id: "node", label: "Node.js", icon: "package" },
    { id: "services", label: t("Serviços"), icon: "database" },
    { id: "about", label: t("Sobre"), icon: "info" },
  ]);

  const initial =
    (window as { __LARAFIM_SECTION__?: string }).__LARAFIM_SECTION__ ??
    new URLSearchParams(location.search).get("section") ??
    "general";
  let section = $state<SettingsSection>(["general", "folders", "network", "php", "terminal", "node", "services", "about"].includes(initial) ? (initial as SettingsSection) : "general");
  onEvent<SettingsSection>("settings:navigate", (s) => (section = s));

  // Versão nova do Larafim: bolinha no item "Sobre".
  let updateAvailable = $state(false);
  $effect(() => {
    updates.status().then((s) => (updateAvailable = s.available)).catch(() => {});
    return onEvent<{ available: boolean }>("app:update", (s) => (updateAvailable = s.available));
  });

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
    const dir = await pickFolder(t("Escolha uma pasta de projetos"));
    if (!dir || !app.config || app.config.roots.includes(dir)) return;
    await app.update({ roots: [...app.config.roots, dir] });
  }

</script>

<div class="window">
  <TitleBar title={t("Configurações")} />

  <div class="layout">
    <nav aria-label={t("Seções")}>
      {#each nav as item (item.id)}
        <button
          class="nav-item"
          class:active={section === item.id}
          aria-current={section === item.id ? "page" : undefined}
          onclick={() => (section = item.id)}
        >
          <Glyph name={item.icon} />
          {item.label}
          {#if item.id === "about" && updateAvailable}<span class="update-dot" title={t("Atualização disponível")}></span>{/if}
        </button>
      {/each}
    </nav>

    <main>
      {#if section !== "php" && section !== "terminal" && section !== "node" && section !== "services"}<h1>{current.label}</h1>{/if}

      {#if app.error}
        <div class="infobar error"><Glyph name="info" /><span>{app.error}</span></div>
      {/if}

      {#if !app.config}
        <p class="caption">{t("Carregando…")}</p>
      {:else if section === "general"}
        <SettingRow icon="globe" title={t("Idioma")} description={t("Idioma da interface, das mensagens e da bandeja")}>
          <div class="select">
            <select value={app.config.language ?? "pt-BR"} onchange={(e) => app.update({ language: e.currentTarget.value })}>
              {#each languages as l (l.code)}<option value={l.code}>{l.name}</option>{/each}
            </select>
          </div>
        </SettingRow>
        <SettingRow icon="brush" title={t("Tema")} description={t("Escolha a aparência do Larafim")}>
          <div class="select">
            <select
              value={app.config.theme}
              onchange={(e) => app.update({ theme: e.currentTarget.value as Theme })}
            >
              <option value="system">{t("Seguir o Windows")}</option>
              <option value="light">{t("Claro")}</option>
              <option value="dark">{t("Escuro")}</option>
            </select>
          </div>
        </SettingRow>
        <SettingRow
          icon="pin"
          title={t("Continuar na bandeja ao fechar")}
          description={t("A janela sai da memória e o Larafim segue no ícone da bandeja")}
        >
          <span class="caption">{app.config.keep_in_tray ? t("Ativado") : t("Desativado")}</span>
          <Toggle
            label={t("Continuar na bandeja ao fechar")}
            checked={app.config.keep_in_tray}
            onchange={(v) => app.update({ keep_in_tray: v })}
          />
        </SettingRow>
        <SettingRow icon="play" title={t("Iniciar com o Windows")} description={t("Abre só na bandeja ao entrar no Windows, sem janela")}>
          <span class="caption">{app.config.start_with_windows ? t("Ativado") : t("Desativado")}</span>
          <Toggle
            label={t("Iniciar com o Windows")}
            checked={app.config.start_with_windows}
            onchange={(v) => app.update({ start_with_windows: v })}
          />
        </SettingRow>
      {:else if section === "folders"}
        <p class="lead caption">{t("Cada subpasta destas pastas aparece como projeto. Nada é ligado sozinho.")}</p>
        {#each app.config.roots as root (root)}
          <SettingRow icon="folder" title={root}>
            <button class="btn" onclick={() => api.openFolder(root)}>{t("Abrir")}</button>
            <button
              class="btn subtle icon"
              title={t("Remover da lista (os arquivos não são apagados)")}
              onclick={() => app.update({ roots: app.config!.roots.filter((r) => r !== root) })}
            >
              <Glyph name="delete" />
            </button>
          </SettingRow>
        {/each}
        <div class="actions">
          <button class="btn accent" onclick={addRoot}><Glyph name="add" /> {t("Adicionar pasta")}</button>
        </div>

        {#if ignored.length}
          <h2 class="subhead">{t("Projetos ignorados")}</h2>
          <p class="lead caption">{t("Não aparecem na lista de projetos. Os arquivos continuam onde estão.")}</p>
          {#each ignored as path (path)}
            <SettingRow icon="folder" title={path.split(/[\\/]/).pop() ?? path} description={path}>
              <button class="btn" onclick={() => unignore(path)}>{t("Mostrar de novo")}</button>
            </SettingRow>
          {/each}
        {/if}
      {:else if section === "network"}
        <SettingRow
          icon="globe"
          title={t("Domínio dos projetos")}
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
          title={t("Porta HTTP")}
          description={t("Use outra porta se a 80 estiver ocupada (Laragon, IIS…)")}
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
        <HttpsSettings />
      {:else if section === "php"}
        <PhpSection />
      {:else if section === "terminal"}
        <TerminalSection />
      {:else if section === "node"}
        <NodeSection />
      {:else if section === "services"}
        <ServicesSection />
      {:else if section === "about"}
        <AboutSection />
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
  .update-dot {
    width: 8px;
    height: 8px;
    margin-left: auto;
    border-radius: 50%;
    background: var(--accent);
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

</style>
