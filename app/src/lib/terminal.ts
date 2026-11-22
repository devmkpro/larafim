// Terminal: atalhos php/composer, PATH do usuário e abrir terminal no projeto.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "./api";

export interface TerminalStatus {
  bin_dir: string;
  installed: boolean;
  source_found: boolean;
  in_path: boolean;
  conflicts: string[];
  default_php: string | null;
}

export interface FolderSelection {
  php: { version: string; minor: string; dir: string } | null;
  reason: string;
  project: string | null;
  request: string | null;
}

// ---------- Dados de exemplo (navegador) ----------
let mockStatus: TerminalStatus = {
  bin_dir: "C:\\Users\\voce\\AppData\\Local\\Larafim\\bin",
  installed: false,
  source_found: true,
  in_path: false,
  conflicts: ["C:\\laragon\\bin\\php\\php-8.1.10-Win32-vs16-x64\\php.exe"],
  default_php: null,
};

const mock = {
  status: async () => ({ ...mockStatus }),
  install: async () => (mockStatus = { ...mockStatus, installed: true }),
  setInPath: async (enabled: boolean) => (mockStatus = { ...mockStatus, installed: true, in_path: enabled }),
  forFolder: async (path: string): Promise<FolderSelection> => ({
    php: { version: "8.4.27", minor: "8.4", dir: "" },
    reason: "versão fixada no projeto (8.4)",
    project: path.split("\\").pop() ?? null,
    request: "^8.3",
  }),
  openTerminal: async (_path: string) => {},
};

export const terminal = inTauri
  ? {
      status: () => invoke<TerminalStatus>("terminal_status"),
      install: () => invoke<TerminalStatus>("install_shims"),
      setInPath: (enabled: boolean) => invoke<TerminalStatus>("set_shims_in_path", { enabled }),
      forFolder: (path: string) => invoke<FolderSelection>("php_for_folder", { path }),
      openTerminal: (path: string) => invoke<void>("open_terminal", { path }),
    }
  : mock;
