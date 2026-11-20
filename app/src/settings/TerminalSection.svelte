<script lang="ts">
  import { pickFolder } from "../lib/folders";
  import Glyph from "../lib/Glyph.svelte";
  import { php } from "../lib/php";
  import { app } from "../lib/state.svelte";
  import { terminal, type FolderSelection, type TerminalStatus } from "../lib/terminal";
  import Toggle from "../lib/Toggle.svelte";
  import SettingRow from "./SettingRow.svelte";

  let status = $state<TerminalStatus | null>(null);
  let minors = $state<string[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let tested = $state<{ path: string; result: FolderSelection } | null>(null);

  $effect(() => {
    terminal.status().then((s) => (status = s)).catch((e) => (error = String(e)));
    php
      .overview()
      .then((o) => (minors = o.rows.filter((r) => r.installed).map((r) => r.minor)))
      .catch(() => {});
  });

  async function run(action: Promise<TerminalStatus>) {
    busy = true;
    try {
      status = await action;
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function testFolder() {
    const path = await pickFolder("Escolha uma pasta para testar");
    if (!path) return;
    tested = { path, result: await terminal.forFolder(path) };
  }
</script>

<h1>Terminal</h1>
<p class="lead caption">
  Os comandos <span class="mono">php</span> e <span class="mono">composer</span> do Sentinel escolhem sozinhos a versão
  do PHP de cada pasta: dentro do projeto da loja, PHP 8.4; dentro do blog, PHP 8.1.
</p>

{#if error}
  <div class="infobar error"><Glyph name="info" /><span>{error}</span></div>
{/if}

{#if status}
  <SettingRow icon="terminal" title="Comandos php e composer" description={status.bin_dir}>
    <span class="caption">{status.installed ? "Instalados" : "Não instalados"}</span>
    <button class="btn" disabled={busy || !status.source_found} onclick={() => run(terminal.install())}>
      {status.installed ? "Reparar" : "Instalar"}
    </button>
  </SettingRow>

  <SettingRow
    icon="globe"
    title="Usar em qualquer terminal"
    description="Coloca a pasta dos comandos no começo do PATH do seu usuário (sem precisar de administrador). Vale para terminais abertos depois."
  >
    <span class="caption">{status.in_path ? "Ativado" : "Desativado"}</span>
    <Toggle
      label="Usar em qualquer terminal"
      checked={status.in_path}
      disabled={busy || (!status.source_found && !status.installed)}
      onchange={(v) => run(terminal.setInPath(v))}
    />
  </SettingRow>

  {#if status.conflicts.length}
    <div class="infobar warn">
      <Glyph name="info" />
      <div>
        <strong>Outro php.exe vem antes do Sentinel.</strong>
        <p class="caption">
          O Windows procura primeiro no PATH do sistema, então estes serão usados no lugar do Sentinel. Tire-os do PATH do
          sistema (Laragon, XAMPP…) ou use o botão "Abrir terminal" do projeto, que já abre com o Sentinel na frente.
        </p>
        <ul class="mono">
          {#each status.conflicts as c (c)}<li>{c}</li>{/each}
        </ul>
      </div>
    </div>
  {/if}

  <SettingRow icon="code" title="Versão fora de projetos" description="Usada em pastas que não pedem uma versão">
    <div class="select">
      <select
        value={app.config?.default_php ?? ""}
        onchange={(e) => app.update({ default_php: e.currentTarget.value || null })}
      >
        <option value="">Mais nova instalada</option>
        {#each minors as m (m)}<option value={m}>PHP {m}</option>{/each}
      </select>
    </div>
  </SettingRow>

  <SettingRow icon="search" title="Testar uma pasta" description="Mostra qual PHP o terminal usaria nela e por quê">
    <button class="btn" onclick={testFolder}>Escolher pasta…</button>
  </SettingRow>

  {#if tested}
    <div class="result card">
      <div class="mono path">{tested.path}</div>
      {#if tested.result.php}
        <div class="big"><Glyph name="check" /> PHP {tested.result.php.version}</div>
      {:else}
        <div class="big none"><Glyph name="info" /> Nenhum PHP serve</div>
      {/if}
      <div class="caption">
        {tested.result.reason}{tested.result.project ? ` · projeto ${tested.result.project}` : ""}
      </div>
    </div>
  {/if}

  <p class="tip caption">
    Dica: para forçar uma versão só no terminal atual, use
    <span class="mono">$env:SENTINEL_PHP = "8.3"</span> (PowerShell) ou <span class="mono">set SENTINEL_PHP=8.3</span> (cmd).
  </p>
{:else}
  <p class="caption">Carregando…</p>
{/if}

<style>
  h1 {
    margin: 0 0 16px;
    font: 600 28px/36px var(--font-display);
  }
  .lead {
    margin: -8px 0 12px;
  }
  .infobar {
    margin: 4px 0 8px;
  }
  .infobar.warn {
    background: color-mix(in srgb, var(--warn) 12%, var(--card));
  }
  .infobar.warn :global(.glyph),
  .infobar.error :global(.glyph) {
    color: var(--warn);
  }
  .infobar p {
    margin: 2px 0 6px;
  }
  .infobar ul {
    margin: 0;
    padding-left: 18px;
    user-select: text;
  }
  .result {
    margin-top: 4px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    user-select: text;
  }
  .path {
    color: var(--text-3);
    overflow-wrap: anywhere;
  }
  .big {
    display: flex;
    align-items: center;
    gap: 8px;
    font: 600 18px/26px var(--font-display);
  }
  .big :global(.glyph) {
    color: var(--ok);
  }
  .big.none :global(.glyph) {
    color: var(--warn);
  }
  .tip {
    margin-top: 16px;
  }
</style>
