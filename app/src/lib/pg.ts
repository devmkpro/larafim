// PostgreSQL (fase 8): download opcional, servidor sob demanda.
import { invoke } from "@tauri-apps/api/core";
import { inTauri, mockEmit } from "./api";

export interface PgRow {
  major: string;
  latest: string | null;
  installed: string | null;
  update_available: boolean;
  installing: boolean;
}

export interface PgOverview {
  rows: PgRow[];
  catalog_error: string | null;
  status: { running: boolean; version: string | null; port: number | null; pid: number | null };
  port: number;
  selected: string | null;
  databases: string[];
}

let mockState: PgOverview = {
  rows: [
    { major: "18", latest: "18.7.0", installed: null, update_available: false, installing: false },
    { major: "17", latest: "17.12.0", installed: null, update_available: false, installing: false },
    { major: "16", latest: "16.16.0", installed: null, update_available: false, installing: false },
  ],
  catalog_error: null,
  status: { running: false, version: null, port: null, pid: null },
  port: 5432,
  selected: null,
  databases: [],
};

const mock = {
  overview: async (_refresh = false): Promise<PgOverview> => structuredClone(mockState),
  install: async (major: string) => {
    mockState.rows = mockState.rows.map((r) => (r.major === major ? { ...r, installed: r.latest } : r));
    mockEmit("pg:done", { major, ok: true, error: null });
  },
  remove: async (major: string) => {
    mockState.rows = mockState.rows.map((r) => (r.major === major ? { ...r, installed: null } : r));
  },
  start: async () => {
    const v = mockState.rows.find((r) => r.installed)?.installed ?? null;
    mockState.status = { running: true, version: v, port: mockState.port, pid: 1 };
  },
  stop: async () => {
    mockState.status = { running: false, version: null, port: null, pid: null };
  },
  configure: async (version: string | null, port: number) => {
    mockState = { ...mockState, selected: version, port };
  },
};

export const pg = inTauri
  ? {
      overview: (refresh = false) => invoke<PgOverview>("pg_overview", { refresh }),
      install: (major: string) => invoke<void>("pg_install", { major }),
      remove: (major: string) => invoke<void>("pg_remove", { major }),
      start: () => invoke<void>("pg_start"),
      stop: () => invoke<void>("pg_stop"),
      configure: (version: string | null, port: number) => invoke<void>("pg_configure", { version, port }),
    }
  : mock;
