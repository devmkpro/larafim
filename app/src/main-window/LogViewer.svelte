<script lang="ts">
  import Glyph from "../lib/Glyph.svelte";
  import { confirm } from "../lib/php";
  import { toolsApi, type LogFile } from "../lib/tools";

  // Logs do projeto ao vivo (storage\logs). Lê só o que entrou de novo.
  let { path, name, onclose }: { path: string; name: string; onclose: () => void } = $props();

  let files = $state<LogFile[]>([]);
  let file = $state("");
  let text = $state("");
  let offset: number | null = null;
  let paused = $state(false);
  let filter = $state("");
  let onlyErrors = $state(false);
  let box = $state<HTMLPreElement>();
  let error = $state<string | null>(null);

  async function loadFiles() {
    files = await toolsApi.logFiles(path).catch(() => []);
    if (!file && files.length) file = files[0].name;
  }

  async function poll() {
    if (!file || paused) return;
    try {
      const chunk = await toolsApi.logRead(path, file, offset);
      if (chunk.reset) text = "";
      if (chunk.text) {
        // Guarda no máximo ~1 MB na tela.
        text = (text + chunk.text).slice(-1_000_000);
      }
      offset = chunk.offset;
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    loadFiles();
  });

  $effect(() => {
    void file;
    text = "";
    offset = null;
    poll();
    const t = setInterval(poll, 1000);
    return () => clearInterval(t);
  });

  const lines = $derived.by(() => {
    let l = text.split("\n");
    if (onlyErrors) l = l.filter((x) => /\.(ERROR|CRITICAL|ALERT|EMERGENCY)|Fatal|Exception/i.test(x));
    const q = filter.trim().toLowerCase();
    if (q) l = l.filter((x) => x.toLowerCase().includes(q));
    return l.join("\n");
  });

  $effect(() => {
    void lines;
    if (box && !paused) box.scrollTop = box.scrollHeight;
  });

  async function clear() {
    if (!(await confirm(`Esvaziar ${file}? O conteúdo atual do arquivo será apagado.`, "Limpar log", "Limpar"))) return;
    await toolsApi.logClear(path, file);
    text = "";
    offset = 0;
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="smoke" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Logs do projeto">
    <div class="head">
      <h2>Logs · {name}</h2>
      <span class="grow"></span>
      <button class="btn subtle icon" title="Fechar" onclick={onclose}><Glyph name="close" size={12} /></button>
    </div>
    {#if files.length}
      <div class="bar">
        <div class="select">
          <select bind:value={file}>
            {#each files as f (f.name)}<option value={f.name}>{f.name}</option>{/each}
          </select>
        </div>
        <label class="search">
          <Glyph name="search" size={14} />
          <input class="input" placeholder="Filtrar" bind:value={filter} />
        </label>
        <label class="check"><input type="checkbox" bind:checked={onlyErrors} /> Só erros</label>
        <span class="grow"></span>
        <button class="btn subtle" onclick={() => (paused = !paused)}>{paused ? "Continuar" : "Pausar"}</button>
        <button class="btn subtle icon" title="Abrir no editor" onclick={() => toolsApi.logOpen(path, file)}><Glyph name="openExternal" /></button>
        <button class="btn subtle icon" title="Limpar arquivo" onclick={clear}><Glyph name="delete" /></button>
      </div>
      <pre bind:this={box}>{lines || "(vazio)"}</pre>
      {#if error}<p class="caption err">{error}</p>{/if}
    {:else}
      <p class="empty caption">Nenhum arquivo .log em storage\logs (ou logs) deste projeto.</p>
    {/if}
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
    width: min(980px, calc(100vw - 48px));
    height: min(640px, calc(100vh - 64px));
    display: flex;
    flex-direction: column;
    border-radius: 8px;
    border: 1px solid var(--stroke);
    background: var(--card);
    box-shadow: var(--overlay-shadow), 0 32px 64px rgb(0 0 0 / 0.19);
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    padding: 12px 12px 4px 20px;
  }
  h2 {
    margin: 0;
    font: 600 18px/26px var(--font-display);
  }
  .grow {
    flex: 1;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px 8px 20px;
  }
  .bar .select {
    width: 220px;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
  }
  .search :global(.glyph) {
    position: absolute;
    left: 10px;
    color: var(--text-3);
  }
  .search input {
    width: 200px;
    padding-left: 32px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  pre {
    flex: 1;
    margin: 0;
    padding: 12px 20px;
    overflow: auto;
    background: var(--win-bg);
    border-top: 1px solid var(--stroke);
    font: 12px/18px var(--mono);
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
  .empty {
    padding: 20px;
  }
  .err {
    padding: 4px 20px;
    color: var(--err);
  }
</style>
