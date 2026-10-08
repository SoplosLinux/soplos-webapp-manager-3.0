// Browser detection, ported from 2.x's core/browser_manager.py.
// Builds the exact Exec command line each browser family needs to behave
// as a Site Specific Browser (SSB): its own profile dir, its own window
// class for desktop-shell integration, and an isolated instance.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone)]
pub struct Browser {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub executable: String,
    #[serde(rename = "isFlatpak")]
    pub is_flatpak: bool,
    #[serde(rename = "flatpakId")]
    pub flatpak_id: String,
}

struct NativeBrowser {
    id: &'static str,
    name: &'static str,
    bins: &'static [&'static str],
}

struct FlatpakBrowser {
    id: &'static str,
    name: &'static str,
    flatpak_id: &'static str,
}

// Every entry here is one Soplos already offers an install button for in
// Recommended.svelte, so a user is likely to actually have it. Chromium- and
// Firefox-based browsers share the same command-line interface as their
// upstream engine (--app/--user-data-dir/--class for any Chromium fork,
// -profile/-class/-no-remote for any Firefox-based one — see
// get_launch_command below), so widening this list is safe even where a
// binary name below turns out wrong for some future build: that's a missed
// detection, never a wrong launch command.
const NATIVE_BROWSERS: &[NativeBrowser] = &[
    NativeBrowser { id: "firefox", name: "Mozilla Firefox", bins: &["firefox", "firefox-esr"] },
    NativeBrowser { id: "librewolf", name: "LibreWolf", bins: &["librewolf"] },
    NativeBrowser { id: "zen", name: "Zen Browser", bins: &["zen", "zen-browser"] },
    NativeBrowser { id: "chromium", name: "Chromium", bins: &["chromium", "chromium-browser"] },
    NativeBrowser { id: "chrome", name: "Google Chrome", bins: &["google-chrome", "google-chrome-stable"] },
    NativeBrowser { id: "brave", name: "Brave", bins: &["brave", "brave-browser"] },
    NativeBrowser { id: "vivaldi", name: "Vivaldi", bins: &["vivaldi", "vivaldi-stable"] },
    NativeBrowser { id: "opera", name: "Opera", bins: &["opera", "opera-stable"] },
    NativeBrowser { id: "opera-gx", name: "Opera GX", bins: &["opera-gx"] },
    NativeBrowser { id: "edge", name: "Microsoft Edge", bins: &["microsoft-edge", "microsoft-edge-stable"] },
    NativeBrowser { id: "helium", name: "Helium", bins: &["helium", "helium-bin"] },
    NativeBrowser { id: "epiphany", name: "GNOME Web", bins: &["epiphany", "epiphany-browser"] },
    NativeBrowser { id: "midori", name: "Midori", bins: &["midori"] },
];

const FLATPAK_BROWSERS: &[FlatpakBrowser] = &[
    FlatpakBrowser { id: "firefox-flatpak", name: "Firefox (Flatpak)", flatpak_id: "org.mozilla.firefox" },
    FlatpakBrowser { id: "librewolf-flatpak", name: "LibreWolf (Flatpak)", flatpak_id: "io.gitlab.librewolf-community" },
    FlatpakBrowser { id: "chromium-flatpak", name: "Chromium (Flatpak)", flatpak_id: "org.chromium.Chromium" },
    FlatpakBrowser { id: "chrome-flatpak", name: "Google Chrome (Flatpak)", flatpak_id: "com.google.Chrome" },
    FlatpakBrowser { id: "brave-flatpak", name: "Brave (Flatpak)", flatpak_id: "com.brave.Browser" },
    FlatpakBrowser { id: "vivaldi-flatpak", name: "Vivaldi (Flatpak)", flatpak_id: "com.vivaldi.Vivaldi" },
    FlatpakBrowser { id: "opera-flatpak", name: "Opera (Flatpak)", flatpak_id: "com.opera.Opera" },
    FlatpakBrowser { id: "opera-gx-flatpak", name: "Opera GX (Flatpak)", flatpak_id: "com.opera.opera-gx" },
    FlatpakBrowser { id: "epiphany-flatpak", name: "GNOME Web (Flatpak)", flatpak_id: "org.gnome.Epiphany" },
];

fn which(bin: &str) -> Option<String> {
    std::env::var("PATH").ok()?.split(':').find_map(|dir| {
        let candidate = std::path::Path::new(dir).join(bin);
        candidate.is_file().then(|| candidate.to_string_lossy().to_string())
    })
}

#[tauri::command]
pub async fn list_browsers() -> Vec<Browser> {
    tauri::async_runtime::spawn_blocking(list_browsers_blocking).await.unwrap_or_default()
}

// Separate from the Tauri command above so internal callers (webapps.rs,
// already running off the main thread inside their own spawn_blocking) can
// call this directly instead of going through another async dispatch.
pub fn list_browsers_blocking() -> Vec<Browser> {
    let mut browsers = Vec::new();

    for b in NATIVE_BROWSERS {
        if let Some(executable) = b.bins.iter().find_map(|bin| which(bin)) {
            browsers.push(Browser {
                id: b.id.to_string(),
                display_name: b.name.to_string(),
                executable,
                is_flatpak: false,
                flatpak_id: String::new(),
            });
        }
    }

    if which("flatpak").is_some() {
        if let Ok(output) = Command::new("flatpak")
            .args(["list", "--app", "--columns=application"])
            .output()
        {
            let installed = String::from_utf8_lossy(&output.stdout);
            for fb in FLATPAK_BROWSERS {
                if installed.lines().any(|l| l.trim() == fb.flatpak_id) {
                    browsers.push(Browser {
                        id: fb.id.to_string(),
                        display_name: fb.name.to_string(),
                        executable: "flatpak".to_string(),
                        is_flatpak: true,
                        flatpak_id: fb.flatpak_id.to_string(),
                    });
                }
            }
        }
    }

    browsers
}

// Real (non-flatpak) id used by get_launch_command's family match, e.g.
// "chrome-flatpak" still behaves like "chrome".
fn family_id(browser_id: &str) -> &str {
    browser_id.strip_suffix("-flatpak").unwrap_or(browser_id)
}

pub fn get_launch_command(
    browser: &Browser,
    url: &str,
    profile_path: &str,
    class_name: &str,
    show_navbar: bool,
    extra_params: &str,
    is_incognito: bool,
) -> String {
    let family = family_id(&browser.id);
    let mut cmd: Vec<String> = Vec::new();

    if browser.is_flatpak {
        cmd.extend(["flatpak".to_string(), "run".to_string(), browser.flatpak_id.clone()]);
    } else {
        cmd.push(browser.executable.clone());
    }

    match family {
        "chromium" | "chrome" | "brave" | "vivaldi" | "edge" | "opera" | "opera-gx" | "helium" => {
            cmd.insert(0, format!("env CHROME_DESKTOP={class_name}.desktop"));
            cmd.push("--ozone-platform-hint=auto".to_string());
            cmd.push(format!("--wayland-app-id={class_name}"));
            cmd.push(format!("--class={class_name}"));
            cmd.push(format!("--name={class_name}"));
            cmd.push(format!("--wm-class={class_name}"));
            cmd.push(format!("--user-data-dir={profile_path}"));
            if is_incognito {
                cmd.push("--incognito".to_string());
            }
            if !extra_params.is_empty() {
                cmd.push(extra_params.to_string());
            }
            cmd.push(format!("--app={url}"));
            let _ = show_navbar; // Chrome family has no navbar toggle, unlike Firefox.
        }
        "firefox" | "librewolf" | "zen" => {
            cmd.insert(0, format!("env MOZ_APP_REMOTINGNAME={class_name}"));
            cmd.push("-no-remote".to_string());
            cmd.push("-new-instance".to_string());
            cmd.push("-profile".to_string());
            cmd.push(format!("\"{profile_path}\""));
            cmd.push("-class".to_string());
            cmd.push(class_name.to_string());
            cmd.push("-name".to_string());
            cmd.push(class_name.to_string());
            if is_incognito {
                cmd.push("-private-window".to_string());
            }
            if !extra_params.is_empty() {
                cmd.push(extra_params.to_string());
            }
            cmd.push(url.to_string());
        }
        "epiphany" => {
            cmd.push("--application-mode".to_string());
            cmd.push(format!("--profile={profile_path}"));
            if is_incognito {
                cmd.push("--incognito-mode".to_string());
            }
            if !extra_params.is_empty() {
                cmd.push(extra_params.to_string());
            }
            cmd.push(url.to_string());
        }
        _ => {}
    }

    cmd.join(" ")
}
