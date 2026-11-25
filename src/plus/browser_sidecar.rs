//! Native Headless Browser Sidecar (`msedge` / `chrome` / `chromium`) for JSFS+ Workstation.
//! Automatically activated when standard HTTP scraping encounters Cloudflare 403/503 or empty JS SPA shells.

use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::Semaphore;

/// Strict process concurrency limit so headless sidecars never exhaust workstation memory.
static SIDECAR_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(2));

/// Search candidate executable locations first, then fall back to `$PATH`.
pub fn detect_headless_browser_in(candidates: &[PathBuf], path_bins: &[&str]) -> Option<PathBuf> {
    for p in candidates {
        if p.is_file() {
            return Some(p.clone());
        }
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for bin in path_bins {
                let full = dir.join(bin);
                if full.is_file() {
                    return Some(full);
                }
            }
        }
    }
    None
}

/// Detect installed Chromium/Edge executable on Windows, Linux, or macOS.
pub fn detect_headless_browser() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "windows")]
    let path_bins: &[&str] = &["msedge.exe", "chrome.exe", "chromium.exe"];
    #[cfg(target_os = "windows")]
    {
        for env_key in ["ProgramFiles", "ProgramFiles(x86)", "LocalAppData"] {
            if let Ok(root) = std::env::var(env_key) {
                let base = Path::new(&root);
                candidates.push(base.join("Microsoft\\Edge\\Application\\msedge.exe"));
                candidates.push(base.join("Google\\Chrome\\Application\\chrome.exe"));
                candidates.push(base.join("Chromium\\Application\\chrome.exe"));
            }
        }
    }

    #[cfg(target_os = "macos")]
    let path_bins: &[&str] = &["google-chrome", "chromium", "microsoft-edge"];
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ));
        candidates.push(PathBuf::from(
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ));
        candidates.push(PathBuf::from(
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    let path_bins: &[&str] = &[
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "microsoft-edge",
    ];
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for p in [
            "/usr/bin/google-chrome",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/usr/bin/microsoft-edge",
            "/snap/bin/chromium",
        ] {
            candidates.push(PathBuf::from(p));
        }
    }

    detect_headless_browser_in(&candidates, path_bins)
}

/// Return `true` when an HTML response is an empty JS SPA shell lacking visible OJS text.
pub fn is_empty_js_spa_shell(html: &str) -> bool {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return true;
    }
    let low = trimmed.to_ascii_lowercase();
    let has_spa_marker = low.contains("id=\"app\"")
        || low.contains("id=\"root\"")
        || low.contains("__next")
        || low.contains("__nuxt")
        || low.contains("enable javascript")
        || low.contains("<noscript");
    if !has_spa_marker {
        return false;
    }
    let plain = crate::parser::focus::sanitize_html_text(trimmed);
    plain.split_whitespace().count() < 25
}

fn validate_http_url(raw: &str) -> Result<String, String> {
    let parsed = url::Url::parse(raw.trim()).map_err(|e| format!("URL tidak valid: {e}"))?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed.to_string()),
        other => Err(format!("Skema URL tidak didukung oleh sidecar: {other}")),
    }
}

/// Render a target URL via local Chromium/Edge `--headless=new --dump-dom` with strict
/// concurrency bound (max 2 processes) and wall-clock timeout.
pub async fn fetch_dom_headless(raw_url: &str, timeout_secs: u64) -> Result<String, String> {
    let clean_url = validate_http_url(raw_url)?;
    let browser = detect_headless_browser().ok_or_else(|| {
        "Peramban Chromium atau Edge tidak ditemukan di sistem untuk mode headless.".to_string()
    })?;

    let _permit = SIDECAR_SEMAPHORE
        .acquire()
        .await
        .map_err(|_| "Sidecar semaphore ditutup".to_string())?;

    let mut cmd = tokio::process::Command::new(&browser);
    cmd.args([
        "--headless=new",
        "--disable-gpu",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-extensions",
        "--disable-background-networking",
        "--virtual-time-budget=8000",
        "--dump-dom",
    ])
    .arg(&clean_url)
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::null());

    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Gagal menjalankan headless browser: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Stdout headless browser tidak tersedia".to_string())?;

    let wait_budget = Duration::from_secs(timeout_secs.clamp(5, 20));
    let read_fut = async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::with_capacity(64 * 1024);
        let cap = crate::config::MAX_HTML_BYTES;
        let mut reader = tokio::io::BufReader::new(stdout).take(cap as u64);
        reader
            .read_to_end(&mut buf)
            .await
            .map_err(|e| format!("Gagal membaca DOM headless: {e}"))?;
        let _ = child.wait().await;
        Ok::<Vec<u8>, String>(buf)
    };

    let bytes = match tokio::time::timeout(wait_budget, read_fut).await {
        Ok(res) => res?,
        Err(_) => {
            return Err("Headless browser timeout sebelum DOM selesai dirender.".to_string());
        }
    };

    let html = String::from_utf8_lossy(&bytes).into_owned();
    if crate::scraper::is_cloudflare_response(200, "", &html) {
        return Err("Halaman masih tertahan verifikasi Cloudflare di mode headless.".to_string());
    }
    if html.trim().len() < 120 {
        return Err("DOM hasil render headless kosong.".to_string());
    }
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_browser_path_detection_and_spa_guard() {
        let dir = std::env::temp_dir().join(format!("jsfs-plus-bin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let fake_browser = dir.join("chrome_mock.exe");
        std::fs::write(&fake_browser, b"mock").expect("write mock");

        let missing = dir.join("nonexistent_edge.exe");
        let detected = detect_headless_browser_in(&[missing, fake_browser.clone()], &[]);
        assert_eq!(detected.as_deref(), Some(fake_browser.as_path()));

        let _ = std::fs::remove_dir_all(&dir);

        let spa_html = r#"<html><body><div id="root"></div><script src="/app.js"></script><noscript>Please enable JavaScript to continue.</noscript></body></html>"#;
        assert!(is_empty_js_spa_shell(spa_html));

        let ojs_html = r#"<html><body><div id="pkp_content"><h1>Jurnal Teknologi Informasi Indonesia</h1><p>Focus and scope covers software engineering, artificial intelligence, information systems, networking, cyber security, and empirical computer science studies published twice a year in May and November with free APC for students and researchers nationwide.</p></div></body></html>"#;
        assert!(!is_empty_js_spa_shell(ojs_html));

        assert!(validate_http_url("https://ejournal.kampus.ac.id/index.php/jsi").is_ok());
        assert!(validate_http_url("file:///C:/Windows/System32").is_err());
    }
}
