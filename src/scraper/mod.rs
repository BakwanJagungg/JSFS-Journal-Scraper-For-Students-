pub mod dedup;
pub mod garuda;
pub mod international;
pub mod ojs_deep;
pub mod sinta;

use crate::config::MAX_HTML_BYTES;
use chrono::Utc;
use dashmap::DashMap;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

pub const MAX_STREAM_CONTENT_LENGTH_BYTES: u64 = 2 * 1024 * 1024; // 2 MB guard
pub const POLITE_BOT_USER_AGENT: &str =
    "JSFS-Academic-Research-Tool/4.7 (+https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-)";

const BROWSER_USER_AGENTS: [&str; 4] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36 Edg/133.0.0.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 Edg/134.0.0.0",
];

static POLITE_BOT_MODE: AtomicBool = AtomicBool::new(false);
static UA_ROTATION_IDX: AtomicUsize = AtomicUsize::new(0);
static HOST_RATE_SLOTS: Lazy<Mutex<HashMap<String, tokio::time::Instant>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn set_polite_bot_mode(enabled: bool) {
    POLITE_BOT_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_polite_bot_mode() -> bool {
    POLITE_BOT_MODE.load(Ordering::Relaxed)
}

pub fn next_user_agent() -> &'static str {
    if is_polite_bot_mode() {
        POLITE_BOT_USER_AGENT
    } else {
        let idx = UA_ROTATION_IDX.fetch_add(1, Ordering::Relaxed);
        BROWSER_USER_AGENTS[idx % BROWSER_USER_AGENTS.len()]
    }
}

fn zero_dep_jitter_ms(min_ms: u64, max_ms: u64) -> u64 {
    let span = max_ms.saturating_sub(min_ms).saturating_add(1);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let seq = UA_ROTATION_IDX.load(Ordering::Relaxed) as u64;
    let mixed = nanos ^ seq.wrapping_mul(2_654_435_761);
    min_ms + (mixed % span)
}

/// Per-host politeness limiter: ensures max 1 request per 150–300 ms + 50–120 ms random jitter per domain.
pub async fn enforce_host_rate_limit(url: &str) {
    let Some(host) = DomainBreaker::host_key(url) else {
        return;
    };
    if host.contains("kemdiktisaintek.go.id") {
        return;
    }
    let wait_until = {
        let mut map = match HOST_RATE_SLOTS.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let now = tokio::time::Instant::now();
        let base_ms = if is_polite_bot_mode() { 260 } else { 180 };
        let gap = Duration::from_millis(base_ms + zero_dep_jitter_ms(50, 120));
        let next_allowed = match map.get(&host).copied() {
            Some(prev) if prev + gap > now => prev + gap,
            _ => now,
        };
        map.insert(host, next_allowed);
        next_allowed
    };
    let now = tokio::time::Instant::now();
    if wait_until > now {
        tokio::time::sleep_until(wait_until).await;
    }
}

static RE_PUB_NOISE_SHARED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:google\s+scholar|editor\s+url|published\s+by|diterbitkan\s+oleh|publisher\s*:?|penerbit\s*:?|website|garuda|sinta|orcid|scopus)\b|https?://\S+",
    )
    .expect("valid regex")
});

/// Strip portal prefixes (`Published by`, `Diterbitkan oleh`) and link-bar noise from publisher names.
pub fn sanitize_publisher_name(raw: &str) -> String {
    let without_noise = RE_PUB_NOISE_SHARED.replace_all(raw, " ");
    let collapsed = without_noise
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = collapsed
        .trim_matches(['|', '-', ':', '/', ',', '•', ';', ' '])
        .trim()
        .to_string();
    if trimmed.len() < 3 {
        String::new()
    } else {
        trimmed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainHealth {
    Healthy,
    Dead,
    Timeout,
}

/// Concurrent host-level circuit breaker shared across worker tasks.
pub struct DomainBreaker {
    states: DashMap<String, (DomainHealth, i64)>,
}

impl DomainBreaker {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }

    pub fn host_key(raw_url: &str) -> Option<String> {
        Url::parse(raw_url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
    }

    pub fn is_blocked(&self, raw_url: &str) -> Option<DomainHealth> {
        let host = Self::host_key(raw_url)?;
        if let Some(entry) = self.states.get(&host) {
            let (health, ts) = *entry;
            if health != DomainHealth::Healthy && (Utc::now().timestamp() - ts) < 120 {
                return Some(health);
            }
        }
        None
    }

    pub fn record(&self, raw_url: &str, health: DomainHealth) {
        if let Some(host) = Self::host_key(raw_url) {
            self.states.insert(host, (health, Utc::now().timestamp()));
        }
    }
}

impl Default for DomainBreaker {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_max_idle_per_host(16)
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
        .gzip(true)
        .cookie_store(true)
        .danger_accept_invalid_certs(true)
        .build()
        .expect("reqwest client")
}

#[derive(Debug)]
pub struct FetchOutcome {
    pub status: u16,
    pub html: String,
    pub final_url: String,
    pub cloudflare_blocked: bool,
    pub timed_out: bool,
}

pub fn is_cloudflare_response(status: u16, server_header: &str, html: &str) -> bool {
    let srv_low = server_header.to_ascii_lowercase();
    if matches!(status, 403 | 503) {
        if srv_low.contains("cloudflare") {
            return true;
        }
        let low = html.to_ascii_lowercase();
        if low.contains("just a moment")
            || low.contains("cf-browser-verification")
            || low.contains("turnstile")
            || low.contains("challenge-running")
            || low.contains("attention required")
            || low.contains("cloudflare ray id")
            || low.contains("ddos-guard")
        {
            return true;
        }
    }
    let prefix: String = html.chars().take(4096).collect();
    let low = prefix.to_ascii_lowercase();
    low.contains("cf-browser-verification")
        || low.contains("challenges.cloudflare.com/turnstile")
        || (low.contains("<title>just a moment") && low.contains("cloudflare"))
}

pub fn is_cloudflare_html(status: u16, html: &str) -> bool {
    is_cloudflare_response(status, "", html)
}

pub fn is_disallowed_stream_headers(
    content_type: Option<&str>,
    content_length: Option<u64>,
) -> Result<(), String> {
    if let Some(cl) = content_length {
        if cl > MAX_STREAM_CONTENT_LENGTH_BYTES {
            return Err(format!("content_too_large:{cl}"));
        }
    }
    if let Some(ct) = content_type {
        let low = ct.to_ascii_lowercase();
        if low.contains("application/pdf")
            || low.contains("octet-stream")
            || low.contains("zip")
            || low.contains("image/")
            || low.contains("video/")
            || low.contains("audio/")
            || (!low.contains("html") && !low.contains("xml") && !low.contains("text"))
        {
            return Err(format!("non_html_content:{low}"));
        }
    }
    Ok(())
}

/// Stream HTML up to `MAX_HTML_BYTES` (1.5 MB / <= 2 MB header guard) with strict wall-clock timeout,
/// DomainBreaker protection, and polite per-host rate limiting.
pub async fn safe_fetch_html(
    client: &reqwest::Client,
    breaker: &DomainBreaker,
    url: &str,
    per_req_timeout: Duration,
) -> Result<FetchOutcome, String> {
    if let Some(health) = breaker.is_blocked(url) {
        return Err(match health {
            DomainHealth::Timeout => "domain_timeout_cached".to_string(),
            _ => "domain_dead_cached".to_string(),
        });
    }

    enforce_host_rate_limit(url).await;
    let ua = next_user_agent();

    let req_fut = async {
        let resp = client
            .get(url)
            .header(reqwest::header::USER_AGENT, ua)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    "timeout".to_string()
                } else {
                    format!("connect:{e}")
                }
            })?;

        let status = resp.status().as_u16();
        let final_url = resp.url().to_string();
        let server_hdr = resp
            .headers()
            .get(reqwest::header::SERVER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let ct_hdr = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok());
        is_disallowed_stream_headers(ct_hdr, resp.content_length())?;

        let cap_bytes = MAX_HTML_BYTES.min(MAX_STREAM_CONTENT_LENGTH_BYTES as usize);
        let mut buf = Vec::with_capacity(32 * 1024);
        let mut stream = resp.bytes_stream();
        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res.map_err(|e| format!("stream:{e}"))?;
            let remaining = cap_bytes.saturating_sub(buf.len());
            if remaining == 0 {
                break;
            }
            if chunk.len() > remaining {
                buf.extend_from_slice(&chunk[..remaining]);
                break;
            }
            buf.extend_from_slice(&chunk);
        }

        let html = String::from_utf8_lossy(&buf).into_owned();
        let cf = is_cloudflare_response(status, &server_hdr, &html);
        Ok::<FetchOutcome, String>(FetchOutcome {
            status,
            html,
            final_url,
            cloudflare_blocked: cf,
            timed_out: false,
        })
    };

    match tokio::time::timeout(per_req_timeout, req_fut).await {
        Ok(Ok(outcome)) => {
            breaker.record(url, DomainHealth::Healthy);
            Ok(outcome)
        }
        Ok(Err(e)) => {
            if e == "timeout" {
                breaker.record(url, DomainHealth::Timeout);
            } else if e.starts_with("connect:") {
                breaker.record(url, DomainHealth::Dead);
            }
            Err(e)
        }
        Err(_) => {
            breaker.record(url, DomainHealth::Timeout);
            Err("timeout".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_guards_and_bot_challenge_detection() {
        assert!(is_disallowed_stream_headers(Some("application/pdf"), Some(1024)).is_err());
        assert!(is_disallowed_stream_headers(Some("application/octet-stream"), Some(100)).is_err());
        assert!(
            is_disallowed_stream_headers(Some("text/html; charset=utf-8"), Some(3_000_000))
                .is_err()
        );
        assert!(
            is_disallowed_stream_headers(Some("text/html; charset=utf-8"), Some(64_000)).is_ok()
        );

        assert!(is_cloudflare_response(403, "cloudflare", "Access denied"));
        assert!(is_cloudflare_response(
            503,
            "",
            "<title>Just a moment...</title>"
        ));
        assert!(!is_cloudflare_response(
            200,
            "nginx",
            "<html><body>OJS Journal</body></html>"
        ));

        set_polite_bot_mode(true);
        assert_eq!(next_user_agent(), POLITE_BOT_USER_AGENT);
        set_polite_bot_mode(false);
        assert!(next_user_agent().contains("Mozilla/5.0"));
    }
}
