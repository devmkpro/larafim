import { api, onEvent, type AppConfig, type AppInfo } from "./api";
import { applyTheme } from "./theme";
import { setLanguage } from "./i18n.svelte";

class AppState {
  config = $state<AppConfig | null>(null);
  info = $state<AppInfo | null>(null);
  error = $state<string | null>(null);
  saving = $state(false);

  async load() {
    try {
      const [config, info] = await Promise.all([api.getConfig(), api.appInfo()]);
      this.config = config;
      this.info = info;
      applyTheme(config.theme);
      setLanguage(config.language);
    } catch (e) {
      this.error = String(e);
    }
    // Outra janela salvou: atualiza esta também (tema, domínio, pastas…).
    onEvent<AppConfig>("config-changed", (config) => {
      this.config = config;
      applyTheme(config.theme);
      setLanguage(config.language);
    });
  }

  /** Aplica uma mudança e grava; em caso de erro, volta ao valor anterior. */
  async update(change: Partial<AppConfig>) {
    if (!this.config) return;
    const previous = this.config;
    const next = { ...previous, ...change };
    this.config = next;
    if (change.theme) applyTheme(change.theme);
    if (change.language) setLanguage(change.language);
    this.saving = true;
    try {
      this.config = await api.saveConfig(next);
      this.error = null;
    } catch (e) {
      this.config = previous;
      applyTheme(previous.theme);
      setLanguage(previous.language);
      this.error = String(e);
    } finally {
      this.saving = false;
    }
  }
}

export const app = new AppState();
