//! Strict 3-status Article Processing Charge (APC) parser with Zero-Trap global scanning,
//! historical/OA-reader/submission-fee guards, and Regular > Fast-Track > Free priority.

use crate::config::{APC_SANITY_MAX_IDR, APC_SANITY_MIN_IDR, APC_UNKNOWN, USD_TO_IDR_RATE};
use crate::models::format_idr_short;
use crate::parser::focus::sanitize_html_text;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};

pub const APC_KEYWORDS: &[&str] = &[
    "apc",
    "article processing charge",
    "publication fee",
    "article publication",
    "biaya publikasi",
    "biaya penerbitan",
    "biaya artikel",
    "author fee",
    "submission fee",
    "page charge",
    "free of charge",
    "no charge",
    "gratis",
    "tanpa biaya",
    "bebas biaya",
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

// Sentences about Open-Access free reading/downloading must NEVER trigger APC Free.
static RE_OA_READER_STRIP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)[^.!?\n;]{0,140}(?:open\s+access[^.!?\n;]{0,120}(?:free|gratis|no\s+charge|readers?)|readers?\s+can\s+(?:read|download|access)[^.!?\n;]{0,140}(?:for\s+free(?:\s+of\s+charge)?|free(?:\s+of\s+charge)?|gratis|no\s+charge|without\s+charge)|(?:for\s+)?free\s+(?:of\s+charge\s+)?(?:for|to)\s+(?:read(?:ers?)?|download|access)|freely\s+available\s+to\s+(?:the\s+public|readers?|everyone)|without\s+(?:a\s+)?subscription|immediate\s+open\s+access|no\s+(?:subscription|access|reading|reader|paywall)\s+fee|membaca\s+(?:dan\s+mengunduh\s+)?(?:artikel\s+)?(?:secara\s+)?gratis|akses\s+terbuka[^.!?\n;]{0,80}gratis|journal\s+charges?\s+(?:nothing|no\s+fee)\s+to\s+read)[^.!?\n;]{0,100}",
    )
    .expect("valid regex")
});

// Strip Submission Fee = 0 / Article Submission = 0 so it isn't misclassified as Publication Fee = 0.
static RE_SUBMISSION_FREE_STRIP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:article\s+submission|submission\s+fees?|biaya\s+(?:submit|pengiriman|pendaftaran|awal)|manuscript\s+submission)\s*[:=\-]?\s*(?:(?:rp\.?|idr)\s*0(?:[,.]00)?|0(?:[,.]00)?\s*(?:\(?idr\)?|\(?rp\.?\)?|\(?usd\)?|\$)?|gratis|free(?:\s+of\s+charge)?|none|tidak\s+ada)\b",
    )
    .expect("valid regex")
});

static RE_PUB_FREE_STRICT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:(?:article\s+publication|article\s+processing\s+charges?|apc|publication\s+fees?|publishing\s+fees?|biaya\s+(?:publikasi|penerbitan|cetak|pemrosesan\s+artikel)|page\s+charges?)\s*[:=\-]?\s*(?:(?:rp\.?|idr)\s*0(?:[,.]00)?|0(?:[,.]00)?\s*(?:\(?idr\)?|\(?rp\.?\)?|\(?usd\)?|\$)?|gratis|free|none|tidak\s+(?:dipungut|dikenakan|ada)\s*biaya|bebas\s*biaya|tanpa\s*biaya|free\s*of\s*charge)\b|(?:free\s*of\s*charge|no\s+(?:publication\s+|apc\s+)?fees?|no\s+charge(?:\s+for\s+apc)?|tidak\s+(?:dipungut|dikenakan)\s*biaya|tanpa\s*biaya|bebas\s*biaya)\s*(?:for|to|pada|untuk)?\s*(?:publication|publishing|publish|publikasi|penerbitan|authors?|apc)\b|(?:publication|publishing|biaya\s+publikasi|jurnal\s+ini)\s+(?:is|adalah|bersifat|sebesar)?\s*(?:free|gratis|bebas\s*biaya|tanpa\s*biaya|free\s*of\s*charge))",
    )
    .expect("valid regex")
});

static RE_IDR_JUTA_RIBU: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:rp\.?|idr)?\s*(\d+(?:[.,]\d{1,2})?)\s*(juta|jt|million|ribu|rb|k)\b")
        .expect("valid regex")
});

static RE_IDR_AMOUNT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:rp\.?|idr)\s*[:.]?\s*([\d.,]+(?:,-)?)").expect("valid regex")
});

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

static RE_SUBMIT_AMOUNT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:article\s+submission|submission\s+fee|biaya\s+(?:submit|pengiriman))\s*[:=\-]?\s*(?:rp\.?|idr)\s*([\d.,]+)",
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
    let after_end = ceil_char_boundary(text, (end + 45).min(text.len()));
    let after = text[end..after_end].to_ascii_lowercase();

    let mentions_sub = before.contains("submission")
        || before.contains("biaya submit")
        || before.contains("pengiriman")
        || after.starts_with(" for submission")
        || after.starts_with(" (submission");

    let mentions_pub = before.contains("publicat")
        || before.contains("publish")
        || before.contains("publikasi")
        || before.contains("penerbitan")
        || before.contains("apc")
        || before.contains("processing charge")
        || before.contains("page charge")
        || before.contains("cetak")
        || before.contains("ready to issue")
        || before.contains("accepted")
        || after.contains("publication")
        || after.contains("publish")
        || after.contains("publikasi");

    mentions_sub && !mentions_pub
}

fn is_fast_track_context(text: &str, start: usize) -> bool {
    let clause_start = text[..start]
        .rfind(['.', ';', '\n'])
        .map(|i| i + 1)
        .unwrap_or(0)
        .max(start.saturating_sub(120));
    let window = &text[clause_start..start];

    let last_fast = RE_FAST_TRACK_TAG.find_iter(window).last().map(|m| m.start());
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

    candidates.sort_by_key(|c| c.start);
    candidates
}

/// Strip historical free statements, Open-Access free-reader phrases, and submission=0 statements
/// before checking for free-publication cues.
pub fn sanitize_free_traps(raw_text: &str) -> String {
    let s1 = RE_HISTORICAL_FREE_STRIP.replace_all(raw_text, " ");
    let s2 = RE_OA_READER_STRIP.replace_all(&s1, " ");
    RE_SUBMISSION_FREE_STRIP.replace_all(&s2, " ").into_owned()
}

/// Global Zero-Trap APC parser across the full sanitized text:
/// - **Paid Always Beats Free**
/// - **Regular Fee Beats Fast Track**
/// - **Free** only when NO paid regular fee exists AND an explicit publication-free statement remains after stripping historical/OA/submission traps.
pub fn parse_apc_from_context(raw_text: &str) -> ApcParsed {
    if raw_text.trim().is_empty() {
        return ApcParsed::Unknown;
    }

    // Strip historical free statements first so old year numbers/ranges never pollute context
    let historical_cleaned = RE_HISTORICAL_FREE_STRIP.replace_all(raw_text, " ");
    let paid_candidates = collect_paid_candidates(&historical_cleaned);

    // Priority 1: Regular Paid fee ALWAYS beats any earlier "IDR 0" / "No charge" or Fast-Track fee
    if let Some(reg) = paid_candidates.into_iter().find(|c| !c.is_fast_track) {
        return reg.parsed;
    }

    // Strip OA-reader & submission=0 traps before checking Free publication cues
    let free_scan = sanitize_free_traps(raw_text);
    if RE_PUB_FREE_STRICT.is_match(&free_scan) {
        return ApcParsed::Free;
    }

    // Fallback: if no explicit Free statement exists and only Fast-Track paid fee was advertised
    if let Some(ft) = collect_paid_candidates(&historical_cleaned).into_iter().next() {
        return ft.parsed;
    }

    ApcParsed::Unknown
}

pub fn extract_submission_fee_idr(raw_text: &str) -> Option<i64> {
    if RE_SUBMISSION_FREE_STRIP.is_match(raw_text) {
        return Some(0);
    }
    if let Some(cap) = RE_SUBMIT_AMOUNT.captures(raw_text) {
        return parse_idr_digits(&cap[1]);
    }
    None
}

/// Inspect ALL OJS HTML fee/editorial blocks globally without stopping at the first Free cue.
pub fn extract_apc_contextual_html(html: &str) -> (ApcParsed, Option<i64>) {
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
                    && !combined_blocks.iter().any(|existing: &String| existing == &cleaned)
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

    (parse_apc_from_context(&combined_text), sub_fee)
}

fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

pub fn apc_display_or_unknown(parsed: &ApcParsed) -> String {
    match parsed {
        ApcParsed::Free => "Gratis (Rp 0)".to_string(),
        ApcParsed::Paid { display, .. } => display.clone(),
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

        let free_pub = "Article Processing Charge (APC): Rp 0 (Gratis / Free of charge for publication).";
        assert_eq!(parse_apc_from_context(free_pub), ApcParsed::Free);
    }

    #[test]
    fn test_apc_usd_and_juta_parsing() {
        assert_eq!(
            parse_apc_from_context("Publication Fee: USD 50"),
            ApcParsed::Paid {
                currency: "USD".to_string(),
                display: "USD 50 (~Rp 875.000)".to_string(),
                amount_idr: 875_000,
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
}
