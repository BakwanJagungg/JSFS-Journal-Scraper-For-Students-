//! Isolated publish-month & schedule parser, filter, and issue estimator (Pantangan V3-6).

use chrono::{Datelike, Local};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;

pub const MONTH_NAMES_ID: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

pub const MONTH_SHORT_ID: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des",
];

static RE_WORDS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Za-z]+").expect("valid regex"));
static RE_X_PER_YEAR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(\d{1,2})\s*(?:kali|times?|x)\s*(?:setahun|a\s+year|per\s*(?:year|tahun)|annually)").expect("valid regex")
});
static RE_EVERY_MONTHS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:every|setiap|tiap)\s*(\d{1,2})\s*(?:months?|bulan)\b").expect("valid regex")
});

fn token_to_month(tok_original: &str, prev_tok: Option<&str>, next_tok: Option<&str>) -> Option<u8> {
    let low = tok_original.to_ascii_lowercase();
    match low.as_str() {
        "januari" | "january" | "jan" => Some(1),
        "februari" | "february" | "pebruari" | "feb" | "peb" => Some(2),
        "maret" | "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "mei" => Some(5),
        "may" => {
            // Disambiguate English modal verb "may" vs Month "May".
            let is_capitalized = tok_original.starts_with('M');
            let prev_l = prev_tok.map(str::to_ascii_lowercase).unwrap_or_default();
            let next_l = next_tok.map(str::to_ascii_lowercase).unwrap_or_default();
            let verb_following = matches!(
                next_l.as_str(),
                "be" | "have" | "not" | "submit" | "include" | "use" | "apply" | "also" | "only" | "require" | "vary" | "take" | "publish" | "appear"
            );
            if verb_following {
                return None;
            }
            let date_context = is_capitalized
                || matches!(prev_l.as_str(), "in" | "of" | "on" | "month" | "bulan" | "edisi" | "issue" | "and" | "dan")
                || matches!(next_l.as_str(), "and" | "dan" | "to" | "hingga" | "sampai");
            if date_context {
                Some(5)
            } else {
                None
            }
        }
        "juni" | "june" | "jun" => Some(6),
        "juli" | "july" | "jul" => Some(7),
        "agustus" | "august" | "agu" | "agt" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "oktober" | "october" | "okt" | "oct" => Some(10),
        "november" | "nov" | "nop" => Some(11),
        "desember" | "december" | "des" | "dec" => Some(12),
        _ => None,
    }
}

/// Extract ordered distinct publish month numbers (`1..=12`) from arbitrary text.
/// Applies a calendar-widget guard: if >= 11 distinct months appear without explicit monthly keywords,
/// treats the block as a calendar widget noise and returns empty.
pub fn extract_months_from_text(text: &str) -> Vec<u8> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let tokens: Vec<&str> = RE_WORDS.find_iter(text).map(|m| m.as_str()).collect();
    let mut found = BTreeSet::new();
    for (idx, tok) in tokens.iter().enumerate() {
        let prev = if idx > 0 { Some(tokens[idx - 1]) } else { None };
        let next = tokens.get(idx + 1).copied();
        if let Some(m) = token_to_month(tok, prev, next) {
            found.insert(m);
        }
    }

    if found.len() >= 11 {
        let low = text.to_ascii_lowercase();
        let explicit_monthly = low.contains("monthly")
            || low.contains("bulanan")
            || low.contains("every month")
            || low.contains("setiap bulan")
            || low.contains("tiap bulan")
            || low.contains("12 kali");
        if !explicit_monthly {
            return Vec::new();
        }
    }

    found.into_iter().collect()
}

pub fn month_nums_to_names_id(months: &[u8]) -> Vec<String> {
    months
        .iter()
        .filter_map(|&m| {
            if (1..=12).contains(&m) {
                Some(MONTH_NAMES_ID[(m - 1) as usize].to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Resolve both numeric months and a synchronized human-readable schedule string.
pub fn resolve_schedule_and_frequency(raw_text: &str) -> (Vec<u8>, Vec<String>, String) {
    let months = extract_months_from_text(raw_text);
    if !months.is_empty() {
        let names = month_nums_to_names_id(&months);
        let cnt = names.len();
        let joined = if cnt > 1 {
            format!("{} & {}", names[..cnt - 1].join(", "), names[cnt - 1])
        } else {
            names[0].clone()
        };
        let sched = format!("{cnt} kali setahun ({joined})");
        return (months, names, sched);
    }

    let low = raw_text.to_ascii_lowercase();
    if let Some(cap) = RE_X_PER_YEAR.captures(raw_text) {
        if let Ok(cnt) = cap[1].parse::<usize>() {
            if (1..=12).contains(&cnt) {
                return (Vec::new(), Vec::new(), format!("{cnt} kali setahun"));
            }
        }
    }
    if let Some(cap) = RE_EVERY_MONTHS.captures(raw_text) {
        if let Ok(step) = cap[1].parse::<usize>() {
            if step > 0 && 12 % step == 0 {
                let cnt = 12 / step;
                return (Vec::new(), Vec::new(), format!("{cnt} kali setahun"));
            }
        }
    }

    let freq_str = if low.contains("quarterly") || low.contains("triwulan") || low.contains("caturwulan") {
        if low.contains("caturwulan") {
            "3 kali setahun".to_string()
        } else {
            "4 kali setahun".to_string()
        }
    } else if low.contains("biannual") || low.contains("semi-annual") || low.contains("semiannual") || low.contains("twice a year") {
        "2 kali setahun".to_string()
    } else if low.contains("monthly") || low.contains("bulanan") {
        "12 kali setahun".to_string()
    } else if low.contains("annual") || low.contains("tahunan") {
        "1 kali setahun".to_string()
    } else {
        String::new()
    };

    (Vec::new(), Vec::new(), freq_str)
}

/// Extract publish months from OJS issue/archive headings (e.g., `"Vol 4 No 2 (Juni 2025)"`).
pub fn extract_months_from_archive_titles(titles: &[String]) -> Vec<u8> {
    let mut found = BTreeSet::new();
    for t in titles.iter().take(15) {
        for m in extract_months_from_text(t) {
            found.insert(m);
        }
    }
    if found.len() >= 11 {
        return Vec::new();
    }
    found.into_iter().collect()
}

/// Estimate the closest upcoming issue month relative to `current_month` (`1..=12`).
pub fn get_next_issue_estimate(months: &[u8], current_month: u8) -> String {
    if months.is_empty() {
        return "-".to_string();
    }
    let cur = current_month.clamp(1, 12);
    if months.contains(&cur) {
        let name = MONTH_NAMES_ID[(cur - 1) as usize];
        return format!("{name} (Bulan Ini)");
    }
    let next = months.iter().copied().find(|&m| m > cur).unwrap_or(months[0]);
    MONTH_NAMES_ID[(next - 1) as usize].to_string()
}

pub fn get_next_issue_estimate_now(months: &[u8]) -> String {
    let cur = Local::now().month() as u8;
    get_next_issue_estimate(months, cur)
}

/// Check whether journal publish months satisfy user's selected month filter.
pub fn lolos_filter_bulan(months: &[u8], selected: &[u8], include_unknown: bool) -> bool {
    if selected.is_empty() || selected.len() >= 12 {
        return true;
    }
    if months.is_empty() {
        return include_unknown;
    }
    months.iter().any(|m| selected.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_parsing_bilingual_and_may_guard() {
        let m = extract_months_from_text("Published twice a year in Mei and November. Authors may submit anytime.");
        assert_eq!(m, vec![5, 11]);

        // Lowercase modal verb "may" alone should NOT produce month 5
        let verb_only = extract_months_from_text("Manuscripts may be submitted online throughout the year.");
        assert!(verb_only.is_empty());

        // Capitalized May + September should match
        let cap_may = extract_months_from_text("Issues appear in May and September.");
        assert_eq!(cap_may, vec![5, 9]);
    }

    #[test]
    fn test_calendar_widget_guard() {
        let calendar = "January February March April May June July August September October November December";
        assert!(extract_months_from_text(calendar).is_empty());

        let explicit_monthly = "This monthly journal publishes 12 kali (Januari..Desember): January February March April May June July August September October November December";
        assert_eq!(extract_months_from_text(explicit_monthly).len(), 12);
    }

    #[test]
    fn test_next_issue_and_filter() {
        assert_eq!(get_next_issue_estimate(&[3, 9], 5), "September");
        assert_eq!(get_next_issue_estimate(&[3, 9], 10), "Maret");
        assert_eq!(get_next_issue_estimate(&[5, 11], 5), "Mei (Bulan Ini)");

        assert!(lolos_filter_bulan(&[3, 9], &[9, 10], false));
        assert!(!lolos_filter_bulan(&[3, 9], &[6, 7], false));
        assert!(lolos_filter_bulan(&[], &[6, 7], true));
        assert!(!lolos_filter_bulan(&[], &[6, 7], false));
    }
}
