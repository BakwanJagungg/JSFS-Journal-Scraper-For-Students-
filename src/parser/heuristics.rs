//! Lightweight heuristic enrichers for JSFS v4.4:
//! - Feature #1: Review & Publication Turnaround Time (`Received` -> `Accepted`/`Published`)
//! - Feature #2: Article Language Detection (`ID`, `EN`, `ID & EN`) & Manuscript Template URL (`.docx`/`.doc`)
//! - Feature #6: Predatory / Hijacked Journal Risk Detector (`evaluate_journal_risk`)

use crate::models::{ApcConfidence, JournalRecord};
use crate::scraper::dedup::normalize_issn_key;
use chrono::NaiveDate;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

static RE_EXPLICIT_WEEKS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:review\s*(?:process|time|period)?|waktu\s*review|lama\s*review|turnaround\s*time|time\s*to\s*(?:publication|decision)|proses\s*(?:review|publikasi))[^.\n]{0,45}?\b(\d{1,2})(?:\s*[-–to/]\s*(\d{1,2}))?\s*(weeks?|minggu|pekan|months?|bulan)\b",
    )
    .expect("valid explicit turnaround regex")
});

static RE_RECEIVED_LABEL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:received|submitted|diterima|tanggal\s*masuk|tgl\s*masuk|submission\s*date)\s*[:\-]?\s*([0-9]{1,4}[\s/\-\.][A-Za-z0-9]+[\s/\-\.,]+[0-9]{1,4}|\b[A-Za-z]{3,10}\s+\d{1,2},?\s+\d{4})")
        .expect("valid received regex")
});

static RE_PUBLISHED_LABEL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:published|accepted|diterbitkan|disetujui|available\s*online|tgl\s*terbit)\s*[:\-]?\s*([0-9]{1,4}[\s/\-\.][A-Za-z0-9]+[\s/\-\.,]+[0-9]{1,4}|\b[A-Za-z]{3,10}\s+\d{1,2},?\s+\d{4})")
        .expect("valid published regex")
});

static RE_PREDATORY_CLAIMS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:publish(?:ed)?\s+(?:within|in)\s+(?:24|48|72)\s*hours?|publish(?:ed)?\s+(?:within|in)\s+[1-5]\s*days?|fast\s+publish(?:ing)?\s+[1-3]\s*days?|guaranteed\s+acceptance|100%\s*acceptance|acceptance\s+guaranteed|without\s+peer\s+review|no\s+peer\s+review|terbit\s+dalam\s+(?:24\s*jam|[1-3]\s*hari)|pasti\s+diterima|tanpa\s+proses\s+review)\b",
    )
    .expect("valid predatory regex")
});

static RE_CITATION_STYLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:\b(?-i:APA)\s*(?:\d+(?:th|rd|nd)?\s*(?:ed(?:ition)?\.?)?|style|format|citation|referenc)|American\s+Psychological\s+Association|(?:sitasi|referensi|daftar\s+pustaka|citation|referencing|gaya\s+(?:selingkung|sitasi|penulisan)|reference\s+style|format\s+penulisan)[^.\n]{0,60}\b(?-i:APA|IEEE|MLA)\b|\b(?-i:IEEE)\s*(?:style|format|citation|referenc|template)\b|\b(Chicago|Vancouver|Harvard)\s*(?:style|format|manual|citation|referencing)\b|(?:sitasi|referensi|daftar\s+pustaka|citation|referencing|gaya\s+(?:selingkung|sitasi|penulisan)|reference\s+style)[^.\n]{0,60}\b(Chicago|Vancouver|Harvard)\b)",
    )
    .expect("valid citation style regex")
});

static RE_EDITOR_ROLE_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^(?:Editor[- ]in[- ]Chief|Chief\s+Editor|Ketua\s+(?:Dewan\s+)?(?:Redaksi|Editor|Penyunting)|Pemimpin\s+Redaksi|Principal\s+Editor|Managing\s+Editor)\b\s*[:\-–|]?\s*(.*)$",
    )
    .expect("valid editor role regex")
});

static RE_EDITOR_NAME_BEFORE_ROLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^([A-Za-z][A-Za-zÀ-ÖØ-öø-ÿ'.,\-\s]{4,68}?)\s*(?:\(|,|[-–])\s*(?:Editor[- ]in[- ]Chief|Chief\s+Editor|Ketua\s+(?:Dewan\s+)?Redaksi|Pemimpin\s+Redaksi|Principal\s+Editor)\b",
    )
    .expect("valid editor name-before-role regex")
});

static RE_HTML_BREAK_TAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<(?:br\s*/?|/(?:p|div|li|tr|h[1-6]|section))>").expect("regex"));
static RE_HTML_ANY_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<[^>]+>").expect("regex"));

static RE_WORD_RANGE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:between\s+|antara\s+)?(\d{1,2}[.,]?\d{3}|\d{3,5})\s*(?:-|–|—|to|and|sampai|hingga|s\.?d\.?)\s*(\d{1,2}[.,]?\d{3}|\d{3,5})\s*(?:words?|kata)\b",
    )
    .expect("valid word range regex")
});

static RE_WORD_MAX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:max(?:imum)?|maks(?:imal|imum)?|up\s*to|tidak\s+lebih\s+dari|paling\s+banyak)\s*(?:of\s+)?(\d{1,2}[.,]?\d{3}|\d{3,5})\s*(?:words?|kata)\b",
    )
    .expect("valid word max regex")
});

static RE_SIMILARITY_FORWARD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:similarity(?:\s+(?:index|score|rate|check|threshold|limit|level))?|plagiarisme?|plagiasi|kemiripan(?:\s+(?:naskah|teks|artikel))?|indeks\s+kemiripan|turnitin|ithenticate)\b[^\n.;]{0,55}?(?:<=|<|≤|max(?:imum)?(?:\s+of)?|maks(?:imal|imum)?|below|under|less\s+than|at\s+most|not\s+(?:exceed|more\s+than)|kurang\s+dari|di\s*bawah|tidak\s+(?:lebih\s+dari|melebihi)|batas(?:\s+maksimal)?|[:=])?\s*(\d{1,2})\s*%",
    )
    .expect("valid similarity forward regex")
});

static RE_SIMILARITY_BACKWARD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:<=|<|≤|max(?:imum)?|maks(?:imal|imum)?|below|under|less\s+than|at\s+most|not\s+(?:exceed|more\s+than)|kurang\s+dari|di\s*bawah|tidak\s+(?:lebih\s+dari|melebihi))\s*(\d{1,2})\s*%[^\n.;]{0,28}?\b(?:similarity|plagiarisme?|plagiasi|kemiripan|turnitin|ithenticate)\b",
    )
    .expect("valid similarity backward regex")
});

static RE_SUBMISSION_CLOSED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:registration\s+(?:is\s+)?(?:currently\s+)?closed|not\s+accepting\s+(?:new\s+)?submissions?|submissions?\s+(?:are|is)\s+(?:currently\s+)?closed|no\s+longer\s+accepting\s+submissions?|tidak\s+menerima\s+(?:naskah|artikel|pengiriman)|pendaftaran\s+(?:sedang\s+)?ditutup|kuota\s+(?:naskah\s+)?penuh)\b",
    )
    .expect("valid submission closed regex")
});

static RE_SUBMISSION_OPEN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:open\s+for\s+submissions?|currently\s+accepting\s+submissions?|accepting\s+(?:new\s+)?(?:submissions?|manuscripts?)|call\s+for\s+papers?|make\s+a\s+new\s+submission|undang(?:an)?\s+naskah|sedang\s+menerima\s+(?:naskah|artikel)|menerima\s+pengiriman\s+naskah|pendaftaran\s+(?:naskah\s+)?dibuka)\b",
    )
    .expect("valid submission open regex")
});

static RE_REDIRECT_HINT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)(?:website\s+(?:ini\s+)?telah\s+(?:ber)?pindah\s+ke|jurnal\s+(?:ini\s+)?telah\s+(?:ber)?pindah\s+ke|halaman\s+(?:telah\s+)?dipindahkan\s+ke|pindah\s+ke\s+alamat(?:\s+baru)?|moved\s+permanently\s+to|(?:website|journal|site)\s+has\s+moved\s+to|migrated\s+to)\s*[:\-]?\s*(https?://[^\s\x22'<>)\]]+)",
    )
    .expect("valid redirect phrase regex")
});

static RE_DOAJ_INDEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bdoaj\b|doaj\.org").expect("valid doaj regex"));
static RE_COPERNICUS_INDEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bindex\s*copernicus\b|\bcopernicus\b|journals\.indexcopernicus\.com")
        .expect("valid copernicus regex")
});
static RE_EBSCO_INDEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bebsco\b|ebscohost\.com").expect("valid ebsco regex"));
static RE_SCOPUS_INDEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bscopus\b|scopus\.com").expect("valid scopus regex"));
static RE_GSCHOLAR_INDEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bgoogle\s+scholar\b|scholar\.google\.").expect("valid gscholar regex")
});

/// Format turnaround weeks into compact human-readable pill label (`~8 mgg` or `~3 bln`).
pub fn format_turnaround_label(weeks: u16) -> String {
    if weeks == 0 {
        return String::new();
    }
    if weeks >= 12 {
        let months = ((weeks as f32) / 4.3).round().max(3.0) as u16;
        format!("~{months} bln")
    } else {
        format!("~{weeks} mgg")
    }
}

fn month_num_from_token(tok: &str) -> Option<u32> {
    let low = tok
        .trim_matches(|c: char| !c.is_ascii_alphabetic())
        .to_ascii_lowercase();
    match low.as_str() {
        "jan" | "january" | "januari" => Some(1),
        "feb" | "february" | "februari" | "pebruari" => Some(2),
        "mar" | "march" | "maret" => Some(3),
        "apr" | "april" => Some(4),
        "may" | "mei" => Some(5),
        "jun" | "june" | "juni" => Some(6),
        "jul" | "july" | "juli" => Some(7),
        "aug" | "august" | "agustus" | "agt" | "agu" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" | "oktober" | "okt" => Some(10),
        "nov" | "november" | "nopember" => Some(11),
        "dec" | "december" | "desember" | "des" => Some(12),
        _ => None,
    }
}

fn parse_flexible_date(raw: &str) -> Option<NaiveDate> {
    let s = raw.trim().trim_matches([',', '.', ';', ' ']);
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y/%m/%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d-%m-%Y") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d/%m/%Y") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d.%m.%Y") {
        return Some(d);
    }

    let cleaned = s.replace([',', '-', '/'], " ");
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if parts.len() == 3 {
        // Pattern 1: DD Month YYYY (e.g. "10 Maret 2025" or "10 March 2025")
        if let (Ok(day), Some(month), Ok(year)) = (
            parts[0].parse::<u32>(),
            month_num_from_token(parts[1]),
            parts[2].parse::<i32>(),
        ) {
            if (1990..=2100).contains(&year) {
                return NaiveDate::from_ymd_opt(year, month, day);
            }
        }
        // Pattern 2: Month DD YYYY (e.g. "March 10 2025")
        if let (Some(month), Ok(day), Ok(year)) = (
            month_num_from_token(parts[0]),
            parts[1].parse::<u32>(),
            parts[2].parse::<i32>(),
        ) {
            if (1990..=2100).contains(&year) {
                return NaiveDate::from_ymd_opt(year, month, day);
            }
        }
    }
    None
}

/// Extract estimated review/publication turnaround time in weeks from OJS page text.
pub fn extract_turnaround_time(text: &str) -> Option<(u16, String)> {
    // 1. Check Received -> Accepted/Published date pairs within nearby windows
    let mut diffs_weeks: Vec<u16> = Vec::new();
    for rec_cap in RE_RECEIVED_LABEL.captures_iter(text) {
        let Some(rec_match) = rec_cap.get(0) else {
            continue;
        };
        let Some(rec_date_str) = rec_cap.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let Some(start_date) = parse_flexible_date(rec_date_str) else {
            continue;
        };

        let window_start = rec_match.end();
        let window_end = (window_start + 320).min(text.len());
        let window = text.get(window_start..window_end).unwrap_or("");
        for pub_cap in RE_PUBLISHED_LABEL.captures_iter(window) {
            if let Some(pub_date_str) = pub_cap.get(1).map(|m| m.as_str()) {
                if let Some(end_date) = parse_flexible_date(pub_date_str) {
                    let days = (end_date - start_date).num_days();
                    if (7..=730).contains(&days) {
                        let weeks = ((days as f32) / 7.0).round().clamp(1.0, 104.0) as u16;
                        diffs_weeks.push(weeks);
                        break;
                    }
                }
            }
        }
        if diffs_weeks.len() >= 5 {
            break;
        }
    }

    if !diffs_weeks.is_empty() {
        diffs_weeks.sort_unstable();
        let median = diffs_weeks[diffs_weeks.len() / 2];
        return Some((median, format_turnaround_label(median)));
    }

    // 2. Check explicit statements ("review process takes 4-8 weeks", "waktu review 2 bulan")
    if let Some(cap) = RE_EXPLICIT_WEEKS.captures(text) {
        let n1 = cap
            .get(1)
            .and_then(|m| m.as_str().parse::<u16>().ok())
            .unwrap_or(0);
        let n2 = cap
            .get(2)
            .and_then(|m| m.as_str().parse::<u16>().ok())
            .unwrap_or(n1);
        let unit = cap
            .get(3)
            .map(|m| m.as_str().to_ascii_lowercase())
            .unwrap_or_default();
        if n1 > 0 {
            let avg = (n1 + n2).div_ceil(2);
            let weeks = if unit.starts_with("month") || unit.starts_with("bulan") {
                avg.saturating_mul(4)
            } else {
                avg
            };
            if (1..=104).contains(&weeks) {
                return Some((weeks, format_turnaround_label(weeks)));
            }
        }
    }

    None
}

/// Detect supported article languages (`"ID"`, `"EN"`, or `"ID & EN"`) from journal metadata and text.
pub fn detect_article_languages(
    title: &str,
    focus_scope: &str,
    sample_text: &str,
    is_international: bool,
) -> String {
    let combined = format!(
        "{title} {focus_scope} {}",
        sample_text.chars().take(2500).collect::<String>()
    )
    .to_ascii_lowercase();

    let mentions_both = (combined.contains("bahasa indonesia") || combined.contains("indonesian"))
        && combined.contains("english");
    if mentions_both
        || combined.contains("indonesia dan inggris")
        || combined.contains("indonesia atau inggris")
        || combined.contains("english or indonesian")
        || combined.contains("indonesian or english")
        || combined.contains("dwibahasa")
        || combined.contains("bilingual")
    {
        return "ID & EN".to_string();
    }
    if combined.contains("written in english only")
        || combined.contains("articles in english only")
        || combined.contains("hanya menerima artikel berbahasa inggris")
    {
        return "EN".to_string();
    }

    let id_tokens = [
        "dan",
        "yang",
        "pada",
        "untuk",
        "dalam",
        "dengan",
        "penelitian",
        "ilmiah",
        "jurnal",
        "analisis",
        "pengaruh",
        "terhadap",
        "mahasiswa",
        "indonesia",
        "pengembangan",
    ];
    let en_tokens = [
        "the",
        "and",
        "for",
        "with",
        "research",
        "study",
        "journal",
        "analysis",
        "international",
        "science",
        "engineering",
        "management",
        "education",
        "development",
        "published",
    ];

    let mut id_hits = 0usize;
    let mut en_hits = 0usize;
    for raw_w in combined.split(|c: char| !c.is_ascii_alphabetic()) {
        if raw_w.len() < 3 {
            continue;
        }
        if id_tokens.contains(&raw_w) {
            id_hits += 1;
        }
        if en_tokens.contains(&raw_w) {
            en_hits += 1;
        }
    }

    if id_hits >= 3 && en_hits >= 3 {
        "ID & EN".to_string()
    } else if id_hits > en_hits && id_hits >= 2 {
        "ID".to_string()
    } else if (en_hits > id_hits && en_hits >= 2) || is_international {
        "EN".to_string()
    } else {
        "ID".to_string()
    }
}

/// Discover direct manuscript template URL (`.docx`/`.doc` or explicit template page) from OJS HTML.
pub fn extract_template_url(html: &str, base_url_str: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let Ok(sel) = Selector::parse("a[href]") else {
        return None;
    };
    let base = Url::parse(base_url_str).ok();

    let mut fallback_guideline: Option<String> = None;

    for a in doc.select(&sel) {
        let Some(href) = a
            .value()
            .attr("href")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        if href.starts_with("javascript:") || href.starts_with("mailto:") || href == "#" {
            continue;
        }
        let resolved = if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else if let Some(ref b) = base {
            match b.join(href) {
                Ok(u) => u.to_string(),
                Err(_) => continue,
            }
        } else {
            continue;
        };

        let low_href = resolved.to_ascii_lowercase();
        let anchor_txt = a.text().collect::<Vec<_>>().join(" ").to_ascii_lowercase();
        let title_attr = a.value().attr("title").unwrap_or("").to_ascii_lowercase();

        let is_doc_ext = low_href.ends_with(".docx")
            || low_href.ends_with(".doc")
            || low_href.ends_with(".rtf")
            || low_href.ends_with(".odt")
            || low_href.contains(".docx?")
            || low_href.contains(".doc?");

        let mentions_template = anchor_txt.contains("template")
            || anchor_txt.contains("format artikel")
            || anchor_txt.contains("format penulisan")
            || title_attr.contains("template")
            || low_href.contains("template");

        if is_doc_ext
            || (mentions_template
                && (low_href.contains("drive.google.com")
                    || low_href.contains("docs.google.com")
                    || !low_href.ends_with(".pdf")))
        {
            return Some(resolved);
        }

        if fallback_guideline.is_none()
            && (mentions_template
                || anchor_txt.contains("author guidelines")
                || anchor_txt.contains("panduan penulis"))
        {
            fallback_guideline = Some(resolved);
        }
    }

    fallback_guideline
}

/// Evaluate lightweight rule-based predatory / hijacked risk warnings on a journal record.
pub fn evaluate_journal_risk(
    rec: &JournalRecord,
    raw_ojs_text: &str,
    ojs_meta_issn: Option<&str>,
    ojs_meta_eissn: Option<&str>,
) -> Option<String> {
    // 1. Check unrealistic rapid-publication / predatory claims
    let combined_text = format!("{} {}", rec.focus_and_scope, raw_ojs_text);
    if RE_PREDATORY_CLAIMS.is_match(&combined_text) {
        return Some(
            "Peringatan: Klaim terbit instan/tanpa review tidak wajar (indikasi predatory)"
                .to_string(),
        );
    }

    // 2. Check ISSN mismatch between authoritative portal metadata and OJS page meta tags
    let portal_p = normalize_issn_key(&rec.issn);
    let portal_e = normalize_issn_key(&rec.eissn);
    let page_p = ojs_meta_issn.map(normalize_issn_key).unwrap_or_default();
    let page_e = ojs_meta_eissn.map(normalize_issn_key).unwrap_or_default();

    let has_portal_issn = !portal_p.is_empty() || !portal_e.is_empty();
    let has_page_issn = !page_p.is_empty() || !page_e.is_empty();
    if has_portal_issn && has_page_issn {
        let mut any_match = false;
        for pp in [&portal_p, &portal_e].into_iter().filter(|s| !s.is_empty()) {
            for pg in [&page_p, &page_e].into_iter().filter(|s| !s.is_empty()) {
                if pp == pg {
                    any_match = true;
                }
            }
        }
        if !any_match {
            let p_disp = if !portal_e.is_empty() {
                &portal_e
            } else {
                &portal_p
            };
            let g_disp = if !page_e.is_empty() { &page_e } else { &page_p };
            return Some(format!(
                "Peringatan: ISSN di situs OJS ({g_disp}) berbeda dari data portal ({p_disp})"
            ));
        }
    }

    // 3. Check suspicious domain TLD, free hosts, or cloned/hijacked Indonesian campus domains
    if let Ok(u) = Url::parse(&rec.website_url) {
        if let Some(host) = u.host_str() {
            let h = host.to_ascii_lowercase();
            let is_official_id = h.ends_with(".ac.id")
                || h.ends_with(".or.id")
                || h.ends_with(".go.id")
                || h.ends_with(".sch.id");
            let suspicious_tlds = [
                ".xyz", ".top", ".biz", ".click", ".loan", ".work", ".gq", ".tk", ".cf", ".ml",
                ".icu",
            ];
            let free_hosts = ["blogspot.", "wordpress.com", "wixsite.com", "weebly.com"];
            if suspicious_tlds.iter().any(|ext| h.ends_with(ext))
                || free_hosts.iter().any(|fh| h.contains(fh))
            {
                return Some(format!(
                    "Peringatan: Domain situs ({h}) tidak lazim / indikasi hijacked untuk jurnal akademik resmi"
                ));
            }
            let aff_low = format!("{} {}", rec.publisher, rec.title).to_ascii_lowercase();
            let claims_id_campus = [
                "universitas",
                "politeknik",
                "institut teknologi",
                "institut agama",
                "institut sains",
                "sekolah tinggi",
                "uin ",
                "iain ",
                "stie ",
                "stik",
            ]
            .iter()
            .any(|kw| aff_low.contains(kw));
            if claims_id_campus
                && !is_official_id
                && !h.ends_with(".id")
                && !h.ends_with(".org")
                && !h.ends_with(".edu")
            {
                return Some(format!(
                    "Peringatan: Domain situs ({h}) indikasi cloned/non-resmi untuk afiliasi kampus Indonesia (bukan .ac.id/.or.id/.go.id/.sch.id)"
                ));
            }
        }
    }

    None
}

/// Validate an 8-character ISSN using the official ISO 3297 Modulo 11 checksum.
pub fn is_valid_issn_checksum(issn: &str) -> bool {
    let cleaned: String = issn
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if cleaned.len() != 8 {
        return false;
    }
    let bytes = cleaned.as_bytes();
    let mut sum: u32 = 0;
    for (idx, &b) in bytes[..7].iter().enumerate() {
        if !b.is_ascii_digit() {
            return false;
        }
        let digit = u32::from(b - b'0');
        let weight = 8 - idx as u32;
        sum += digit * weight;
    }
    let check_val = match bytes[7] {
        b'0'..=b'9' => u32::from(bytes[7] - b'0'),
        b'x' | b'X' => 10,
        _ => return false,
    };
    let rem = sum % 11;
    let expected = if rem == 0 { 0 } else { 11 - rem };
    check_val == expected
}

/// Validate p-ISSN and e-ISSN strings together: at least one valid ISSN digit sequence and none malformed.
pub fn has_valid_issn_pair(issn: &str, eissn: &str) -> bool {
    let mut found_any = false;
    for field in [issn, eissn] {
        for part in field.split([',', ';', '/']) {
            let trimmed = part.trim();
            if trimmed.chars().any(|c| c.is_ascii_digit()) {
                found_any = true;
                if !is_valid_issn_checksum(trimmed) {
                    return false;
                }
            }
        }
    }
    found_any
}

/// Compute token Jaccard similarity (0.0 to 1.0) between two journal titles
/// to detect hijacked journals or severe title mismatches.
pub fn jaccard_token_similarity(a: &str, b: &str) -> f32 {
    let stop = [
        "and",
        "dan",
        "of",
        "the",
        "in",
        "for",
        "on",
        "journal",
        "jurnal",
        "indonesian",
        "international",
    ];
    let get_tokens = |s: &str| -> std::collections::HashSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .map(|w| w.trim().to_ascii_lowercase())
            .filter(|w| w.len() >= 2 && !stop.contains(&w.as_str()))
            .collect()
    };
    let set_a = get_tokens(a);
    let set_b = get_tokens(b);
    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }
    if set_a.is_empty() || set_b.is_empty() {
        return 0.0;
    }
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    (intersection as f32) / (union as f32)
}

/// Check if one title is an abbreviation / acronym of another (e.g. "BEEI" vs "Bulletin of Electrical Engineering and Informatics").
pub fn is_acronym_match(a: &str, b: &str) -> bool {
    let clean_a = a.trim().to_ascii_lowercase();
    let clean_b = b.trim().to_ascii_lowercase();
    let (short, long) = if clean_a.len() <= 8 && clean_b.contains(' ') {
        (clean_a.as_str(), clean_b.as_str())
    } else if clean_b.len() <= 8 && clean_a.contains(' ') {
        (clean_b.as_str(), clean_a.as_str())
    } else {
        return false;
    };

    let stopwords = [
        "of", "and", "the", "in", "for", "dan", "di", "ke", "dari", "&", "-",
    ];
    let words: Vec<&str> = long
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    if words.is_empty() {
        return false;
    }

    // 1. Initials (full and non-stopword)
    let full_acr: String = words.iter().filter_map(|w| w.chars().next()).collect();
    let non_stop_acr: String = words
        .iter()
        .filter(|w| !stopwords.contains(&w.to_ascii_lowercase().as_str()))
        .filter_map(|w| w.chars().next())
        .collect();

    if full_acr == short
        || full_acr.contains(short)
        || non_stop_acr == short
        || non_stop_acr.contains(short)
    {
        return true;
    }

    // 2. Subsequence of initials (e.g. BEEI in boeeai)
    let mut chars = full_acr.chars();
    if short.chars().all(|sc| chars.any(|fc| fc == sc)) {
        return true;
    }

    // 3. High overlap with initials (e.g. JTIEMB vs JTIEB where 5 of 6 initials match)
    let matched_initials = short.chars().filter(|&sc| full_acr.contains(sc)).count();
    matched_initials >= 3 && (matched_initials as f64 / short.len() as f64) >= 0.75
}

/// Evaluate APC confidence level (High, Medium, NeedsCheck) and extract evidence snippet
/// using a +/-120 char context window and +3/-3 keyword weights.
pub fn score_apc_confidence(text: &str, candidate_pos: usize) -> (ApcConfidence, String) {
    let len = text.len();
    let start = candidate_pos.saturating_sub(120);
    let end = (candidate_pos + 120).min(len);
    let mut safe_start = start;
    while safe_start < candidate_pos && !text.is_char_boundary(safe_start) {
        safe_start += 1;
    }
    let mut safe_end = end;
    while safe_end > candidate_pos && !text.is_char_boundary(safe_end) {
        safe_end -= 1;
    }
    let window = &text[safe_start..safe_end];
    let low = window.to_ascii_lowercase();

    let mut score: i32 = 0;

    // Positive keywords (+3)
    let positives = [
        "apc",
        "article processing charge",
        "publication fee",
        "publication charge",
        "author fee",
        "biaya publikasi",
        "biaya penerbitan",
        "biaya pemrosesan",
        "free of charge",
        "no publication fee",
        "tidak dipungut biaya",
        "bebas biaya",
        "author contribution",
    ];
    for pos in positives {
        if low.contains(pos) {
            score += 3;
        }
    }

    // Negative keywords (-3)
    let negatives = [
        "fast track",
        "fast-track",
        "cetak",
        "hardcopy",
        "print",
        "langganan",
        "subscription",
        "withdrawal",
        "revisi",
        "denda",
    ];
    for neg in negatives {
        if low.contains(neg) {
            score -= 3;
        }
    }

    let confidence = if score >= 3 {
        ApcConfidence::High
    } else if score >= 0 {
        ApcConfidence::Medium
    } else {
        ApcConfidence::NeedsCheck
    };

    let snippet = window.split_whitespace().collect::<Vec<_>>().join(" ");
    let snippet_clean = if snippet.chars().count() > 140 {
        let cut: String = snippet.chars().take(137).collect();
        format!("{cut}...")
    } else {
        snippet
    };

    (confidence, snippet_clean)
}

/// Extract major secondary indexing badges/links mentioned on an OJS site (`DOAJ`, `Scopus`, `EBSCO`, `Copernicus`, `Google Scholar`).
pub fn extract_extra_indexes(html_or_text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let checks: [(&str, &Lazy<Regex>); 5] = [
        ("DOAJ", &RE_DOAJ_INDEX),
        ("Scopus", &RE_SCOPUS_INDEX),
        ("EBSCO", &RE_EBSCO_INDEX),
        ("Copernicus", &RE_COPERNICUS_INDEX),
        ("Google Scholar", &RE_GSCHOLAR_INDEX),
    ];
    for (label, re) in checks {
        if re.is_match(html_or_text) {
            out.push(label.to_string());
        }
    }
    out
}

/// Detect OJS 2 -> OJS 3 migration banners or `<meta http-equiv="refresh">` redirect targets in HTML.
pub fn extract_html_redirect_url(html: &str, plain: &str, base_url: &str) -> Option<String> {
    let base_parsed = Url::parse(base_url).ok()?;
    let base_norm = base_url.trim_end_matches('/').to_ascii_lowercase();

    let is_acceptable = |candidate: &Url| -> bool {
        if candidate.scheme() != "http" && candidate.scheme() != "https" {
            return false;
        }
        let s = candidate
            .as_str()
            .trim_end_matches('/')
            .to_ascii_lowercase();
        !(s == base_norm || s.contains("kemdiktisaintek.go.id") || s.contains("doi.org"))
    };

    // 1) Explicit Indonesian/English migration text followed directly by a URL
    if let Some(cap) = RE_REDIRECT_HINT.captures(plain) {
        if let Some(raw_url) = cap.get(1).map(|m| {
            m.as_str()
                .trim_end_matches(['.', ',', ';', ':', ')', ']', '"', '\''])
        }) {
            if let Ok(u) = base_parsed.join(raw_url) {
                if is_acceptable(&u) {
                    return Some(u.to_string());
                }
            }
        }
    }

    let doc = Html::parse_document(html);

    // 2) HTML <meta http-equiv="refresh" content="0;url=...">
    if let Ok(meta_sel) = Selector::parse("meta[http-equiv]") {
        for el in doc.select(&meta_sel) {
            let eq = el.value().attr("http-equiv").unwrap_or("");
            if eq.eq_ignore_ascii_case("refresh") {
                if let Some(content) = el.value().attr("content") {
                    let low = content.to_ascii_lowercase();
                    if let Some(pos) = low.find("url=") {
                        let raw = content[pos + 4..]
                            .trim()
                            .trim_matches(|c| c == '\'' || c == '"');
                        if let Ok(u) = base_parsed.join(raw) {
                            if is_acceptable(&u) {
                                return Some(u.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // 3) Anchor link paired with OJS migration phrase inside main content/announcements
    if let Ok(a_sel) = Selector::parse("a[href]") {
        let plain_low = plain.to_ascii_lowercase();
        if plain_low.contains("berpindah")
            || plain_low.contains("dipindahkan")
            || plain_low.contains("moved permanently")
            || plain_low.contains("has moved to")
        {
            for el in doc.select(&a_sel) {
                let text = el.text().collect::<Vec<_>>().join(" ").to_ascii_lowercase();
                let href = el.value().attr("href").unwrap_or("").trim();
                if href.starts_with("http")
                    && (text.contains("http")
                        || text.contains("sini")
                        || text.contains("here")
                        || text.contains("new")
                        || text.contains("baru")
                        || text.contains("ojs"))
                {
                    if let Ok(u) = base_parsed.join(href) {
                        if is_acceptable(&u) {
                            return Some(u.to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Extract canonical citation style (`APA`, `IEEE`, `Chicago`, `Vancouver`, `Harvard`, `MLA`) from OJS text.
pub fn extract_citation_style(text: &str) -> Option<String> {
    if let Some(m) = RE_CITATION_STYLE.find(text) {
        let s = m.as_str();
        let low = s.to_ascii_lowercase();
        if s.contains("IEEE") {
            return Some("IEEE".to_string());
        }
        if s.contains("APA") || low.contains("american psychological association") {
            return Some("APA".to_string());
        }
        if low.contains("chicago") {
            return Some("Chicago".to_string());
        }
        if low.contains("vancouver") {
            return Some("Vancouver".to_string());
        }
        if low.contains("harvard") {
            return Some("Harvard".to_string());
        }
        if s.contains("MLA") {
            return Some("MLA".to_string());
        }
    }
    None
}

fn parse_word_count_token(tok: &str) -> Option<u32> {
    let digits: String = tok.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().ok()
}

/// Extract manuscript word-count bounds `(min_words, max_words)` from OJS author guidelines.
pub fn extract_word_limit(text: &str) -> Option<(u32, u32)> {
    for cap in RE_WORD_RANGE.captures_iter(text) {
        let min_w = cap.get(1).and_then(|m| parse_word_count_token(m.as_str()));
        let max_w = cap.get(2).and_then(|m| parse_word_count_token(m.as_str()));
        if let (Some(a), Some(b)) = (min_w, max_w) {
            if a >= 500 && b <= 50_000 && a < b {
                return Some((a, b));
            }
        }
    }
    for cap in RE_WORD_MAX.captures_iter(text) {
        if let Some(max_w) = cap.get(1).and_then(|m| parse_word_count_token(m.as_str())) {
            if (1_000..=50_000).contains(&max_w) {
                return Some((0, max_w));
            }
        }
    }
    None
}

/// Detect whether the journal is currently open for submissions / CFP (`Some(true)`) or closed (`Some(false)`).
pub fn extract_submission_open(text: &str) -> Option<bool> {
    if RE_SUBMISSION_CLOSED.is_match(text) {
        return Some(false);
    }
    if RE_SUBMISSION_OPEN.is_match(text) {
        return Some(true);
    }
    None
}

fn clean_editor_name_candidate(raw: &str) -> Option<String> {
    let mut s = raw
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .trim()
        .to_string();
    for sep in ["(", "[", "|", " - ", " – ", "mailto:", "http", "@"] {
        if let Some(pos) = s.to_ascii_lowercase().find(sep) {
            // Avoid truncating before email if `@` is at end of line; take part before it
            s = s[..pos].trim().to_string();
        }
    }
    // Strip trailing affiliation starting after `, ` if an institutional keyword follows
    if let Some(pos) = s
        .to_ascii_lowercase()
        .find(", universitas")
        .or_else(|| s.to_ascii_lowercase().find(", university"))
        .or_else(|| s.to_ascii_lowercase().find(", institut"))
        .or_else(|| s.to_ascii_lowercase().find(", politeknik"))
        .or_else(|| s.to_ascii_lowercase().find(", fakultas"))
        .or_else(|| s.to_ascii_lowercase().find(", faculty"))
        .or_else(|| s.to_ascii_lowercase().find(", department"))
        .or_else(|| s.to_ascii_lowercase().find(", school of"))
    {
        s = s[..pos].trim().to_string();
    }

    let cleaned = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| matches!(c, ':' | '-' | '–' | ',' | ';'))
        .trim()
        .to_string();

    if cleaned.len() < 5 || cleaned.len() > 75 {
        return None;
    }
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.len() < 2 || words.len() > 9 {
        return None;
    }
    if cleaned.chars().filter(|c| c.is_ascii_digit()).count() > 0 {
        return None;
    }

    let low = cleaned.to_ascii_lowercase();
    let blocked = [
        "editorial team",
        "editorial board",
        "dewan redaksi",
        "chief editor",
        "editor in chief",
        "editor-in-chief",
        "managing editor",
        "about the journal",
        "contact",
        "submission",
        "email",
        "phone",
        "telepon",
        "address",
        "alamat",
        "scopus",
        "sinta",
        "orcid",
        "affiliation",
    ];
    if blocked.iter().any(|b| low.contains(b)) {
        return None;
    }
    if !cleaned.chars().any(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(cleaned)
}

/// Extract the *Editor-in-Chief* / *Ketua Dewan Redaksi* name from OJS editorial/contact HTML or text.
pub fn extract_editor_in_chief(html_or_text: &str) -> Option<String> {
    let text = if html_or_text.contains('<') && html_or_text.contains('>') {
        let with_breaks = RE_HTML_BREAK_TAG.replace_all(html_or_text, "\n");
        RE_HTML_ANY_TAG.replace_all(&with_breaks, " ").into_owned()
    } else {
        html_or_text.to_string()
    };

    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    for (idx, line) in lines.iter().enumerate() {
        if let Some(cap) = RE_EDITOR_NAME_BEFORE_ROLE.captures(line) {
            if let Some(m) = cap.get(1) {
                if let Some(name) = clean_editor_name_candidate(m.as_str()) {
                    return Some(name);
                }
            }
        }
        if let Some(cap) = RE_EDITOR_ROLE_LINE.captures(line) {
            let rest = cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            if !rest.is_empty() {
                if let Some(name) = clean_editor_name_candidate(rest) {
                    return Some(name);
                }
            } else if let Some(next_line) = lines.get(idx + 1) {
                if let Some(name) = clean_editor_name_candidate(next_line) {
                    return Some(name);
                }
            }
        }
    }

    None
}

/// Extract realistic maximum similarity / plagiarism percentage (`1..=50%`) from OJS text.
pub fn extract_max_similarity_pct(text: &str) -> Option<u8> {
    if text.trim().is_empty() {
        return None;
    }
    for cap in RE_SIMILARITY_FORWARD
        .captures_iter(text)
        .chain(RE_SIMILARITY_BACKWARD.captures_iter(text))
    {
        if let Some(m) = cap.get(1) {
            if let Ok(pct) = m.as_str().parse::<u8>() {
                if (1..=50).contains(&pct) {
                    return Some(pct);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turnaround_time_extraction_dates_and_explicit() {
        let article_history = "Article History: Received: 10 January 2025; Revised: 20 February 2025; Accepted: 07 March 2025; Published: 15 March 2025.";
        let (weeks, label) = extract_turnaround_time(article_history).expect("should parse dates");
        assert!(
            (8..=10).contains(&weeks),
            "expected ~8-9 weeks, got {weeks}"
        );
        assert!(label.contains("mgg"));

        let explicit_id = "Proses review memakan waktu 12 - 16 minggu sejak naskah dikirim.";
        let (w2, l2) = extract_turnaround_time(explicit_id).expect("should parse explicit weeks");
        assert_eq!(w2, 14);
        assert_eq!(l2, "~3 bln");
    }

    #[test]
    fn test_language_and_template_url_detection() {
        let lang_both = detect_article_languages(
            "Jurnal Sistem Informasi",
            "Accepts manuscripts written in Bahasa Indonesia and English covering information systems.",
            "",
            false,
        );
        assert_eq!(lang_both, "ID & EN");

        let html = r#"
        <html><body>
          <div class="block_custom">
            <a href="/public/journals/1/template-jurnal-2026.docx">Download Article Template</a>
          </div>
        </body></html>"#;
        let tpl = extract_template_url(html, "https://ejournal.kampus.ac.id/index.php/jsi")
            .expect("should find docx template");
        assert_eq!(
            tpl,
            "https://ejournal.kampus.ac.id/public/journals/1/template-jurnal-2026.docx"
        );
    }

    #[test]
    fn test_evaluate_journal_risk_predatory_and_issn_mismatch() {
        let mut rec = JournalRecord {
            title: "International Journal of Fast Science".to_string(),
            issn: "1234-5678".to_string(),
            website_url: "https://fastjournal.org/index.php/ijfs".to_string(),
            ..Default::default()
        };
        let warn1 = evaluate_journal_risk(
            &rec,
            "We offer guaranteed acceptance and publish in 48 hours for all authors!",
            None,
            None,
        );
        assert!(warn1.is_some());
        assert!(warn1.unwrap().contains("predatory"));

        let warn2 = evaluate_journal_risk(&rec, "Normal peer review.", Some("9999-8888"), None);
        assert!(warn2.is_some());
        assert!(warn2.unwrap().contains("ISSN"));

        rec.website_url = "https://scopus-clone-journal.xyz/ojs".to_string();
        let warn3 = evaluate_journal_risk(&rec, "Normal peer review.", Some("1234-5678"), None);
        assert!(warn3.is_some());
        assert!(warn3.unwrap().contains("Domain"));
    }

    #[test]
    fn test_citation_style_and_word_limit_extraction() {
        let g1 = "Author Guidelines: Manuscripts should be between 4.000 - 7.000 kata and follow APA 7th edition reference style. Note: apa saja topik diterima.";
        assert_eq!(extract_citation_style(g1).as_deref(), Some("APA"));
        assert_eq!(extract_word_limit(g1), Some((4000, 7000)));

        let g2 = "Format sitasi menggunakan IEEE format. Panjang artikel 3000-5000 words.";
        assert_eq!(extract_citation_style(g2).as_deref(), Some("IEEE"));
        assert_eq!(extract_word_limit(g2), Some((3000, 5000)));

        let g3 = "Gaya selingkung jurnal ini adalah apa adanya tanpa ketentuan khusus.";
        assert_eq!(extract_citation_style(g3), None);

        let g4 = "Naskah menggunakan gaya sitasi Chicago style dengan panjang maksimal 6.000 kata.";
        assert_eq!(extract_citation_style(g4).as_deref(), Some("Chicago"));
        assert_eq!(extract_word_limit(g4), Some((0, 6000)));

        let g5 = "Referencing follows Vancouver style.";
        assert_eq!(extract_citation_style(g5).as_deref(), Some("Vancouver"));
    }

    #[test]
    fn test_submission_open_detection() {
        assert_eq!(
            extract_submission_open("Currently open for submissions and Call for Papers 2026."),
            Some(true)
        );
        assert_eq!(
            extract_submission_open("Jurnal ini sedang menerima naskah ilmiah."),
            Some(true)
        );
        assert_eq!(
            extract_submission_open("Notice: Registration is currently closed until next year."),
            Some(false)
        );
        assert_eq!(
            extract_submission_open("Mohon maaf, pendaftaran ditutup karena kuota penuh."),
            Some(false)
        );
        assert_eq!(
            extract_submission_open("About the Journal and Editorial Team."),
            None
        );
    }

    #[test]
    fn test_extract_max_similarity_pct() {
        assert_eq!(
            extract_max_similarity_pct("Persyaratan naskah: maksimal similarity 20% via Turnitin."),
            Some(20)
        );
        assert_eq!(
            extract_max_similarity_pct(
                "Jurnal menerapkan batas plagiarisme 25% untuk semua artikel."
            ),
            Some(25)
        );
        assert_eq!(
            extract_max_similarity_pct("Indeks kemiripan maksimal 15% menggunakan iThenticate."),
            Some(15)
        );
        assert_eq!(
            extract_max_similarity_pct("Turnitin maksimal 20%, artikel di atas batas ditolak."),
            Some(20)
        );
        assert_eq!(
            extract_max_similarity_pct("Tingkat kemiripan naskah tidak lebih dari 20%."),
            Some(20)
        );
        assert_eq!(
            extract_max_similarity_pct(
                "The similarity index must be below 20% excluding bibliography."
            ),
            Some(20)
        );
        assert_eq!(
            extract_max_similarity_pct(
                "Maximum similarity of 25% is allowed by the editorial board."
            ),
            Some(25)
        );
        assert_eq!(
            extract_max_similarity_pct("All papers undergo plagiarism check below 15%."),
            Some(15)
        );
        assert_eq!(
            extract_max_similarity_pct("Manuscripts must have less than 20% similarity score."),
            Some(20)
        );
        assert_eq!(
            extract_max_similarity_pct("Author submission gate: Turnitin <= 20%."),
            Some(20)
        );
        // Must ignore VAT/tax or unrealistic percentages (>50%).
        assert_eq!(
            extract_max_similarity_pct(
                "Article Publication Charge: IDR 2,000,000 (+tax 11%) = IDR 2,220,000,-"
            ),
            None
        );
        assert_eq!(
            extract_max_similarity_pct("Discount 15% for early bird registration. Similarity check uses Turnitin 80% original."),
            None
        );
    }

    #[test]
    fn test_issn_checksum_cloned_domain_indexes_and_redirect() {
        assert!(is_valid_issn_checksum("2502-2377"));
        assert!(is_valid_issn_checksum("0378-5955"));
        assert!(is_valid_issn_checksum("2434-561X"));
        assert!(!is_valid_issn_checksum("2502-2379"));
        assert!(!is_valid_issn_checksum("1234-567"));
        assert!(has_valid_issn_pair("-", "2502-2377"));
        assert!(!has_valid_issn_pair("2502-2377", "2502-2379"));

        let rec = JournalRecord {
            title: "Jurnal Informatika Universitas Nusantara".to_string(),
            publisher: "Universitas Nusantara".to_string(),
            website_url: "https://jurnal-unik.biz/ojs".to_string(),
            ..Default::default()
        };
        let warn = evaluate_journal_risk(&rec, "Peer review process.", None, None);
        assert!(warn.is_some());
        assert!(warn.unwrap().contains("Domain"));

        let rec_ok = JournalRecord {
            title: "Jurnal Sistem Informasi".to_string(),
            publisher: "Universitas Contoh".to_string(),
            website_url: "https://ejournal.contoh.ac.id/index.php/jsi".to_string(),
            ..Default::default()
        };
        assert!(evaluate_journal_risk(&rec_ok, "Peer review", None, None).is_none());

        let idx_html = r#"Indexed by <a href="https://doaj.org">DOAJ</a>, Copernicus, EBSCO, and Google Scholar."#;
        let idx = extract_extra_indexes(idx_html);
        assert_eq!(
            idx,
            vec![
                "DOAJ".to_string(),
                "EBSCO".to_string(),
                "Copernicus".to_string(),
                "Google Scholar".to_string()
            ]
        );

        let mig_html = r#"<html><body><p>Website ini telah berpindah ke https://ojs3.kampus.ac.id/index.php/ji</p></body></html>"#;
        let redir = extract_html_redirect_url(
            mig_html,
            "Website ini telah berpindah ke https://ojs3.kampus.ac.id/index.php/ji",
            "https://jurnal.kampus.ac.id/ji",
        );
        assert_eq!(
            redir.as_deref(),
            Some("https://ojs3.kampus.ac.id/index.php/ji")
        );
    }

    #[test]
    fn test_extract_editor_in_chief_html_and_plain() {
        let html = r#"
        <div class="editorial_team">
          <h3>Editor-in-Chief</h3>
          <p>Prof. Dr. Budi Santoso, M.Kom. (Universitas Indonesia)</p>
        </div>"#;
        assert_eq!(
            extract_editor_in_chief(html).as_deref(),
            Some("Prof. Dr. Budi Santoso, M.Kom.")
        );

        let inline = "Ketua Dewan Redaksi: Dr. Rina Wijaya, Universitas Gadjah Mada";
        assert_eq!(
            extract_editor_in_chief(inline).as_deref(),
            Some("Dr. Rina Wijaya")
        );

        let trailing_role = "Ahmad Fauzi, M.T. (Editor-in-Chief)";
        assert_eq!(
            extract_editor_in_chief(trailing_role).as_deref(),
            Some("Ahmad Fauzi, M.T.")
        );

        assert_eq!(
            extract_editor_in_chief("About the Journal and Editorial Team"),
            None
        );
    }

    #[test]
    fn test_jaccard_token_similarity_and_acronym() {
        // High similarity match
        let t1 = "Bulletin of Electrical Engineering and Informatics";
        let t2 = "Bulletin of Electrical Engineering and Informatics";
        assert!((jaccard_token_similarity(t1, t2) - 1.0).abs() < 1e-4);

        // Low similarity mismatch (Hijacked journal indicator)
        let t3 = "International Journal of Electrical Science";
        let t4 = "Agronomy and Veterinary Medical Journal";
        assert!(jaccard_token_similarity(t3, t4) < 0.35);

        // Acronym match
        assert!(is_acronym_match(
            "BEEI",
            "Bulletin of Electrical Engineering and Informatics"
        ));
        assert!(is_acronym_match(
            "JTIEMB",
            "Jurnal Teknologi Informasi Ekonomi dan Bisnis"
        ));
        assert!(!is_acronym_match(
            "XYZ",
            "Bulletin of Electrical Engineering and Informatics"
        ));
    }

    #[test]
    fn test_score_apc_confidence() {
        let text_high = "Author Fee and Article Processing Charge (APC): Publication fee for accepted manuscripts is IDR 1.500.000,- paid upon acceptance.";
        let pos = text_high.find("1.500.000").unwrap();
        let (conf_high, snippet) = score_apc_confidence(text_high, pos);
        assert_eq!(conf_high, ApcConfidence::High);
        assert!(snippet.contains("1.500.000"));

        let text_low = "Fast track revision and print hardcopy version costs Rp 750.000.";
        let pos2 = text_low.find("750.000").unwrap();
        let (conf_low, _) = score_apc_confidence(text_low, pos2);
        assert_eq!(conf_low, ApcConfidence::NeedsCheck);
    }
}
