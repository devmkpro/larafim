<script lang="ts">
  import { t } from "./i18n.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { inTauri } from "./api";
  import Glyph from "./Glyph.svelte";

  let { title }: { title: string } = $props();
  let maximized = $state(false);

  const win = inTauri ? getCurrentWindow() : null;

  if (win) {
    const sync = async () => (maximized = await win.isMaximized());
    sync();
    win.onResized(sync);
  }
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="title" data-tauri-drag-region>
    <svg class="logo" viewBox="0 0 512 512" aria-hidden="true">
      <rect x="16" y="16" width="480" height="480" rx="112" fill="var(--accent)" />
      <path d="M256 104 136 148v84c0 78 51 146 120 172 69-26 120-94 120-172v-84z" fill="none" stroke="var(--accent-fg)" stroke-width="40" stroke-linejoin="round" />
    </svg>
    <span data-tauri-drag-region>{title}</span>
  </div>

  <div class="caption-buttons">
    <button class="cb" title={t("Minimizar")} onclick={() => win?.minimize()}>
      <Glyph name="minimize" size={10} />
    </button>
    <button class="cb" title={maximized ? "Restaurar" : "Maximizar"} onclick={() => win?.toggleMaximize()}>
      <Glyph name={maximized ? "restore" : "maximize"} size={10} />
    </button>
    <button class="cb close" title={t("Fechar")} onclick={() => win?.close()}>
      <Glyph name="close" size={10} />
    </button>
  </div>
</header>

<style>
  .titlebar {
    display: flex;
    align-items: stretch;
    justify-content: space-between;
    height: 32px;
    flex: none;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-left: 14px;
    font-size: 12px;
    color: var(--text-2);
    flex: 1;
  }
  .logo {
    width: 16px;
    height: 16px;
  }
  .caption-buttons {
    display: flex;
  }
  .cb {
    width: 46px;
    border: 0;
    background: transparent;
    color: var(--text);
    display: grid;
    place-items: center;
  }
  .cb:hover {
    background: var(--subtle-hover);
  }
  .cb:active {
    background: var(--subtle-press);
    color: var(--text-2);
  }
  .cb.close:hover {
    background: var(--close-hover);
    color: #fff;
  }
</style>
