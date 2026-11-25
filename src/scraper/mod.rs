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

#[cfg(feature = "plus")]
pub const MAX_STREAM_CONTENT_LENGTH_BYTES: u64 = 15 * 1024 * 1024;
#[cfg(not(feature = "plus"))]
pub const MAX_STREAM_CONTENT_LENGTH_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_PDF_APC_BYTES: usize = 3 * 1024 * 1024; // 2 MB guard
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

fn build_http_client_inner(accept_invalid_certs: bool) -> reqwest::Client {
    reqwest::Client::builder()
        .pool_max_idle_per_host(16)
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
        .gzip(true)
        .cookie_store(true)
        .danger_accept_invalid_certs(accept_invalid_certs)
        .build()
        .expect("reqwest client")
}

/// Build the primary HTTP client with strict TLS certificate verification.
pub fn build_http_client() -> reqwest::Client {
    build_http_client_inner(false)
}

static INSECURE_TLS_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| build_http_client_inner(true));

fn is_tls_certificate_error(err: &reqwest::Error) -> bool {
    let mut cur: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = cur {
        let low = e.to_string().to_ascii_lowercase();
        if low.contains("certificate")
            || low.contains("webpki")
            || low.contains("invalid peer")
            || low.contains("self signed")
            || low.contains("self-signed")
            || low.contains("unknown issuer")
            || low.contains("expired")
            || low.contains("tls handshake")
            || low.contains("ssl")
        {
            return true;
        }
        cur = e.source();
    }
    false
}

#[derive(Debug)]
pub struct FetchOutcome {
    pub status: u16,
    pub html: String,
    pub final_url: String,
    pub cloudflare_blocked: bool,
    pub timed_out: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub insecure_tls: bool,
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

#[derive(Debug, Clone, Default)]
pub struct HostLatencyStats {
    pub total_reqs: u32,
    pub total_latency_ms: u64,
    pub timeout_count: u32,
    pub last_latency_ms: u64,
    pub is_slow: bool,
}

static HOST_LATENCY_MAP: Lazy<DashMap<String, HostLatencyStats>> = Lazy::new(DashMap::new);

/// Record latency or timeout for a given host URL during scraping sessions.
pub fn record_host_latency(raw_url: &str, latency_ms: u64, is_timeout: bool) {
    let Some(host) = DomainBreaker::host_key(raw_url) else {
        return;
    };
    let mut entry = HOST_LATENCY_MAP.entry(host).or_default();
    entry.total_reqs += 1;
    entry.total_latency_ms = entry.total_latency_ms.saturating_add(latency_ms);
    entry.last_latency_ms = latency_ms;
    if is_timeout {
        entry.timeout_count += 1;
    }
    // Slow host threshold: single response > 4000ms or >= 2 timeouts or avg > 3500ms
    let avg = entry.total_latency_ms / (entry.total_reqs as u64).max(1);
    if latency_ms > 4000 || entry.timeout_count >= 2 || (entry.total_reqs >= 2 && avg > 3500) {
        entry.is_slow = true;
    }
}

/// Check if a host has been identified as slow (> 4,000 ms or repeated timeouts).
pub fn is_slow_host(raw_url: &str) -> bool {
    let Some(host) = DomainBreaker::host_key(raw_url) else {
        return false;
    };
    if let Some(entry) = HOST_LATENCY_MAP.get(&host) {
        entry.is_slow
    } else {
        false
    }
}

/// Reset host latency tracking (for tests or new search runs).
pub fn reset_host_latency_tracker() {
    HOST_LATENCY_MAP.clear();
}

/// Get latency stats for a host.
pub fn get_host_latency_stats(raw_url: &str) -> Option<HostLatencyStats> {
    let host = DomainBreaker::host_key(raw_url)?;
    HOST_LATENCY_MAP.get(&host).map(|e| e.clone())
}

/// Stream HTML up to `MAX_HTML_BYTES` (1.5 MB / <= 2 MB header guard) with strict wall-clock timeout,
/// DomainBreaker protection, and polite per-host rate limiting.
pub async fn safe_fetch_html(
    client: &reqwest::Client,
    breaker: &DomainBreaker,
    url: &str,
    per_req_timeout: Duration,
) -> Result<FetchOutcome, String> {
    safe_fetch_html_conditional(client, breaker, url, per_req_timeout, None, None).await
}

/// Conditional variant of `safe_fetch_html` sending `If-None-Match` and `If-Modified-Since`
/// headers for HTTP 304 cache validation.
pub async fn safe_fetch_html_conditional(
    client: &reqwest::Client,
    breaker: &DomainBreaker,
    url: &str,
    per_req_timeout: Duration,
    if_none_match: Option<&str>,
    if_modified_since: Option<&str>,
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
        let (resp, insecure_tls) = send_get_with_tls_fallback(
            client,
            url,
            ua,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            if_none_match,
            if_modified_since,
        )
        .await?;

        let status = resp.status().as_u16();
        let final_url = resp.url().to_string();

        let resp_etag = resp
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let resp_last_mod = resp
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if status == 304 {
            return Ok(FetchOutcome {
                status: 304,
                html: String::new(),
                final_url,
                cloudflare_blocked: false,
                timed_out: false,
                etag: resp_etag,
                last_modified: resp_last_mod,
                insecure_tls,
            });
        }

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
        let buf = stream_bounded_bytes(resp, cap_bytes).await?;

        let html = String::from_utf8_lossy(&buf).into_owned();
        let cf = is_cloudflare_response(status, &server_hdr, &html);
        Ok::<FetchOutcome, String>(FetchOutcome {
            status,
            html,
            final_url,
            cloudflare_blocked: cf,
            timed_out: false,
            etag: resp_etag,
            last_modified: resp_last_mod,
            insecure_tls,
        })
    };

    let effective_timeout = if is_slow_host(url) {
        per_req_timeout.min(Duration::from_millis(4500))
    } else {
        per_req_timeout
    };

    let start_t = std::time::Instant::now();
    let res = tokio::time::timeout(effective_timeout, req_fut).await;
    let elapsed_ms = start_t.elapsed().as_millis() as u64;

    let is_timeout = match &res {
        Ok(Err(e)) if e == "timeout" => true,
        Err(_) => true,
        _ => false,
    };
    record_host_latency(url, elapsed_ms, is_timeout);

    match res {
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

async fn send_get_with_tls_fallback(
    client: &reqwest::Client,
    url: &str,
    ua: &str,
    accept: &str,
    if_none_match: Option<&str>,
    if_modified_since: Option<&str>,
) -> Result<(reqwest::Response, bool), String> {
    let build_req = |cli: &reqwest::Client| {
        let mut builder = cli
            .get(url)
            .header(reqwest::header::USER_AGENT, ua)
            .header("Accept", accept)
            .header("Accept-Language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7");
        if let Some(etag) = if_none_match {
            builder = builder.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(last_mod) = if_modified_since {
            builder = builder.header(reqwest::header::IF_MODIFIED_SINCE, last_mod);
        }
        builder.send()
    };

    match build_req(client).await {
        Ok(resp) => Ok((resp, false)),
        Err(err) if url.starts_with("https://") && is_tls_certificate_error(&err) => {
            let fallback_resp = build_req(&INSECURE_TLS_CLIENT).await.map_err(|e| {
                if e.is_timeout() {
                    "timeout".to_string()
                } else {
                    format!("connect:{e}")
                }
            })?;
            Ok((fallback_resp, true))
        }
        Err(err) => Err(if err.is_timeout() {
            "timeout".to_string()
        } else {
            format!("connect:{err}")
        }),
    }
}

async fn stream_bounded_bytes(
    resp: reqwest::Response,
    cap_bytes: usize,
) -> Result<Vec<u8>, String> {
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
    Ok(buf)
}

/// Safely download an OJS `.pdf` document strictly bounded below `MAX_PDF_APC_BYTES` (3 MB).
pub async fn safe_fetch_pdf_bytes(
    client: &reqwest::Client,
    breaker: &DomainBreaker,
    url: &str,
    per_req_timeout: Duration,
) -> Result<(Vec<u8>, bool), String> {
    if breaker.is_blocked(url).is_some() {
        return Err("domain_blocked_cached".to_string());
    }
    enforce_host_rate_limit(url).await;
    let ua = next_user_agent();
    let req_fut = async {
        let (resp, insecure_tls) = send_get_with_tls_fallback(
            client,
            url,
            ua,
            "application/pdf,application/octet-stream;q=0.9,*/*;q=0.5",
            None,
            None,
        )
        .await?;
        if !resp.status().is_success() {
            return Err(format!("http_{}", resp.status().as_u16()));
        }
        if let Some(cl) = resp.content_length() {
            if cl > MAX_PDF_APC_BYTES as u64 {
                return Err(format!("pdf_too_large:{cl}"));
            }
        }
        if let Some(ct) = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
        {
            let low = ct.to_ascii_lowercase();
            if !low.contains("pdf") && !low.contains("octet-stream") {
                return Err(format!("non_pdf_content:{low}"));
            }
        }
        let bytes = stream_bounded_bytes(resp, MAX_PDF_APC_BYTES).await?;
        Ok::<(Vec<u8>, bool), String>((bytes, insecure_tls))
    };

    tokio::time::timeout(per_req_timeout, req_fut)
        .await
        .map_err(|_| "timeout".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_guards_and_bot_challenge_detection() {
        assert!(is_disallowed_stream_headers(Some("application/pdf"), Some(1024)).is_err());
        assert!(is_disallowed_stream_headers(Some("application/octet-stream"), Some(100)).is_err());
        assert!(is_disallowed_stream_headers(
            Some("text/html; charset=utf-8"),
            Some(MAX_STREAM_CONTENT_LENGTH_BYTES + 100_000)
        )
        .is_err());
        assert!(
            is_disallowed_stream_headers(Some("text/html; charset=utf-8"), Some(3_000_000)).is_ok()
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

    #[test]
    fn test_slow_host_deprioritization_tracker() {
        reset_host_latency_tracker();
        let fast_url = "https://fast-journal.ac.id/index.php/jti";
        let slow_url = "https://slow-campus.ac.id/journal/index";

        // Fast host: 250ms latency
        record_host_latency(fast_url, 250, false);
        assert!(!is_slow_host(fast_url));

        // Slow host: > 4000ms latency
        record_host_latency(slow_url, 4200, false);
        assert!(is_slow_host(slow_url));

        let stats = get_host_latency_stats(slow_url).expect("stats present");
        assert_eq!(stats.last_latency_ms, 4200);
        assert!(stats.is_slow);

        // Repeated timeouts make host slow
        let timeout_url = "https://timeout-journal.org/index";
        record_host_latency(timeout_url, 1500, true);
        assert!(!is_slow_host(timeout_url));
        record_host_latency(timeout_url, 1600, true);
        assert!(is_slow_host(timeout_url));
    }
}
