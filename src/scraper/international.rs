//! International Q1–Q4 journal search via open REST APIs (OpenAlex & DOAJ) without Cloudflare blocks.

use crate::config::{APC_UNKNOWN, CACHE_VERSION, INFO_NOT_FOUND, USD_TO_IDR_RATE};
use crate::models::{format_apc_unified, format_idr_short, DisplayCurrency, JournalRecord};
use crate::parser::heuristics;
use crate::scraper::sanitize_publisher_name;
use chrono::Datelike;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const INTERNATIONAL_USER_AGENT: &str =
    "JSFS-Academic-Research-Tool/4.3.0 (mailto:research@jsfs.local)";

const API_TIMEOUT_SECS: u64 = 12;

/// Compute Q1–Q4 quartile from OpenAlex 2-year mean citedness, h-index, and DOAJ status.
pub fn compute_quartile(mean_citedness_2yr: f64, h_index: u64, in_doaj: bool) -> Option<String> {
    if mean_citedness_2yr >= 3.5 || h_index >= 65 {
        Some("Q1".to_string())
    } else if mean_citedness_2yr >= 1.8 || h_index >= 35 {
        Some("Q2".to_string())
    } else if mean_citedness_2yr >= 0.8 || h_index >= 18 {
        Some("Q3".to_string())
    } else if mean_citedness_2yr > 0.0 || in_doaj {
        Some("Q4".to_string())
    } else {
        None
    }
}

fn assign_diamond_free(rec: &mut JournalRecord, evidence: &str) {
    rec.apc_status = format_apc_unified("free", Some(0), "", DisplayCurrency::Idr, USD_TO_IDR_RATE);
    rec.publication_fee = rec.apc_status.clone();
    rec.publication_fee_idr = Some(0);
    rec.apc_idr_numeric = Some(0);
    rec.fee_currency = "USD".to_string();
    rec.fee_amount = "0".to_string();
    rec.apc_evidence = evidence.to_string();
}

fn assign_paid_usd(rec: &mut JournalRecord, usd: i64, usd_to_idr: i64, source_name: &str) {
    let rate = if usd_to_idr > 0 {
        usd_to_idr
    } else {
        USD_TO_IDR_RATE
    };
    let idr = usd.saturating_mul(rate);
    rec.apc_idr_numeric = (idr > 0).then_some(idr as u64);
    rec.publication_fee_idr = Some(idr);
    rec.apc_status = format_idr_short(idr);
    rec.publication_fee = rec.apc_status.clone();
    rec.fee_currency = "USD".to_string();
    rec.fee_amount = usd.to_string();
    rec.apc_evidence = format!("{source_name}: USD {usd} (kurs Rp {rate})");
}

fn json_number_as_i64(v: &Value) -> Option<i64> {
    if let Some(n) = v.as_i64() {
        return Some(n);
    }
    v.as_f64().map(|f| f.round() as i64)
}

fn extract_issn_pair(issn_l: Option<&str>, arr: Option<&Vec<Value>>) -> (String, String) {
    let mut list: Vec<String> = Vec::new();
    if let Some(l) = issn_l.map(str::trim).filter(|s| !s.is_empty()) {
        list.push(l.to_string());
    }
    if let Some(vals) = arr {
        for item in vals {
            if let Some(s) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                if !list.iter().any(|x| x.eq_ignore_ascii_case(s)) {
                    list.push(s.to_string());
                }
            }
        }
    }
    let issn = list.first().cloned().unwrap_or_default();
    let eissn = list.get(1).cloned().unwrap_or_else(|| issn.clone());
    (issn, eissn)
}

/// Parse OpenAlex Sources API JSON payload into `JournalRecord` entries.
pub fn parse_openalex_response(raw: &str, usd_to_idr: i64) -> Vec<JournalRecord> {
    let Ok(val) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let Some(results) = val.get("results").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let current_year = chrono::Local::now().year() as u16;
    let mut out = Vec::with_capacity(results.len());

    for item in results {
        let Some(title) = item
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };

        let publisher = item
            .get("host_organization_name")
            .and_then(|v| v.as_str())
            .map(sanitize_publisher_name)
            .unwrap_or_default();

        let (issn, eissn) = extract_issn_pair(
            item.get("issn_l").and_then(|v| v.as_str()),
            item.get("issn").and_then(|v| v.as_array()),
        );

        let openalex_id = item
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");
        let homepage = item
            .get("homepage_url")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| s.starts_with("http"))
            .unwrap_or(openalex_id);

        let stats = item.get("summary_stats");
        let citedness = stats
            .and_then(|s| s.get("2yr_mean_citedness"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let h_index = stats
            .and_then(|s| s.get("h_index"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let in_doaj = item
            .get("is_in_doaj")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_oa = item.get("is_oa").and_then(|v| v.as_bool()).unwrap_or(false);

        let quartile = compute_quartile(citedness, h_index, in_doaj);

        let mut topics: Vec<String> = Vec::new();
        if let Some(arr) = item.get("topics").and_then(|v| v.as_array()) {
            for t in arr.iter().take(5) {
                if let Some(name) = t.get("display_name").and_then(|v| v.as_str()) {
                    let clean = name.trim();
                    if !clean.is_empty() && !topics.iter().any(|x| x.eq_ignore_ascii_case(clean)) {
                        topics.push(clean.to_string());
                    }
                }
            }
        }
        if topics.is_empty() {
            if let Some(arr) = item.get("x_concepts").and_then(|v| v.as_array()) {
                for c in arr.iter().take(5) {
                    if let Some(name) = c.get("display_name").and_then(|v| v.as_str()) {
                        let clean = name.trim();
                        if !clean.is_empty()
                            && !topics.iter().any(|x| x.eq_ignore_ascii_case(clean))
                        {
                            topics.push(clean.to_string());
                        }
                    }
                }
            }
        }
        let subject_str = topics.join(", ");
        let focus_scope = if subject_str.is_empty() {
            INFO_NOT_FOUND.to_string()
        } else {
            format!(
                "Topik riset OpenAlex: {subject_str} (2yr Citedness: {citedness:.2}, h-index: {h_index})."
            )
        };

        let last_issue_year = item
            .get("counts_by_year")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .filter(|y| {
                        y.get("works_count")
                            .and_then(|w| w.as_u64())
                            .is_some_and(|w| w > 0)
                    })
                    .filter_map(|y| y.get("year").and_then(|yr| yr.as_u64()))
                    .max()
                    .and_then(|y| u16::try_from(y).ok())
            });

        let mut rec = JournalRecord {
            title: title.to_string(),
            publisher,
            issn,
            eissn,
            website_url: homepage.to_string(),
            source_portal: "OpenAlex".to_string(),
            subject_area: subject_str,
            focus_and_scope: focus_scope,
            publish_schedule: if is_oa {
                "Open Access Internasional".to_string()
            } else {
                "Internasional".to_string()
            },
            last_issue_year,
            cache_version: CACHE_VERSION.to_string(),
            quartile,
            is_international: true,
            ..Default::default()
        };
        rec.refresh_activity_status(false, current_year);
        rec.article_languages =
            heuristics::detect_article_languages(&rec.title, &rec.focus_and_scope, "", true);
        rec.risk_warning = heuristics::evaluate_journal_risk(&rec, "", None, None);

        match item.get("apc_usd").and_then(json_number_as_i64) {
            Some(0) => assign_diamond_free(&mut rec, "OpenAlex verified: apc_usd = 0 (Diamond OA)"),
            Some(usd) if usd > 0 => assign_paid_usd(&mut rec, usd, usd_to_idr, "OpenAlex APC"),
            _ => {
                rec.apc_status = APC_UNKNOWN.to_string();
                rec.publication_fee = APC_UNKNOWN.to_string();
                if in_doaj {
                    rec.apc_evidence =
                        "Terindeks OpenAlex + DOAJ (nominal APC tidak dicantumkan OpenAlex)"
                            .to_string();
                }
            }
        }

        out.push(rec);
    }

    out
}

/// Parse DOAJ Journals API JSON payload into `JournalRecord` entries.
pub fn parse_doaj_response(raw: &str, usd_to_idr: i64) -> Vec<JournalRecord> {
    let Ok(val) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let Some(results) = val.get("results").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let current_year = chrono::Local::now().year() as u16;
    let mut out = Vec::with_capacity(results.len());

    for item in results {
        let Some(bib) = item.get("bibjson") else {
            continue;
        };
        let Some(title) = bib
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };

        let publisher_raw = bib
            .get("publisher")
            .and_then(|p| {
                p.get("name")
                    .and_then(|v| v.as_str())
                    .or_else(|| p.as_str())
            })
            .unwrap_or("");
        let publisher = sanitize_publisher_name(publisher_raw);

        let issn = bib
            .get("pissn")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        let eissn = bib
            .get("eissn")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("")
            .to_string();

        let ref_url = bib
            .get("ref")
            .and_then(|r| r.get("journal"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| s.starts_with("http"))
            .map(str::to_string)
            .or_else(|| {
                bib.get("link").and_then(|v| v.as_array()).and_then(|arr| {
                    arr.iter()
                        .find_map(|l| l.get("url").and_then(|u| u.as_str()))
                        .filter(|s| s.starts_with("http"))
                        .map(str::to_string)
                })
            })
            .unwrap_or_default();

        let mut subjects: Vec<String> = Vec::new();
        if let Some(arr) = bib.get("subject").and_then(|v| v.as_array()) {
            for s in arr.iter().take(5) {
                if let Some(term) = s.get("term").and_then(|v| v.as_str()) {
                    let clean = term.trim();
                    if !clean.is_empty() && !subjects.iter().any(|x| x.eq_ignore_ascii_case(clean))
                    {
                        subjects.push(clean.to_string());
                    }
                }
            }
        }
        if let Some(arr) = bib.get("keywords").and_then(|v| v.as_array()) {
            for k in arr.iter().take(5) {
                if let Some(kw) = k.as_str() {
                    let clean = kw.trim();
                    if !clean.is_empty() && !subjects.iter().any(|x| x.eq_ignore_ascii_case(clean))
                    {
                        subjects.push(clean.to_string());
                    }
                }
            }
        }
        let subject_str = subjects.join(", ");
        let focus_scope = if subject_str.is_empty() {
            INFO_NOT_FOUND.to_string()
        } else {
            format!("Bidang keilmuan DOAJ: {subject_str}.")
        };

        let last_issue_year = item
            .get("last_updated")
            .and_then(|v| v.as_str())
            .and_then(|s| s.get(..4))
            .and_then(|y| y.parse::<u16>().ok())
            .or(Some(current_year));

        let mut rec = JournalRecord {
            title: title.to_string(),
            publisher,
            issn,
            eissn,
            website_url: ref_url,
            source_portal: "DOAJ".to_string(),
            subject_area: subject_str,
            focus_and_scope: focus_scope,
            publish_schedule: "DOAJ Open Access".to_string(),
            last_issue_year,
            cache_version: CACHE_VERSION.to_string(),
            quartile: compute_quartile(0.0, 0, true),
            is_international: true,
            ..Default::default()
        };
        rec.refresh_activity_status(false, current_year);
        rec.article_languages =
            heuristics::detect_article_languages(&rec.title, &rec.focus_and_scope, "", true);
        rec.risk_warning = heuristics::evaluate_journal_risk(&rec, "", None, None);

        let apc_obj = bib.get("apc");
        let has_apc = apc_obj
            .and_then(|a| a.get("has_apc"))
            .and_then(|v| v.as_bool());
        match has_apc {
            Some(false) => {
                assign_diamond_free(&mut rec, "DOAJ verified: has_apc = false (Diamond OA)");
            }
            Some(true) => {
                let mut assigned = false;
                if let Some(max_arr) = apc_obj
                    .and_then(|a| a.get("max"))
                    .and_then(|v| v.as_array())
                {
                    for entry in max_arr {
                        let cur = entry
                            .get("currency")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_ascii_uppercase();
                        if let Some(price) = entry.get("price").and_then(json_number_as_i64) {
                            if price == 0 {
                                assign_diamond_free(&mut rec, "DOAJ verified: APC price = 0");
                                assigned = true;
                                break;
                            } else if cur == "USD" {
                                assign_paid_usd(&mut rec, price, usd_to_idr, "DOAJ APC");
                                assigned = true;
                                break;
                            } else if cur == "IDR" {
                                rec.apc_status = format_idr_short(price);
                                rec.publication_fee = rec.apc_status.clone();
                                rec.publication_fee_idr = Some(price);
                                rec.apc_idr_numeric = (price > 0).then_some(price as u64);
                                rec.fee_currency = "IDR".to_string();
                                rec.fee_amount = price.to_string();
                                rec.apc_evidence = format!("DOAJ APC: IDR {price}");
                                assigned = true;
                                break;
                            }
                        }
                    }
                }
                if !assigned {
                    rec.apc_status = APC_UNKNOWN.to_string();
                    rec.publication_fee = APC_UNKNOWN.to_string();
                }
            }
            None => {
                rec.apc_status = APC_UNKNOWN.to_string();
                rec.publication_fee = APC_UNKNOWN.to_string();
            }
        }

        if let Some(weeks) = bib
            .get("publication_time_weeks")
            .and_then(json_number_as_i64)
            .and_then(|w| u16::try_from(w).ok())
            .filter(|&w| (1..=104).contains(&w))
        {
            rec.turnaround_weeks = Some(weeks);
            rec.turnaround_label = heuristics::format_turnaround_label(weeks);
        }
        if let Some(langs) = bib.get("language").and_then(|v| v.as_array()) {
            let mut has_id = false;
            let mut has_en = false;
            for l in langs {
                if let Some(code) = l.as_str() {
                    let up = code.trim().to_ascii_uppercase();
                    if up == "ID" || up == "IND" || up == "IN" {
                        has_id = true;
                    } else if up == "EN" || up == "ENG" {
                        has_en = true;
                    }
                }
            }
            if has_id && has_en {
                rec.article_languages = "ID & EN".to_string();
            } else if has_id {
                rec.article_languages = "ID".to_string();
            } else if has_en {
                rec.article_languages = "EN".to_string();
            }
        }

        out.push(rec);
    }

    out
}

/// Fetch international journals from OpenAlex Sources REST API (polite pool header, zero Cloudflare).
pub async fn scrape_openalex<F>(
    client: &reqwest::Client,
    keyword: &str,
    usd_to_idr: i64,
    cancel_flag: Arc<AtomicBool>,
    on_progress: F,
) -> Vec<JournalRecord>
where
    F: Fn(usize, usize, &str),
{
    if cancel_flag.load(Ordering::Relaxed) || keyword.trim().is_empty() {
        return Vec::new();
    }
    on_progress(1, 2, &format!("OpenAlex Q1–Q4: '{keyword}'..."));
    let url = format!(
        "https://api.openalex.org/sources?search={}&filter=type:journal&per-page=50",
        urlencoding::encode(keyword.trim())
    );
    let Ok(resp) = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, INTERNATIONAL_USER_AGENT)
        .timeout(Duration::from_secs(API_TIMEOUT_SECS))
        .send()
        .await
    else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(raw) = resp.text().await else {
        return Vec::new();
    };
    let parsed = parse_openalex_response(&raw, usd_to_idr);
    on_progress(2, 2, &format!("OpenAlex: {} jurnal", parsed.len()));
    parsed
}

/// Fetch international OA journals from DOAJ Search API.
pub async fn scrape_doaj<F>(
    client: &reqwest::Client,
    keyword: &str,
    usd_to_idr: i64,
    cancel_flag: Arc<AtomicBool>,
    on_progress: F,
) -> Vec<JournalRecord>
where
    F: Fn(usize, usize, &str),
{
    if cancel_flag.load(Ordering::Relaxed) || keyword.trim().is_empty() {
        return Vec::new();
    }
    on_progress(1, 2, &format!("DOAJ Diamond OA: '{keyword}'..."));
    let url = format!(
        "https://doaj.org/api/search/journals/{}?pageSize=50",
        urlencoding::encode(keyword.trim())
    );
    let Ok(resp) = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, INTERNATIONAL_USER_AGENT)
        .timeout(Duration::from_secs(API_TIMEOUT_SECS))
        .send()
        .await
    else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(raw) = resp.text().await else {
        return Vec::new();
    };
    let parsed = parse_doaj_response(&raw, usd_to_idr);
    on_progress(2, 2, &format!("DOAJ: {} jurnal", parsed.len()));
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quartile_mapping_and_diamond_oa_detection() {
        assert_eq!(compute_quartile(4.1, 20, false).as_deref(), Some("Q1"));
        assert_eq!(compute_quartile(1.0, 65, false).as_deref(), Some("Q1"));
        assert_eq!(compute_quartile(2.2, 10, false).as_deref(), Some("Q2"));
        assert_eq!(compute_quartile(0.5, 35, false).as_deref(), Some("Q2"));
        assert_eq!(compute_quartile(0.9, 5, false).as_deref(), Some("Q3"));
        assert_eq!(compute_quartile(0.1, 18, false).as_deref(), Some("Q3"));
        assert_eq!(compute_quartile(0.2, 4, false).as_deref(), Some("Q4"));
        assert_eq!(compute_quartile(0.0, 0, true).as_deref(), Some("Q4"));
        assert_eq!(compute_quartile(0.0, 0, false), None);

        let openalex_json = r#"{
            "results": [
                {
                    "id": "https://openalex.org/S1",
                    "display_name": "Journal of Open Systems",
                    "issn_l": "1111-2222",
                    "issn": ["1111-2222", "3333-4444"],
                    "host_organization_name": "Open Academic Press",
                    "homepage_url": "https://jos.example.org",
                    "is_oa": true,
                    "is_in_doaj": true,
                    "apc_usd": 0,
                    "summary_stats": { "2yr_mean_citedness": 3.8, "h_index": 42 },
                    "counts_by_year": [{ "year": 2026, "works_count": 30 }],
                    "topics": [{ "display_name": "Information Systems" }]
                },
                {
                    "id": "https://openalex.org/S2",
                    "display_name": "Applied Data Analytics",
                    "issn_l": "5555-6666",
                    "host_organization_name": "Science House",
                    "homepage_url": "https://ada.example.org",
                    "is_oa": false,
                    "is_in_doaj": false,
                    "apc_usd": 150,
                    "summary_stats": { "2yr_mean_citedness": 1.1, "h_index": 22 },
                    "topics": [{ "display_name": "Data Mining" }]
                }
            ]
        }"#;

        let oa_recs = parse_openalex_response(openalex_json, 16_500);
        assert_eq!(oa_recs.len(), 2);
        assert_eq!(oa_recs[0].quartile.as_deref(), Some("Q1"));
        assert!(oa_recs[0].is_international);
        assert_eq!(oa_recs[0].apc_status, "Gratis (Rp 0)");
        assert_eq!(oa_recs[0].apc_category(), "free");
        assert_eq!(oa_recs[0].publication_fee_idr, Some(0));
        assert_eq!(oa_recs[0].apc_idr_numeric, Some(0));

        assert_eq!(oa_recs[1].quartile.as_deref(), Some("Q3"));
        assert_eq!(oa_recs[1].apc_status, "Rp 2.475.000");
        assert_eq!(oa_recs[1].publication_fee_idr, Some(2_475_000));
        assert_eq!(oa_recs[1].apc_idr_numeric, Some(2_475_000));
        assert_eq!(oa_recs[1].apc_category(), "paid");

        let doaj_json = r#"{
            "results": [
                {
                    "last_updated": "2026-02-01T00:00:00Z",
                    "bibjson": {
                        "title": "Diamond Computer Science",
                        "pissn": "1234-9999",
                        "eissn": "9999-1234",
                        "publisher": { "name": "University Society" },
                        "apc": { "has_apc": false },
                        "ref": { "journal": "https://dcs.example.org" },
                        "subject": [{ "term": "Computer Science" }]
                    }
                }
            ]
        }"#;
        let doaj_recs = parse_doaj_response(doaj_json, 16_500);
        assert_eq!(doaj_recs.len(), 1);
        assert_eq!(doaj_recs[0].quartile.as_deref(), Some("Q4"));
        assert!(doaj_recs[0].is_international);
        assert_eq!(doaj_recs[0].apc_status, "Gratis (Rp 0)");
        assert_eq!(doaj_recs[0].publication_fee_idr, Some(0));
        assert_eq!(doaj_recs[0].apc_idr_numeric, Some(0));
    }
}
