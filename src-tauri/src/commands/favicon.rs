// Favicon download, ported from 2.x's utils/favicon.py: try DuckDuckGo's
// icon proxy for the full domain, then its root domain, then fall back to
// Google's s2 favicon service.

use crate::utils::icons_dir;

fn root_domain(domain: &str) -> String {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2..].join(".")
    } else {
        domain.to_string()
    }
}

// Not async itself — this used to be a plain `fn` with no timeout anywhere
// on the HTTP calls at all. Non-async Tauri commands run on the main GTK
// thread, so an unresponsive icon server (DuckDuckGo or Google down, or
// just a stalled DNS lookup) could freeze the whole app indefinitely, not
// just for a few seconds. spawn_blocking moves the wait off the main
// thread, and explicit timeouts (including DNS resolution, which
// timeout_connect alone does not cover) bound it to a few seconds even in
// the worst case.
#[tauri::command]
pub async fn download_favicon(url: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || download_favicon_blocking(url)).await.map_err(|e| e.to_string())?
}

fn download_favicon_blocking(url: String) -> Result<Option<String>, String> {
    let with_scheme = if url.contains("://") { url } else { format!("http://{url}") };
    let parsed = url::parse_host(&with_scheme);
    let Some(domain) = parsed else { return Ok(None) };

    let root = root_domain(&domain);
    let candidates = [
        format!("https://icons.duckduckgo.com/ip3/{domain}.ico"),
        format!("https://icons.duckduckgo.com/ip3/{root}.ico"),
        format!("https://www.google.com/s2/favicons?domain={domain}&sz=128"),
    ];

    let dir = icons_dir()?;
    let dest = dir.join(format!("favicon-{domain}.png"));

    for candidate in candidates {
        let result = ureq::get(&candidate)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .config()
            .timeout_resolve(Some(std::time::Duration::from_secs(5)))
            .timeout_connect(Some(std::time::Duration::from_secs(5)))
            .timeout_recv_body(Some(std::time::Duration::from_secs(10)))
            .build()
            .call();
        if let Ok(mut response) = result {
            if let Ok(bytes) = response.body_mut().read_to_vec() {
                if !bytes.is_empty() {
                    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
                    return Ok(Some(dest.to_string_lossy().to_string()));
                }
            }
        }
    }

    Ok(None)
}

// Tiny host parser so this module doesn't need a full URL crate dependency
// just to read the netloc out of a URL, same scope as urllib.parse.urlparse
// in the 2.x code.
mod url {
    pub fn parse_host(url: &str) -> Option<String> {
        let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
        let host_part = after_scheme.split(['/', '?', '#']).next()?;
        let host = host_part.rsplit_once('@').map(|(_, h)| h).unwrap_or(host_part);
        let host = host.split(':').next()?;
        if host.is_empty() { None } else { Some(host.to_string()) }
    }
}
