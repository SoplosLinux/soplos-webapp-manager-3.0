# Changelog

All notable changes to Soplos WebApp Manager are documented in this file.

## [3.0.0-4-beta] - 2026-10-24

### Added

- Extensions for each WebApp. For Firefox-based browsers (Firefox, LibreWolf, Zen Browser and Midori, Flatpak or not) the form searches addons.mozilla.org by name, so the extension wanted is found without leaving the program, and it can also be added from an `.xpi` file. The extensions are installed in the profile of that WebApp when it is saved, and can be removed from the same list. For browsers based on Chrome the form has a button that opens the extension store of the browser in a normal window with the profile of the WebApp, and what is installed there works in the WebApp too.
- Launch parameters as checkboxes for the browsers based on Chrome: start maximized, dark mode, no notifications, no tab hover cards, window size, user agent, proxy and shared memory. The checkboxes and the text field are the same list: ticking a box writes its parameter in the field, and writing one in the field ticks its box. Anything else stays in the field.
- Every section of the application menu is now a category: Graphics, Education, Science, System and Settings join the six there were, so a photo editor can be filed under Graphics.
- The category can also be changed when editing a WebApp. The form shows the one it has; if it is left alone the categories are kept exactly as they were, and a value that is not in the list stays selectable instead of being replaced.
- Brave Origin is detected, as a browser based on Chrome.

### Fixed

- A WebApp of a Flatpak browser did not open: the browser said its profile was missing or inaccessible, because the sandbox of the Flatpak could not see the folder of the profile. The launch command now allows it for that run, without changing the settings of the Flatpak. WebApps created before this version have to be saved again to get the new command.
- Editing a WebApp of a Flatpak browser could show another browser, the first Flatpak one installed, and saving it changed the browser. The browser of a WebApp is now found by the whole name of the program or of the Flatpak, so a WebApp of Opera GX is no longer taken for one of Opera either.
- Midori was detected but its WebApps were launched without the address or the profile. It is now launched like the other Firefox-based browsers.
- In a dialog, scrolling past the end of its content no longer moves the window behind it.

## [3.0.0-3-beta] - 2026-10-23

### Added

- The window follows the light or dark theme of the desktop. It starts in the theme of the desktop and notices when it changes. The theme is read from the XDG portal and, when that says nothing, from each desktop: the colors of `kdeglobals` on Plasma, the name of the GTK theme on Xfce (a guess from words such as "dark") and the color scheme of GNOME. The button of the window still switches the theme and keeps what it chose; switching to the theme the desktop has goes back to following it.

## [3.0.0-2-beta] - 2026-10-22

### Added

- `CHANGELOG.md` is installed as `/usr/share/doc/soplos-webapp-manager/changelog.gz`.
- AppStream metainfo: categories and keywords in the eight languages, the same ones as the desktop entry.
- The Debian copyright now declares Lucide (ISC), used for the interface icons.

### Changed

- The manual page is installed compressed, as `/usr/share/man/man1/soplos-webapp-manager.1.gz`.

### Fixed

- AppStream metainfo: the developer and project group identifiers were `org.soploslinux`. They are now `org.soplos`, matching the soplos.org domain and the other Soplos apps. The developer name is now Sergi Perich, like the other apps, instead of Soplos Linux.
- Manual page: an em dash written as a raw character made groff report invalid input characters.

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
