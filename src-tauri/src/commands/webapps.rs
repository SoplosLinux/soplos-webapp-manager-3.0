// WebApp CRUD, ported from 2.x's core/webapp_manager.py. A WebApp is just a
// soplos-webapp-*.desktop file plus an isolated profile directory under
// ~/.local/share/soplos-webapps/ — nothing here needs root, so there is no
// pkexec involved anywhere in this module.

use crate::commands::browsers::{get_launch_command, list_browsers_blocking, Browser};
use crate::utils::{desktop_dir, icon_data_url, icons_dir, profiles_dir, safe_id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize, Clone)]
pub struct WebApp {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(rename = "browserId")]
    pub browser_id: String,
    #[serde(rename = "iconPath")]
    pub icon_path: String,
    // Ready-to-use data: URI for <img src>, or null for a system icon name
    // (the frontend falls back to a placeholder glyph in that case).
    #[serde(rename = "iconPreview")]
    pub icon_preview: Option<String>,
    #[serde(rename = "showNavbar")]
    pub show_navbar: bool,
    #[serde(rename = "extraParams")]
    pub extra_params: String,
    #[serde(rename = "isIncognito")]
    pub is_incognito: bool,
}

#[derive(Deserialize)]
pub struct WebAppInput {
    pub name: String,
    pub url: String,
    #[serde(rename = "iconPath")]
    pub icon_path: String,
    pub category: String,
    #[serde(rename = "browserId")]
    pub browser_id: String,
    #[serde(rename = "showNavbar")]
    pub show_navbar: bool,
    #[serde(rename = "extraParams")]
    pub extra_params: String,
    #[serde(rename = "isIncognito")]
    pub is_incognito: bool,
}

// Minimal .desktop reader — create_webapp/update_webapp always rewrite the
// file from scratch (like the 2.x code's create path did), so this only
// needs to read keys back for list_webapps and run_webapp.
struct DesktopEntry {
    values: HashMap<String, String>,
}

impl DesktopEntry {
    fn parse(content: &str) -> Option<Self> {
        let mut in_section = false;
        let mut values = HashMap::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[Desktop Entry]" {
                in_section = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_section = false;
                continue;
            }
            if !in_section || trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                values.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        if values.is_empty() { None } else { Some(Self { values }) }
    }

    fn get(&self, key: &str) -> String {
        self.values.get(key).cloned().unwrap_or_default()
    }
}

fn extract_url(exec_cmd: &str) -> String {
    for part in exec_cmd.split_whitespace() {
        if part.starts_with("http://") || part.starts_with("https://") {
            return part.to_string();
        }
        if let Some(v) = part.strip_prefix("--app=") {
            return v.to_string();
        }
    }
    String::new()
}

fn find_browser_id(exec_cmd: &str, browsers: &[Browser]) -> String {
    for b in browsers {
        let matches = (!b.executable.is_empty() && exec_cmd.contains(&b.executable))
            || (b.is_flatpak && exec_cmd.contains(&b.flatpak_id));
        if matches {
            return b.id.clone();
        }
    }
    "unknown".to_string()
}

#[tauri::command]
pub async fn list_webapps() -> Result<Vec<WebApp>, String> {
    tauri::async_runtime::spawn_blocking(list_webapps_blocking).await.map_err(|e| e.to_string())?
}

fn list_webapps_blocking() -> Result<Vec<WebApp>, String> {
    let dir = desktop_dir()?;
    let browsers = list_browsers_blocking();
    let mut webapps = Vec::new();

    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let Some(id) = stem.strip_prefix("soplos-webapp-") else { continue };
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else { continue };
        let Some(desktop) = DesktopEntry::parse(&content) else { continue };

        let exec_cmd = desktop.get("Exec");
        let icon_path = desktop.get("Icon");
        webapps.push(WebApp {
            id: id.to_string(),
            name: desktop.get("Name"),
            url: extract_url(&exec_cmd),
            browser_id: find_browser_id(&exec_cmd, &browsers),
            icon_preview: icon_data_url(&icon_path),
            icon_path,
            show_navbar: desktop.get("X-Soplos-Navbar") == "true",
            extra_params: desktop.get("X-Soplos-ExtraParams"),
            is_incognito: desktop.get("X-Soplos-Incognito") == "true",
        });
    }

    webapps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(webapps)
}

// Copies an arbitrary icon file into the persistent icons directory so it
// survives cache cleans. Returns it unchanged if it's a system icon name
// (no leading '/') or already stored there.
fn persist_icon(id: &str, icon_path: &str) -> String {
    if icon_path.is_empty() || !icon_path.starts_with('/') {
        return icon_path.to_string();
    }
    let src = Path::new(icon_path);
    if !src.exists() {
        return icon_path.to_string();
    }
    let Ok(dir) = icons_dir() else { return icon_path.to_string() };
    if src.parent() == Some(dir.as_path()) {
        return icon_path.to_string();
    }
    let suffix = src.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let dest = dir.join(format!("{id}.{suffix}"));
    match fs::copy(src, &dest) {
        Ok(_) => dest.to_string_lossy().to_string(),
        Err(_) => icon_path.to_string(),
    }
}

// KDE Plasma 6 on Wayland often ignores --wayland-app-id and instead looks
// for a .desktop matching Chrome's own internal id for the site
// ("chrome-host.com__-Default"). A symlink under that name pointing at our
// real .desktop file fixes taskbar grouping and icons there.
fn chrome_compat_id(url: &str) -> Option<String> {
    let host = url.split_once("://").map(|(_, rest)| rest)?.split(['/', '?', '#']).next()?;
    if host.is_empty() {
        return None;
    }
    Some(format!("chrome-{host}__-Default"))
}

fn manage_chrome_compat_links(dir: &Path, id: &str, url: &str, browser_id: &str) {
    let family = browser_id.strip_suffix("-flatpak").unwrap_or(browser_id);
    if !["chrome", "chromium", "brave", "vivaldi", "edge", "opera", "opera-gx", "helium"].contains(&family) {
        return;
    }
    let Some(chrome_id) = chrome_compat_id(url) else { return };

    // Remove any existing compat symlink for this webapp first.
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("chrome-")) != Some(true) {
                continue;
            }
            if let Ok(target) = fs::read_link(&path) {
                if target.to_string_lossy().ends_with(&format!("soplos-webapp-{id}.desktop")) {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    let compat_path = dir.join(format!("{chrome_id}.desktop"));
    let target_name = format!("soplos-webapp-{id}.desktop");
    if dir.join(&target_name).exists() && !compat_path.exists() {
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&target_name, &compat_path);
    }
}

fn setup_firefox_profile(profile_path: &Path, show_navbar: bool) -> std::io::Result<()> {
    let chrome_dir = profile_path.join("chrome");
    fs::create_dir_all(&chrome_dir)?;

    let user_js = r#"
// Enable custom CSS
user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);
// Disable default browser checks
user_pref("browser.shell.checkDefaultBrowser", false);
user_pref("browser.tabs.warnOnClose", false);
// Session restore and privacy
user_pref("browser.startup.page", 3);
// Block telemetry
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);
// Force system titlebar to show window controls (since tabs are hidden)
user_pref("browser.tabs.drawInTitlebar", false);
user_pref("browser.tabs.inTitlebar", 0);
"#;
    fs::write(profile_path.join("user.js"), user_js)?;

    let mut user_chrome = String::from(
        r#"
@namespace url("http://www.mozilla.org/keymaster/gatekeeper/there.is.only.xul");
/* Hide tabs bar */
#TabsToolbar { visibility: collapse !important; }
/* Hide sidebar header */
#sidebar-header { visibility: collapse !important; }
"#,
    );
    if !show_navbar {
        user_chrome.push_str(
            r#"
/* Hide Navigation bar */
#nav-bar { visibility: collapse !important; }
/* Hide Bookmarks bar */
#PersonalToolbar { visibility: collapse !important; }
"#,
        );
    }
    fs::write(chrome_dir.join("userChrome.css"), user_chrome)
}

fn update_desktop_database() {
    let _ = std::process::Command::new("update-desktop-database")
        .arg(desktop_dir().unwrap_or_default())
        .output();
}

#[tauri::command]
pub async fn create_webapp(input: WebAppInput) -> Result<WebApp, String> {
    tauri::async_runtime::spawn_blocking(move || create_webapp_blocking(input)).await.map_err(|e| e.to_string())?
}

fn create_webapp_blocking(input: WebAppInput) -> Result<WebApp, String> {
    let browsers = list_browsers_blocking();
    let browser = browsers
        .iter()
        .find(|b| b.id == input.browser_id)
        .ok_or_else(|| format!("Browser {} not found", input.browser_id))?;

    let id = safe_id(&input.name);
    let profile_path = profiles_dir()?.join(&id);
    fs::create_dir_all(&profile_path).map_err(|e| e.to_string())?;

    let family = browser.id.strip_suffix("-flatpak").unwrap_or(&browser.id);
    if family == "firefox" || family == "librewolf" || family == "zen" {
        setup_firefox_profile(&profile_path, input.show_navbar).map_err(|e| e.to_string())?;
    }

    let class_name = format!("soplos-webapp-{id}");
    let startup_class = if ["chrome", "chromium", "brave", "vivaldi", "edge", "opera", "opera-gx", "helium"].contains(&family) {
        chrome_compat_id(&input.url).unwrap_or_else(|| class_name.clone())
    } else {
        class_name.clone()
    };

    let exec_cmd = get_launch_command(
        browser,
        &input.url,
        &profile_path.to_string_lossy(),
        &class_name,
        input.show_navbar,
        &input.extra_params,
        input.is_incognito,
    );

    let persistent_icon = persist_icon(&id, &input.icon_path);
    let dir = desktop_dir()?;
    let desktop_path = dir.join(format!("soplos-webapp-{id}.desktop"));

    let content = format!(
        "[Desktop Entry]\nVersion=1.0\nName={name}\nComment=Soplos WebApp for {name}\nExec={exec}\nTerminal=false\nX-MultipleArgs=false\nType=Application\nIcon={icon}\nCategories={category};\nStartupWMClass={startup_class}\nStartupNotify=true\nX-Soplos-Navbar={navbar}\nX-Soplos-ExtraParams={extra}\nX-Soplos-Incognito={incognito}\n",
        name = input.name,
        exec = exec_cmd,
        icon = persistent_icon,
        category = input.category,
        navbar = input.show_navbar,
        extra = input.extra_params,
        incognito = input.is_incognito,
    );
    fs::write(&desktop_path, content).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&desktop_path, fs::Permissions::from_mode(0o755));
    }

    manage_chrome_compat_links(&dir, &id, &input.url, &browser.id);
    update_desktop_database();

    Ok(WebApp {
        id,
        name: input.name,
        url: input.url,
        browser_id: input.browser_id,
        icon_preview: icon_data_url(&persistent_icon),
        icon_path: persistent_icon,
        show_navbar: input.show_navbar,
        extra_params: input.extra_params,
        is_incognito: input.is_incognito,
    })
}

#[tauri::command]
pub async fn update_webapp(id: String, input: WebAppInput) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || update_webapp_blocking(id, input)).await.map_err(|e| e.to_string())?
}

fn update_webapp_blocking(id: String, input: WebAppInput) -> Result<bool, String> {
    let dir = desktop_dir()?;
    let desktop_path = dir.join(format!("soplos-webapp-{id}.desktop"));
    if !desktop_path.exists() {
        return Ok(false);
    }

    // The edit form doesn't offer a category selector (same as 2.x), so an
    // empty category here means "keep whatever this webapp already has".
    let category = if input.category.is_empty() {
        let existing = fs::read_to_string(&desktop_path).ok().and_then(|c| DesktopEntry::parse(&c).map(|d| d.get("Categories")));
        existing.unwrap_or_default().trim_end_matches(';').to_string()
    } else {
        input.category.clone()
    };

    let browsers = list_browsers_blocking();
    let browser = browsers
        .iter()
        .find(|b| b.id == input.browser_id)
        .ok_or_else(|| format!("Browser {} not found", input.browser_id))?;

    let family = browser.id.strip_suffix("-flatpak").unwrap_or(&browser.id);
    let profile_path = profiles_dir()?.join(&id);
    if family == "firefox" || family == "librewolf" || family == "zen" {
        setup_firefox_profile(&profile_path, input.show_navbar).map_err(|e| e.to_string())?;
    }

    let class_name = format!("soplos-webapp-{id}");
    let startup_class = if ["chrome", "chromium", "brave", "vivaldi", "edge", "opera", "opera-gx", "helium"].contains(&family) {
        chrome_compat_id(&input.url).unwrap_or_else(|| class_name.clone())
    } else {
        class_name.clone()
    };

    let exec_cmd = get_launch_command(
        browser,
        &input.url,
        &profile_path.to_string_lossy(),
        &class_name,
        input.show_navbar,
        &input.extra_params,
        input.is_incognito,
    );

    let persistent_icon = persist_icon(&id, &input.icon_path);

    let content = format!(
        "[Desktop Entry]\nVersion=1.0\nName={name}\nComment=Soplos WebApp for {name}\nExec={exec}\nTerminal=false\nX-MultipleArgs=false\nType=Application\nIcon={icon}\nCategories={category};\nStartupWMClass={startup_class}\nStartupNotify=true\nX-Soplos-Navbar={navbar}\nX-Soplos-ExtraParams={extra}\nX-Soplos-Incognito={incognito}\n",
        name = input.name,
        exec = exec_cmd,
        icon = persistent_icon,
        category = category,
        navbar = input.show_navbar,
        extra = input.extra_params,
        incognito = input.is_incognito,
    );
    fs::write(&desktop_path, content).map_err(|e| e.to_string())?;

    manage_chrome_compat_links(&dir, &id, &input.url, &browser.id);
    update_desktop_database();
    Ok(true)
}

#[tauri::command]
pub async fn delete_webapp(id: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || delete_webapp_blocking(id)).await.map_err(|e| e.to_string())?
}

fn delete_webapp_blocking(id: String) -> Result<bool, String> {
    let dir = desktop_dir()?;
    let desktop_path = dir.join(format!("soplos-webapp-{id}.desktop"));
    let mut success = true;

    if desktop_path.exists() {
        if fs::remove_file(&desktop_path).is_err() {
            success = false;
        }
    }

    // Compatibility symlinks pointing at this webapp.
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("chrome-")) != Some(true) {
                continue;
            }
            if let Ok(target) = fs::read_link(&path) {
                if target.to_string_lossy().ends_with(&format!("soplos-webapp-{id}.desktop")) {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    let profile_path = profiles_dir()?.join(&id);
    if profile_path.exists() && fs::remove_dir_all(&profile_path).is_err() {
        success = false;
    }

    if let Ok(dir) = icons_dir() {
        for ext in ["png", "jpg", "jpeg", "svg", "ico", "webp"] {
            let icon_file = dir.join(format!("{id}.{ext}"));
            if icon_file.exists() {
                let _ = fs::remove_file(icon_file);
                break;
            }
        }
    }

    update_desktop_database();
    Ok(success)
}

// Used by the add/edit form to preview a just-picked or just-downloaded
// icon before the webapp is actually saved.
#[tauri::command]
pub async fn read_icon_preview(path: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || icon_data_url(&path)).await.unwrap_or(None)
}

// The confirmed cause of the reported hang: this used to be a plain `fn`
// (not `async`), and non-async Tauri commands run on the main GTK thread —
// blocking_pick_file() there waits for the user to interact with the
// native dialog while the whole UI can't process any other events, which
// reads as the app having frozen/crashed. spawn_blocking moves that wait
// off the main thread, same pattern as every other file-picker command in
// the other three Soplos Tauri ports.
#[tauri::command]
pub async fn pick_icon_file(app: tauri::AppHandle) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Select an icon")
            .add_filter("Images", &["png", "svg", "jpg", "jpeg", "ico", "webp"])
            .blocking_pick_file()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .unwrap_or(None)
}

#[tauri::command]
pub async fn run_webapp(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || run_webapp_blocking(id)).await.map_err(|e| e.to_string())?
}

fn run_webapp_blocking(id: String) -> Result<(), String> {
    let dir = desktop_dir()?;
    let desktop_path = dir.join(format!("soplos-webapp-{id}.desktop"));
    let content = fs::read_to_string(&desktop_path).map_err(|e| e.to_string())?;
    let desktop = DesktopEntry::parse(&content).ok_or("Invalid .desktop file")?;
    let exec_cmd = desktop.get("Exec");
    if exec_cmd.is_empty() {
        return Err("Empty Exec line".to_string());
    }
    std::process::Command::new("sh")
        .arg("-c")
        .arg(&exec_cmd)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
