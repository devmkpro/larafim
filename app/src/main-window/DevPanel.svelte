<script lang="ts">
  import { onEvent } from "../lib/api";
  import Glyph from "../lib/Glyph.svelte";
  import { devKey, nodeApi, type DevLog, type DevStatus } from "../lib/node";
  import { asServerError } from "../lib/server";

  // Status e saída ao vivo do servidor de desenvolvimento do Node de um projeto.
  let { path, online }: { path: string; online: boolean } = $props();

  let status = $state<DevStatus | null>(null);
  let logs = $state<string[]>([]);
  let open = $state(false);
  let error = $state<string | null>(null);
  let box = $state<HTMLPreElement>();

  $effect(() => {
    const key = devKey(path);
    void online;
    nodeApi.devInfo(path).then((info) => {
      status = info.status;
      logs = info.logs;
    });
    const stops = [
      onEvent<DevStatus>("node:status", (s) => {
        if (s.key === key) status = s;
      }),
      onEvent<DevLog>("node:log", (l) => {
        if (l.key !== key) return;
        logs = [...logs.slice(-399), l.line];
      }),
    ];
    return () => stops.forEach((s) => s());
  });

  // Rola para o fim quando chegam linhas novas.
  $effect(() => {
    void logs.length;
    if (box && open) box.scrollTop = box.scrollHeight;
  });

  async function restart() {
    error = null;
    logs = [];
    try {
      await nodeApi.restartDev(path);
    } catch (e) {
      error = asServerError(e).message;
    }
  }

  const label = $derived.by(() => {
    if (!status) return online ? "Não iniciado" : "Desligado (liga junto com o projeto)";
    if (status.state === "starting") return "Iniciando…";
    if (status.state === "running") return `Rodando em localhost:${status.port}`;
    return status.exit_code === null ? "Parado" : `Parou (código ${status.exit_code})`;
  });
</script>

<div class="dev card">
  <div class="head">
    <span class="dot" class:on={status?.state === "running"} class:busy={status?.state === "starting"} class:bad={status?.state === "exited" && online}></span>
    <div class="text">
      <strong>Node</strong>
      <span class="caption" title={status?.command}>{label}</span>
    </div>
    {#if online}
      <button class="btn subtle icon" title="Reiniciar o servidor do Node" onclick={restart}><Glyph name="refresh" /></button>
    {/if}
    <button
      class="btn subtle icon"
      title={open ? "Ocultar saída" : "Ver saída do comando"}
      onclick={() => (open = !open)}
      disabled={!logs.length}
    >
      <Glyph name={open ? "chevronDown" : "terminal"} />
    </button>
  </div>
  {#if error}<p class="err caption">{error}</p>{/if}
  {#if open}
    <pre bind:this={box}>{logs.join("\n")}</pre>
  {/if}
</div>

<style>
  .dev {
    padding: 10px 10px 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .text .caption {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot.busy {
    background: var(--warn);
  }
  .dot.bad {
    background: var(--err);
  }
  pre {
    margin: 0;
    max-height: 220px;
    overflow: auto;
    padding: 8px 10px;
    border-radius: var(--r-ctrl);
    background: var(--win-bg);
    border: 1px solid var(--stroke);
    font: 11px/16px var(--mono);
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
