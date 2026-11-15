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
use std::time::Duration;
use url::Url;

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

pub fn is_cloudflare_html(status: u16, html: &str) -> bool {
    if matches!(status, 403 | 503) {
        let low = html.to_ascii_lowercase();
        if low.contains("just a moment")
            || low.contains("cf-browser-verification")
            || low.contains("turnstile")
            || low.contains("challenge-running")
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

/// Stream HTML up to `MAX_HTML_BYTES` (1.5 MB guard) with strict wall-clock timeout and DomainBreaker integration.
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

    let req_fut = async {
        let resp = client
            .get(url)
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

        if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE) {
            if let Ok(ct_str) = ct.to_str() {
                let low = ct_str.to_ascii_lowercase();
                if !low.contains("html") && !low.contains("xml") && !low.contains("text") {
                    return Err(format!("non_html_content:{low}"));
                }
            }
        }

        let mut buf = Vec::with_capacity(32 * 1024);
        let mut stream = resp.bytes_stream();
        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res.map_err(|e| format!("stream:{e}"))?;
            let remaining = MAX_HTML_BYTES.saturating_sub(buf.len());
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
        let cf = is_cloudflare_html(status, &html);
        Ok(FetchOutcome {
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
