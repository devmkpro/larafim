// Serviços: MariaDB, banco por projeto (.env), backups e Adminer.
import { invoke } from "@tauri-apps/api/core";
import { inTauri, mockEmit } from "./api";

export interface MariaRow {
  major: string;
  latest: string | null;
  installed: string | null;
  eol: string | null;
  update_available: boolean;
  installing: boolean;
}

export interface MariaStatus {
  running: boolean;
  version: string | null;
  port: number | null;
  pid: number | null;
}

export interface ServicesOverview {
  rows: MariaRow[];
  catalog_error: string | null;
  status: MariaStatus;
  port: number;
  selected: string | null;
  databases: string[];
  adminer: boolean;
  adminer_url: string;
  own: OwnStatus[];
}

export interface DiffLine {
  key: string;
  before: string | null;
  after: string | null;
}

export interface DbPreview {
  diff: DiffLine[];
  database: string | null;
  from_example: boolean;
  no_env: boolean;
}

export interface Backup {
  file: string;
  name: string;
  size: number;
  modified: number;
}

export type DbKind = "mariadb" | "mariadb-own" | "postgres" | "sqlite" | "none" | `external:${string}`;

export interface OwnDb {
  version: string;
  port: number;
}

export interface OwnStatus {
  project: string;
  path: string;
  version: string;
  port: number;
  running: boolean;
}

export interface ConnectionInfo {
  id: string;
  name: string;
  kind: "mysql" | "mariadb";
  host: string;
  port: number;
  user: string;
  has_password: boolean;
  client_dir: string | null;
  used_by: string[];
}

/** Formulário. `password: null` = manter a senha guardada. */
export interface ConnectionInput {
  id: string | null;
  name: string;
  kind: "mysql" | "mariadb";
  host: string;
  port: number;
  user: string;
  password: string | null;
  client_dir: string | null;
}

/** Painel de detalhes de um serviço. */
export interface ServiceInfo {
  data_dir: string | null;
  log_file: string | null;
  log: string[];
  env: [string, string][];
}

export interface DetectedDb {
  port: number;
  kind: "mysql" | "mariadb";
  process: string;
  hint: string | null;
  client_dir: string | null;
  known: boolean;
}

// ---------- Dados de exemplo (navegador) ----------
let running = false;
const rows: MariaRow[] = [
  { major: "12.3", latest: "12.3.3", installed: null, eol: "2029-06-12", update_available: false, installing: false },
  { major: "11.8", latest: "11.8.9", installed: "11.8.9", eol: "2028-06-04", update_available: false, installing: false },
  { major: "11.4", latest: "11.4.12", installed: null, eol: "2029-05-29", update_available: false, installing: false },
];
const mockBackups: Backup[] = [
  { file: "C:\\b\\loja_virtual-20261120-142200.sql", name: "loja_virtual-20261120-142200.sql", size: 48213, modified: 1795190000 },
];

const mock = {
  overview: async (_refresh = false): Promise<ServicesOverview> => ({
    rows: rows.map((r) => ({ ...r })),
    catalog_error: null,
    status: { running, version: running ? "11.8.9" : null, port: running ? 3307 : null, pid: running ? 1234 : null },
    port: 3307,
    selected: null,
    databases: running ? ["blog", "loja_virtual"] : [],
    adminer: true,
    adminer_url: "http://db.offline",
    own: [{ project: "sistema-legado", path: "C:\Projetos\sistema-legado", version: "10.11", port: 3310, running: false }],
  }),
  install: async (_major: string) => {},
  remove: async (_major: string) => {},
  start: async () => {
    running = true;
    mockEmit("services-changed", null);
  },
  stop: async () => {
    running = false;
    mockEmit("services-changed", null);
  },
  configure: async (_version: string | null, _port: number) => {},
  dbPreview: async (_path: string, kind: DbKind, own: OwnDb | null = null): Promise<DbPreview> => ({
    diff:
      kind === "mariadb"
        ? [
            { key: "DB_CONNECTION", before: "sqlite", after: "mariadb" },
            { key: "DB_HOST", before: null, after: "127.0.0.1" },
            { key: "DB_PORT", before: null, after: String(own?.port ?? 3307) },
            { key: "DB_DATABASE", before: "laravel", after: "loja_virtual" },
            { key: "DB_USERNAME", before: null, after: "root" },
            { key: "DB_PASSWORD", before: null, after: "" },
          ]
        : kind === "mariadb-own"
          ? [{ key: "DB_PORT", before: "3307", after: String(own?.port ?? 3310) }]
          : [{ key: "DB_CONNECTION", before: "mariadb", after: "sqlite" }],
    database: kind.startsWith("mariadb") ? "loja_virtual" : null,
    from_example: false,
    no_env: false,
  }),
  dbApply: async (_path: string, _kind: DbKind, _own: OwnDb | null = null) => {},
  suggestOwnPort: async (_path: string) => 3310,
  ownStart: async (_path: string) => {},
  ownStop: async (_path: string) => {},
  envUndo: async (_path: string) => {},
  envHasBackup: async (_path: string) => true,
  backups: async (_path: string) => mockBackups.map((b) => ({ ...b })),
  backup: async (_path: string) => "C:\\b\\novo.sql",
  restore: async (_path: string, _file: string) => {},
  openBackups: async (_path: string) => {},
  openAdminer: async (_db: string | null = null, _connection: string | null = null, _project: string | null = null) =>
    void window.open("about:blank"),
  connections: async (): Promise<ConnectionInfo[]> => mockConnections.map((c) => ({ ...c })),
  testConnection: async (_input: ConnectionInput) => "8.0.30",
  saveConnection: async (input: ConnectionInput) => {
    const id = input.id ?? "mysql-do-laragon";
    mockConnections = [
      ...mockConnections.filter((c) => c.id !== id),
      { ...input, id, has_password: !!input.password, used_by: [] } as ConnectionInfo,
    ];
    mockEmit("services-changed", null);
    return id;
  },
  removeConnection: async (id: string) => {
    mockConnections = mockConnections.filter((c) => c.id !== id);
    mockEmit("services-changed", null);
  },
  info: async (service: string, _project: string | null = null): Promise<ServiceInfo> => ({
    data_dir: "C:\\Users\\voce\\AppData\\Local\\Sentinel\\data\\" + service,
    log_file: null,
    log: ["2026-11-21 10:00:00 0 [Note] Server ready for connections", "2026-11-21 10:00:00 0 [Note] port: 3307"],
    env: [
      ["DB_CONNECTION", "mariadb"],
      ["DB_HOST", "127.0.0.1"],
      ["DB_PORT", "3307"],
      ["DB_DATABASE", "laravel"],
      ["DB_USERNAME", "root"],
      ["DB_PASSWORD", ""],
    ],
  }),
  detect: async (): Promise<DetectedDb[]> => [
    { port: 3306, kind: "mysql", process: "mysqld.exe", hint: "MySQL/MariaDB", client_dir: "C:\laragon\bin\mysql\mysql-8.0.30-winx64\bin", known: mockConnections.some((c) => c.port === 3306) },
  ],
};
let mockConnections: ConnectionInfo[] = [];

export const services = inTauri
  ? {
      overview: (refresh = false) => invoke<ServicesOverview>("services_overview", { refresh }),
      install: (major: string) => invoke<void>("maria_install", { major }),
      remove: (major: string) => invoke<void>("maria_remove", { major }),
      start: () => invoke<void>("maria_start"),
      stop: () => invoke<void>("maria_stop"),
      configure: (version: string | null, port: number) => invoke<void>("maria_configure", { version, port }),
      dbPreview: (path: string, kind: DbKind, own: OwnDb | null = null) =>
        invoke<DbPreview>("project_db_preview", { path, kind, own }),
      dbApply: (path: string, kind: DbKind, own: OwnDb | null = null) => invoke<void>("project_db_apply", { path, kind, own }),
      suggestOwnPort: (path: string) => invoke<number>("suggest_own_port", { path }),
      ownStart: (path: string) => invoke<void>("own_start", { path }),
      ownStop: (path: string) => invoke<void>("own_stop", { path }),
      envUndo: (path: string) => invoke<void>("project_env_undo", { path }),
      envHasBackup: (path: string) => invoke<boolean>("project_env_has_backup", { path }),
      backups: (path: string) => invoke<Backup[]>("db_backups", { path }),
      backup: (path: string) => invoke<string>("db_backup", { path }),
      restore: (path: string, file: string) => invoke<void>("db_restore", { path, file }),
      openBackups: (path: string) => invoke<void>("open_backups_folder", { path }),
      openAdminer: (db: string | null = null, connection: string | null = null, project: string | null = null) =>
        invoke<void>("open_adminer", { database: db, connection, project }),
      connections: () => invoke<ConnectionInfo[]>("db_connections"),
      testConnection: (input: ConnectionInput) => invoke<string>("db_connection_test", { input }),
      saveConnection: (input: ConnectionInput) => invoke<string>("db_connection_save", { input }),
      removeConnection: (id: string) => invoke<void>("db_connection_remove", { id }),
      detect: () => invoke<DetectedDb[]>("db_detect"),
      info: (service: string, project: string | null = null) => invoke<ServiceInfo>("service_info", { service, project }),
    }
  : mock;
