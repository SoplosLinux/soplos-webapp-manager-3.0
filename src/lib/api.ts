import { invoke } from "@tauri-apps/api/core";

// Thin wrapper around Tauri's invoke so views never call it directly.
export const api = {
  invoke,
};
