<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import Glyph from "../lib/Glyph.svelte";
  import { https, type HttpsStatus } from "../lib/https";
  import { confirm } from "../lib/php";
  import { app } from "../lib/state.svelte";
  import Toggle from "../lib/Toggle.svelte";
  import SettingRow from "./SettingRow.svelte";

  let status = $state<HttpsStatus | null>(null);
  let port = $state(443);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function load() {
    status = await https.status().catch((e) => ((error = String(e)), null));
    if (status) port = status.port;
  }
  $effect(() => {
    load();
  });

  async function run(action: () => Promise<unknown>) {
    busy = true;
    error = null;
    try {
      await action();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
      await load();
    }
  }

  function commitPort() {
    if (!status) return;
    const p = Number(port);
    if (!Number.isInteger(p) || p < 1 || p > 65535) {
      port = status.port;
      return;
    }
    if (p !== status.port) run(() => https.configure(status!.enabled, p));
  }

  async function trust() {
    const ok = await confirm(
      t('O Sentinel vai instalar a autoridade certificadora dele nas "Autoridades raiz confiáveis" do seu usuário.') +
        " " +
        t("Ela só consegue emitir certificados para domínios .{tld} (restrição gravada no próprio certificado).", { tld: app.config?.tld ?? "" }) +
        "\n\n" +
        t('O Windows vai mostrar um aviso de segurança: confira que o nome começa com "Sentinel CA local" e confirme.'),
      t("Confiar no certificado do Sentinel"),
      "Continuar",
    );
    if (ok) run(https.trust);
  }

  async function untrust() {
    const ok = await confirm(
      t("Remover a autoridade do Sentinel das raízes confiáveis? Os endereços https:// dos projetos passam a mostrar aviso no navegador."),
      t("Remover certificado"),
      t("Remover"),
    );
    if (ok) run(https.untrust);
  }
</script>

{#if status}
  <SettingRow
    icon="shield"
    title={t("HTTPS local")}
    description={status.enabled
      ? `Projetos também em https://… na porta ${status.port}`
      : "Abre os projetos em https:// com certificado do próprio Sentinel"}
  >
    <span class="caption">{status.enabled ? "Ativado" : "Desativado"}</span>
    <Toggle label={t("HTTPS local")} checked={status.enabled} disabled={busy} onchange={(v) => run(() => https.configure(v, Number(port)))} />
  </SettingRow>
  {#if status.enabled}
    <SettingRow icon="server" title={t("Porta HTTPS")} description={t("Use outra se a 443 estiver ocupada")}>
      <input
        class="input port"
        type="number"
        min="1"
        max="65535"
        bind:value={port}
        onblur={commitPort}
        onkeydown={(e) => e.key === "Enter" && commitPort()}
      />
    </SettingRow>
    <SettingRow
      icon="link"
      title={t("Certificado do Sentinel")}
      description={status.trusted
        ? t("Confiável neste usuário: Edge e Chrome abrem os projetos sem aviso")
        : t("Ainda não confiável: o navegador vai avisar que a conexão não é segura")}
    >
      {#if status.trusted}
        <span class="ok"><Glyph name="check" size={12} /> {t("Confiável")}</span>
        <button class="btn subtle" disabled={busy} onclick={untrust}>{t("Remover")}</button>
      {:else}
        <button class="btn accent" disabled={busy} onclick={trust}>{t("Confiar…")}</button>
      {/if}
      <button class="btn subtle icon" title={t("Abrir pasta do certificado (para importar no Firefox)")} onclick={() => https.openFolder()}>
        <Glyph name="folderOpen" />
      </button>
    </SettingRow>
    <p class="caption note">
      {t("O Firefox usa uma lista própria: importe o arquivo")} <span class="mono">.crt</span> {t("da pasta em Configurações do Firefox › Certificados, ou ligue")} <span class="mono">security.enterprise_roots.enabled</span>.
    </p>
  {/if}
  {#if error}<p class="caption err">{error}</p>{/if}
{/if}

<style>
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--ok, #0f7b0f);
    font-size: 12px;
  }
  .note {
    margin: 6px 4px 0;
  }
  .err {
    color: var(--err);
    margin: 6px 4px 0;
  }
  .port {
    width: 90px;
  }
</style>
