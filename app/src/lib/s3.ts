// Storage S3 local (fase 8): RustFS, um bucket por projeto.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "./api";
import type { DiffLine } from "./services";

export interface S3Status {
  installed: boolean;
  version: string | null;
  running: boolean;
  installing: boolean;
  console_url: string;
  endpoint: string;
  access_key: string;
}

export interface StoragePreview {
  diff: DiffLine[];
  bucket: string;
  needs_package: boolean;
}

let mockStatus: S3Status = {
  installed: false,
  version: null,
  running: false,
  installing: false,
  console_url: "http://s3.offline",
  endpoint: "http://127.0.0.1:9000",
  access_key: "sentinel",
};

const mock = {
  status: async (): Promise<S3Status> => ({ ...mockStatus }),
  install: async () => {
    mockStatus = { ...mockStatus, installed: true, version: "1.0.6" };
  },
  start: async () => {
    mockStatus = { ...mockStatus, running: true };
  },
  stop: async () => {
    mockStatus = { ...mockStatus, running: false };
  },
  open: async () => void window.open("about:blank"),
  preview: async (_path: string): Promise<StoragePreview> => ({
    bucket: "loja-virtual",
    needs_package: true,
    diff: [
      { key: "FILESYSTEM_DISK", before: "local", after: "s3" },
      { key: "AWS_BUCKET", before: "", after: "loja-virtual" },
    ],
  }),
  apply: async (_path: string, _kind: string) => {},
};

export const s3 = inTauri
  ? {
      status: () => invoke<S3Status>("s3_status"),
      install: () => invoke<void>("s3_install"),
      start: () => invoke<void>("s3_start"),
      stop: () => invoke<void>("s3_stop"),
      open: () => invoke<void>("open_s3"),
      preview: (path: string) => invoke<StoragePreview>("project_storage_preview", { path }),
      apply: (path: string, kind: string) => invoke<void>("project_storage_apply", { path, kind }),
    }
  : mock;
