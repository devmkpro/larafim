import { getCurrentWindow } from "@tauri-apps/api/window";
import { inTauri, type Theme } from "./api";

const media = window.matchMedia("(prefers-color-scheme: dark)");
let current: Theme = "system";

function resolved(theme: Theme): "light" | "dark" {
  if (theme === "system") return media.matches ? "dark" : "light";
  return theme;
}

/** Aplica o tema na página e na barra de título da janela. */
export function applyTheme(theme: Theme) {
  current = theme;
  document.documentElement.dataset.theme = resolved(theme);
  if (!inTauri) return;
  // `null` faz a barra de título seguir o Windows.
  getCurrentWindow()
    .setTheme(theme === "system" ? null : theme)
    .catch(() => {});
}

media.addEventListener("change", () => {
  if (current === "system") applyTheme("system");
});
