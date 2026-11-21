// Ponte entre a interface e o núcleo em Rust (comandos Tauri).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type Theme = "system" | "light" | "dark";

export interface AppConfig {
  theme: Theme;
  tld: string;
  http_port: number;
  roots: string[];
  start_with_windows: boolean;
  keep_in_tray: boolean;
  onboarding_done: boolean;
  default_php?: string | null;
  db_version?: string | null;
  db_port?: number;
}

export interface AppInfo {
  version: string;
  data_dir: string;
  logs_dir: string;
}

export type ProjectKind = "laravel" | "php" | "node";

export type SettingsSection = "general" | "folders" | "network" | "php" | "terminal" | "node" | "services" | "about";

/** `false` quando a interface roda num navegador comum (`npm run dev`). */
export const inTauri = "__TAURI_INTERNALS__" in window;

/** Qual janela é esta: a principal ou a de configurações. */
export const windowLabel: string = inTauri
  ? getCurrentWindow().label
  : (new URLSearchParams(location.search).get("window") ?? "main");

// ---------- Dados de exemplo para ver as telas no navegador ----------
export let mockConfig: AppConfig = {
  theme: "system",
  tld: "offline",
  http_port: 80,
  roots: ["D:\\www"],
  start_with_windows: false,
  keep_in_tray: true,
  onboarding_done: false,
};

const mock = {
  getConfig: async () => ({ ...mockConfig }),
  saveConfig: async (config: AppConfig) => (mockConfig = { ...config }),
  appInfo: async (): Promise<AppInfo> => ({
    version: "0.1.0",
    data_dir: "C:\\Users\\voce\\AppData\\Local\\Sentinel",
    logs_dir: "C:\\Users\\voce\\AppData\\Local\\Sentinel\\logs",
  }),
  openDataDir: async () => {},
  openFolder: async (_path: string) => {},
  openSettings: async (section: SettingsSection = "general") => {
    window.open(`/?window=settings&section=${section}`, "_blank");
  },
};

export const api = inTauri
  ? {
      getConfig: () => invoke<AppConfig>("get_config"),
      saveConfig: (config: AppConfig) => invoke<AppConfig>("save_config", { config }),
      appInfo: () => invoke<AppInfo>("app_info"),
      openDataDir: () => invoke<void>("open_data_dir"),
      openFolder: (path: string) => invoke<void>("open_folder", { path }),
      openSettings: (section: SettingsSection = "general") => invoke<void>("open_settings", { section }),
    }
  : mock;

// No navegador, os dados de exemplo emitem eventos por este barramento local.
const localBus = new Map<string, Set<(payload: unknown) => void>>();

export function mockEmit(name: string, payload: unknown) {
  localBus.get(name)?.forEach((h) => h(payload));
}

/** Escuta um evento do Rust. Devolve a função para parar de escutar. */
export function onEvent<T>(name: string, handler: (payload: T) => void): () => void {
  if (!inTauri) {
    const set = localBus.get(name) ?? new Set();
    set.add(handler as (p: unknown) => void);
    localBus.set(name, set);
    return () => set.delete(handler as (p: unknown) => void);
  }
  const pending = listen<T>(name, (e) => handler(e.payload));
  return () => void pending.then((stop) => stop());
}

/** URL completa de um projeto (igual à regra do Rust). */
export function projectUrl(cfg: AppConfig, domain: string): string {
  const base = `http://${domain}.${cfg.tld}`;
  return cfg.http_port === 80 ? base : `${base}:${cfg.http_port}`;
}

export const kindLabel: Record<ProjectKind, string> = {
  laravel: "Laravel",
  php: "PHP",
  node: "Node.js",
};
