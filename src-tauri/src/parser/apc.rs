//! Strict 3-status Article Processing Charge (APC) parser with OA-reader & submission-fee guards.

use crate::config::{APC_UNKNOWN, USD_TO_IDR_RATE};
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

// Sentences about Open-Access free reading must NEVER trigger APC Free.
static RE_OA_READER_STRIP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)[^.!?\n]{0,180}(?:open\s+access[^.!?\n]{0,120}(?:free|gratis|no\s+charge|readers?)|free\s+(?:for|to)\s+read(?:ers?)?|no\s+(?:subscription|access)\s+fee|membaca\s+(?:secara\s+)?gratis|akses\s+terbuka[^.!?\n]{0,80}gratis|journal\s+charges?\s+(?:nothing|no\s+fee)\s+to\s+read)[^.!?\n]{0,180}",
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
        r"(?is)(?:(?:article\s+publication|article\s+processing\s+charges?|apc|publication\s+fees?|publishing\s+fees?|biaya\s+(?:publikasi|penerbitan|cetak|pemrosesan\s+artikel)|page\s+charges?)\s*[:=\-]?\s*(?:(?:rp\.?|idr)\s*0(?:[,.]00)?|0(?:[,.]00)?\s*(?:\(?idr\)?|\(?rp\.?\)?|\(?usd\)?|\$)?|gratis|free|none|tidak\s+(?:dipungut|dikenakan|ada)\s*biaya|bebas\s*biaya|tanpa\s*biaya|free\s*of\s*charge)\b|(?:free\s*of\s*charge|no\s+(?:publication\s+|apc\s+)?fees?|tidak\s+(?:dipungut|dikenakan)\s*biaya|tanpa\s*biaya|bebas\s*biaya)\s*(?:for|to|pada|untuk)?\s*(?:publication|publishing|publish|publikasi|penerbitan|authors?)\b|(?:publication|publishing|biaya\s+publikasi|jurnal\s+ini)\s+(?:is|adalah|bersifat|sebesar)?\s*(?:free|gratis|bebas\s*biaya|tanpa\s*biaya|free\s*of\s*charge))",
    )
    .expect("valid regex")
});

static RE_IDR_JUTA_RIBU: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:rp\.?|idr)?\s*(\d+(?:[.,]\d{1,2})?)\s*(juta|jt|million|ribu|rb|k)\b")
        .expect("valid regex")
});

static RE_IDR_AMOUNT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:rp\.?|idr)\s*([\d.,]+)").expect("valid regex")
});

static RE_IDR_TRAILING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(\d{1,3}(?:[.,]\d{3})+|\d{5,8})\s*(?:\(?\s*(?:idr|rp\.?|rupiah)\s*\)?)")
        .expect("valid regex")
});

static RE_USD_AMOUNT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:usd|\$)\s*(\d+(?:[.,]\d{1,2})?)|\b(\d+(?:[.,]\d{1,2})?)\s*(?:usd|dollar)\b")
        .expect("valid regex")
});

static RE_SUBMIT_AMOUNT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:article\s+submission|submission\s+fee|biaya\s+(?:submit|pengiriman))\s*[:=\-]?\s*(?:rp\.?|idr)\s*([\d.,]+)",
    )
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

fn parse_idr_digits(raw: &str) -> Option<i64> {
    let mut s = raw.trim().trim_end_matches(",-").trim_end_matches('.').to_string();
    // Handle decimal suffix like ".00" or ",00"
    if let Some(idx) = s.rfind([',', '.']) {
        let tail = &s[idx + 1..];
        if tail.len() == 2 && tail.chars().all(|c| c.is_ascii_digit()) {
            s.truncate(idx);
        }
    }
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    let val = digits.parse::<i64>().ok()?;
    // Guard against years (1990..2035) or tiny noise (< 10_000 unless handled by ribu/juta)
    if (1990..=2035).contains(&val) {
        return None;
    }
    if (10_000..=50_000_000).contains(&val) {
        Some(val)
    } else {
        None
    }
}

/// Extract publication fee strictly from cleaned contextual text.
pub fn parse_apc_from_context(raw_text: &str) -> ApcParsed {
    if raw_text.trim().is_empty() {
        return ApcParsed::Unknown;
    }
    let stripped_oa = RE_OA_READER_STRIP.replace_all(raw_text, " ");
    let cleaned = RE_SUBMISSION_FREE_STRIP.replace_all(&stripped_oa, " ");

    // Also remove lines mentioning submission fee explicitly before scanning numeric amounts
    // so submission cost doesn't masquerade as publication fee unless separated.
    let mut pub_focus_lines = Vec::new();
    for seg in cleaned.split(['\n', ';']) {
        let low = seg.to_ascii_lowercase();
        let is_sub_only = (low.contains("submission fee") || low.contains("article submission") || low.contains("biaya submit"))
            && !low.contains("publication")
            && !low.contains("publikasi")
            && !low.contains("apc")
            && !low.contains("processing");
        if !is_sub_only {
            pub_focus_lines.push(seg);
        }
    }
    let pub_text = pub_focus_lines.join("\n");

    // Check paid amounts first when paired with IDR / USD so "Submission 0, Publication Rp 500.000" yields Paid!
    if let Some(cap) = RE_IDR_JUTA_RIBU.captures(&pub_text) {
        let num_str = cap[1].replace(',', ".");
        let unit = cap[2].to_ascii_lowercase();
        if let Ok(base) = num_str.parse::<f64>() {
            let mult = if matches!(unit.as_str(), "juta" | "jt" | "million") {
                1_000_000.0
            } else {
                1_000.0
            };
            let idr = (base * mult).round() as i64;
            if (10_000..=50_000_000).contains(&idr) {
                return ApcParsed::Paid {
                    currency: "IDR".to_string(),
                    display: format_idr_short(idr),
                    amount_idr: idr,
                    raw_amount: idr.to_string(),
                };
            }
        }
    }

    if let Some(cap) = RE_IDR_AMOUNT.captures(&pub_text) {
        if let Some(idr) = parse_idr_digits(&cap[1]) {
            return ApcParsed::Paid {
                currency: "IDR".to_string(),
                display: format_idr_short(idr),
                amount_idr: idr,
                raw_amount: idr.to_string(),
            };
        }
    }

    if let Some(cap) = RE_IDR_TRAILING.captures(&pub_text) {
        if let Some(idr) = parse_idr_digits(&cap[1]) {
            return ApcParsed::Paid {
                currency: "IDR".to_string(),
                display: format_idr_short(idr),
                amount_idr: idr,
                raw_amount: idr.to_string(),
            };
        }
    }

    if let Some(cap) = RE_USD_AMOUNT.captures(&pub_text) {
        let num_raw = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().replace(',', "."))
            .unwrap_or_default();
        if let Ok(usd) = num_raw.parse::<f64>() {
            if (5.0..=5_000.0).contains(&usd) {
                let idr = (usd * USD_TO_IDR_RATE as f64).round() as i64;
                let usd_disp = if (usd - usd.round()).abs() < 0.01 {
                    format!("{}", usd.round() as i64)
                } else {
                    format!("{usd:.2}")
                };
                return ApcParsed::Paid {
                    currency: "USD".to_string(),
                    display: format!("USD {usd_disp} (~{})", format_idr_short(idr)),
                    amount_idr: idr,
                    raw_amount: usd_disp,
                };
            }
        }
    }

    // Only accept explicit free publication statement after guarding OA reader + Submission Fee
    if RE_PUB_FREE_STRICT.is_match(&pub_text) {
        return ApcParsed::Free;
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

/// Inspect OJS HTML with targeted fee blocks first, then short tags, then keyword windows.
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
        ".custom_block",
    ];

    for sel_str in block_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for el in doc.select(&sel) {
                let txt = el.text().collect::<Vec<_>>().join(" ");
                let low = txt.to_ascii_lowercase();
                if APC_KEYWORDS.iter().any(|k| low.contains(k)) {
                    let parsed = parse_apc_from_context(&txt);
                    if parsed != ApcParsed::Unknown {
                        return (parsed, sub_fee);
                    }
                }
            }
        }
    }

    // Fallback: keyword-window scanning across sanitized plain text
    let lower = full_text.to_ascii_lowercase();
    for kw in APC_KEYWORDS {
        let mut search_from = 0usize;
        while let Some(rel) = lower[search_from..].find(kw) {
            let abs = search_from + rel;
            let start = floor_char_boundary(&full_text, abs.saturating_sub(100));
            let end = ceil_char_boundary(&full_text, (abs + 350).min(full_text.len()));
            let snip = &full_text[start..end];
            let parsed = parse_apc_from_context(snip);
            if parsed != ApcParsed::Unknown {
                return (parsed, sub_fee);
            }
            search_from = abs + kw.len();
        }
    }

    (ApcParsed::Unknown, sub_fee)
}

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
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
        // OA statement alone must NOT mark publication fee as Free
        let oa_only = "This journal provides immediate open access to its content and readers can read articles free of charge.";
        assert_eq!(parse_apc_from_context(oa_only), ApcParsed::Unknown);

        // Submission fee = 0 alone must NOT mark publication fee as Free (Pantangan V3-5)
        let sub_only = "Author Fees: Article Submission: 0.00 (IDR). Authors are required to pay for fast review if needed.";
        assert_eq!(parse_apc_from_context(sub_only), ApcParsed::Unknown);
        assert_eq!(extract_submission_fee_idr(sub_only), Some(0));

        // Combined: Submission 0, Publication Rp 750.000 -> Paid 750.000
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

        // Explicit publication free
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
}
