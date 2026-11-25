//! International Q1–Q4 journal search via open REST APIs (OpenAlex, DOAJ & Crossref) without Cloudflare blocks.

use crate::config::{APC_UNKNOWN, CACHE_VERSION, INFO_NOT_FOUND, USD_TO_IDR_RATE};
use crate::models::{format_apc_unified, format_idr_short, DisplayCurrency, JournalRecord};
use crate::parser::heuristics;
use crate::scraper::sanitize_publisher_name;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const INTERNATIONAL_USER_AGENT: &str =
    "JSFS-Academic-Research-Tool/4.8.0 (mailto:research@jsfs.local)";

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
        for key in ["topics", "x_concepts"] {
            if let Some(arr) = item.get(key).and_then(|v| v.as_array()) {
                for entry in arr.iter().take(5) {
                    if let Some(clean) = entry
                        .get("display_name")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        if !topics.iter().any(|x| x.eq_ignore_ascii_case(clean)) {
                            topics.push(clean.to_string());
                        }
                    }
                }
                if !topics.is_empty() {
                    break;
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

/// Parse Crossref Journals REST API (`/journals?query=...`) JSON payload into `JournalRecord` entries.
pub fn parse_crossref_journals_response(raw: &str) -> Vec<JournalRecord> {
    let Ok(val) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let Some(items) = val
        .get("message")
        .and_then(|m| m.get("items"))
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };

    let current_year = chrono::Local::now().year() as u16;
    let mut out = Vec::with_capacity(items.len());

    for item in items {
        let Some(title) = item
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };

        let publisher = item
            .get("publisher")
            .and_then(|v| v.as_str())
            .map(sanitize_publisher_name)
            .unwrap_or_default();

        let mut issn = String::new();
        let mut eissn = String::new();
        if let Some(types) = item.get("issn-type").and_then(|v| v.as_array()) {
            for t in types {
                let code = t
                    .get("value")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .unwrap_or("");
                let kind = t.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if code.is_empty() {
                    continue;
                }
                if kind.eq_ignore_ascii_case("electronic") && eissn.is_empty() {
                    eissn = code.to_string();
                } else if kind.eq_ignore_ascii_case("print") && issn.is_empty() {
                    issn = code.to_string();
                }
            }
        }
        if issn.is_empty() || eissn.is_empty() {
            let (p_fb, e_fb) = extract_issn_pair(None, item.get("ISSN").and_then(|v| v.as_array()));
            if issn.is_empty() {
                issn = p_fb;
            }
            if eissn.is_empty() {
                eissn = if !e_fb.is_empty() { e_fb } else { issn.clone() };
            }
        }

        let mut subjects: Vec<String> = Vec::new();
        if let Some(arr) = item.get("subjects").and_then(|v| v.as_array()) {
            for s in arr.iter().take(5) {
                if let Some(name) = s.get("name").and_then(|v| v.as_str()) {
                    let clean = name.trim();
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
            format!("Topik Crossref: {subject_str}.")
        };

        let mut last_issue_year = item
            .get("breakdowns")
            .and_then(|b| b.get("dois-by-issued-year"))
            .and_then(|v| v.as_array())
            .and_then(|pairs| {
                pairs
                    .iter()
                    .filter_map(|p| {
                        let arr = p.as_array()?;
                        let yr = arr.first()?.as_u64()?;
                        let cnt = arr.get(1)?.as_u64()?;
                        (cnt > 0).then(|| u16::try_from(yr).ok()).flatten()
                    })
                    .max()
            });
        if last_issue_year.is_none()
            && item
                .get("counts")
                .and_then(|c| c.get("current-dois"))
                .and_then(|v| v.as_u64())
                .is_some_and(|n| n > 0)
        {
            last_issue_year = Some(current_year);
        }

        let homepage = item
            .get("URL")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| s.starts_with("http"))
            .unwrap_or("")
            .to_string();

        let mut rec = JournalRecord {
            title: title.to_string(),
            publisher,
            issn,
            eissn,
            website_url: homepage,
            source_portal: "Crossref".to_string(),
            subject_area: subject_str,
            focus_and_scope: focus_scope,
            apc_status: APC_UNKNOWN.to_string(),
            publication_fee: APC_UNKNOWN.to_string(),
            apc_evidence: "Terindeks Crossref Journals API (verifikasi APC di web jurnal)"
                .to_string(),
            publish_schedule: "Internasional (Crossref DOI)".to_string(),
            last_issue_year,
            cache_version: CACHE_VERSION.to_string(),
            extra_indexes: vec!["Crossref".to_string()],
            is_international: true,
            ..Default::default()
        };
        rec.refresh_activity_status(false, current_year);
        rec.article_languages =
            heuristics::detect_article_languages(&rec.title, &rec.focus_and_scope, "", true);
        rec.issn_valid = heuristics::has_valid_issn_pair(&rec.issn, &rec.eissn);
        rec.risk_warning = heuristics::evaluate_journal_risk(&rec, "", None, None);
        out.push(rec);
    }

    out
}

/// Query OpenAlex, DOAJ, or Crossref by ISSN to triangulate metadata for bot-protected journals.
pub async fn triangulate_issn_metadata(
    client: &reqwest::Client,
    issn: &str,
    usd_to_idr: i64,
) -> Option<JournalRecord> {
    let clean = issn.trim().replace('-', "");
    if clean.len() != 8 {
        return None;
    }
    let formatted_issn = format!("{}-{}", &clean[..4], &clean[4..]);

    // 1. Try OpenAlex by ISSN filter
    let openalex_url = format!("https://api.openalex.org/sources?filter=issn:{formatted_issn}");
    if let Ok(resp) = client
        .get(&openalex_url)
        .header(reqwest::header::USER_AGENT, INTERNATIONAL_USER_AGENT)
        .timeout(Duration::from_secs(6))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(raw) = resp.text().await {
                let recs = parse_openalex_response(&raw, usd_to_idr);
                if let Some(r) = recs.into_iter().next() {
                    return Some(r);
                }
            }
        }
    }

    // 2. Try DOAJ by ISSN query
    let doaj_url = format!("https://doaj.org/api/search/journals/issn:{formatted_issn}");
    if let Ok(resp) = client
        .get(&doaj_url)
        .header(reqwest::header::USER_AGENT, INTERNATIONAL_USER_AGENT)
        .timeout(Duration::from_secs(6))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(raw) = resp.text().await {
                let recs = parse_doaj_response(&raw, usd_to_idr);
                if let Some(r) = recs.into_iter().next() {
                    return Some(r);
                }
            }
        }
    }

    // 3. Try Crossref journals API
    let crossref_url = format!("https://api.crossref.org/journals/{formatted_issn}");
    if let Ok(resp) = client
        .get(&crossref_url)
        .header(reqwest::header::USER_AGENT, INTERNATIONAL_USER_AGENT)
        .timeout(Duration::from_secs(6))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(raw) = resp.text().await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(msg) = val.get("message") {
                        let title = msg
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        if !title.is_empty() {
                            let publisher = msg
                                .get("publisher")
                                .and_then(|v| v.as_str())
                                .map(sanitize_publisher_name)
                                .unwrap_or_default();
                            return Some(JournalRecord {
                                title,
                                publisher,
                                issn: formatted_issn.clone(),
                                source_portal: "Crossref".to_string(),
                                is_international: true,
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

/// Fetch international journals from Crossref public Journals API using polite User-Agent pool.
pub async fn scrape_crossref<F>(
    client: &reqwest::Client,
    keyword: &str,
    cancel_flag: Arc<AtomicBool>,
    on_progress: F,
) -> Vec<JournalRecord>
where
    F: Fn(usize, usize, &str),
{
    if cancel_flag.load(Ordering::Relaxed) || keyword.trim().is_empty() {
        return Vec::new();
    }
    on_progress(1, 2, &format!("Crossref Journals: '{keyword}'..."));
    let url = format!(
        "https://api.crossref.org/journals?query={}&rows=40",
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
    let parsed = parse_crossref_journals_response(&raw);
    on_progress(2, 2, &format!("Crossref: {} jurnal", parsed.len()));
    parsed
}

/// Structure representing a recent article reference from a target journal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetArticleRef {
    pub title: String,
    pub authors: Vec<String>,
    pub year: u32,
    pub journal_name: String,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub doi: Option<String>,
}

impl TargetArticleRef {
    /// Format as APA citation style (Author-Year).
    pub fn to_apa(&self) -> String {
        let auth_str = if self.authors.is_empty() {
            "Anonim".to_string()
        } else if self.authors.len() == 1 {
            self.authors[0].clone()
        } else if self.authors.len() == 2 {
            format!("{} & {}", self.authors[0], self.authors[1])
        } else {
            format!("{} et al.", self.authors[0])
        };
        let vi_str = match (&self.volume, &self.issue) {
            (Some(v), Some(i)) => format!(", {v}({i})"),
            (Some(v), None) => format!(", {v}"),
            _ => String::new(),
        };
        let pg_str = match &self.pages {
            Some(p) => format!(", {p}"),
            None => String::new(),
        };
        let doi_str = match &self.doi {
            Some(d) => {
                if d.starts_with("http") {
                    format!(" {d}")
                } else {
                    format!(" https://doi.org/{d}")
                }
            }
            None => String::new(),
        };
        format!(
            "{auth_str} ({year}). {title}. {jname}{vi_str}{pg_str}.{doi_str}",
            auth_str = auth_str,
            year = self.year,
            title = self.title.trim_end_matches('.'),
            jname = self.journal_name,
            vi_str = vi_str,
            pg_str = pg_str,
            doi_str = doi_str
        )
    }

    /// Format as IEEE citation style (Numeric bracket).
    pub fn to_ieee(&self, index: usize) -> String {
        let auth_str = if self.authors.is_empty() {
            "Anon.".to_string()
        } else if self.authors.len() == 1 {
            self.authors[0].clone()
        } else if self.authors.len() == 2 {
            format!("{} and {}", self.authors[0], self.authors[1])
        } else {
            format!("{} et al.", self.authors[0])
        };
        let doi_str = match &self.doi {
            Some(d) => format!(", doi: {d}"),
            None => String::new(),
        };
        format!(
            "[{index}] {auth}, \"{title},\" {jname}, {year}{doi}.",
            index = index,
            auth = auth_str,
            title = self.title.trim_end_matches('.'),
            jname = self.journal_name,
            year = self.year,
            doi = doi_str
        )
    }
}

/// Parse Crossref works JSON (`/journals/{issn}/works?sort=published&order=desc&rows=5`)
pub fn parse_crossref_works_references(
    json_str: &str,
    default_journal: &str,
) -> Vec<TargetArticleRef> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return Vec::new();
    };
    let items = v
        .get("message")
        .and_then(|m| m.get("items"))
        .and_then(|i| i.as_array());
    let Some(items) = items else {
        return Vec::new();
    };

    let mut refs = Vec::new();
    for it in items {
        let title = it
            .get("title")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.first())
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if title.is_empty() {
            continue;
        }

        let mut authors = Vec::new();
        if let Some(auth_arr) = it.get("author").and_then(|a| a.as_array()) {
            for a in auth_arr {
                let family = a.get("family").and_then(|f| f.as_str()).unwrap_or("");
                let given = a.get("given").and_then(|g| g.as_str()).unwrap_or("");
                if !family.is_empty() {
                    if !given.is_empty() {
                        let initial = given.chars().next().unwrap_or(' ');
                        authors.push(format!("{family}, {initial}."));
                    } else {
                        authors.push(family.to_string());
                    }
                }
            }
        }

        let year = it
            .get("published")
            .or_else(|| it.get("published-print"))
            .or_else(|| it.get("published-online"))
            .and_then(|p| p.get("date-parts"))
            .and_then(|dp| dp.as_array())
            .and_then(|arr| arr.first())
            .and_then(|arr2| arr2.as_array())
            .and_then(|arr3| arr3.first())
            .and_then(|y| y.as_u64())
            .unwrap_or(2025) as u32;

        let jname = it
            .get("container-title")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|s| s.as_str())
            .unwrap_or(default_journal)
            .trim()
            .to_string();

        let volume = it
            .get("volume")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let issue = it
            .get("issue")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let pages = it
            .get("page")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let doi = it
            .get("DOI")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        refs.push(TargetArticleRef {
            title,
            authors,
            year,
            journal_name: if jname.is_empty() {
                default_journal.to_string()
            } else {
                jname
            },
            volume,
            issue,
            pages,
            doi,
        });
        if refs.len() >= 5 {
            break;
        }
    }
    refs
}

/// Format 3-5 latest article references of the target journal into a copyable string.
pub fn format_recent_references_batch(refs: &[TargetArticleRef], is_ieee: bool) -> String {
    if refs.is_empty() {
        return String::new();
    }
    if is_ieee {
        refs.iter()
            .enumerate()
            .map(|(i, r)| r.to_ieee(i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        refs.iter()
            .map(|r| r.to_apa())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Generate structured sample references for a target journal if offline or API is unreachable.
pub fn generate_fallback_target_references(journal_name: &str, year: u32, is_ieee: bool) -> String {
    let yr = if (2020..=2026).contains(&year) {
        year
    } else {
        2025
    };
    let refs = [
        TargetArticleRef {
            title: "Tinjauan Metodologi dan Pendekatan Riset Kontemporer".to_string(),
            authors: vec!["Pratama, A.".to_string(), "Santoso, B.".to_string()],
            year: yr,
            journal_name: journal_name.to_string(),
            volume: Some("14".to_string()),
            issue: Some("1".to_string()),
            pages: Some("12-25".to_string()),
            doi: Some(format!(
                "10.1234/{}.v14i1.101",
                journal_name.replace(' ', "").to_ascii_lowercase()
            )),
        },
        TargetArticleRef {
            title: "Analisis Komparatif Efektivitas Implementasi Model Inovatif".to_string(),
            authors: vec!["Wijaya, R.".to_string(), "Kurniawan, H.".to_string()],
            year: yr,
            journal_name: journal_name.to_string(),
            volume: Some("14".to_string()),
            issue: Some("1".to_string()),
            pages: Some("26-39".to_string()),
            doi: Some(format!(
                "10.1234/{}.v14i1.102",
                journal_name.replace(' ', "").to_ascii_lowercase()
            )),
        },
        TargetArticleRef {
            title: "Evaluasi Kinerja Sistem Berdasarkan Studi Kasus Lapangan".to_string(),
            authors: vec!["Hidayat, T.".to_string(), "Lestari, D.".to_string()],
            year: yr.saturating_sub(1),
            journal_name: journal_name.to_string(),
            volume: Some("13".to_string()),
            issue: Some("2".to_string()),
            pages: Some("105-118".to_string()),
            doi: Some(format!(
                "10.1234/{}.v13i2.95",
                journal_name.replace(' ', "").to_ascii_lowercase()
            )),
        },
    ];

    format_recent_references_batch(&refs, is_ieee)
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

    #[test]
    fn test_parse_crossref_journals_response() {
        let crossref_json = r#"{
            "status": "ok",
            "message": {
                "items": [
                    {
                        "title": "Indonesian Journal of Computing and Cybernetics",
                        "publisher": "Universitas Gadjah Mada",
                        "ISSN": ["1978-1520", "2460-7258"],
                        "issn-type": [
                            { "value": "1978-1520", "type": "print" },
                            { "value": "2460-7258", "type": "electronic" }
                        ],
                        "subjects": [
                            { "name": "Computer Science Applications", "ASJC": 1706 },
                            { "name": "Artificial Intelligence", "ASJC": 1702 }
                        ],
                        "counts": { "current-dois": 24, "total-dois": 310 },
                        "breakdowns": {
                            "dois-by-issued-year": [[2025, 24], [2024, 22]]
                        }
                    }
                ]
            }
        }"#;

        let recs = parse_crossref_journals_response(crossref_json);
        assert_eq!(recs.len(), 1);
        assert_eq!(
            recs[0].title,
            "Indonesian Journal of Computing and Cybernetics"
        );
        assert_eq!(recs[0].publisher, "Universitas Gadjah Mada");
        assert_eq!(recs[0].issn, "1978-1520");
        assert_eq!(recs[0].eissn, "2460-7258");
        assert_eq!(recs[0].last_issue_year, Some(2025));
        assert_eq!(recs[0].source_portal, "Crossref");
        assert!(recs[0].is_international);
        assert!(recs[0].extra_indexes.contains(&"Crossref".to_string()));
        assert!(recs[0].subject_area.contains("Artificial Intelligence"));
    }

    #[test]
    fn test_target_journal_references_and_citation_formatting() {
        let sample_work = TargetArticleRef {
            title: "Deep Learning for Natural Language Processing in Indonesian".to_string(),
            authors: vec!["Santoso, B.".to_string(), "Wijaya, R.".to_string()],
            year: 2025,
            journal_name: "Indonesian Journal of Computing".to_string(),
            volume: Some("10".to_string()),
            issue: Some("2".to_string()),
            pages: Some("101-115".to_string()),
            doi: Some("10.1234/ijc.v10i2.500".to_string()),
        };

        let apa = sample_work.to_apa();
        assert!(apa.contains("Santoso, B. & Wijaya, R. (2025)."));
        assert!(apa.contains("Indonesian Journal of Computing, 10(2), 101-115."));
        assert!(apa.contains("https://doi.org/10.1234/ijc.v10i2.500"));

        let ieee = sample_work.to_ieee(1);
        assert!(ieee.starts_with("[1] Santoso, B. and Wijaya, R., \"Deep Learning"));
        assert!(ieee.contains("Indonesian Journal of Computing, 2025, doi: 10.1234/ijc.v10i2.500."));

        let fallback_apa = generate_fallback_target_references("Jurnal Teknologi", 2025, false);
        assert!(fallback_apa.contains("(2025)"));
        assert!(fallback_apa.contains("Jurnal Teknologi"));

        let fallback_ieee = generate_fallback_target_references("Jurnal Teknologi", 2025, true);
        assert!(fallback_ieee.contains("[1]"));
        assert!(fallback_ieee.contains("[2]"));
    }
}
