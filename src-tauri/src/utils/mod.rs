use base64::Engine;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Detects whether the process is running inside a virtual machine.
///
/// WebKitGTK's DMA-BUF renderer can fail silently on virtual GPUs (VMware,
/// VirtualBox, QEMU/KVM, Hyper-V), leaving the window blank with no error.
/// main.rs uses this to decide whether to force
/// `WEBKIT_DISABLE_DMABUF_RENDERER=1` before the webview initializes. Ported
/// from soplos-welcome-3.0's utils::is_running_in_vm.
pub fn is_running_in_vm() -> bool {
    if detect_virt_via_dmi() {
        return true;
    }
    detect_virt_via_systemd()
}

fn detect_virt_via_dmi() -> bool {
    const NEEDLES: [&str; 6] = ["vmware", "virtualbox", "qemu", "kvm", "xen", "innotek"];

    for path in [
        "/sys/class/dmi/id/sys_vendor",
        "/sys/class/dmi/id/product_name",
        "/sys/class/dmi/id/bios_vendor",
    ] {
        if let Ok(content) = std::fs::read_to_string(path) {
            let content = content.to_lowercase();
            if NEEDLES.iter().any(|needle| content.contains(needle)) {
                return true;
            }
        }
    }
    false
}

fn detect_virt_via_systemd() -> bool {
    Command::new("systemd-detect-virt")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME is not set".to_string())
}

pub fn desktop_dir() -> Result<PathBuf, String> {
    let dir = home_dir()?.join(".local/share/applications");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn profiles_dir() -> Result<PathBuf, String> {
    let dir = home_dir()?.join(".local/share/soplos-webapps");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn icons_dir() -> Result<PathBuf, String> {
    let dir = profiles_dir()?.join("icons");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

// WebKitGTK's <img> tag can't load arbitrary filesystem paths without
// enabling and scoping Tauri's asset protocol, which is more surface than a
// small icon preview needs — a data: URI sidesteps that entirely. Returns
// None for a system icon name (no leading '/') or a file that no longer
// exists, so callers can fall back to a placeholder glyph.
pub fn icon_data_url(icon_path: &str) -> Option<String> {
    if icon_path.is_empty() || !icon_path.starts_with('/') {
        return None;
    }
    let path = Path::new(icon_path);
    let bytes = std::fs::read(path).ok()?;
    let mime = match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
        "svg" => "image/svg+xml",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        _ => "image/png",
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

// Same sanitization as the 2.x Python app: keep letters, digits, spaces,
// hyphens and underscores, then turn it into a slug.
pub fn safe_id(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect();
    let slug = cleaned.trim().replace(' ', "-").to_lowercase();
    let suffix = uuid::Uuid::new_v4().to_string()[..6].to_string();
    format!("{slug}-{suffix}")
}
