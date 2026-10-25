// Browser extensions of a webapp, one list for each, whichever the user wants.
// Only for Firefox-based browsers: an extension is a signed .xpi copied into
// the extensions folder of the webapp's own profile, named after the add-on
// ID, which Firefox finds when it starts (user.js, written by webapps.rs, lets
// it use them without asking). It comes from the link of its page in
// addons.mozilla.org or from a file; its ID and its name are read from the
// manifest inside, so nothing about any extension is known beforehand. The
// list is also kept in a small file of the profile, because the ID alone says
// nothing to a person. Deleting the webapp deletes its profile, and the
// extensions with it.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri_plugin_dialog::DialogExt;

use crate::utils::profiles_dir;

const MAX_XPI_BYTES: u64 = 64 * 1024 * 1024;
const LIST_FILE: &str = "soplos-extensions.json";
const AMO: &str = "https://addons.mozilla.org/";

#[derive(Serialize, Deserialize, Clone)]
pub struct WebAppExtension {
    pub id: String,
    pub name: String,
}

/// The folder of the profile of a webapp that exists.
fn profile_of(webapp_id: &str) -> Result<PathBuf, String> {
    if webapp_id.is_empty() || webapp_id.starts_with('.') || webapp_id.contains(['/', '\\', '\0']) {
        return Err("Not a webapp".to_string());
    }
    let dir = profiles_dir()?.join(webapp_id);
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err("The webapp has no profile".to_string())
    }
}

// The ID becomes a file name, so it may only hold what add-on IDs hold.
fn valid_addon_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && !id.contains("..") && id.chars().all(|c| c.is_ascii_alphanumeric() || "@._{}-".contains(c))
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.len() <= 100 && slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// The name an extension has in addons.mozilla.org, from the link of its page
/// (https://addons.mozilla.org/en-US/firefox/addon/<name>/) or from the name
/// itself. Links to any other site are refused: the extension is downloaded
/// from the address built here, never from one the user typed.
fn slug_from_input(input: &str) -> Option<String> {
    let text = input.trim();
    let slug = if text.contains("://") {
        let rest = text.strip_prefix(AMO)?;
        let after = rest.split("/addon/").nth(1)?;
        after.split(['/', '?', '#']).next()?
    } else {
        text
    };
    valid_slug(slug).then(|| slug.to_string())
}

fn read_list(profile: &Path) -> Vec<WebAppExtension> {
    fs::read_to_string(profile.join(LIST_FILE)).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn write_list(profile: &Path, list: &[WebAppExtension]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    fs::write(profile.join(LIST_FILE), text).map_err(|e| e.to_string())
}

fn extension_file(profile: &Path, id: &str) -> PathBuf {
    profile.join("extensions").join(format!("{id}.xpi"))
}

/// Copies the extension into the profile and adds it to the list.
fn install(profile: &Path, extension: WebAppExtension, bytes: &[u8]) -> Result<WebAppExtension, String> {
    // An .xpi is a zip file.
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err("The file is not a Firefox extension".to_string());
    }
    if !valid_addon_id(&extension.id) {
        return Err("The extension has no valid ID".to_string());
    }
    let target = extension_file(profile, &extension.id);
    let folder = target.parent().ok_or("No folder")?;
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    // Written whole and then renamed, so Firefox never finds half a file.
    let partial = folder.join(format!(".{}.part", extension.id));
    fs::write(&partial, bytes).map_err(|e| e.to_string())?;
    fs::rename(&partial, &target).map_err(|e| e.to_string())?;

    let mut list = read_list(profile);
    list.retain(|item| item.id != extension.id);
    list.push(extension.clone());
    write_list(profile, &list)?;
    Ok(extension)
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let mut response = ureq::get(url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
        .config()
        .timeout_resolve(Some(Duration::from_secs(5)))
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .call()
        .map_err(|e| e.to_string())?;
    response.body_mut().with_config().limit(MAX_XPI_BYTES).read_to_vec().map_err(|e| e.to_string())
}

/// The add-on ID and the name an .xpi says in its manifest.
fn manifest_info(bytes: &[u8], fallback_name: &str) -> Result<(String, String), String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| "The file is not a Firefox extension".to_string())?;
    let mut manifest = archive.by_name("manifest.json").map_err(|_| "The extension has no manifest".to_string())?;
    let mut text = String::new();
    manifest.read_to_string(&mut text).map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    let id = json["browser_specific_settings"]["gecko"]["id"]
        .as_str()
        .or_else(|| json["applications"]["gecko"]["id"].as_str())
        .ok_or("The extension does not say its add-on ID, so it cannot be installed")?
        .to_string();
    // A name such as "__MSG_extensionName__" is a key of the extension's own
    // translations, not something to show.
    let name = json["name"].as_str().filter(|name| !name.starts_with("__MSG_")).unwrap_or(fallback_name).to_string();
    Ok((id, name))
}

/// One extension found by a search in addons.mozilla.org.
#[derive(Serialize)]
pub struct ExtensionResult {
    pub slug: String,
    pub name: String,
    pub summary: String,
    #[serde(rename = "iconUrl")]
    pub icon_url: String,
    pub users: u64,
}

// The language codes of addons.mozilla.org for the languages of the window.
fn amo_language(lang: &str) -> &'static str {
    match lang {
        "es" => "es",
        "fr" => "fr",
        "de" => "de",
        "pt" => "pt-PT",
        "it" => "it",
        "ro" => "ro",
        "ru" => "ru",
        _ => "en-US",
    }
}

// A text of the answer: a plain string, or one for each language.
fn localized(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Object(map) => map.values().find_map(|item| item.as_str()).unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// Searches the extensions of addons.mozilla.org by name, so the user finds
/// the one wanted without leaving the program.
#[tauri::command]
pub async fn search_extensions(query: String, lang: String) -> Result<Vec<ExtensionResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let query = query.trim().to_string();
        if query.is_empty() || query.chars().count() > 100 {
            return Ok(Vec::new());
        }
        let mut response = ureq::get(&format!("{AMO}api/v5/addons/search/"))
            .query("q", &query)
            .query("type", "extension")
            .query("app", "firefox")
            .query("page_size", "10")
            .query("lang", amo_language(&lang))
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .config()
            .timeout_resolve(Some(Duration::from_secs(5)))
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(Duration::from_secs(20)))
            .build()
            .call()
            .map_err(|e| e.to_string())?;
        let text = response.body_mut().with_config().limit(4 * 1024 * 1024).read_to_string().map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let results = json["results"]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|item| {
                        let slug = item["slug"].as_str().filter(|slug| valid_slug(slug))?;
                        Some(ExtensionResult {
                            slug: slug.to_string(),
                            name: localized(&item["name"]),
                            summary: localized(&item["summary"]),
                            icon_url: item["icon_url"].as_str().filter(|url| url.starts_with("https://")).unwrap_or("").to_string(),
                            users: item["average_daily_users"].as_u64().unwrap_or(0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The extensions of a webapp whose file is still there.
#[tauri::command]
pub async fn list_webapp_extensions(webapp_id: String) -> Result<Vec<WebAppExtension>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = profile_of(&webapp_id)?;
        Ok(read_list(&profile).into_iter().filter(|item| valid_addon_id(&item.id) && extension_file(&profile, &item.id).is_file()).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Downloads the latest version of the extension of an addons.mozilla.org link.
#[tauri::command]
pub async fn add_url_extension(webapp_id: String, link: String) -> Result<WebAppExtension, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = profile_of(&webapp_id)?;
        let slug = slug_from_input(&link).ok_or("Not a link to an extension of addons.mozilla.org")?;
        let bytes = download(&format!("{AMO}firefox/downloads/latest/{slug}/latest.xpi"))?;
        let (id, name) = manifest_info(&bytes, &slug)?;
        install(&profile, WebAppExtension { id, name }, &bytes)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_local_extension(webapp_id: String, path: String) -> Result<WebAppExtension, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = profile_of(&webapp_id)?;
        let file = Path::new(&path);
        let meta = fs::metadata(file).map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > MAX_XPI_BYTES {
            return Err("Not an extension file".to_string());
        }
        let bytes = fs::read(file).map_err(|e| e.to_string())?;
        let fallback = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let (id, name) = manifest_info(&bytes, &fallback)?;
        install(&profile, WebAppExtension { id, name }, &bytes)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_webapp_extension(webapp_id: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = profile_of(&webapp_id)?;
        if !valid_addon_id(&id) {
            return Err("Not an extension".to_string());
        }
        match fs::remove_file(extension_file(&profile, &id)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        let mut list = read_list(&profile);
        list.retain(|item| item.id != id);
        write_list(&profile, &list)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The file chosen in the dialog, for "add from a file".
#[tauri::command]
pub async fn pick_extension_file(app: tauri::AppHandle) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Select an extension")
            .add_filter("Firefox extensions", &["xpi"])
            .blocking_pick_file()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .unwrap_or(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_addon_id_is_what_can_name_a_file() {
        for id in ["uBlock0@raymondhill.net", "{446900e4-71c2-419f-a6a7-df9c091e268b}", "addon@darkreader.org"] {
            assert!(valid_addon_id(id), "{id}");
        }
        for id in ["", "../evil", "a/b", "a b", "a\\b", "..", "x;rm"] {
            assert!(!valid_addon_id(id), "{id}");
        }
    }

    #[test]
    fn the_name_of_an_extension_comes_from_its_amo_link_or_by_itself() {
        let cases = [
            ("https://addons.mozilla.org/en-US/firefox/addon/ublock-origin/", Some("ublock-origin")),
            ("https://addons.mozilla.org/es/firefox/addon/darkreader/?utm_source=x", Some("darkreader")),
            ("https://addons.mozilla.org/firefox/addon/sponsorblock#reviews", Some("sponsorblock")),
            ("  ublock-origin  ", Some("ublock-origin")),
            ("https://evil.example/addon/ublock-origin/", None),
            ("http://addons.mozilla.org/en-US/firefox/addon/x/", None),
            ("https://addons.mozilla.org/en-US/firefox/", None),
            ("https://addons.mozilla.org/en-US/firefox/addon//", None),
            ("a/b", None),
            ("", None),
        ];
        for (input, expected) in cases {
            assert_eq!(slug_from_input(input).as_deref(), expected, "{input}");
        }
    }

    #[test]
    fn the_texts_of_a_search_come_as_a_string_or_one_for_each_language() {
        assert_eq!(localized(&serde_json::json!("uBlock")), "uBlock");
        assert_eq!(localized(&serde_json::json!({"en-US": "uBlock"})), "uBlock");
        assert_eq!(localized(&serde_json::json!(null)), "");
        assert_eq!(amo_language("pt"), "pt-PT");
        assert_eq!(amo_language("xx"), "en-US");
    }

    #[test]
    fn only_zip_files_are_installed() {
        let dir = std::env::temp_dir().join(format!("soplos-ext-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let extension = WebAppExtension { id: "a@b.c".to_string(), name: "A".to_string() };
        assert!(install(&dir, extension.clone(), b"not a zip").is_err());
        assert!(install(&dir, extension.clone(), b"PK\x03\x04rest").is_ok());
        assert!(dir.join("extensions").join("a@b.c.xpi").is_file());
        assert_eq!(read_list(&dir).len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
