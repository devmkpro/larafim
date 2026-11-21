<script lang="ts">
  import { windowLabel } from "./lib/api";
  import { app } from "./lib/state.svelte";
  import ProjectsWindow from "./main-window/ProjectsWindow.svelte";
  import SettingsWindow from "./settings/SettingsWindow.svelte";
  import LogsWindow from "./logs/LogsWindow.svelte";
  import TrayPanel from "./tray/TrayPanel.svelte";

  app.load();

  // No app final, sem menu de contexto do navegador nem atalhos de recarregar.
  if (import.meta.env.PROD) {
    window.addEventListener("contextmenu", (e) => {
      const t = e.target as HTMLElement;
      if (!t.closest("input, textarea")) e.preventDefault();
    });
    window.addEventListener("keydown", (e) => {
      const k = e.key.toLowerCase();
      if (k === "f5" || ((e.ctrlKey || e.metaKey) && (k === "r" || k === "p" || k === "f" || k === "u"))) {
        e.preventDefault();
      }
    });
  }
</script>

{#if windowLabel === "settings"}
  <SettingsWindow />
{:else if windowLabel === "logs"}
  <LogsWindow />
{:else if windowLabel === "tray"}
  <TrayPanel />
{:else}
  <ProjectsWindow />
{/if}
