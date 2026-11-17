//! Lightweight one-shot GitHub release checker for JSFS.
//! Spawns a dedicated background OS thread on startup; fires at most once per session.
//! Completely panic-safe with catch_unwind and fallible error handling.

use crate::engine::{EngineEvent, EventSink};

/// Compare two semver strings (major.minor.patch). Returns `true` if `latest_tag`
/// is strictly newer than `current_version`. Leading `v` is stripped automatically.
pub fn is_newer_version(latest_tag: &str, current_version: &str) -> bool {
    let latest = latest_tag.trim_start_matches('v');
    let current = current_version.trim_start_matches('v');
    fn parse(s: &str) -> (u32, u32, u32) {
        let mut parts = s.splitn(3, '.').map(|p| p.parse::<u32>().unwrap_or(0));
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    }
    parse(latest) > parse(current)
}

/// Spawn a one-shot background update check against the GitHub releases API.
/// Results are delivered via `sink` as `EngineEvent::UpdateAvailable(tag, body_snippet)`.
/// Runs on a dedicated OS worker thread with `catch_unwind` so it NEVER panics or requires
/// an ambient Tokio runtime on the calling thread.
pub fn spawn_update_check(sink: EventSink) {
    std::thread::Builder::new()
        .name("jsfs-updater".to_string())
        .spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    rt.block_on(async {
                        if let Some((tag, body)) = check_for_update().await {
                            if is_newer_version(&tag, crate::config::APP_VERSION) {
                                sink.send(EngineEvent::UpdateAvailable(tag, body));
                            }
                        }
                    });
                }
            }));
        })
        .ok();
}

async fn check_for_update() -> Option<(String, String)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("JSFS-Update-Checker")
        .build()
        .ok()?;

    let resp = client
        .get(
            "https://api.github.com/repos/BakwanJagungg/\
             JSFS-Journal-Scraper-For-Students-/releases/latest",
        )
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let tag = json["tag_name"].as_str()?.to_string();
    let body: String = json["body"]
        .as_str()
        .unwrap_or("")
        .chars()
        .take(220)
        .collect();

    Some((tag, body))
}

// ─── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::is_newer_version;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("v4.11.1", "4.11.0"));
        assert!(is_newer_version("v4.11.0", "4.10.0"));
        assert!(is_newer_version("v4.10.1", "4.10.0"));
        assert!(is_newer_version("v5.0.0", "4.10.0"));
        assert!(!is_newer_version("v4.10.0", "4.10.0")); // same version
        assert!(!is_newer_version("v4.9.1", "4.10.0")); // older
        assert!(!is_newer_version("v4.10.0", "v4.10.0")); // both have v prefix
        assert!(is_newer_version("v4.10.1", "v4.10.0"));
    }

    #[test]
    fn test_adaptive_column_width_narrow_vs_wide() {
        // narrow screen: table_w = 780 → compact widths
        let table_w = 780.0_f32;
        let (col_sinta, col_match, col_apc, col_activity, col_action) = if table_w < 880.0 {
            (72.0_f32, 52.0, 126.0, 148.0, 132.0)
        } else {
            (80.0_f32, 62.0, 144.0, 170.0, 146.0)
        };
        assert_eq!(col_sinta, 72.0, "compact: sinta width");
        let fixed_w = col_sinta + col_match + col_apc + col_activity + col_action;
        let col0_w = (table_w - fixed_w).max(220.0);
        assert!(col0_w >= 220.0, "col0 must be at least 220 px on narrow");

        // wide screen: table_w = 1280 → standard widths
        let table_w2 = 1280.0_f32;
        let (col_sinta2, col_match2, col_apc2, col_activity2, col_action2) = if table_w2 < 880.0 {
            (72.0_f32, 52.0, 126.0, 148.0, 132.0)
        } else {
            (80.0_f32, 62.0, 144.0, 170.0, 146.0)
        };
        assert_eq!(col_sinta2, 80.0, "standard: sinta width");
        let fixed_w2 = col_sinta2 + col_match2 + col_apc2 + col_activity2 + col_action2;
        let col0_w2 = (table_w2 - fixed_w2).max(220.0);
        assert!(col0_w2 > 220.0, "col0 must be wide on full-HD");
    }
}
