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
    let dst_paid =
        dst.apc_category() == "paid" || dst.resolved_apc_idr_numeric().is_some_and(|v| v > 0);
    let src_paid =
        src.apc_category() == "paid" || src.resolved_apc_idr_numeric().is_some_and(|v| v > 0);

    dst.is_international = dst.is_international || src.is_international;
    if let Some(src_q) = src_quartile_rank {
        let take_src_q = dst.quartile_rank_num().is_none_or(|dst_q| src_q < dst_q);
        if take_src_q {
            dst.quartile = src.quartile;
        }
    }
    // Paid APC (e.g. verified OJS editorialPolicies) MUST never stay overwritten by older DOAJ Free (Rp 0)
    if (dst.apc_category() == "unknown" && src_apc_known) || (!dst_paid && src_paid) {
        dst.apc_status = src.apc_status;
        dst.publication_fee = src.publication_fee;
        dst.publication_fee_idr = src.publication_fee_idr;
        dst.apc_idr_numeric = src.apc_idr_numeric;
        dst.fee_currency = src.fee_currency;
        dst.fee_amount = src.fee_amount;
        if !src.apc_evidence.is_empty() {
            dst.apc_evidence = src.apc_evidence;
        }
        if dst.apc_raw_text.is_empty() && !src.apc_raw_text.is_empty() {
            dst.apc_raw_text = src.apc_raw_text;
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
    if dst.submission_fee_idr.is_none() && src.submission_fee_idr.is_some() {
        dst.submission_fee_idr = src.submission_fee_idr;
    }
    dst.bot_protected = dst.bot_protected || src.bot_protected;
    dst.issn_valid = dst.issn_valid || src.issn_valid;
    for idx_name in src.extra_indexes {
        if !dst
            .extra_indexes
            .iter()
            .any(|x| x.eq_ignore_ascii_case(&idx_name))
        {
            dst.extra_indexes.push(idx_name);
        }
    }
    if dst.turnaround_weeks.is_none() && src.turnaround_weeks.is_some() {
        dst.turnaround_weeks = src.turnaround_weeks;
        dst.turnaround_days = src.turnaround_days;
        dst.turnaround_label = src.turnaround_label;
    } else if dst.turnaround_days.is_none() && src.turnaround_days.is_some() {
        dst.turnaround_days = src.turnaround_days;
        if dst.turnaround_label.trim().is_empty() && !src.turnaround_label.trim().is_empty() {
            dst.turnaround_label = src.turnaround_label;
        }
    }
    if dst.publish_month_nums.is_empty() && !src.publish_month_nums.is_empty() {
        dst.publish_month_nums = src.publish_month_nums;
        dst.publish_months = src.publish_months;
        if dst.publish_schedule.is_empty() {
            dst.publish_schedule = src.publish_schedule;
        }
        if dst.next_issue_estimate.is_empty() {
            dst.next_issue_estimate = src.next_issue_estimate;
        }
    }
    if dst.citation_style.is_none() && src.citation_style.is_some() {
        dst.citation_style = src.citation_style;
    }
    if dst.word_limit.is_none() && src.word_limit.is_some() {
        dst.word_limit = src.word_limit;
    }
    if is_empty_field(&dst.email) && !is_empty_field(&src.email) {
        dst.email = src.email;
    }
    if is_empty_field(&dst.phone) && !is_empty_field(&src.phone) {
        dst.phone = src.phone;
    }
    if dst.data_source_note.trim().is_empty() && !src.data_source_note.trim().is_empty() {
        dst.data_source_note = src.data_source_note;
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
    if dst.max_similarity_pct.is_none() && src.max_similarity_pct.is_some() {
        dst.max_similarity_pct = src.max_similarity_pct;
    }
    if dst.editor_in_chief.is_none() && src.editor_in_chief.is_some() {
        dst.editor_in_chief = src.editor_in_chief;
    }
    if dst.submission_open.is_none() && src.submission_open.is_some() {
        dst.submission_open = src.submission_open;
    }
    if dst.cited_by_count.is_none() && src.cited_by_count.is_some() {
        dst.cited_by_count = src.cited_by_count;
    }
    if dst.works_count.is_none() && src.works_count.is_some() {
        dst.works_count = src.works_count;
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
            })
            .or_else(|| {
                // Fuzzy cross-source merge (title token Jaccard similarity >= 85%)
                // when portal entries lack matching ISSN/URL metadata.
                let token_count = t_key.split_whitespace().count();
                if token_count < 2 || rec.title.trim().is_empty() {
                    return None;
                }
                let mut best: Option<(usize, f32)> = None;
                for (i, existing) in merged.iter().enumerate() {
                    let ex_p = normalize_issn_key(&existing.issn);
                    let ex_e = normalize_issn_key(&existing.eissn);
                    let both_have_p_or_e = (!p_key.is_empty() || !e_key.is_empty())
                        && (!ex_p.is_empty() || !ex_e.is_empty());
                    if both_have_p_or_e {
                        let any_overlap = (!p_key.is_empty() && (p_key == ex_p || p_key == ex_e))
                            || (!e_key.is_empty() && (e_key == ex_p || e_key == ex_e));
                        if !any_overlap {
                            continue;
                        }
                    }
                    let sim = crate::parser::heuristics::jaccard_token_similarity(
                        &existing.title,
                        &rec.title,
                    );
                    if sim >= 0.85 && best.is_none_or(|(_, b_sim)| sim > b_sim) {
                        best = Some((i, sim));
                    }
                }
                best.map(|(i, _)| i)
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

    for rec in &mut merged {
        rec.issn_valid = crate::parser::heuristics::has_valid_issn_pair(&rec.issn, &rec.eissn);
        if rec.source_portal.contains("DOAJ") && !rec.extra_indexes.iter().any(|x| x == "DOAJ") {
            rec.extra_indexes.push("DOAJ".to_string());
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

        // Verify Paid OJS evidence always overrides DOAJ Free metadata
        let doaj_free = JournalRecord {
            title: "Jurnal Maliyah".to_string(),
            eissn: "2502-2377".to_string(),
            apc_status: "Gratis (Rp 0)".to_string(),
            publication_fee_idr: Some(0),
            apc_idr_numeric: Some(0),
            source_portal: "DOAJ".to_string(),
            ..Default::default()
        };
        let ojs_paid = JournalRecord {
            title: "Jurnal Maliyah".to_string(),
            eissn: "2502-2377".to_string(),
            apc_status: "Rp 2.220.000".to_string(),
            publication_fee: "Rp 2.220.000".to_string(),
            publication_fee_idr: Some(2_220_000),
            apc_idr_numeric: Some(2_220_000),
            source_portal: "SINTA".to_string(),
            ..Default::default()
        };
        let paid_merged = deduplicate_and_merge(vec![doaj_free, ojs_paid]);
        assert_eq!(paid_merged.len(), 1);
        assert_eq!(paid_merged[0].apc_category(), "paid");
        assert_eq!(paid_merged[0].apc_idr_numeric, Some(2_220_000));
    }

    #[test]
    fn test_fuzzy_cross_source_dedup_merge_and_turnaround_transfer() {
        let sinta = JournalRecord {
            title: "Jurnal Sistem Informasi Bisnis Indonesia".to_string(),
            sinta_score: "Sinta 2".to_string(),
            source_portal: "SINTA".to_string(),
            apc_status: "Belum diketahui".to_string(),
            ..Default::default()
        };
        let doaj_missing_issn = JournalRecord {
            title: "Jurnal Sistem Informasi dan Bisnis Indonesia".to_string(),
            source_portal: "DOAJ".to_string(),
            apc_status: "Gratis (Rp 0)".to_string(),
            apc_idr_numeric: Some(0),
            turnaround_weeks: Some(10),
            turnaround_days: Some(70),
            turnaround_label: "Sekitar 2,3 Bulan (70 Hari)".to_string(),
            citation_style: Some("APA".to_string()),
            ..Default::default()
        };
        let distinct_journal = JournalRecord {
            title: "Jurnal Keperawatan Klinis Nusantara".to_string(),
            source_portal: "DOAJ".to_string(),
            ..Default::default()
        };

        let merged = deduplicate_and_merge(vec![sinta, doaj_missing_issn, distinct_journal]);
        assert_eq!(
            merged.len(),
            2,
            "SINTA and DOAJ fuzzy title match (>=85%) should merge into 1 while keeping distinct journal"
        );
        assert_eq!(merged[0].sinta_score, "Sinta 2");
        assert!(merged[0].extra_indexes.iter().any(|x| x == "DOAJ"));
        assert_eq!(merged[0].turnaround_days, Some(70));
        assert_eq!(merged[0].turnaround_weeks, Some(10));
        assert_eq!(merged[0].citation_style.as_deref(), Some("APA"));
    }
}
