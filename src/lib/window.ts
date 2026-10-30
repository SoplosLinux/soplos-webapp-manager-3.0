import { writable } from "svelte/store";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Whether the system draws a frame around the window. With `decorations` off in
// tauri.conf.json it does not: the window is one card with rounded corners (the
// .window-card rule of global.css) and the program shows its own window buttons
// (WindowControls.svelte). With it on, none of that shows.
export const frameless = writable(false);
export const maximized = writable(false);

(async () => {
  try {
    const win = getCurrentWindow();
    if (await win.isDecorated()) return;
    frameless.set(true);
    maximized.set(await win.isMaximized());
    await win.onResized(async () => maximized.set(await win.isMaximized()));
  } catch {
    // Not inside Tauri (a browser while developing): leave the window as it is.
  }
})();
