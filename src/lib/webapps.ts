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
  /** The Categories= value of the .desktop file, as it is written there. */
  category: string;
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

// An extension of a webapp, and one found by a search in addons.mozilla.org.
export interface WebAppExtension {
  id: string;
  name: string;
}

export interface ExtensionResult {
  slug: string;
  name: string;
  summary: string;
  iconUrl: string;
  users: number;
}

/** What the form asks to do with the extensions once the webapp is saved. */
export interface ExtensionChanges {
  /** The names, in addons.mozilla.org, of the extensions found by the search. */
  addSlugs: string[];
  addFiles: string[];
  remove: string[];
}

// Extensions go in the profile of a Firefox-based browser (Flatpak or not).
const EXTENSION_FAMILIES = ["firefox", "librewolf", "zen", "midori"];

export const supportsExtensions = (browserId: string) => EXTENSION_FAMILIES.includes(browserId.replace(/-flatpak$/, ""));

// The browsers based on Chrome: their extensions come from the store of the
// browser, and they share the launch parameters below.
const CHROMIUM_FAMILIES = ["chromium", "chrome", "brave", "brave-origin", "vivaldi", "edge", "opera", "opera-gx", "helium"];

export const isChromiumFamily = (browserId: string) => CHROMIUM_FAMILIES.includes(browserId.replace(/-flatpak$/, ""));

// The launch parameters offered as checkboxes. Some need a value, which is
// asked for under the checkbox. Each has its explanation in
// webapp.paramsHelp.<key>, and anything else is typed in the free field.
export interface LaunchParam {
  key: string;
  flag: string;
  value?: { placeholder: string; quoted: boolean };
}

export const LAUNCH_PARAMS: LaunchParam[] = [
  { key: "startMaximized", flag: "--start-maximized" },
  { key: "forceDarkMode", flag: "--force-dark-mode" },
  { key: "disableNotifications", flag: "--disable-notifications" },
  { key: "disableFeatures", flag: "--disable-features=TabHoverCard" },
  { key: "windowSize", flag: "--window-size=", value: { placeholder: "1200,800", quoted: false } },
  { key: "userAgent", flag: "--user-agent=", value: { placeholder: "Mozilla/5.0 ...", quoted: true } },
  { key: "proxyServer", flag: "--proxy-server=", value: { placeholder: "IP:PORT", quoted: false } },
  { key: "shmSize", flag: "--shm-size=", value: { placeholder: "2gb", quoted: false } },
];

export interface ParsedParams {
  checked: Record<string, boolean>;
  values: Record<string, string>;
  /** What is not one of the checkbox parameters, as it was written. */
  other: string;
}

/** Splits the saved parameters into the checkboxes and what is left over. */
export function parseExtraParams(text: string): ParsedParams {
  const checked: Record<string, boolean> = {};
  const values: Record<string, string> = {};
  const other: string[] = [];
  // Words, where a quoted piece (a user agent with spaces) stays whole.
  for (const token of text.match(/(?:[^\s"]+|"[^"]*")+/g) ?? []) {
    const plain = LAUNCH_PARAMS.find((param) => !param.value && param.flag === token);
    const valued = LAUNCH_PARAMS.find((param) => param.value && token.startsWith(param.flag));
    if (plain) {
      checked[plain.key] = true;
    } else if (valued) {
      checked[valued.key] = true;
      values[valued.key] = token.slice(valued.flag.length).replace(/^"|"$/g, "");
    } else {
      other.push(token);
    }
  }
  return { checked, values, other: other.join(" ") };
}

/** The parameters to save: the checked ones, then the free field. */
export function composeExtraParams(checked: Record<string, boolean>, values: Record<string, string>, other: string): string {
  const parts: string[] = [];
  for (const param of LAUNCH_PARAMS) {
    if (!checked[param.key]) continue;
    if (!param.value) {
      parts.push(param.flag);
      continue;
    }
    const value = (values[param.key] ?? "").trim().replaceAll('"', "");
    if (value === "") continue;
    parts.push(param.value.quoted ? `${param.flag}"${value}"` : `${param.flag}${value}`);
  }
  if (other.trim() !== "") parts.push(other.trim());
  return parts.join(" ");
}

// Every section of the application menu of the desktops (the main categories
// of the freedesktop menu spec), so a webapp can be filed wherever it belongs,
// a photo editor under Graphics, for example. The order is the one of the
// list in the form; the first one is the default.
export const CATEGORIES = [
  "network",
  "office",
  "graphics",
  "audiovideo",
  "game",
  "education",
  "science",
  "development",
  "system",
  "settings",
  "utility",
] as const;

// Category ids sent to the .desktop file's Categories= key — freedesktop
// menu spec names, distinct from the i18n keys used to label them.
export const CATEGORY_FREEDESKTOP_ID: Record<(typeof CATEGORIES)[number], string> = {
  network: "Network",
  office: "Office",
  graphics: "Graphics",
  audiovideo: "AudioVideo",
  game: "Game",
  education: "Education",
  science: "Science",
  development: "Development",
  system: "System",
  settings: "Settings",
  utility: "Utility",
};
