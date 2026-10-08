import { writable } from "svelte/store";

export type Theme = "light" | "dark";

const STORAGE_KEY = "soplos-webapp-manager-theme";

function readStoredTheme(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

function applyTheme(value: Theme) {
  document.documentElement.classList.toggle("light", value === "light");
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // Private/blocked storage — the toggle still works for this session.
  }
}

export const theme = writable<Theme>(readStoredTheme());
theme.subscribe(applyTheme);

export function toggleTheme() {
  theme.update((current) => (current === "dark" ? "light" : "dark"));
}
