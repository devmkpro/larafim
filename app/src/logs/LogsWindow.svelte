<script lang="ts">
  import { locale, plural, t } from "../lib/i18n.svelte";
  import { onEvent } from "../lib/api";
  import Glyph from "../lib/Glyph.svelte";
  import TitleBar from "../lib/TitleBar.svelte";
  import { confirm } from "../lib/php";
  import { projectsApi, type Project } from "../lib/projects";
  import { toolsApi, type LogFile } from "../lib/tools";
  import { parseLog, relative, toMarkdown, type Frame, type Level, type LogEntry } from "../lib/laravelLog";

  // Visualizador de logs: projetos e arquivos à esquerda, entradas no meio,
  // a entrada escolhida embaixo com a pilha (frames do vendor recolhidos).

  let projects = $state<Project[]>([]);
  let files = $state<Record<string, LogFile[]>>({});
  let expanded = $state<Record<string, boolean>>({});
  let project = $state<string | null>(null);
  let file = $state<string | null>(null);
  let showTree = $state(true);

  const requested =
    (window as { __LARAFIM_LOGS_PROJECT__?: string | null }).__LARAFIM_LOGS_PROJECT__ ??
    new URLSearchParams(location.search).get("project");

  async function loadFiles(path: string) {
    files[path] = await toolsApi.logFiles(path).catch(() => []);
    return files[path];
  }

  async function open(path: string, name: string | null = null) {
    expanded[path] = true;
    const list = files[path] ?? (await loadFiles(path));
    project = path;
    file = name ?? list.find((f) => f.name === "laravel.log")?.name ?? list[0]?.name ?? null;
  }

  async function loadProjects() {
    const all = await projectsApi.list().catch(() => null);
    projects = (all?.projects ?? []).filter((p) => p.status === "imported").sort((a, b) => a.name.localeCompare(b.name));
    if (project && projects.some((p) => p.path === project)) return;
    const want = projects.find((p) => p.path.toLowerCase() === requested?.toLowerCase());
    if (want) return open(want.path);
    // Sem pedido: o primeiro projeto que tem algum log.
    for (const p of projects) {
      if ((await loadFiles(p.path)).length) return open(p.path);
    }
  }

  $effect(() => {
    loadProjects();
    const stops = [onEvent<string>("logs:navigate", (p) => open(p)), onEvent("projects-changed", loadProjects)];
    return () => stops.forEach((s) => s());
  });

  function toggle(path: string) {
    expanded[path] = !expanded[path];
    if (expanded[path] && !files[path]) loadFiles(path);
  }

  async function refresh() {
    await loadProjects();
    for (const p of Object.keys(expanded)) if (expanded[p]) loadFiles(p);
    restart();
  }

  // ---------- Leitura ao vivo ----------
  let text = $state("");
  let offset: number | null = null;
  let paused = $state(false);
  let error = $state<string | null>(null);
  let generation = 0;

  async function poll() {
    if (!project || !file || paused) return;
    const gen = generation;
    try {
      const chunk = await toolsApi.logRead(project, file, offset);
      if (gen !== generation) return;
      if (chunk.reset) text = "";
      if (chunk.text) text = (text + chunk.text).slice(-1_000_000); // ~1 MB na tela
      offset = chunk.offset;
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  function restart() {
    generation++;
    text = "";
    offset = null;
    selectedId = null;
    poll();
  }

  $effect(() => {
    void project;
    void file;
    restart();
    const timer = setInterval(poll, 1000);
    return () => clearInterval(timer);
  });

  // ---------- Entradas ----------
  const entries = $derived(parseLog(text).reverse()); // mais nova primeiro
  const levels: { id: Level | "all"; label: () => string }[] = [
    { id: "all", label: () => t("Todos") },
    { id: "error", label: () => "Error" },
    { id: "warning", label: () => "Warning" },
    { id: "info", label: () => "Info" },
    { id: "debug", label: () => "Debug" },
  ];
  let level = $state<Level | "all">("all");
  let search = $state("");
  let grouped = $state(false);

  const counts = $derived.by(() => {
    const c: Record<string, number> = { all: entries.length, error: 0, warning: 0, info: 0, debug: 0, other: 0 };
    for (const e of entries) c[e.level]++;
    return c;
  });

  interface Row {
    entry: LogEntry;
    count: number;
  }
  const rows = $derived.by<Row[]>(() => {
    const q = search.trim().toLowerCase();
    const list = entries.filter((e) => (level === "all" || e.level === level) && (!q || e.raw.toLowerCase().includes(q)));
    if (!grouped) return list.map((entry) => ({ entry, count: 1 }));
    const map = new Map<string, Row>();
    for (const e of list) {
      const key = e.level + "|" + (e.exception?.cls ?? "") + "|" + e.message;
      const row = map.get(key);
      if (row) row.count++;
      else map.set(key, { entry: e, count: 1 });
    }
    return [...map.values()];
  });

  let selectedId = $state<number | null>(null);
  const selected = $derived(entries.find((e) => e.id === selectedId) ?? null);
  // A lista é refeita a cada leitura; a entrada escolhida é achada de novo pelo texto.
  let selectedRaw = "";
  $effect(() => {
    if (selectedId === null || selected) return;
    const again = entries.find((e) => e.raw === selectedRaw);
    selectedId = again?.id ?? null;
  });
  function select(e: LogEntry) {
    selectedId = e.id;
    selectedRaw = e.raw;
    openVendor = new Set();
  }

  function onKey(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest("input, select, textarea")) return;
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const i = rows.findIndex((r) => r.entry.id === selectedId);
    const next = rows[Math.max(0, Math.min(rows.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
    if (next) {
      select(next.entry);
      document.getElementById(`entry-${next.entry.id}`)?.scrollIntoView({ block: "nearest" });
    }
  }

  // ---------- Pilha ----------
  type Segment = { vendor: true; frames: Frame[]; key: number } | { vendor: false; frame: Frame; key: number };
  const segments = $derived.by<Segment[]>(() => {
    const out: Segment[] = [];
    for (const f of selected?.frames ?? []) {
      const last = out[out.length - 1];
      if (f.vendor && last?.vendor) last.frames.push(f);
      else if (f.vendor) out.push({ vendor: true, frames: [f], key: f.n });
      else out.push({ vendor: false, frame: f, key: f.n });
    }
    return out;
  });
  let openVendor = $state(new Set<number>()); // grupos do vendor que o usuário abriu
  function toggleVendor(key: number) {
    const s = new Set(openVendor);
    if (s.has(key)) s.delete(key);
    else s.add(key);
    openVendor = s;
  }

  let copied = $state<"text" | "md" | null>(null);
  async function copy(kind: "text" | "md") {
    if (!selected || !project) return;
    await navigator.clipboard.writeText(kind === "md" ? toMarkdown(selected, project) : selected.raw);
    copied = kind;
    setTimeout(() => (copied = null), 1500);
  }

  async function clear() {
    if (!project || !file) return;
    if (!(await confirm(t("Esvaziar {file}? O conteúdo atual do arquivo será apagado.", { file }), t("Limpar log"), t("Limpar")))) return;
    await toolsApi.logClear(project, file);
    restart();
    loadFiles(project);
  }

  const kb = (n: number) => (n < 1024 ? `${n} B` : n < 1024 * 1024 ? `${Math.round(n / 1024).toLocaleString(locale())} KB` : `${(n / 1024 / 1024).toLocaleString(locale(), { maximumFractionDigits: 1 })} MB`);
  const projectName = $derived(projects.find((p) => p.path === project)?.name ?? "");
  const levelLabel = (l: Level, name: string) =>
    l === "error" ? (name === "ERROR" || !name ? "Error" : name[0] + name.slice(1).toLowerCase()) : l === "other" ? name || "Log" : l[0].toUpperCase() + l.slice(1);
</script>

<svelte:window onkeydown={onKey} />

<div class="window">
  <TitleBar title={projectName ? t("Logs · {name}", { name: projectName }) : t("Logs")} />

  <div class="toolbar">
    <button class="btn subtle icon" title={showTree ? t("Esconder projetos") : t("Mostrar projetos")} onclick={() => (showTree = !showTree)}>
      <Glyph name="logs" />
    </button>
    <button class="btn subtle icon" title={t("Atualizar")} onclick={refresh}><Glyph name="refresh" /></button>
    <span class="grow"></span>
    <label class="search">
      <Glyph name="search" size={14} />
      <input class="input" placeholder={t("Buscar nos logs")} bind:value={search} />
    </label>
  </div>

  <div class="layout">
    {#if showTree}
      <nav class="tree" aria-label={t("Projetos")}>
        {#each projects as p (p.path)}
          <button class="node" class:current={project === p.path && !expanded[p.path]} onclick={() => toggle(p.path)} title={p.path}>
            <Glyph name={expanded[p.path] ? "chevronDown" : "chevronRight"} size={10} />
            <span class="ellipsis">{p.name}</span>
          </button>
          {#if expanded[p.path]}
            {#each files[p.path] ?? [] as f (f.name)}
              <button class="leaf" class:active={project === p.path && file === f.name} onclick={() => open(p.path, f.name)}>
                <span class="ellipsis">{f.name}</span>
                <span class="size">{kb(f.size)}</span>
              </button>
            {:else}
              <span class="leaf none caption">{files[p.path] ? t("Sem arquivos .log") : t("Carregando…")}</span>
            {/each}
          {/if}
        {:else}
          <p class="caption pad">{t("Nenhum projeto importado.")}</p>
        {/each}
      </nav>
    {/if}

    <section class="main">
      {#if !project || !file}
        <div class="placeholder">
          <Glyph name="logs" size={32} />
          <strong>{project ? t("Nenhum arquivo .log") : t("Escolha um projeto")}</strong>
          <span class="caption">
            {project ? t("Nenhum arquivo .log em storage\\logs (ou logs) deste projeto.") : t("Os logs ficam em storage\\logs (Laravel) ou na pasta logs.")}
          </span>
        </div>
      {:else}
        <div class="filters">
          {#each levels as l (l.id)}
            <button class="chip lv-{l.id}" class:on={level === l.id} onclick={() => (level = l.id)}>
              {#if l.id !== "all"}<span class="bullet"></span>{/if}
              {l.label()}
              <span class="n">{counts[l.id]}</span>
            </button>
          {/each}
          <span class="grow"></span>
          <button class="chip" class:on={grouped} title={t("Juntar mensagens iguais")} onclick={() => (grouped = !grouped)}>
            <Glyph name="logs" size={12} />
            {t("Agrupar")}
          </button>
          <button class="btn subtle icon" title={t("Abrir no editor")} onclick={() => toolsApi.logOpen(project!, file!)}><Glyph name="openExternal" /></button>
          <button class="btn subtle icon" title={t("Limpar arquivo")} onclick={clear}><Glyph name="delete" /></button>
        </div>

        <div class="entries" role="listbox" aria-label={t("Entradas do log")}>
          {#each rows as r (r.entry.id)}
            {@const e = r.entry}
            <button
              id="entry-{e.id}"
              class="entry"
              class:sel={selectedId === e.id}
              class:err={e.level === "error"}
              role="option"
              aria-selected={selectedId === e.id}
              onclick={() => select(e)}
            >
              <span class="badge lv-{e.level}">{levelLabel(e.level, e.levelName)}</span>
              <span class="time">{e.time}</span>
              <span class="msg ellipsis">{e.message}</span>
              {#if r.count > 1}<span class="count">×{r.count}</span>{/if}
            </button>
          {:else}
            <p class="caption pad">{text ? t("Nenhuma entrada com esse filtro.") : t("O arquivo está vazio.")}</p>
          {/each}
        </div>

        {#if selected}
          {@const e = selected}
          <div class="detail">
            <div class="detail-head">
              <span class="badge lv-{e.level}">{levelLabel(e.level, e.levelName)}</span>
              <span class="mono caption">{e.datetime}{e.env ? ` · ${e.env}` : ""}</span>
              <span class="grow"></span>
              <button class="btn subtle" onclick={() => copy("text")}><Glyph name={copied === "text" ? "check" : "copy"} size={14} /> {t("Copiar")}</button>
              <button class="btn subtle" onclick={() => copy("md")}><Glyph name={copied === "md" ? "check" : "copy"} size={14} /> Markdown</button>
              <button class="btn subtle icon" title={t("Fechar")} onclick={() => (selectedId = null)}><Glyph name="close" size={12} /></button>
            </div>
            <div class="detail-body">
              <h3>{e.message}</h3>
              {#if e.exception}
                <p class="exc">
                  <span class="mono">{e.exception.cls}</span>
                  {#if e.exception.file}
                    <span class="caption mono">{relative(e.exception.file, project)}:{e.exception.line}</span>
                  {/if}
                </p>
              {/if}

              {#if segments.length}
                <div class="section-title">{t("Pilha")}</div>
                <div class="frames">
                  {#each segments as s (s.key)}
                    {#if s.vendor && !openVendor.has(s.key)}
                      <button class="vendor" onclick={() => toggleVendor(s.key)}>
                        <Glyph name="chevronRight" size={10} />
                        {plural(s.frames.length, "{n} frame do vendor", "{n} frames do vendor")}
                      </button>
                    {:else}
                      {#if s.vendor}
                        <button class="vendor" onclick={() => toggleVendor(s.key)}>
                          <Glyph name="chevronDown" size={10} />
                          {plural(s.frames.length, "{n} frame do vendor", "{n} frames do vendor")}
                        </button>
                      {/if}
                      {#each s.vendor ? s.frames : [s.frame] as f (f.n)}
                        <div class="frame" class:dim={f.vendor}>
                          <span class="n">#{f.n}</span>
                          <div>
                            <div class="mono call">{f.call}</div>
                            {#if f.file}<div class="mono caption">{relative(f.file, project)}:{f.line}</div>{/if}
                          </div>
                        </div>
                      {/each}
                    {/if}
                  {/each}
                </div>
              {/if}

              {#if e.context}
                <div class="section-title">{t("Contexto")}</div>
                <pre>{e.context}</pre>
              {/if}
            </div>
          </div>
        {/if}

        <footer class="status">
          <span class="dot" class:on={!paused}></span>
          <button class="link" onclick={() => (paused = !paused)} title={paused ? t("Continuar") : t("Pausar")}>
            {paused ? t("Pausado") : t("Ao vivo")}
          </button>
          <span class="caption">{plural(entries.length, "{n} entrada", "{n} entradas")}</span>
          <span class="caption mono">{file}</span>
          {#if error}<span class="caption errtext ellipsis">{error}</span>{/if}
        </footer>
      {/if}
    </section>
  </div>
</div>

<style>
  .window {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 12px 8px;
  }
  .grow {
    flex: 1;
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
    width: 280px;
    padding-left: 32px;
  }
  .layout {
    flex: 1;
    min-height: 0;
    display: flex;
    border-top: 1px solid var(--stroke);
  }
  .ellipsis {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .pad {
    padding: 12px 16px;
    margin: 0;
  }

  /* Árvore de projetos */
  .tree {
    width: 240px;
    flex: none;
    overflow: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    border-right: 1px solid var(--stroke);
  }
  .node,
  .leaf {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-ctrl);
    background: transparent;
    text-align: left;
    flex: none;
  }
  .node :global(.glyph) {
    color: var(--text-3);
    flex: none;
  }
  .node:hover,
  .leaf:hover {
    background: var(--subtle-hover);
  }
  .node.current {
    background: var(--selected);
  }
  .leaf {
    padding-left: 28px;
  }
  .leaf .ellipsis {
    flex: 1;
  }
  .leaf.active {
    position: relative;
    background: var(--selected);
  }
  .leaf.active::before {
    content: "";
    position: absolute;
    left: 14px;
    top: 50%;
    width: 3px;
    height: 16px;
    border-radius: 2px;
    background: var(--accent);
    transform: translateY(-50%);
  }
  .leaf.none {
    height: 28px;
  }
  .size {
    font-size: 12px;
    color: var(--text-3);
  }

  /* Conteúdo */
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--card);
  }
  .placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-2);
    text-align: center;
    padding: 24px;
  }
  .placeholder strong {
    color: var(--text);
    font-size: 16px;
  }
  .filters {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--stroke);
    flex-wrap: wrap;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 14px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-2);
  }
  .chip:hover {
    background: var(--subtle-hover);
  }
  .chip.on {
    border-color: var(--stroke-strong);
    background: var(--layer);
    color: var(--text);
    font-weight: 600;
  }
  .chip .n {
    color: var(--text-3);
    font-weight: 400;
    font-size: 12px;
  }
  .bullet {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--lv);
  }

  /* Cores por nível */
  .lv-error {
    --lv: var(--err);
  }
  .lv-warning {
    --lv: var(--warn);
  }
  .lv-info {
    --lv: #3b82f6;
  }
  .lv-debug,
  .lv-other {
    --lv: var(--text-3);
  }

  .entries {
    flex: 1;
    min-height: 80px;
    overflow: auto;
  }
  .entry {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 16px;
    height: 40px;
    padding: 0 16px;
    border: 0;
    border-bottom: 1px solid var(--stroke);
    background: transparent;
    text-align: left;
  }
  .entry:hover {
    background: var(--subtle-hover);
  }
  .entry.sel {
    background: var(--selected);
  }
  .entry.err.sel {
    background: color-mix(in srgb, var(--err) 14%, var(--card));
  }
  .badge {
    flex: none;
    min-width: 64px;
    text-align: center;
    font-size: 12px;
    font-weight: 600;
    padding: 1px 8px;
    border-radius: 10px;
    color: var(--lv);
    background: color-mix(in srgb, var(--lv) 14%, transparent);
  }
  .time {
    flex: none;
    font: 12px var(--mono);
    color: var(--text-2);
    width: 64px;
  }
  .msg {
    flex: 1;
  }
  .count {
    flex: none;
    font-size: 12px;
    color: var(--text-2);
    padding: 0 8px;
    border-radius: 10px;
    border: 1px solid var(--stroke);
  }

  /* Detalhe da entrada */
  .detail {
    height: 46%;
    min-height: 160px;
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--stroke-strong);
  }
  .detail-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 16px;
    border-bottom: 1px solid var(--stroke);
  }
  .detail-head .btn :global(.glyph) {
    margin-right: 4px;
  }
  .detail-body {
    flex: 1;
    overflow: auto;
    padding: 12px 16px 16px;
    user-select: text;
  }
  h3 {
    margin: 0 0 6px;
    font: 600 15px/22px var(--font);
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }
  .exc {
    margin: 0 0 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    font-size: 12px;
  }
  .detail-body .section-title {
    margin: 14px 0 6px;
  }
  .frames {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .vendor {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    padding: 0 6px;
    border: 0;
    border-radius: var(--r-ctrl);
    background: transparent;
    color: var(--text-2);
    font-size: 12px;
    width: fit-content;
  }
  .vendor:hover {
    background: var(--subtle-hover);
  }
  .frame {
    display: flex;
    gap: 10px;
    padding: 3px 6px;
  }
  .frame .n {
    flex: none;
    width: 28px;
    text-align: right;
    font: 12px/18px var(--mono);
    color: var(--text-3);
  }
  .call {
    font-size: 12px;
    line-height: 18px;
    overflow-wrap: anywhere;
  }
  .frame .caption {
    overflow-wrap: anywhere;
  }
  .frame.dim .call {
    color: var(--text-2);
  }
  pre {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--r-ctrl);
    border: 1px solid var(--stroke);
    background: var(--layer);
    font: 12px/1.5 var(--mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 32px;
    padding: 0 16px;
    border-top: 1px solid var(--stroke);
    background: var(--layer);
  }
  .link {
    border: 0;
    padding: 0;
    background: none;
    font-size: 12px;
    color: var(--text);
  }
  .link:hover {
    text-decoration: underline;
  }
  .errtext {
    color: var(--err);
  }
</style>
