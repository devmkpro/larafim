// HTTPS local (fase 9): CA do Sentinel e certificado por domínio.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "./api";

export interface HttpsStatus {
  enabled: boolean;
  port: number;
  trusted: boolean;
  ca_file: string | null;
}

let mock: HttpsStatus = { enabled: false, port: 443, trusted: false, ca_file: null };

export const https = inTauri
  ? {
      status: () => invoke<HttpsStatus>("https_status"),
      configure: (enabled: boolean, port: number) => invoke<void>("https_configure", { enabled, port }),
      trust: () => invoke<void>("https_trust"),
      untrust: () => invoke<void>("https_untrust"),
      openFolder: () => invoke<void>("https_open_ca_folder"),
    }
  : {
      status: async () => ({ ...mock }),
      configure: async (enabled: boolean, port: number) => {
        mock = { ...mock, enabled, port, ca_file: "C:\\sentinel\\data\\ca\\sentinel-ca-offline.crt" };
      },
      trust: async () => {
        mock = { ...mock, trusted: true };
      },
      untrust: async () => {
        mock = { ...mock, trusted: false };
      },
      openFolder: async () => {},
    };
