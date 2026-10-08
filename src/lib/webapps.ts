// Shared types for the WebApp domain, mirroring the serde field names in
// src-tauri/src/commands/webapps.rs and browsers.rs.

export interface WebApp {
  id: string;
  name: string;
  url: string;
  browserId: string;
  iconPath: string;
  iconPreview: string | null;
  showNavbar: boolean;
  extraParams: string;
  isIncognito: boolean;
}

export interface WebAppInput {
  name: string;
  url: string;
  iconPath: string;
  category: string;
  browserId: string;
  showNavbar: boolean;
  extraParams: string;
  isIncognito: boolean;
}

export interface Browser {
  id: string;
  displayName: string;
  executable: string;
  isFlatpak: boolean;
  flatpakId: string;
}

export const CATEGORIES = ["network", "game", "office", "audiovideo", "development", "utility"] as const;

// Category ids sent to the .desktop file's Categories= key — freedesktop
// menu spec names, distinct from the i18n keys used to label them.
export const CATEGORY_FREEDESKTOP_ID: Record<(typeof CATEGORIES)[number], string> = {
  network: "Network",
  game: "Game",
  office: "Office",
  audiovideo: "AudioVideo",
  development: "Development",
  utility: "Utility",
};
