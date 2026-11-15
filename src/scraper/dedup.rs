//! Cross-source deduplication and record merger (SINTA + Garuda).

use crate::config::INFO_NOT_FOUND;
use crate::models::JournalRecord;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use url::Url;

static RE_NON_ALNUM: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-z0-9\s]").expect("valid regex"));
static RE_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").expect("valid regex"));

pub fn normalize_issn_key(raw: &str) -> String {
    let clean: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if clean.len() == 8 {
        clean
    } else {
        String::new()
    }
}

pub fn normalize_title_key(title: &str) -> String {
    let low = title.to_ascii_lowercase();
    let alnum = RE_NON_ALNUM.replace_all(&low, " ");
    let collapsed = RE_WS.replace_all(&alnum, " ");
    let mut kept = Vec::new();
    for w in collapsed.split_whitespace() {
        if !matches!(w, "jurnal" | "journal" | "of" | "the" | "and" | "dan") {
            kept.push(w);
        }
    }
    if kept.is_empty() {
        RE_WS.replace_all(&alnum, " ").trim().to_string()
    } else {
        kept.join(" ")
    }
}

pub fn normalize_ojs_root_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() || !trimmed.starts_with("http") {
        return trimmed.to_string();
    }
    if let Ok(mut u) = Url::parse(trimmed) {
        u.set_query(None);
        u.set_fragment(None);
        let path = u.path().trim_end_matches('/').to_string();
        let clean_path = if let Some(p) = path.strip_suffix("/index") {
            p.to_string()
        } else {
            path
        };
        u.set_path(&clean_path);
        return u.to_string().trim_end_matches('/').to_string();
    }
    trimmed.to_string()
}

fn is_empty_field(val: &str) -> bool {
    let t = val.trim();
    t.is_empty() || t == INFO_NOT_FOUND || t == "-"
}

fn merge_into(dst: &mut JournalRecord, src: JournalRecord) {
    let src_has_sinta = src.sinta_rank_num().is_some();
    let src_quartile_rank = src.quartile_rank_num();
    let src_apc_known = src.apc_category() != "unknown";

    dst.is_international = dst.is_international || src.is_international;
    if let Some(src_q) = src_quartile_rank {
        let take_src_q = dst.quartile_rank_num().is_none_or(|dst_q| src_q < dst_q);
        if take_src_q {
            dst.quartile = src.quartile;
        }
    }
    if dst.apc_category() == "unknown" && src_apc_known {
        dst.apc_status = src.apc_status;
        dst.publication_fee = src.publication_fee;
        dst.publication_fee_idr = src.publication_fee_idr;
        dst.fee_currency = src.fee_currency;
        dst.fee_amount = src.fee_amount;
        if !src.apc_evidence.is_empty() {
            dst.apc_evidence = src.apc_evidence;
        }
    }
    if dst.last_issue_year.is_none() && src.last_issue_year.is_some() {
        dst.last_issue_year = src.last_issue_year;
        dst.activity_status = src.activity_status;
        dst.is_dormant = src.is_dormant;
    }

    if is_empty_field(&dst.publisher) && !is_empty_field(&src.publisher) {
        dst.publisher = src.publisher;
    }
    if is_empty_field(&dst.issn) && !is_empty_field(&src.issn) {
        dst.issn = src.issn;
    }
    if is_empty_field(&dst.eissn) && !is_empty_field(&src.eissn) {
        dst.eissn = src.eissn;
    }
    if dst.garuda_url.is_empty() && !src.garuda_url.is_empty() {
        dst.garuda_url = src.garuda_url;
    }
    if dst.sinta_rank_num().is_none() && src_has_sinta {
        dst.sinta_score = src.sinta_score;
    }
    if dst.sinta_url.is_empty() && !src.sinta_url.is_empty() {
        dst.sinta_url = src.sinta_url;
    }
    if (!dst.website_url.starts_with("http") || dst.website_url.contains("kemdiktisaintek.go.id"))
        && src.website_url.starts_with("http")
        && !src.website_url.contains("kemdiktisaintek.go.id")
    {
        dst.website_url = src.website_url;
    }
    if !is_empty_field(&src.subject_area) {
        if is_empty_field(&dst.subject_area) {
            dst.subject_area = src.subject_area;
        } else if !dst
            .subject_area
            .to_ascii_lowercase()
            .contains(&src.subject_area.to_ascii_lowercase())
        {
            dst.subject_area = format!("{}, {}", dst.subject_area, src.subject_area);
        }
    }
    if is_empty_field(&dst.focus_and_scope) && !is_empty_field(&src.focus_and_scope) {
        dst.focus_and_scope = src.focus_and_scope;
    }
    if !dst.source_portal.is_empty()
        && !src.source_portal.is_empty()
        && !dst.source_portal.contains(&src.source_portal)
    {
        dst.source_portal = format!("{} + {}", dst.source_portal, src.source_portal);
    }
    if dst.turnaround_weeks.is_none() && src.turnaround_weeks.is_some() {
        dst.turnaround_weeks = src.turnaround_weeks;
        dst.turnaround_label = src.turnaround_label;
    }
    if dst.article_languages.is_empty() && !src.article_languages.is_empty() {
        dst.article_languages = src.article_languages;
    }
    if dst.template_url.is_none() && src.template_url.is_some() {
        dst.template_url = src.template_url;
    }
    if dst.risk_warning.is_none() && src.risk_warning.is_some() {
        dst.risk_warning = src.risk_warning;
    }
}

/// Deduplicate raw candidate pool across E-ISSN, P-ISSN, canonical title, and normalized OJS URL.
pub fn deduplicate_and_merge(entries: Vec<JournalRecord>) -> Vec<JournalRecord> {
    let mut merged: Vec<JournalRecord> = Vec::with_capacity(entries.len());
    let mut issn_map: HashMap<String, usize> = HashMap::new();
    let mut url_map: HashMap<String, usize> = HashMap::new();
    let mut title_map: HashMap<String, usize> = HashMap::new();

    for rec in entries {
        let p_key = normalize_issn_key(&rec.issn);
        let e_key = normalize_issn_key(&rec.eissn);
        let u_key = {
            let norm = normalize_ojs_root_url(&rec.website_url).to_ascii_lowercase();
            if norm.starts_with("http") && !norm.contains("kemdiktisaintek.go.id") {
                norm
            } else {
                String::new()
            }
        };
        let t_key = normalize_title_key(&rec.title);

        let existing_idx = (!e_key.is_empty())
            .then(|| issn_map.get(&e_key).copied())
            .flatten()
            .or_else(|| {
                (!p_key.is_empty())
                    .then(|| issn_map.get(&p_key).copied())
                    .flatten()
            })
            .or_else(|| {
                (!u_key.is_empty())
                    .then(|| url_map.get(&u_key).copied())
                    .flatten()
            })
            .or_else(|| {
                (!t_key.is_empty())
                    .then(|| title_map.get(&t_key).copied())
                    .flatten()
            });

        if let Some(idx) = existing_idx {
            merge_into(&mut merged[idx], rec);
        } else {
            let idx = merged.len();
            if !e_key.is_empty() {
                issn_map.insert(e_key, idx);
            }
            if !p_key.is_empty() {
                issn_map.insert(p_key, idx);
            }
            if !u_key.is_empty() {
                url_map.insert(u_key, idx);
            }
            if !t_key.is_empty() {
                title_map.insert(t_key, idx);
            }
            merged.push(rec);
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_source_dedup_merge() {
        let sinta_rec = JournalRecord {
            title: "Jurnal Sistem Informasi Bisnis".to_string(),
            sinta_score: "Sinta 3".to_string(),
            eissn: "2502-2377".to_string(),
            sinta_url: "https://sinta.kemdiktisaintek.go.id/journals/profile/123".to_string(),
            source_portal: "SINTA".to_string(),
            ..Default::default()
        };
        let garuda_rec = JournalRecord {
            title: "JSIB - Sistem Informasi Bisnis".to_string(),
            eissn: "25022377".to_string(),
            publisher: "UNDIP".to_string(),
            website_url: "https://ejournal.undip.ac.id/index.php/jsinbis".to_string(),
            garuda_url: "https://garuda.kemdiktisaintek.go.id/journal/view/456".to_string(),
            source_portal: "Garuda".to_string(),
            ..Default::default()
        };

        let out = deduplicate_and_merge(vec![sinta_rec, garuda_rec]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].sinta_score, "Sinta 3");
        assert_eq!(out[0].publisher, "UNDIP");
        assert!(out[0].website_url.contains("undip.ac.id"));
        assert!(out[0].source_portal.contains("SINTA + Garuda"));
    }
}
