//! Strict 3-status Article Processing Charge (APC) parser with Zero-Trap global scanning,
//! historical/OA-reader/submission-fee guards, and Regular > Fast-Track > Free priority.

use crate::config::{APC_SANITY_MAX_IDR, APC_SANITY_MIN_IDR, APC_UNKNOWN, USD_TO_IDR_RATE};
use crate::models::{format_apc_unified, format_idr_short, DisplayCurrency};
use crate::parser::focus::sanitize_html_text;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};

pub const APC_KEYWORDS: &[&str] = &[
    "apc",
    "article processing charge",
    "publication fee",
    "publication charge",
    "article publication",
    "article charge",
    "biaya publikasi",
    "biaya penerbitan",
    "biaya pemrosesan artikel",
    "biaya pengelolaan",
    "biaya artikel",
    "biaya cetak",
    "kontribusi penulis",
    "author contribution",
    "author fee",
    "submission fee",
    "submission charge",
    "page charge",
    "free of charge",
    "no charge",
    "gratis",
    "tanpa biaya",
    "bebas biaya",
    "bebas biaya publikasi",
    "tidak dipungut biaya",
    "rupiah",
    "idr",
    "usd",
];

// Historical year statements ("from 2008 to 2018 was free of charge", "prior to 2021 free")
// must NEVER classify current journal APC as Free.
static RE_HISTORICAL_FREE_STRIP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:(?:from|dari|sejak|during|between)\s+(?:tahun\s+)?(?:19|20)\d{2}\s*(?:to|until|hingga|sampai|s\.?d\.?|and|-|–)\s*(?:tahun\s+)?(?:19|20)\d{2}|(?:19|20)\d{2}\s*[-–]\s*(?:19|20)\d{2}|(?:prior\s+to|before|sebelum(?:\s+tahun)?|until|hingga|sampai(?:\s+tahun)?)\s+(?:19|20)\d{2})[^.!?\n]{0,140}?(?:free(?:\s+of\s+charge)?|no\s+(?:publication\s+|apc\s+)?charges?|no\s+fees?|gratis|waived|tanpa\s+biaya|bebas\s+biaya|(?:rp\.?|idr)\s*0(?:[,.]00)?|0(?:[,.]00)?\s*\(?idr\)?)",
    )
    .expect("valid regex")
});

// Withdrawal / retraction penalty fines ("Article Withdrawal ... punished by paying US$1000", "denda penarikan naskah")
// must ALWAYS be ignored so they are never misclassified as APC.
static RE_WITHDRAWAL_PENALTY_STRIP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)\b(?:article\s+withdrawal(?:\s+policy)?|withdrawal\s+(?:fee|charge|penalty|policy)|authors?\s+(?:who\s+)?withdraw\b|withdrawn|penarikan\s+(?:artikel|naskah)|denda\s+penarikan|punished\s+by\s+paying|retraction\s+(?:fee|charge|penalty))[^.!?\n;]{0,220}",
    )
    .expect("valid regex")
});

// Sentences about Open-Access free reading/downloading must NEVER trigger APC Free.
static RE_OA_READER_STRIP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)[^.!?\n;]{0,140}(?:open\s+access[^.!?\n;]{0,120}(?:free|gratis|no\s+charge|readers?)|readers?\s+can\s+(?:read|download|access)[^.!?\n;]{0,140}(?:for\s+free(?:\s+of\s+charge)?|free(?:\s+of\s+charge)?|gratis|no\s+charge|without\s+charge)|(?:for\s+)?free\s+(?:of\s+charge\s+)?(?:for|to)\s+(?:read(?:ers?)?|download|access)|freely\s+available\s+to\s+(?:the\s+public|readers?|everyone)|without\s+(?:a\s+)?subscription|immediate\s+open\s+access|no\s+(?:subscription|access|reading|reader|paywall)\s+fee|membaca\s+(?:dan\s+mengunduh\s+)?(?:artikel\s+)?(?:secara\s+)?gratis|akses\s+terbuka[^.!?\n;]{0,80}gratis|journal\s+charges?\s+(?:nothing|no\s+fee)\s+to\s+read)[^.!?\n;]{0,100}",
    )
    .expect("valid regex")
});

// Strip Submission Fee / Charge = 0 so it isn't misclassified as Publication Fee = 0.
static RE_SUBMISSION_FREE_STRIP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:article\s+submission(?:\s+(?:fees?|charges?))?|submission\s+(?:fees?|charges?)|biaya\s+(?:submit|pengiriman|pendaftaran|awal)|manuscript\s+submission(?:\s+(?:fees?|charges?))?)\s*[:=\-]?\s*(?:(?:rp\.?|idr)\s*0(?:[,.]00)?(?:\s*\(?idr\)?)?|0(?:[,.]00)?\s*(?:\(?idr\)?|\(?rp\.?\)?|\(?usd\)?|\$)?|gratis|free(?:\s+of\s+charge)?|none|tidak\s+ada)\b",
    )
    .expect("valid regex")
});

static RE_PUB_FREE_STRICT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:(?:article\s+publication(?:\s+(?:fees?|charges?))?|article\s+processing\s+charges?|apc|publication\s+(?:fees?|charges?)|publishing\s+(?:fees?|charges?)|biaya\s+(?:publikasi|penerbitan|cetak|pengelolaan|pemrosesan\s+artikel)|kontribusi\s+penulis|page\s+charges?)\s*[:=\-]?\s*(?:(?:rp\.?|idr)\s*0(?:[,.]00)?|0(?:[,.]00)?\s*(?:\(?idr\)?|\(?rp\.?\)?|\(?usd\)?|\$)?|gratis|free|none|tidak\s+(?:dipungut|dikenakan|ada)\s*biaya|bebas\s*biaya(?:\s+publikasi)?|tanpa\s*biaya|free\s*of\s*charge)\b|(?:free\s*of\s*charge|no\s+(?:publication\s+|apc\s+)?fees?|no\s+charge(?:\s+for\s+apc)?|tidak\s+(?:dipungut|dikenakan)\s*biaya|tanpa\s*biaya|bebas\s*biaya(?:\s+publikasi)?)\s*(?:for|to|pada|untuk)?\s*(?:publication|publishing|publish|publikasi|penerbitan|authors?|apc)\b|(?:publication|publishing|biaya\s+publikasi|jurnal\s+ini)\s+(?:is|adalah|bersifat|sebesar)?\s*(?:free|gratis|bebas\s*biaya(?:\s+publikasi)?|tanpa\s*biaya|free\s*of\s*charge)|(?:bebas\s+biaya\s+publikasi|tidak\s+dipungut\s+biaya\s+(?:publikasi|penerbitan)?))",
    )
    .expect("valid regex")
});

static RE_IDR_JUTA_RIBU: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:rp\.?|idr)?\s*(\d+(?:[.,]\d{1,2})?)\s*(juta|jt|million|ribu|rb|k)\b")
        .expect("valid regex")
});

static RE_IDR_AMOUNT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:rp\.?|idr)\s*[:.]?\s*([\d.,]+(?:,-)?)").expect("valid regex"));

static RE_IDR_TRAILING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(\d{1,3}(?:[.,]\d{3})+(?:[.,]\d{2})?|\d{5,8}(?:[.,]\d{2})?)(?:,-)?\s*(?:\(?\s*(?:idr|rp\.?|rupiah)\s*\)?)",
    )
    .expect("valid regex")
});

static RE_USD_AMOUNT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:usd|\$)\s*[:.]?\s*(\d+(?:[.,]\d{1,2})?)|\b(\d+(?:[.,]\d{1,2})?)\s*(?:\(?\s*(?:usd|dollar)\s*\)?)\b",
    )
    .expect("valid regex")
});

static RE_FOREIGN_CURRENCY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(€|£|¥|₹|A\$|\b(?:EUR|GBP|MYR|RM|AUD|JPY|INR)\b)\s*[:.]?\s*(\d+(?:[.,]\d{1,2})?)|\b(\d+(?:[.,]\d{1,2})?)\s*\(?\s*(EUR|GBP|MYR|AUD|JPY|INR)\s*\)?\b",
    )
    .expect("valid foreign currency regex")
});

static RE_SUBMIT_AMOUNT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:article\s+submission(?:\s+(?:fee|charge)s?)?|submission\s+(?:fee|charge)s?|biaya\s+(?:submit|pengiriman))\s*[:=\-]?\s*(?:rp\.?|idr)\s*([\d.,]+)",
    )
    .expect("valid regex")
});

static RE_FAST_TRACK_TAG: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:fast[\s\-]*track|jalur\s+cepat|express\s*(?:review|publish|publication|track)|layanan\s+cepat|percepatan)\b")
        .expect("valid regex")
});

static RE_REGULAR_TAG: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:regular|reguler|normal\s+(?:fee|review|publish|track)|standard\s+(?:fee|publication|track))\b")
        .expect("valid regex")
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApcParsed {
    Free,
    Paid {
        currency: String,
        display: String,
        amount_idr: i64,
        raw_amount: String,
    },
    Unknown,
}

#[derive(Debug, Clone)]
struct PaidCandidate {
    start: usize,
    is_fast_track: bool,
    parsed: ApcParsed,
}

/// Normalize Indonesian/US digit strings (e.g. `3.000.000,00`, `400.000,-`, `1.000,000`) to integer IDR.
fn parse_idr_digits(raw: &str) -> Option<i64> {
    let trimmed = raw.trim().trim_end_matches([',', '.', '-', '/']);
    if trimmed.is_empty() {
        return None;
    }

    let mut s = trimmed.to_string();
    // Drop decimal cents suffix (".00", ",00", ".0") while keeping thousands groups (".000", ",000")
    if let Some(idx) = s.rfind([',', '.']) {
        let suffix = &s[idx + 1..];
        if (1..=2).contains(&suffix.len()) && suffix.chars().all(|c| c.is_ascii_digit()) {
            s = s[..idx].to_string();
        }
    }

    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<i64>().ok()
}

fn is_submission_only_context(text: &str, start: usize, end: usize) -> bool {
    let clause_start = text[..start]
        .rfind(['.', ';', '\n'])
        .map(|i| i + 1)
        .unwrap_or(0)
        .max(start.saturating_sub(110));
    let before = text[clause_start..start].to_ascii_lowercase();

    let before_sub = before.contains("submission")
        || before.contains("biaya submit")
        || before.contains("pengiriman");
    let before_pub = before.contains("publicat")
        || before.contains("publish")
        || before.contains("publikasi")
        || before.contains("penerbitan")
        || before.contains("apc")
        || before.contains("processing charge")
        || before.contains("page charge")
        || before.contains("cetak")
        || before.contains("ready to issue")
        || before.contains("accepted");

    if before_sub && !before_pub {
        return true;
    }

    if is_withdrawal_penalty_context(text, start, end) {
        return true;
    }

    let after_end = ceil_char_boundary(text, (end + 28).min(text.len()));
    let after_raw = &text[end..after_end];
    let after_clause_end = after_raw.find(['.', ';', '\n']).unwrap_or(after_raw.len());
    let after = after_raw[..after_clause_end].to_ascii_lowercase();

    (after.contains("for submission") || after.contains("(submission)")) && !before_pub
}

fn is_fast_track_context(text: &str, start: usize) -> bool {
    let clause_start = text[..start]
        .rfind(['.', ';', '\n'])
        .map(|i| i + 1)
        .unwrap_or(0)
        .max(start.saturating_sub(120));
    let window = &text[clause_start..start];

    let last_fast = RE_FAST_TRACK_TAG
        .find_iter(window)
        .last()
        .map(|m| m.start());
    let last_reg = RE_REGULAR_TAG.find_iter(window).last().map(|m| m.start());

    match (last_fast, last_reg) {
        (Some(f), Some(r)) => f > r,
        (Some(_), None) => true,
        _ => false,
    }
}

fn collect_paid_candidates(scan_text: &str) -> Vec<PaidCandidate> {
    let mut candidates = Vec::new();

    // 1) "Rp 1,5 juta" / "500 rb" / "750k"
    for cap in RE_IDR_JUTA_RIBU.captures_iter(scan_text) {
        let m = match cap.get(0) {
            Some(m) => m,
            None => continue,
        };
        if is_submission_only_context(scan_text, m.start(), m.end()) {
            continue;
        }
        let num_str = cap[1].replace(',', ".");
        let unit = cap[2].to_ascii_lowercase();
        let val: f64 = match num_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let mult = match unit.as_str() {
            "juta" | "jt" | "million" => 1_000_000.0,
            "ribu" | "rb" | "k" => 1_000.0,
            _ => continue,
        };
        let idr = (val * mult).round() as i64;
        if (APC_SANITY_MIN_IDR..=APC_SANITY_MAX_IDR).contains(&idr) {
            candidates.push(PaidCandidate {
                start: m.start(),
                is_fast_track: is_fast_track_context(scan_text, m.start()),
                parsed: ApcParsed::Paid {
                    currency: "IDR".to_string(),
                    display: format_idr_short(idr),
                    amount_idr: idr,
                    raw_amount: idr.to_string(),
                },
            });
        }
    }

    // 2) "Rp. 400.000,-" / "IDR 3.000.000,00" / "Rp 1.000,000"
    for cap in RE_IDR_AMOUNT.captures_iter(scan_text) {
        let m = match cap.get(0) {
            Some(m) => m,
            None => continue,
        };
        if is_submission_only_context(scan_text, m.start(), m.end()) {
            continue;
        }
        if let Some(idr) = parse_idr_digits(&cap[1]) {
            if (APC_SANITY_MIN_IDR..=APC_SANITY_MAX_IDR).contains(&idr) {
                candidates.push(PaidCandidate {
                    start: m.start(),
                    is_fast_track: is_fast_track_context(scan_text, m.start()),
                    parsed: ApcParsed::Paid {
                        currency: "IDR".to_string(),
                        display: format_idr_short(idr),
                        amount_idr: idr,
                        raw_amount: idr.to_string(),
                    },
                });
            }
        }
    }

    // 3) Trailing currency marker: "3.000.000,00 (IDR)" / "400.000 Rupiah"
    for cap in RE_IDR_TRAILING.captures_iter(scan_text) {
        let m = match cap.get(0) {
            Some(m) => m,
            None => continue,
        };
        if is_submission_only_context(scan_text, m.start(), m.end()) {
            continue;
        }
        if let Some(idr) = parse_idr_digits(&cap[1]) {
            if (APC_SANITY_MIN_IDR..=APC_SANITY_MAX_IDR).contains(&idr) {
                candidates.push(PaidCandidate {
                    start: m.start(),
                    is_fast_track: is_fast_track_context(scan_text, m.start()),
                    parsed: ApcParsed::Paid {
                        currency: "IDR".to_string(),
                        display: format_idr_short(idr),
                        amount_idr: idr,
                        raw_amount: idr.to_string(),
                    },
                });
            }
        }
    }

    // 4) USD amounts: "USD 50", "$100"
    for cap in RE_USD_AMOUNT.captures_iter(scan_text) {
        let m = match cap.get(0) {
            Some(m) => m,
            None => continue,
        };
        if is_submission_only_context(scan_text, m.start(), m.end()) {
            continue;
        }
        let raw_num = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|g| g.as_str())
            .unwrap_or("0");
        let usd: f64 = match raw_num.replace(',', ".").parse() {
            Ok(v) if v > 0.0 => v,
            _ => continue,
        };
        let idr = (usd * USD_TO_IDR_RATE as f64).round() as i64;
        if (APC_SANITY_MIN_IDR..=APC_SANITY_MAX_IDR).contains(&idr) {
            let usd_disp = if (usd.fract()).abs() < 1e-6 {
                format!("{}", usd as i64)
            } else {
                format!("{usd:.2}")
            };
            candidates.push(PaidCandidate {
                start: m.start(),
                is_fast_track: is_fast_track_context(scan_text, m.start()),
                parsed: ApcParsed::Paid {
                    currency: "USD".to_string(),
                    display: format!("USD {usd_disp} (~{})", format_idr_short(idr)),
                    amount_idr: idr,
                    raw_amount: usd_disp,
                },
            });
        }
    }

    // 5) Multi-currency foreign APCs: EUR (€), GBP (£), MYR (RM), AUD (A$), JPY (¥), INR (₹)
    for cap in RE_FOREIGN_CURRENCY.captures_iter(scan_text) {
        let m = match cap.get(0) {
            Some(m) => m,
            None => continue,
        };
        if is_submission_only_context(scan_text, m.start(), m.end()) {
            continue;
        }
        let (sym_raw, num_raw) = if let (Some(sym), Some(num)) = (cap.get(1), cap.get(2)) {
            (sym.as_str(), num.as_str())
        } else if let (Some(num), Some(sym)) = (cap.get(3), cap.get(4)) {
            (sym.as_str(), num.as_str())
        } else {
            continue;
        };
        let amount: f64 = match num_raw.replace(',', ".").parse() {
            Ok(v) if v > 0.0 => v,
            _ => continue,
        };
        let (code, rate_idr) = match sym_raw.to_ascii_uppercase().as_str() {
            "EUR" | "€" => ("EUR", 17_800.0),
            "GBP" | "£" => ("GBP", 20_900.0),
            "MYR" | "RM" => ("MYR", 3_700.0),
            "AUD" | "A$" => ("AUD", 10_700.0),
            "JPY" | "¥" => ("JPY", 108.0),
            "INR" | "₹" => ("INR", 195.0),
            _ => continue,
        };
        let idr = (amount * rate_idr).round() as i64;
        if (APC_SANITY_MIN_IDR..=50_000_000).contains(&idr) {
            let amt_disp = if (amount.fract()).abs() < 1e-6 {
                format!("{}", amount as i64)
            } else {
                format!("{amount:.2}")
            };
            candidates.push(PaidCandidate {
                start: m.start(),
                is_fast_track: is_fast_track_context(scan_text, m.start()),
                parsed: ApcParsed::Paid {
                    currency: code.to_string(),
                    display: format!("{code} {amt_disp} (~{})", format_idr_short(idr)),
                    amount_idr: idr,
                    raw_amount: amt_disp,
                },
            });
        }
    }

    candidates.sort_by_key(|c| c.start);

    // Prefer RHS final total in same-clause tax/sum equations (e.g. "IDR 2,000,000 (+tax 11%) = IDR 2,220,000,-")
    let mut keep = vec![true; candidates.len()];
    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            let delta = candidates[j].start.saturating_sub(candidates[i].start);
            if delta > 70 {
                break;
            }
            if candidates[i].is_fast_track == candidates[j].is_fast_track {
                let between = &scan_text[candidates[i].start..candidates[j].start];
                if between.contains('=') && !between.contains('\n') && !between.contains(';') {
                    keep[i] = false;
                    break;
                }
            }
        }
    }
    let mut filtered = Vec::with_capacity(candidates.len());
    for (idx, c) in candidates.into_iter().enumerate() {
        if keep[idx] {
            filtered.push(c);
        }
    }
    filtered
}

fn is_withdrawal_penalty_context(text: &str, start: usize, end: usize) -> bool {
    let clause_start = text[..start]
        .rfind(['.', ';', '\n'])
        .map(|i| i + 1)
        .unwrap_or(0)
        .max(start.saturating_sub(140));
    let before = text[clause_start..start].to_ascii_lowercase();
    let before_penalty = before.contains("withdrawal")
        || before.contains("withdrawn")
        || before.contains("penarikan naskah")
        || before.contains("penarikan artikel")
        || before.contains("denda penarikan")
        || before.contains("punished by paying")
        || before.contains("retraction fee");
    let before_pub = before.contains("publication charge")
        || before.contains("publication fee")
        || before.contains("article processing charge")
        || before.contains("biaya publikasi");
    if before_penalty && !before_pub {
        return true;
    }

    let after_end = ceil_char_boundary(text, (end + 32).min(text.len()));
    let after = text[end..after_end].to_ascii_lowercase();
    after.contains("withdrawal fee")
        || after.contains("for withdrawal")
        || after.contains("retraction fee")
        || after.contains("denda penarikan")
}

/// Strip historical free statements, withdrawal penalties, Open-Access free-reader phrases,
/// and submission=0 statements before checking for free-publication cues.
pub fn sanitize_free_traps(raw_text: &str) -> String {
    let s0 = RE_WITHDRAWAL_PENALTY_STRIP.replace_all(raw_text, " ");
    let s1 = RE_HISTORICAL_FREE_STRIP.replace_all(&s0, " ");
    let s2 = RE_OA_READER_STRIP.replace_all(&s1, " ");
    RE_SUBMISSION_FREE_STRIP.replace_all(&s2, " ").into_owned()
}

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn extract_sentence_evidence(text: &str, focus_start: usize, focus_end: usize) -> String {
    let min_left = floor_char_boundary(text, focus_start.saturating_sub(95));
    let mut left = min_left;
    for (rel, ch) in text[min_left..focus_start].char_indices().rev() {
        let abs = min_left + rel;
        if ch == '\n' || ch == ';' {
            left = abs + ch.len_utf8();
            break;
        }
        if ch == '.' {
            let prev = text[min_left..abs].chars().next_back();
            let next = text[abs + 1..].chars().next();
            if next.is_some_and(char::is_whitespace)
                && prev.is_some_and(|p| !p.is_ascii_digit() && p != 'p' && p != 'P')
            {
                left = abs + 1;
                break;
            }
        }
    }

    let max_right = ceil_char_boundary(text, (focus_end + 85).min(text.len()));
    let mut right = max_right;
    for (rel, ch) in text[focus_end..max_right].char_indices() {
        let abs = focus_end + rel;
        if ch == '\n' || ch == ';' {
            right = abs;
            break;
        }
        if ch == '.' {
            let next = text[abs + 1..].chars().next();
            if next.is_none() || next.is_some_and(char::is_whitespace) {
                right = abs + 1;
                break;
            }
        }
    }

    let raw_snip = text[left..right]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = raw_snip
        .trim_start_matches(['-', '•', '*', ' ', ':', ','])
        .trim();
    if trimmed.chars().count() <= 140 {
        trimmed.to_string()
    } else {
        let cut: String = trimmed.chars().take(137).collect();
        if let Some(sp) = cut.rfind(' ') {
            format!("{}...", &cut[..sp])
        } else {
            format!("{cut}...")
        }
    }
}

/// Global Zero-Trap APC parser returning `(ApcParsed, evidence_snippet)`.
pub fn parse_apc_with_evidence(raw_text: &str) -> (ApcParsed, String) {
    if raw_text.trim().is_empty() {
        return (ApcParsed::Unknown, String::new());
    }

    let penalty_cleaned = RE_WITHDRAWAL_PENALTY_STRIP.replace_all(raw_text, " ");
    let historical_cleaned = RE_HISTORICAL_FREE_STRIP.replace_all(&penalty_cleaned, " ");
    let paid_candidates = collect_paid_candidates(&historical_cleaned);

    if let Some(reg) = paid_candidates.iter().find(|c| !c.is_fast_track) {
        let ev = extract_sentence_evidence(
            &historical_cleaned,
            reg.start,
            (reg.start + 24).min(historical_cleaned.len()),
        );
        return (reg.parsed.clone(), ev);
    }

    let free_scan = sanitize_free_traps(raw_text);
    if let Some(m) = RE_PUB_FREE_STRICT.find(&free_scan) {
        let ev = extract_sentence_evidence(&free_scan, m.start(), m.end());
        return (ApcParsed::Free, ev);
    }

    if let Some(ft) = paid_candidates.first() {
        let ev = extract_sentence_evidence(
            &historical_cleaned,
            ft.start,
            (ft.start + 24).min(historical_cleaned.len()),
        );
        return (ft.parsed.clone(), ev);
    }

    (ApcParsed::Unknown, String::new())
}

/// Global Zero-Trap APC parser across the full sanitized text:
/// - **Paid Always Beats Free**
/// - **Regular Fee Beats Fast Track**
/// - **Free** only when NO paid regular fee exists AND an explicit publication-free statement remains after stripping historical/OA/submission traps.
pub fn parse_apc_from_context(raw_text: &str) -> ApcParsed {
    parse_apc_with_evidence(raw_text).0
}

pub fn extract_submission_fee_idr(raw_text: &str) -> Option<u64> {
    if RE_SUBMISSION_FREE_STRIP.is_match(raw_text) {
        return Some(0);
    }
    if let Some(cap) = RE_SUBMIT_AMOUNT.captures(raw_text) {
        return parse_idr_digits(&cap[1])
            .filter(|&v| v >= 0)
            .map(|v| v as u64);
    }
    None
}

/// Inspect ALL OJS HTML fee/editorial blocks globally, returning `(ApcParsed, sub_fee, evidence)`.
pub fn extract_apc_contextual_html_with_evidence(html: &str) -> (ApcParsed, Option<u64>, String) {
    let doc = Html::parse_document(html);
    let full_text = sanitize_html_text(html);
    let sub_fee = extract_submission_fee_idr(&full_text);

    let block_selectors = [
        "#authorFees",
        ".author_fees",
        "#submissionFees",
        "#fees",
        "table.listing",
        "div.sub_item",
        "div.pkp_block",
        "[id*='fee' i]",
        "[id*='apc' i]",
        "[class*='fee' i]",
        "[id^='custom-']",
        ".custom_block",
        "div.pkp_page_content",
    ];

    let mut combined_blocks = Vec::new();
    for sel_str in block_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for el in doc.select(&sel) {
                let txt = el.text().collect::<Vec<_>>().join(" ");
                let cleaned = txt.split_whitespace().collect::<Vec<_>>().join(" ");
                if cleaned.is_empty() {
                    continue;
                }
                let low = cleaned.to_ascii_lowercase();
                if APC_KEYWORDS.iter().any(|k| low.contains(k))
                    && !combined_blocks
                        .iter()
                        .any(|existing: &String| existing == &cleaned)
                {
                    combined_blocks.push(cleaned);
                }
            }
        }
    }

    let combined_text = if combined_blocks.is_empty() {
        full_text
    } else {
        format!("{}\n{}", combined_blocks.join("\n"), full_text)
    };

    let (parsed, evidence) = parse_apc_with_evidence(&combined_text);
    (parsed, sub_fee, evidence)
}

/// Inspect ALL OJS HTML fee/editorial blocks globally without stopping at the first Free cue.
pub fn extract_apc_contextual_html(html: &str) -> (ApcParsed, Option<u64>) {
    let (parsed, sub_fee, _) = extract_apc_contextual_html_with_evidence(html);
    (parsed, sub_fee)
}

fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

pub fn apc_display_or_unknown(parsed: &ApcParsed) -> String {
    apc_display_for_currency(parsed, DisplayCurrency::Idr, USD_TO_IDR_RATE)
}

/// Format parsed APC cleanly in a single selected currency (`Idr` or `Usd`).
pub fn apc_display_for_currency(
    parsed: &ApcParsed,
    currency: DisplayCurrency,
    usd_to_idr: i64,
) -> String {
    match parsed {
        ApcParsed::Free => format_apc_unified("free", Some(0), "", currency, usd_to_idr),
        ApcParsed::Paid {
            amount_idr,
            display,
            ..
        } => {
            let numeric = (*amount_idr > 0).then_some(*amount_idr as u64);
            format_apc_unified("paid", numeric, display, currency, usd_to_idr)
        }
        ApcParsed::Unknown => APC_UNKNOWN.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apc_oa_reader_and_submission_guards() {
        let oa_only = "This journal provides immediate open access to its content and readers can read articles free of charge.";
        assert_eq!(parse_apc_from_context(oa_only), ApcParsed::Unknown);

        let sub_only = "Author Fees: Article Submission: 0.00 (IDR). Authors are required to pay for fast review if needed.";
        assert_eq!(parse_apc_from_context(sub_only), ApcParsed::Unknown);
        assert_eq!(extract_submission_fee_idr(sub_only), Some(0));

        let both = "Article Submission: 0 (IDR)\nArticle Publication Fee: Rp 750.000,-";
        assert_eq!(
            parse_apc_from_context(both),
            ApcParsed::Paid {
                currency: "IDR".to_string(),
                display: "Rp 750.000".to_string(),
                amount_idr: 750_000,
                raw_amount: "750000".to_string(),
            }
        );

        let free_pub =
            "Article Processing Charge (APC): Rp 0 (Gratis / Free of charge for publication).";
        assert_eq!(parse_apc_from_context(free_pub), ApcParsed::Free);
    }

    #[test]
    fn test_apc_usd_and_juta_parsing() {
        assert_eq!(
            parse_apc_from_context("Publication Fee: USD 50"),
            ApcParsed::Paid {
                currency: "USD".to_string(),
                display: "USD 50 (~Rp 825.000)".to_string(),
                amount_idr: 825_000,
                raw_amount: "50".to_string(),
            }
        );
        assert_eq!(
            parse_apc_from_context("Biaya Publikasi: Rp 1,5 juta per artikel"),
            ApcParsed::Paid {
                currency: "IDR".to_string(),
                display: "Rp 1.500.000".to_string(),
                amount_idr: 1_500_000,
                raw_amount: "1500000".to_string(),
            }
        );
    }

    #[test]
    fn test_jsinbis_historical_free_with_decimal_apc() {
        let html = r#"
        <html><body>
        <div id="custom-0" class="pkp_page_content">
            <p>The journal from 2008 to 2018 was free of charge. Readers can read and download any full-text articles for free of charge.</p>
            <p>Starting from Volume 9, Article Publication Charge (APC): 3.000.000,00 (IDR)</p>
        </div>
        </body></html>
        "#;
        let (parsed, _) = extract_apc_contextual_html(html);
        assert_eq!(
            parsed,
            ApcParsed::Paid {
                currency: "IDR".to_string(),
                display: "Rp 3.000.000".to_string(),
                amount_idr: 3_000_000,
                raw_amount: "3000000".to_string(),
            }
        );

        // Historical + OA-reader text alone without paid APC must NOT become Free
        let trap_only = "From 2008 to 2018 was free of charge. Readers can read and download any full-text articles for free of charge.";
        assert_eq!(parse_apc_from_context(trap_only), ApcParsed::Unknown);
    }

    #[test]
    fn test_jurbisman_numbered_list_trap_with_fast_track() {
        let jurbisman_text = r#"
            Author Fees:
            1. Article Submission: 0.00 (IDR)
            2. Article Processing Charge: IDR 0 (USD 0)
            3. Fast-Track Review: 0.00 (IDR)
            4. No charge for APC during peer review.
            5. Copyediting: IDR 0
            6. Proofreading: IDR 0
            7. After the article is received and ready to issue, There is an article Publication Fee Regular Rp. 400.000,- (IDR). Fast Track Publish Fee Rp.500.000 - Rp. 750.000,-
        "#;
        assert_eq!(
            parse_apc_from_context(jurbisman_text),
            ApcParsed::Paid {
                currency: "IDR".to_string(),
                display: "Rp 400.000".to_string(),
                amount_idr: 400_000,
                raw_amount: "400000".to_string(),
            }
        );
    }

    #[test]
    fn test_unified_currency_idr_and_usd_conversion() {
        use crate::models::{reformat_ui_results_currency, JournalRecord, UiJournalItem};

        let free_parsed = ApcParsed::Free;
        assert_eq!(
            apc_display_for_currency(&free_parsed, DisplayCurrency::Idr, 16_500),
            "Gratis (Rp 0)"
        );
        assert_eq!(
            apc_display_for_currency(&free_parsed, DisplayCurrency::Usd, 16_500),
            "Free ($0)"
        );

        let paid_500k = ApcParsed::Paid {
            currency: "IDR".to_string(),
            display: "Rp 500.000".to_string(),
            amount_idr: 500_000,
            raw_amount: "500000".to_string(),
        };
        assert_eq!(
            apc_display_for_currency(&paid_500k, DisplayCurrency::Idr, 16_500),
            "Rp 500.000"
        );
        assert_eq!(
            apc_display_for_currency(&paid_500k, DisplayCurrency::Usd, 16_500),
            "$30"
        );

        // Verify round-trip IDR -> USD -> IDR preserves exact apc_idr_numeric precision
        let mut items = vec![
            UiJournalItem::from(&JournalRecord {
                title: "Jurnal Nasional Berbayar".to_string(),
                apc_status: "Rp 500.000".to_string(),
                publication_fee_idr: Some(500_000),
                apc_idr_numeric: Some(500_000),
                ..Default::default()
            }),
            UiJournalItem::from(&JournalRecord {
                title: "OpenAlex Journal".to_string(),
                apc_status: "Rp 2.475.000".to_string(),
                publication_fee_idr: Some(2_475_000),
                apc_idr_numeric: Some(2_475_000),
                ..Default::default()
            }),
            UiJournalItem::from(&JournalRecord {
                title: "Diamond OA Free".to_string(),
                apc_status: "Gratis (Rp 0)".to_string(),
                publication_fee_idr: Some(0),
                apc_idr_numeric: Some(0),
                ..Default::default()
            }),
        ];

        reformat_ui_results_currency(&mut items, DisplayCurrency::Usd, 16_500);
        assert_eq!(items[0].apc_label, "$30");
        assert_eq!(items[1].apc_label, "$150");
        assert_eq!(items[2].apc_label, "Free ($0)");
        assert_eq!(items[0].apc_idr_numeric, Some(500_000));

        reformat_ui_results_currency(&mut items, DisplayCurrency::Idr, 16_500);
        assert_eq!(items[0].apc_label, "Rp 500.000");
        assert_eq!(items[1].apc_label, "Rp 2.475.000");
        assert_eq!(items[2].apc_label, "Gratis (Rp 0)");
    }

    #[test]
    fn test_submission_fee_separation_and_multi_currency_apc() {
        let html_split = r#"
            <html><body>
            <div id="authorFees">
                <p>Article Submission: Rp 150.000 (IDR)</p>
                <p>Article Publication Fee: Rp 650.000</p>
            </div>
            </body></html>
        "#;
        let (parsed, sub_fee, _) = extract_apc_contextual_html_with_evidence(html_split);
        assert_eq!(sub_fee, Some(150_000));
        match parsed {
            ApcParsed::Paid { amount_idr, .. } => assert_eq!(amount_idr, 650_000),
            other => panic!("expected Paid 650_000, got {other:?}"),
        }

        match parse_apc_from_context("Article Processing Charge: EUR 100") {
            ApcParsed::Paid {
                currency,
                amount_idr,
                ..
            } => {
                assert_eq!(currency, "EUR");
                assert_eq!(amount_idr, 1_780_000);
            }
            other => panic!("expected EUR 100, got {other:?}"),
        }

        match parse_apc_from_context("Publication fee is £50 for accepted manuscripts.") {
            ApcParsed::Paid {
                currency,
                amount_idr,
                ..
            } => {
                assert_eq!(currency, "GBP");
                assert_eq!(amount_idr, 1_045_000);
            }
            other => panic!("expected GBP 50, got {other:?}"),
        }

        match parse_apc_from_context("Publication fee RM 250 per paper.") {
            ApcParsed::Paid {
                currency,
                amount_idr,
                ..
            } => {
                assert_eq!(currency, "MYR");
                assert_eq!(amount_idr, 925_000);
            }
            other => panic!("expected MYR 250, got {other:?}"),
        }

        match parse_apc_from_context("Article processing fee: A$120") {
            ApcParsed::Paid { currency, .. } => assert_eq!(currency, "AUD"),
            other => panic!("expected AUD, got {other:?}"),
        }
        match parse_apc_from_context("APC: JPY 15000") {
            ApcParsed::Paid { currency, .. } => assert_eq!(currency, "JPY"),
            other => panic!("expected JPY, got {other:?}"),
        }
        match parse_apc_from_context("Publication charge: ₹1800") {
            ApcParsed::Paid { currency, .. } => assert_eq!(currency, "INR"),
            other => panic!("expected INR, got {other:?}"),
        }
    }

    #[test]
    fn test_v48_tax_equation_submission_charge_and_withdrawal_penalty_ignored() {
        let html = r#"
            <html><body>
            <div class="pkp_page_content">
                <p>Article Submission Charge: 0.00 (IDR)</p>
                <p>Article Publication Charge: IDR 2,000,000 (+tax 11%) = IDR 2,220,000,-</p>
                <p>Article Withdrawal Policy: Authors who withdraw their manuscript after review will be punished by paying US$1000 or denda penarikan naskah Rp 1.500.000,-.</p>
            </div>
            </body></html>
        "#;
        let (parsed, sub_fee, _) = extract_apc_contextual_html_with_evidence(html);
        assert_eq!(sub_fee, Some(0));
        match parsed {
            ApcParsed::Paid {
                currency,
                amount_idr,
                display,
                ..
            } => {
                assert_eq!(currency, "IDR");
                assert_eq!(amount_idr, 2_220_000);
                assert_eq!(display, "Rp 2.220.000");
            }
            other => panic!("expected Paid 2_220_000, got {other:?}"),
        }

        // Withdrawal penalty alone must never become APC even if > 50_000 IDR
        let penalty_only = "Article Withdrawal: Authors who withdraw their manuscript will be punished by paying US$100.";
        assert_eq!(parse_apc_from_context(penalty_only), ApcParsed::Unknown);
    }
}
