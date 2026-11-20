import { open } from "@tauri-apps/plugin-dialog";

/** Abre o seletor de pasta do Windows. Devolve `null` se cancelado. */
export async function pickFolder(title: string): Promise<string | null> {
  if (!("__TAURI_INTERNALS__" in window)) return "D:\\www";

  const result = await open({ directory: true, multiple: false, title });
  return typeof result === "string" ? result : null;
}
