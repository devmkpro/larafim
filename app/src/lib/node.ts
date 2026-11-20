// Node.js: versões (instalar/remover) e servidores de desenvolvimento.
import { invoke } from "@tauri-apps/api/core";
import { inTauri, mockEmit } from "./api";

export interface NodeRow {
  major: string;
  installed: string | null;
  latest: string | null;
  lts: string | null;
  update_available: boolean;
  installing: boolean;
}

export interface NodeOverview {
  rows: NodeRow[];
  catalog_error: string | null;
}

export interface NodeProgress {
  major: string;
  received: number;
  total: number | null;
}

export interface NodeDone {
  major: string;
  ok: boolean;
  error: string | null;
}

export type DevState = "starting" | "running" | "exited";

export interface DevStatus {
  key: string;
  state: DevState;
  port: number | null;
  exit_code: number | null;
  command: string;
}

export interface DevInfo {
  status: DevStatus | null;
  logs: string[];
}

export interface DevLog {
  key: string;
  line: string;
}

/** Mesma chave usada no Rust para identificar o servidor de um projeto. */
export const devKey = (path: string) => path.toLowerCase();

// ---------- Dados de exemplo (navegador) ----------
const mockRows: NodeRow[] = [
  { major: "26", installed: null, latest: "26.13.1", lts: "Lithium", update_available: false, installing: false },
  { major: "24", installed: "24.21.0", latest: "24.22.3", lts: "Krypton", update_available: true, installing: false },
  { major: "22", installed: "22.23.5", latest: "22.23.5", lts: "Jod", update_available: false, installing: false },
  { major: "20", installed: null, latest: "20.20.2", lts: "Iron", update_available: false, installing: false },
];

const mock = {
  overview: async (_refresh = false): Promise<NodeOverview> => ({ rows: mockRows.map((r) => ({ ...r })), catalog_error: null }),
  install: async (major: string) => {
    const row = mockRows.find((r) => r.major === major)!;
    let received = 0;
    const total = 32_000_000;
    const t = setInterval(() => {
      received += 4_000_000;
      if (received < total) return mockEmit("node:progress", { major, received, total });
      clearInterval(t);
      Object.assign(row, { installed: row.latest, update_available: false });
      mockEmit("node:done", { major, ok: true, error: null });
    }, 120);
  },
  cancel: async (_major: string) => {},
  remove: async (major: string) => void Object.assign(mockRows.find((r) => r.major === major)!, { installed: null }),
  installed: async () => mockRows.filter((r) => r.installed).map((r) => r.major),
  devInfo: async (path: string): Promise<DevInfo> => ({
    status: { key: devKey(path), state: "running", port: 5173, exit_code: null, command: "npm run dev" },
    logs: ["> dev", "> vite", "", "  VITE v6.3.5  ready in 412 ms", "", "  ➜  Local:   http://localhost:5173/"],
  }),
  restartDev: async (_path: string) => {},
};

export const nodeApi = inTauri
  ? {
      overview: (refresh = false) => invoke<NodeOverview>("node_overview", { refresh }),
      install: (major: string) => invoke<void>("node_install", { major }),
      cancel: (major: string) => invoke<void>("node_cancel", { major }),
      remove: (major: string) => invoke<void>("node_remove", { major }),
      installed: () => invoke<string[]>("node_installed"),
      devInfo: (path: string) => invoke<DevInfo>("dev_info", { path }),
      restartDev: (path: string) => invoke<void>("restart_dev", { path }),
    }
  : mock;
