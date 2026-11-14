//! Optional selective Ollama local LLM integration with 8-item cap, 45s hard budget, and 5-fail circuit breaker.

use crate::config::{
    APC_UNKNOWN, DEFAULT_AI_MODEL, INFO_NOT_FOUND, OLLAMA_BASE_URL, OLLAMA_MAX_ITEMS,
    OLLAMA_PER_ITEM_TIMEOUT_SECS, OLLAMA_PHASE_BUDGET_SECS,
};
use crate::models::{format_idr_short, AiStatus, JournalRecord};
use crate::parser::{apc, focus};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

static RE_NUMS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d+(?:[.,]\d+)?").expect("valid regex"));
static RE_SENT_SPLIT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?]+\s+").expect("valid regex"));
static RE_PUB_FREE_CUE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:publication|publishing|apc|biaya\s+publikasi|biaya\s+penerbitan).{0,40}(?:0|gratis|free|tanpa\s+biaya|bebas\s+biaya)|(?:free\s+of\s+charge|tanpa\s+biaya|bebas\s+biaya).{0,40}(?:publication|publishing|publikasi|penerbitan)")
        .expect("valid regex")
});

const PROMPT_FOCUS_TPL: &str = r#"Anda adalah asisten akademik. Ekstrak ruang lingkup riset (focus and scope) jurnal dari teks OJS di tag <teks>.
Aturan: tulis MAKSIMAL 2 kalimat dan MAKSIMAL 40 kata berbahasa Indonesia. Hanya gunakan informasi di teks. Jika tidak ada, isi "Informasi tidak ditemukan".
Balas HANYA JSON: {"focus_scope": "<ringkasan>"}

<teks>
[[TEKS]]
</teks>"#;

const PROMPT_APC_TPL: &str = r#"Anda adalah asisten ekstraksi biaya jurnal dari teks di tag <teks>.
Aturan ketat:
1. Jangan samakan submission fee = 0 dengan biaya publikasi! Jika hanya submission fee gratis, isi biaya_submit=0 dan biaya_publikasi=null.
2. Hanya isi biaya_publikasi=0 jika teks secara eksplisit menyebut biaya publikasi/APC gratis/free of charge/Rp 0.
3. mata_uang hanya "IDR", "USD", atau null.
Balas HANYA JSON: {"biaya_submit": <int|null>, "biaya_publikasi": <int|null>, "mata_uang": <"IDR"|"USD"|null>}

<teks>
[[TEKS]]
</teks>"#;

pub struct AiBreaker {
    fail_streak: AtomicUsize,
    tripped_until: Mutex<Option<Instant>>,
}

impl AiBreaker {
    pub const fn new() -> Self {
        Self {
            fail_streak: AtomicUsize::new(0),
            tripped_until: Mutex::new(None),
        }
    }

    pub fn is_open(&self) -> bool {
        if let Ok(guard) = self.tripped_until.lock() {
            if let Some(until) = *guard {
                if Instant::now() < until {
                    return true;
                }
            }
        }
        false
    }

    pub fn record_success(&self) {
        self.fail_streak.store(0, Ordering::Relaxed);
        if let Ok(mut guard) = self.tripped_until.lock() {
            *guard = None;
        }
    }

    pub fn record_failure(&self) {
        let streak = self.fail_streak.fetch_add(1, Ordering::Relaxed) + 1;
        if streak >= 5 {
            if let Ok(mut guard) = self.tripped_until.lock() {
                *guard = Some(Instant::now() + Duration::from_secs(60));
            }
        }
    }
}

impl Default for AiBreaker {
    fn default() -> Self {
        Self::new()
    }
}

pub static GLOBAL_AI_BREAKER: Lazy<AiBreaker> = Lazy::new(AiBreaker::new);

pub fn find_ollama_executable() -> Option<PathBuf> {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let cand = dir.join("ollama.exe");
            if cand.is_file() {
                return Some(cand);
            }
            let cand_nix = dir.join("ollama");
            if cand_nix.is_file() {
                return Some(cand_nix);
            }
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local).join("Programs\\Ollama\\ollama.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(pf) = std::env::var("PROGRAMFILES") {
        let p = PathBuf::from(pf).join("Ollama\\ollama.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

pub async fn get_ai_status(client: &reqwest::Client) -> AiStatus {
    let exe_found = find_ollama_executable().is_some();
    let mut running = false;
    let mut models = Vec::new();

    if let Ok(resp) = client
        .get(format!("{OLLAMA_BASE_URL}/api/tags"))
        .timeout(Duration::from_millis(1500))
        .send()
        .await
    {
        if resp.status().is_success() {
            running = true;
            if let Ok(val) = resp.json::<Value>().await {
                if let Some(arr) = val.get("models").and_then(|v| v.as_array()) {
                    for m in arr {
                        if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                            models.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    AiStatus {
        installed: exe_found || running,
        running,
        models,
        recommended_model: DEFAULT_AI_MODEL.to_string(),
        circuit_open: GLOBAL_AI_BREAKER.is_open(),
        task_running: false,
        task_percent: 0.0,
        task_message: String::new(),
    }
}

fn numbers_in_snippet(snippet: &str) -> HashSet<i64> {
    let mut set = HashSet::new();
    for m in RE_NUMS.find_iter(snippet) {
        let raw = m.as_str();
        let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<i64>() {
            set.insert(v);
        }
        if let Ok(f) = raw.replace(',', ".").parse::<f64>() {
            set.insert(f.round() as i64);
            set.insert((f * 1_000.0).round() as i64);
            set.insert((f * 1_000_000.0).round() as i64);
        }
    }
    set
}

pub fn validate_apc_json(data: &Value, snippet: &str) -> Option<(Option<i64>, Option<i64>, String)> {
    let obj = data.as_object()?;
    let allowed = numbers_in_snippet(snippet);
    let parse_opt = |k: &str| -> Option<Option<i64>> {
        match obj.get(k) {
            None | Some(Value::Null) => Some(None),
            Some(Value::Number(n)) => n.as_i64().filter(|&v| v >= 0).map(Some),
            Some(Value::String(s)) => {
                let t = s.trim();
                if t.is_empty() || t.eq_ignore_ascii_case("null") {
                    Some(None)
                } else {
                    t.parse::<i64>().ok().filter(|&v| v >= 0).map(Some)
                }
            }
            _ => None,
        }
    };

    let sub = parse_opt("biaya_submit")?;
    let pub_fee = parse_opt("biaya_publikasi")?;
    if sub.is_none() && pub_fee.is_none() {
        return None;
    }

    if let Some(p) = pub_fee {
        if p == 0 && !RE_PUB_FREE_CUE.is_match(snippet) {
            return None;
        }
        if p > 0 && !allowed.contains(&p) {
            return None;
        }
    }
    if let Some(s) = sub {
        if s > 0 && !allowed.contains(&s) {
            return None;
        }
    }

    let cur = obj
        .get("mata_uang")
        .and_then(|v| v.as_str())
        .map(str::to_ascii_uppercase)
        .filter(|s| s == "IDR" || s == "USD")
        .unwrap_or_else(|| "IDR".to_string());

    Some((sub, pub_fee, cur))
}

pub fn validate_focus_json(data: &Value) -> Option<String> {
    let raw = data.get("focus_scope")?.as_str()?.trim();
    if raw.is_empty() || raw.to_ascii_lowercase().contains("tidak ditemukan") {
        return None;
    }
    if focus::is_boilerplate_noise(raw) {
        return None;
    }
    // Max 2 sentences & max 60 words
    let parts: Vec<&str> = RE_SENT_SPLIT.split(raw).take(2).collect();
    let mut joined = parts.join(". ").trim().to_string();
    if !joined.ends_with('.') && !joined.ends_with('!') {
        joined.push('.');
    }
    if joined.split_whitespace().count() > 60 {
        return None;
    }
    Some(joined)
}

async fn call_ollama_json(client: &reqwest::Client, model: &str, prompt: &str) -> Result<Value, String> {
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "format": "json",
        "keep_alive": "30m",
        "options": { "temperature": 0, "num_ctx": 4096, "num_predict": 256 }
    });
    let resp = client
        .post(format!("{OLLAMA_BASE_URL}/api/generate"))
        .json(&body)
        .timeout(Duration::from_secs(OLLAMA_PER_ITEM_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("http_{}", resp.status()));
    }
    let outer: Value = resp.json().await.map_err(|e| e.to_string())?;
    let resp_str = outer
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing response".to_string())?;
    serde_json::from_str::<Value>(resp_str).map_err(|e| e.to_string())
}

/// Enrich up to `OLLAMA_MAX_ITEMS` (8) highest-priority incomplete records within `OLLAMA_PHASE_BUDGET_SECS` (45s).
pub async fn enrich_records_selective<F>(
    client: &reqwest::Client,
    records: &mut [JournalRecord],
    requested_model: Option<&str>,
    cancel_flag: Arc<AtomicBool>,
    on_progress: F,
) where
    F: Fn(usize, usize, &str) + Send + Sync,
{
    if GLOBAL_AI_BREAKER.is_open() {
        return;
    }
    let status = get_ai_status(client).await;
    if !status.running || status.models.is_empty() {
        return;
    }

    let chosen_model = requested_model
        .filter(|m| !m.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            status
                .models
                .iter()
                .find(|m| m.starts_with(DEFAULT_AI_MODEL))
                .cloned()
        })
        .unwrap_or_else(|| status.models[0].clone());

    let mut eligible: Vec<(usize, f64, bool, bool)> = records
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let need_focus =
                (r.focus_and_scope.is_empty() || r.focus_and_scope == INFO_NOT_FOUND)
                    && !r.apc_raw_text.trim().is_empty();
            let need_apc = (r.apc_status.is_empty() || r.apc_status == APC_UNKNOWN)
                && !r.apc_raw_text.trim().is_empty();
            if need_focus || need_apc {
                Some((i, r.match_score, need_focus, need_apc))
            } else {
                None
            }
        })
        .collect();

    eligible.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    eligible.truncate(OLLAMA_MAX_ITEMS);
    if eligible.is_empty() {
        return;
    }

    let total = eligible.len();
    let phase_work = async {
        for (step, (idx, _, need_focus, need_apc)) in eligible.into_iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) || GLOBAL_AI_BREAKER.is_open() {
                break;
            }
            let title = records[idx].title.clone();
            on_progress(step + 1, total, &title);

            let raw_snip: String = records[idx].apc_raw_text.chars().take(2800).collect();

            if need_apc {
                let prompt = PROMPT_APC_TPL.replace("[[TEKS]]", &raw_snip);
                match call_ollama_json(client, &chosen_model, &prompt).await {
                    Ok(val) => {
                        GLOBAL_AI_BREAKER.record_success();
                        if let Some((sub, pub_fee, cur)) = validate_apc_json(&val, &raw_snip) {
                            if let Some(s) = sub {
                                records[idx].submission_fee_idr = Some(s);
                            }
                            if let Some(p) = pub_fee {
                                records[idx].ai_validated = true;
                                records[idx].ai_source = format!("Ollama:{chosen_model}");
                                if p == 0 {
                                    records[idx].apc_status = "Gratis (Rp 0)".to_string();
                                    records[idx].publication_fee_idr = Some(0);
                                    records[idx].fee_currency = "IDR".to_string();
                                    records[idx].fee_amount = "0".to_string();
                                } else {
                                    let idr = if cur == "USD" {
                                        p * crate::config::USD_TO_IDR_RATE
                                    } else {
                                        p
                                    };
                                    records[idx].publication_fee_idr = Some(idr);
                                    records[idx].fee_currency = cur.clone();
                                    records[idx].fee_amount = p.to_string();
                                    records[idx].apc_status = if cur == "USD" {
                                        format!("USD {p} (~{})", format_idr_short(idr))
                                    } else {
                                        format_idr_short(idr)
                                    };
                                }
                            }
                        }
                    }
                    Err(_) => GLOBAL_AI_BREAKER.record_failure(),
                }
            }

            if need_focus && !GLOBAL_AI_BREAKER.is_open() {
                let prompt = PROMPT_FOCUS_TPL.replace("[[TEKS]]", &raw_snip);
                match call_ollama_json(client, &chosen_model, &prompt).await {
                    Ok(val) => {
                        GLOBAL_AI_BREAKER.record_success();
                        if let Some(fs) = validate_focus_json(&val) {
                            records[idx].focus_and_scope = fs;
                            records[idx].ai_validated = true;
                            if records[idx].ai_source.is_empty() {
                                records[idx].ai_source = format!("Ollama:{chosen_model}");
                            }
                        }
                    }
                    Err(_) => GLOBAL_AI_BREAKER.record_failure(),
                }
            }

            // Re-run fast regex fallback on apc_raw_text if still unknown
            if records[idx].apc_status == APC_UNKNOWN && !raw_snip.trim().is_empty() {
                if let apc::ApcParsed::Paid {
                    currency,
                    display,
                    amount_idr,
                    raw_amount,
                } = apc::parse_apc_from_context(&raw_snip)
                {
                    records[idx].apc_status = display;
                    records[idx].fee_currency = currency;
                    records[idx].fee_amount = raw_amount;
                    records[idx].publication_fee_idr = Some(amount_idr);
                }
            }
        }
    };

    let _ = tokio::time::timeout(Duration::from_secs(OLLAMA_PHASE_BUDGET_SECS), phase_work).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_apc_hallucination_and_submission_guard() {
        let snippet = "Author Fees: Article Submission: 0 (IDR). Publication charge not listed.";
        let hallucinated = serde_json::json!({
            "biaya_submit": 0,
            "biaya_publikasi": 0,
            "mata_uang": "IDR"
        });
        // Reject biaya_publikasi = 0 when only submission is explicitly 0!
        assert!(validate_apc_json(&hallucinated, snippet).is_none());

        let valid_pub = serde_json::json!({
            "biaya_submit": 0,
            "biaya_publikasi": 750000,
            "mata_uang": "IDR"
        });
        assert!(
            validate_apc_json(&valid_pub, "Submission 0. Publication fee: Rp 750.000").is_some()
        );
        // Reject hallucinated number missing from snippet
        assert!(validate_apc_json(&valid_pub, "Submission 0. Publication fee: Rp 500.000").is_none());
    }
}
