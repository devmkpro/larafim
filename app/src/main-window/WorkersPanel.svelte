<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { onEvent } from "../lib/api";
  import Glyph from "../lib/Glyph.svelte";
  import type { DevLog, DevStatus } from "../lib/node";
  import type { Project } from "../lib/projects";
  import Toggle from "../lib/Toggle.svelte";
  import { toolsApi, workerKey, type Worker } from "../lib/tools";

  // Workers do Laravel: ligam junto com o projeto e reiniciam se caírem.
  let { project }: { project: Project } = $props();

  const labels = $derived<Record<Worker, { title: string; hint: string }>>({
    queue: { title: t("Fila"), hint: "queue:work" },
    schedule: { title: t("Agendador"), hint: "schedule:work" },
    reverb: { title: "Reverb", hint: "reverb:start (WebSockets)" },
  });

  let reverb = $state(false);
  let status = $state<Record<string, DevStatus | null>>({});
  let logs = $state<Record<string, string[]>>({});
  let open = $state<Worker | null>(null);
  let error = $state<string | null>(null);

  const workers = $derived<Worker[]>(reverb ? ["queue", "schedule", "reverb"] : ["queue", "schedule"]);

  $effect(() => {
    const path = project.path;
    toolsApi.hasReverb(path).then((r) => (reverb = r));
    for (const w of ["queue", "schedule", "reverb"] as Worker[]) {
      toolsApi.processInfo(workerKey(path, w)).then((i) => {
        status[w] = i.status;
        logs[w] = i.logs;
      });
    }
    const keys = new Map((["queue", "schedule", "reverb"] as Worker[]).map((w) => [workerKey(path, w), w]));
    const stops = [
      onEvent<DevStatus>("node:status", (s) => {
        const w = keys.get(s.key);
        if (w) status[w] = s;
      }),
      onEvent<DevLog>("node:log", (l) => {
        const w = keys.get(l.key);
        if (w) logs[w] = [...(logs[w] ?? []).slice(-299), l.line];
      }),
    ];
    return () => stops.forEach((s) => s());
  });

  async function toggle(w: Worker, enabled: boolean) {
    error = null;
    try {
      await toolsApi.workerSet(project.path, w, enabled);
    } catch (e) {
      error = String(e);
    }
  }

  const stateText = (w: Worker) => {
    const s = status[w];
    if (!project.workers.includes(w)) return t("desligado");
    if (!project.online) return t("liga com o projeto");
    if (!s) return t("parado");
    if (s.state === "exited") return s.exit_code === null ? t("parado") : t("parou (código {code})", { code: s.exit_code });
    return t("rodando");
  };
</script>

<div class="section-title">{t("Workers")}</div>
{#each workers as w (w)}
  {@const running = status[w]?.state === "running" || status[w]?.state === "starting"}
  <div class="worker">
    <span class="dot" class:on={running && project.workers.includes(w)}></span>
    <div class="text">
      <span>{labels[w].title}</span>
      <span class="caption">{labels[w].hint} · {stateText(w)}</span>
    </div>
    <button
      class="btn subtle icon"
      title={t("Ver saída")}
      disabled={!(logs[w]?.length)}
      onclick={() => (open = open === w ? null : w)}
    >
      <Glyph name={open === w ? "chevronDown" : "terminal"} />
    </button>
    <Toggle label={t("Worker {name}", { name: labels[w].title })} checked={project.workers.includes(w)} onchange={(v) => toggle(w, v)} />
  </div>
  {#if open === w}
    <pre>{(logs[w] ?? []).join("\n")}</pre>
  {/if}
{/each}
{#if error}<p class="caption err">{error}</p>{/if}

<style>
  .section-title {
    margin-top: 4px;
  }
  .worker {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
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
  pre {
    margin: 0;
    max-height: 180px;
    overflow: auto;
    padding: 8px 10px;
    border-radius: var(--r-ctrl);
    background: var(--layer);
    border: 1px solid var(--stroke);
    font: 11px/16px var(--mono);
    white-space: pre-wrap;
    user-select: text;
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
