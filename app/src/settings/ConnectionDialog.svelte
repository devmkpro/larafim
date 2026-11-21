<script lang="ts">
  import Glyph from "../lib/Glyph.svelte";
  import { pickFolder } from "../lib/folders";
  import { services, type ConnectionInput } from "../lib/services";

  // Cadastrar/editar um banco que já existe no PC (MySQL/MariaDB).
  let {
    initial,
    hasPassword = false,
    onclose,
    onsaved,
  }: { initial: ConnectionInput; hasPassword?: boolean; onclose: () => void; onsaved: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let form = $state<ConnectionInput>({ ...initial, password: null });
  let password = $state("");
  // svelte-ignore state_referenced_locally
  let changePassword = $state(!hasPassword);
  let testing = $state(false);
  let result = $state<{ ok: boolean; text: string } | null>(null);
  let saving = $state(false);

  const input = (): ConnectionInput => ({ ...form, port: Number(form.port), password: changePassword ? password : null });

  async function test() {
    testing = true;
    result = null;
    try {
      result = { ok: true, text: `Conectou: versão ${await services.testConnection(input())}` };
    } catch (e) {
      result = { ok: false, text: String(e) };
    } finally {
      testing = false;
    }
  }

  async function save() {
    saving = true;
    try {
      await services.saveConnection(input());
      onsaved();
    } catch (e) {
      result = { ok: false, text: String(e) };
    } finally {
      saving = false;
    }
  }

  async function chooseClientDir() {
    const dir = await pickFolder("Pasta com mysql.exe e mysqldump.exe (ex.: ...\\mysql-8.0\\bin)");
    if (dir) form.client_dir = dir;
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="smoke" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="conn-title">
    <div class="content">
      <h2 id="conn-title">{initial.id ? "Editar banco" : "Usar um banco que já tenho"}</h2>
      <p class="caption">
        O Sentinel não liga nem desliga esse servidor: só cria bancos nele, ajusta o .env dos projetos e faz backups.
      </p>

      <label class="row"><span>Nome</span><input class="input" bind:value={form.name} placeholder="MySQL do Laragon" /></label>
      <div class="row">
        <span>Tipo</span>
        <div class="select">
          <select bind:value={form.kind}>
            <option value="mysql">MySQL</option>
            <option value="mariadb">MariaDB</option>
          </select>
        </div>
      </div>
      <div class="row">
        <span>Servidor</span>
        <span class="pair">
          <input class="input host" bind:value={form.host} />
          <span class="caption">:</span>
          <input class="input port" type="number" bind:value={form.port} />
        </span>
      </div>
      <label class="row"><span>Usuário</span><input class="input" bind:value={form.user} /></label>
      <div class="row">
        <span>Senha</span>
        {#if changePassword}
          <input class="input" type="password" bind:value={password} placeholder="vazia = sem senha" autocomplete="off" />
        {:else}
          <span class="pair">
            <span class="caption">Guardada (protegida pelo Windows)</span>
            <button class="btn subtle" onclick={() => (changePassword = true)}>Trocar</button>
          </span>
        {/if}
      </div>
      <div class="row">
        <span>Programas</span>
        <span class="pair">
          <span class="caption dir" title={form.client_dir ?? ""}>{form.client_dir ?? "Automático"}</span>
          <button class="btn subtle icon" title="Escolher pasta do mysql.exe" onclick={chooseClientDir}><Glyph name="folderOpen" /></button>
        </span>
      </div>

      {#if result}
        <div class="notice" class:ok={result.ok}>
          <Glyph name={result.ok ? "check" : "info"} size={14} />
          <span>{result.text}</span>
        </div>
      {/if}
    </div>
    <div class="footer">
      <button class="btn" onclick={test} disabled={testing}>{testing ? "Testando…" : "Testar conexão"}</button>
      <span class="grow"></span>
      <button class="btn accent" onclick={save} disabled={saving}>Salvar</button>
      <button class="btn" onclick={onclose}>Cancelar</button>
    </div>
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
    width: 500px;
    max-width: calc(100vw - 48px);
    border-radius: 8px;
    border: 1px solid var(--stroke);
    background: var(--card);
    box-shadow: var(--overlay-shadow), 0 32px 64px rgb(0 0 0 / 0.19);
    overflow: hidden;
  }
  .content {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h2 {
    margin: 0;
    font: 600 20px/28px var(--font-display);
  }
  p {
    margin: 0 0 6px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 32px;
  }
  .row > .input,
  .row > .select {
    width: 260px;
  }
  .pair {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 260px;
    justify-content: flex-end;
  }
  .host {
    flex: 1;
    min-width: 0;
  }
  .port {
    width: 80px;
  }
  .dir {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
  }
  .notice {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 8px 10px;
    border-radius: var(--r-card);
    border: 1px solid var(--stroke);
    font-size: 12px;
    background: color-mix(in srgb, var(--err) 10%, var(--card));
    user-select: text;
  }
  .notice.ok {
    background: color-mix(in srgb, var(--ok) 12%, var(--card));
  }
  .footer {
    display: flex;
    gap: 8px;
    padding: 20px 24px;
    background: var(--win-bg);
    border-top: 1px solid var(--stroke);
  }
  .grow {
    flex: 1;
  }
</style>
