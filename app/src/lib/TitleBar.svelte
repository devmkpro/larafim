<script lang="ts">
  import { t } from "./i18n.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { inTauri } from "./api";
  import Glyph from "./Glyph.svelte";
  import Logo from "./Logo.svelte";

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
    <Logo size={16} />
    <span data-tauri-drag-region>{title}</span>
  </div>

  <div class="caption-buttons">
    <button class="cb" title={t("Minimizar")} onclick={() => win?.minimize()}>
      <Glyph name="minimize" size={10} />
    </button>
    <button class="cb" title={maximized ? t("Restaurar") : t("Maximizar")} onclick={() => win?.toggleMaximize()}>
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
