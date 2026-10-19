# Changelog

All notable changes to Soplos WebApp Manager are documented in this file.

## [3.0.0-1-beta] - 2026-10-19

### Added

- Unix manual page `soplos-webapp-manager(1)`, installed to `/usr/share/man/man1/`.
- Desktop entry translated to 8 languages (en, es, fr, de, pt, it, ro, ru): `GenericName`, `Comment` and `Keywords`.
- AppStream metainfo translated to 8 languages: `name`, `summary`, `description` and screenshot captions.

### Changed

- README updated with badges, architecture tree, keyboard shortcuts table, `man` usage and build instructions.
- CHANGELOG updated with this entry.

## [3.0.0-beta] - 2026-10-02

First beta of Soplos WebApp Manager 3.0.

### Added

- Hero banner with the app's identity, language switcher and the Add/Remove controls, replacing a plain toolbar header.
- Keyboard shortcuts: F1 (About), Ctrl+N (Add WebApp), Delete (remove the selected WebApp), Ctrl+Q (quit).
- Drag and drop: dropping a link from the browser onto the window opens the Add WebApp dialog with the URL and a suggested name already filled in, and starts the favicon lookup automatically.
- Browser detection widened well beyond the original app: LibreWolf, Zen Browser, Opera, Opera GX and Helium are now detected (native and Flatpak) alongside Firefox, Chromium, Google Chrome, Brave, Vivaldi, Microsoft Edge and GNOME Web.
- Automatic detection of virtual machines to work around a WebKitGTK rendering bug that otherwise leaves the window blank with no error.
- Launching the app again while it's already open focuses the existing window instead of opening a second one.

### Changed

- Icons are read back as `data:` URIs instead of filesystem paths, so they display without granting the window access to arbitrary files.
- Popup dialogs now cap their height and scroll internally, so a long one (like the browser parameters help) never grows past the window.
- New app icon.

### Fixed

- Selecting a custom icon froze the whole window: `pick_icon_file` ran the native file dialog directly on the main GTK thread instead of off it. This, and 6 other commands with the same issue (listing, creating, updating, deleting and running a WebApp), now run through `spawn_blocking`.
- Favicon download (`download_favicon`) had no HTTP timeouts at all, so an unresponsive DuckDuckGo/Google icon service could freeze the app indefinitely instead of failing after a few seconds.

## [3.0.0-alpha] - 2026-09-29

Full architecture rewrite of Soplos WebApp Manager, from Python 3 + GTK3 to Tauri 2. See the Architecture section of the README for details.

### Added

- Rust backend (`src-tauri/`) replacing the Python 3 + GTK3 codebase: WebApp creation, editing, deletion and launching; browser detection (native and Flatpak); favicon download with a DuckDuckGo/Google fallback chain; a native file picker for manually choosing an icon.
- Svelte 4 + Tailwind CSS frontend (`src/`) replacing the GTK3 UI, with a shared popup component (info, confirm, warning and danger variants) for every dialog, including the Add/Edit WebApp form and the delete confirmation.
- Interface in eight languages: English, Spanish, German, French, Italian, Portuguese, Romanian and Russian. It starts in the system language and can be changed from the interface.
- Soplos's own light/dark theme, independent of the desktop environment's GTK/Qt theme.
