//! Multi-provider AI enrichment engine (Cloud API & Local Ollama) with shared validators & circuit breaker.

pub mod cloud;
pub mod ollama;

use crate::config::{
    APC_UNKNOWN, INFO_NOT_FOUND, OLLAMA_MAX_ITEMS, OLLAMA_PHASE_BUDGET_SECS, USD_TO_IDR_RATE,
};
use crate::models::{format_idr_short, JournalRecord};
use crate::parser::{apc, focus};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

static RE_NUMS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d+(?:[.,]\d+)?").expect("valid regex"));
static RE_SENT_SPLIT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?]+\s+").expect("valid regex"));
static RE_PUB_FREE_CUE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:publication|publishing|apc|biaya\s+publikasi|biaya\s+penerbitan).{0,40}(?:0|gratis|free|tanpa\s+biaya|bebas\s+biaya)|(?:free\s+of\s+charge|tanpa\s+biaya|bebas\s+biaya).{0,40}(?:publication|publishing|publikasi|penerbitan)")
        .expect("valid regex")
});

pub const PROMPT_FOCUS_TPL: &str = r#"Anda adalah asisten akademik. Ekstrak ruang lingkup riset (focus and scope) jurnal dari teks OJS di tag <teks>.
Aturan: tulis MAKSIMAL 2 kalimat dan MAKSIMAL 40 kata berbahasa Indonesia. Hanya gunakan informasi di teks. Jika tidak ada, isi "Informasi tidak ditemukan".
Balas HANYA JSON: {"focus_scope": "<ringkasan>"}

<teks>
[[TEKS]]
</teks>"#;

pub const PROMPT_APC_TPL: &str = r#"Anda adalah asisten ekstraksi biaya jurnal dari teks di tag <teks>.
Aturan ketat (WAJIB DIPATUHI):
1. Abaikan pernyataan historis tahun lalu (mis. "from 2008 to 2018 was free of charge") dan frasa Open Access untuk pembaca (mis. "readers can read and download articles for free of charge").
2. Abaikan klaim awal "IDR 0" / "No charge" jika di poin bawahnya (mis. "after accepted", "ready to issue") disebutkan adanya biaya publikasi berbayar.
3. Jika ada biaya REGULER dan biaya FAST TRACK, WAJIB ekstrak biaya REGULER sebagai biaya_publikasi (jangan ambil Fast Track).
4. Jangan samakan submission fee = 0 dengan biaya publikasi! Jika hanya submission fee gratis, isi biaya_submit=0 dan biaya_publikasi=null.
5. Hanya isi biaya_publikasi=0 jika teks secara eksplisit menyebut biaya publikasi/APC saat ini benar-benar gratis dan tidak ada nominal berbayar di bagian lain.
6. mata_uang hanya "IDR", "USD", atau null.
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

/// Extract a JSON object even if an LLM wrapped it in markdown fences or surrounding prose.
pub fn parse_json_lenient(raw: &str) -> Result<Value, String> {
    let trimmed = raw.trim();
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return Ok(v);
    }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start < end {
            return serde_json::from_str::<Value>(&trimmed[start..=end])
                .map_err(|e| e.to_string());
        }
    }
    Err("invalid json response".to_string())
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

pub fn validate_apc_json(
    data: &Value,
    snippet: &str,
) -> Option<(Option<i64>, Option<i64>, String)> {
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

/// Shared selective enrichment loop (8-item cap, 45s hard budget, 5-fail circuit breaker)
/// parameterized by provider source label and an async prompt -> JSON caller.
pub async fn run_selective_enrichment<F, P, Fut>(
    records: &mut [JournalRecord],
    source_label: &str,
    cancel_flag: Arc<AtomicBool>,
    on_progress: P,
    call_json: F,
) where
    P: Fn(usize, usize, &str) + Send + Sync,
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Value, String>> + Send,
{
    if GLOBAL_AI_BREAKER.is_open() {
        return;
    }

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
                match call_json(prompt).await {
                    Ok(val) => {
                        GLOBAL_AI_BREAKER.record_success();
                        if let Some((sub, pub_fee, cur)) = validate_apc_json(&val, &raw_snip) {
                            if let Some(s) = sub {
                                records[idx].submission_fee_idr = Some(s);
                            }
                            if let Some(p) = pub_fee {
                                records[idx].ai_validated = true;
                                records[idx].ai_source = source_label.to_string();
                                if p == 0 {
                                    records[idx].apc_status = "Gratis (Rp 0)".to_string();
                                    records[idx].publication_fee_idr = Some(0);
                                    records[idx].fee_currency = "IDR".to_string();
                                    records[idx].fee_amount = "0".to_string();
                                } else {
                                    let idr = if cur == "USD" {
                                        p * USD_TO_IDR_RATE
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
                match call_json(prompt).await {
                    Ok(val) => {
                        GLOBAL_AI_BREAKER.record_success();
                        if let Some(fs) = validate_focus_json(&val) {
                            records[idx].focus_and_scope = fs;
                            records[idx].ai_validated = true;
                            if records[idx].ai_source.is_empty() {
                                records[idx].ai_source = source_label.to_string();
                            }
                        }
                    }
                    Err(_) => GLOBAL_AI_BREAKER.record_failure(),
                }
            }

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
