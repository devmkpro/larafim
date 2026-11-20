<script lang="ts">
  import type { Snippet } from "svelte";
  import Glyph, { type GlyphName } from "../lib/Glyph.svelte";

  // Linha de configuração no estilo do app Configurações do Windows 11.
  let {
    icon,
    title,
    description,
    children,
  }: { icon?: GlyphName; title: string; description?: string; children?: Snippet } = $props();
</script>

<div class="row card">
  {#if icon}<span class="icon"><Glyph name={icon} size={18} /></span>{/if}
  <div class="text">
    <span>{title}</span>
    {#if description}<span class="caption">{description}</span>{/if}
  </div>
  {#if children}<div class="control">{@render children()}</div>{/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 68px;
    padding: 12px 16px;
  }
  .icon {
    width: 20px;
    display: flex;
    justify-content: center;
    color: var(--text);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow-wrap: anywhere;
  }
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
