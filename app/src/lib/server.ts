// Servidor local: ligar/desligar projetos, status e erros com ação.
import { invoke } from "@tauri-apps/api/core";
import { inTauri, mockConfig, mockEmit } from "./api";
import { mockSetProjectOnline } from "./projects";

export interface OnlineSite {
  name: string;
  url: string;
  path: string;
}

export interface ServerStatus {
  running: boolean;
  port: number;
  online: OnlineSite[];
  php_running: string[];
  hosts_outdated: boolean;
}

export interface PortOwner {
  pid: number;
  name: string;
  hint: string | null;
}

export interface ServerError {
  code: "port_busy" | "uac_denied" | "hosts_failed" | "no_php" | "node_later" | "error";
  message: string;
  owner: PortOwner | null;
  suggest: string | null;
}

/** Converte o que veio do Rust (objeto ou texto) num ServerError. */
export function asServerError(e: unknown): ServerError {
  if (e && typeof e === "object" && "code" in e) return e as ServerError;
  return { code: "error", message: String(e), owner: null, suggest: null };
}

// ---------- Dados de exemplo (navegador) ----------
let mockOnline: OnlineSite[] = [];
const mockStatus = (): ServerStatus => ({
  running: mockOnline.length > 0,
  port: mockConfig.http_port,
  online: [...mockOnline],
  php_running: mockOnline.length ? ["8.4.27"] : [],
  hosts_outdated: false,
});

const mock = {
  setOnline: async (path: string, online: boolean) => {
    await new Promise((r) => setTimeout(r, 400));
    const name = path.split("\\").pop()!;
    if (online && name === "painel-admin") {
      throw {
        code: "port_busy",
        message: "A porta 80 está ocupada por httpd.exe (Apache — provavelmente do Laragon ou XAMPP).",
        owner: { pid: 4321, name: "httpd.exe", hint: "Apache — provavelmente do Laragon ou XAMPP" },
        suggest: null,
      } satisfies ServerError;
    }
    mockOnline = online
      ? [...mockOnline.filter((s) => s.path !== path), { name, path, url: `http://${name}.${mockConfig.tld}` }]
      : mockOnline.filter((s) => s.path !== path);
    mockSetProjectOnline(path, online);
    const status = mockStatus();
    mockEmit("server-status", status);
    return { status };
  },
  status: async () => mockStatus(),
  stopAll: async () => {
    mockOnline = [];
    mockEmit("server-status", mockStatus());
    return mockStatus();
  },
  fixHosts: async () => mockStatus(),
  openUrl: async (url: string) => void window.open(url, "_blank"),
};

export const server = inTauri
  ? {
      setOnline: (path: string, online: boolean) =>
        invoke<{ status: ServerStatus }>("set_project_online", { path, online }),
      status: () => invoke<ServerStatus>("server_status"),
      stopAll: () => invoke<ServerStatus>("stop_all_projects"),
      fixHosts: () => invoke<ServerStatus>("fix_hosts"),
      openUrl: (url: string) => invoke<void>("open_url", { url }),
    }
  : mock;
