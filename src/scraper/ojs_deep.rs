//! 5-point OJS deep crawler with 5.5-second hard wall-clock deadline and SQLite cache write-back.

use crate::cache::JournalCache;
use crate::config::{
    APC_UNKNOWN, CACHE_VERSION, DEAD_LINK_TEXT, INFO_NOT_FOUND, OJS_HARD_TIMEOUT_MS,
    TIMEOUT_APC_LABEL,
};
use crate::models::JournalRecord;
use crate::parser::{apc, contact, focus, heuristics, month};
use crate::scraper::dedup::normalize_ojs_root_url;
use crate::scraper::{safe_fetch_html, sanitize_publisher_name, DomainBreaker};
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use url::Url;

/// Normalize an OJS URL to its journal home root by stripping 404-prone sub-paths
/// such as `/PublicationFees`, `/about/submissions`, `/about/editorialPolicies`, or `/issue/current`.
pub fn normalize_journal_home_url(raw: &str) -> String {
    let base = normalize_ojs_root_url(raw);
    if base.is_empty() || !base.starts_with("http") {
        return base;
    }
    let Ok(mut u) = Url::parse(&base) else {
        return base;
    };
    let segments: Vec<&str> = u.path().split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return base;
    }

    let cut_idx = segments.iter().position(|seg| {
        let low = seg.to_ascii_lowercase();
        matches!(
            low.as_str(),
            "about"
                | "publicationfees"
                | "publication-fees"
                | "publicationfee"
                | "publication-fee"
                | "author-fees"
                | "authorfees"
                | "fees"
                | "issue"
                | "article"
                | "announcement"
                | "user"
                | "search"
                | "login"
        )
    });

    if let Some(idx) = cut_idx {
        let kept = &segments[..idx];
        let new_path = if kept.is_empty() {
            String::new()
        } else {
            format!("/{}", kept.join("/"))
        };
        u.set_path(&new_path);
        return normalize_ojs_root_url(u.as_str());
    }

    base
}

#[derive(Debug, Default)]
struct HtmlMeta {
    title: String,
    publisher: String,
    issn: String,
    eissn: String,
    emails: Vec<String>,
    subjects: Vec<String>,
    issue_dates: Vec<String>,
}

fn extract_html_meta(html: &str) -> HtmlMeta {
    let doc = Html::parse_document(html);
    let mut meta = HtmlMeta::default();
    if let Ok(sel) = Selector::parse("meta") {
        for m in doc.select(&sel) {
            let name = m
                .value()
                .attr("name")
                .or_else(|| m.value().attr("property"))
                .unwrap_or("")
                .to_ascii_lowercase();
            let content = m.value().attr("content").unwrap_or("").trim();
            if content.is_empty() {
                continue;
            }
            if matches!(
                name.as_str(),
                "citation_journal_title" | "og:site_name" | "dc.source"
            ) && meta.title.is_empty()
            {
                meta.title = content.to_string();
            } else if matches!(name.as_str(), "dc.publisher" | "citation_publisher")
                && meta.publisher.is_empty()
            {
                meta.publisher = sanitize_publisher_name(content);
            } else if matches!(
                name.as_str(),
                "citation_publication_date" | "citation_date" | "dc.date.issued"
            ) {
                meta.issue_dates.push(content.to_string());
            } else if matches!(
                name.as_str(),
                "citation_issn" | "dc.source.issn" | "dc.identifier.issn" | "prism.issn"
            ) {
                let digits: String = content
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
                    .to_ascii_uppercase();
                if digits.len() == 8 {
                    let norm = format!("{}-{}", &digits[..4], &digits[4..]);
                    if meta.issn.is_empty() {
                        meta.issn = norm;
                    } else if meta.eissn.is_empty() && norm != meta.issn {
                        meta.eissn = norm;
                    }
                }
            } else if matches!(name.as_str(), "citation_eissn" | "prism.eissn") {
                let digits: String = content
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
                    .to_ascii_uppercase();
                if digits.len() == 8 {
                    meta.eissn = format!("{}-{}", &digits[..4], &digits[4..]);
                }
            } else if matches!(
                name.as_str(),
                "dc.subject" | "keywords" | "citation_keywords" | "dc.subject.classification"
            ) {
                meta.subjects.push(content.to_string());
            } else if (name.contains("email") || content.contains('@')) && content.contains('@') {
                meta.emails.push(content.to_ascii_lowercase());
            }
        }
    }
    if meta.title.is_empty() {
        if let Ok(sel) = Selector::parse("title") {
            if let Some(el) = doc.select(&sel).next() {
                let t = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                let clean = t
                    .split(['|', '»', '—'])
                    .next_back()
                    .unwrap_or(&t)
                    .trim()
                    .to_string();
                if clean.len() >= 4 {
                    meta.title = clean;
                }
            }
        }
    }
    meta
}

fn discover_ojs_links(
    html: &str,
    clean_base: &str,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let doc = Html::parse_document(html);
    let Ok(base_url) = Url::parse(&format!("{clean_base}/")) else {
        return (vec![], vec![], vec![], vec![]);
    };
    let mut fee_links = Vec::new();
    let mut contact_links = Vec::new();
    let mut scope_links = Vec::new();
    let mut archive_links = Vec::new();

    let Ok(sel) = Selector::parse(
        "nav a, .navbar a, #navMenu a, #navigation a, .pkp_navigation_primary a, #sidebar a, .block a, header a, footer a, a[href]",
    ) else {
        return (vec![], vec![], vec![], vec![]);
    };

    for a in doc.select(&sel) {
        let Some(href) = a.value().attr("href") else {
            continue;
        };
        let h_trim = href.trim();
        let h_low = h_trim.to_ascii_lowercase();
        if h_trim.is_empty()
            || [".pdf", ".doc", ".docx", ".jpg", ".png", ".zip", ".xml"]
                .iter()
                .any(|ext| h_low.ends_with(ext))
        {
            continue;
        }
        let Ok(joined) = base_url.join(h_trim) else {
            continue;
        };
        let full = joined.to_string();
        if !full.starts_with("http") || full.contains("kemdiktisaintek") {
            continue;
        }
        let txt = a.text().collect::<Vec<_>>().join(" ").to_ascii_lowercase();

        if ([
            "fee",
            "apc",
            "biaya",
            "charge",
            "payment",
            "tarif",
            "author-fees",
            "biaya-publikasi",
            "publication-fee",
            "editorialpolicies",
            "editorial-policies",
            "custom-",
        ]
        .iter()
        .any(|k| txt.contains(k) || h_low.contains(k))
            || h_low.contains("/af"))
            && !fee_links.contains(&full)
        {
            fee_links.push(full.clone());
        }
        if [
            "contact",
            "kontak",
            "hubungi",
            "about",
            "editorial team",
            "editorialteam",
            "dewan redaksi",
        ]
        .iter()
        .any(|k| txt.contains(k) || h_low.contains(k))
            && !contact_links.contains(&full)
        {
            contact_links.push(full.clone());
        }
        if ["focus", "scope", "ruang lingkup", "editorialpolicies"]
            .iter()
            .any(|k| txt.contains(k) || h_low.contains(k))
            && !scope_links.contains(&full)
        {
            scope_links.push(full.clone());
        }
        if [
            "issue/archive",
            "archive",
            "arsip",
            "terbitan",
            "past issues",
        ]
        .iter()
        .any(|k| txt.contains(k) || h_low.contains(k))
            && !archive_links.contains(&full)
        {
            archive_links.push(full);
        }
    }

    (fee_links, contact_links, scope_links, archive_links)
}

fn has_complete_core_data(rec: &JournalRecord) -> bool {
    let has_paid_apc = rec.apc_category() == "paid";
    let has_contact = !rec.email.trim().is_empty() || !rec.phone.trim().is_empty();
    let has_months = !rec.publish_month_nums.is_empty();
    let has_year = rec.last_issue_year.is_some();
    has_paid_apc && has_contact && has_months && has_year
}

/// Apply parsed APC enforcing **Paid Always Beats Free** across multiple OJS pages.
fn apply_apc_with_paid_priority(
    rec: &mut JournalRecord,
    parsed_apc: apc::ApcParsed,
    evidence: String,
) {
    match parsed_apc {
        apc::ApcParsed::Paid {
            currency,
            display,
            amount_idr,
            raw_amount,
        } => {
            let numeric = (amount_idr > 0).then_some(amount_idr as u64);
            let clean_disp = crate::models::format_apc_unified(
                "paid",
                numeric,
                &display,
                crate::models::DisplayCurrency::Idr,
                crate::config::USD_TO_IDR_RATE,
            );
            rec.apc_status = clean_disp.clone();
            rec.publication_fee = clean_disp;
            rec.fee_currency = currency;
            rec.fee_amount = raw_amount;
            rec.publication_fee_idr = Some(amount_idr);
            rec.apc_idr_numeric = numeric;
            if !evidence.is_empty() {
                rec.apc_evidence = evidence;
            }
        }
        apc::ApcParsed::Free => {
            // Never overwrite an already-detected Paid APC with Free
            if rec.apc_category() != "paid"
                && (rec.apc_status.is_empty()
                    || rec.apc_status == APC_UNKNOWN
                    || rec.apc_status == INFO_NOT_FOUND
                    || rec.apc_status == TIMEOUT_APC_LABEL)
            {
                rec.apc_status = "Gratis (Rp 0)".to_string();
                rec.publication_fee = "Rp 0".to_string();
                rec.fee_currency = "IDR".to_string();
                rec.fee_amount = "0".to_string();
                rec.publication_fee_idr = Some(0);
                rec.apc_idr_numeric = Some(0);
                if !evidence.is_empty() {
                    rec.apc_evidence = evidence;
                }
            }
        }
        apc::ApcParsed::Unknown => {}
    }
}

fn update_max_year(rec: &mut JournalRecord, texts: &[String]) {
    let max_cur = month::current_local_year();
    if let Some(y) = month::extract_max_issue_year(texts, max_cur) {
        rec.last_issue_year = Some(rec.last_issue_year.map_or(y, |cur| cur.max(y)));
    }
}

fn absorb_page(
    rec: &mut JournalRecord,
    page_url: &str,
    html: &str,
    is_homepage: bool,
    agg_text: &mut String,
    archive_hints: &mut Vec<String>,
    broken_archive: &mut bool,
) {
    let plain = focus::sanitize_html_text(html);
    if agg_text.len() < 12000 {
        agg_text.push('\n');
        agg_text.push_str(&plain.chars().take(3500).collect::<String>());
    }

    if html.to_ascii_lowercase().contains("navigation.archives") {
        *broken_archive = true;
    }

    let meta = extract_html_meta(html);
    if rec.risk_warning.is_none() && (!meta.issn.is_empty() || !meta.eissn.is_empty()) {
        rec.risk_warning = heuristics::evaluate_journal_risk(
            rec,
            &plain,
            (!meta.issn.is_empty()).then_some(meta.issn.as_str()),
            (!meta.eissn.is_empty()).then_some(meta.eissn.as_str()),
        );
    }
    if rec.title.trim().is_empty() && !meta.title.is_empty() {
        rec.title = meta.title;
    }
    if rec.publisher.trim().is_empty() && !meta.publisher.is_empty() {
        rec.publisher = meta.publisher;
    }
    if rec.editor_in_chief.is_none() {
        rec.editor_in_chief = heuristics::extract_editor_in_chief(html);
    }
    if (rec.issn.is_empty() || rec.issn == INFO_NOT_FOUND) && !meta.issn.is_empty() {
        rec.issn = meta.issn;
    }
    if (rec.eissn.is_empty() || rec.eissn == INFO_NOT_FOUND) && !meta.eissn.is_empty() {
        rec.eissn = meta.eissn;
    }
    if !meta.subjects.is_empty() && (rec.subject_area.is_empty() || rec.subject_area == "Umum") {
        rec.subject_area = meta.subjects.join(", ");
    }
    if !meta.issue_dates.is_empty() {
        update_max_year(rec, &meta.issue_dates);
    }
    if rec.template_url.is_none() {
        rec.template_url = heuristics::extract_template_url(html, page_url);
    }
    if rec.turnaround_weeks.is_none() {
        if let Some((w, lbl)) = heuristics::extract_turnaround_time(&plain) {
            rec.turnaround_weeks = Some(w);
            rec.turnaround_label = lbl;
        }
    }
    if rec.citation_style.is_none() {
        rec.citation_style = heuristics::extract_citation_style(&plain);
    }
    if rec.word_limit.is_none() {
        rec.word_limit = heuristics::extract_word_limit(&plain);
    }
    for idx_name in heuristics::extract_extra_indexes(html) {
        if !rec
            .extra_indexes
            .iter()
            .any(|x| x.eq_ignore_ascii_case(&idx_name))
        {
            rec.extra_indexes.push(idx_name);
        }
    }
    if let Some(open_flag) = heuristics::extract_submission_open(&plain) {
        if rec.submission_open.is_none() || !open_flag {
            rec.submission_open = Some(open_flag);
        }
    }

    if rec.focus_and_scope.trim().is_empty()
        || rec.focus_and_scope == INFO_NOT_FOUND
        || focus::is_boilerplate_noise(&rec.focus_and_scope)
    {
        let extracted = focus::extract_focus_and_scope_from_html(html, is_homepage);
        if extracted != INFO_NOT_FOUND {
            rec.focus_and_scope = extracted;
            if page_url.to_ascii_lowercase().contains("about") && rec.about_url.is_empty() {
                rec.about_url = page_url.to_string();
            }
        } else if focus::is_boilerplate_noise(&rec.focus_and_scope) {
            rec.focus_and_scope = INFO_NOT_FOUND.to_string();
        }
    }

    // Extract contacts
    let (new_em, new_ph) = contact::extract_contacts(html, &plain, &rec.website_url);
    if !new_em.is_empty() {
        let mut combined: Vec<String> = rec
            .email
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for e in new_em.split(',').map(str::trim) {
            if !e.is_empty() && !combined.iter().any(|x| x.eq_ignore_ascii_case(e)) {
                combined.push(e.to_string());
            }
        }
        combined.truncate(3);
        rec.email = combined.join(", ");
        if rec.contact_url.is_empty() && page_url.to_ascii_lowercase().contains("contact") {
            rec.contact_url = page_url.to_string();
        }
    }
    if !new_ph.is_empty() {
        let mut combined: Vec<String> = rec
            .phone
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for p in new_ph.split(',').map(str::trim) {
            if !p.is_empty() && !combined.contains(&p.to_string()) {
                combined.push(p.to_string());
            }
        }
        combined.truncate(3);
        rec.phone = combined.join(", ");
    }

    // Always scan fee/editorial pages until a concrete Paid APC is confirmed (Paid Always Beats Free)
    if rec.apc_category() != "paid" {
        let (parsed_apc, sub_fee, evidence) = apc::extract_apc_contextual_html_with_evidence(html);
        if sub_fee.is_some() && rec.submission_fee_idr.is_none() {
            rec.submission_fee_idr = sub_fee;
        }
        apply_apc_with_paid_priority(rec, parsed_apc, evidence);
    }

    let url_low = page_url.to_ascii_lowercase();
    if rec.apc_raw_text.len() < 2800
        && (url_low.contains("submission")
            || url_low.contains("fee")
            || url_low.contains("apc")
            || url_low.contains("biaya")
            || url_low.contains("editorial")
            || url_low.contains("policy")
            || url_low.contains("custom")
            || is_homepage)
    {
        rec.apc_raw_text.push('\n');
        rec.apc_raw_text
            .push_str(&plain.chars().take(1500).collect::<String>());
    }

    // Grab current issue & announcement recency hints on homepage
    if is_homepage {
        let doc = Html::parse_document(html);
        let mut year_texts = Vec::new();
        if let Ok(sel) = Selector::parse(
            ".current_issue, #currentIssue, .obj_issue_toc, .issue-summary, .current_issue_title, .cmp_announcements, .obj_announcement_summary, #announcementsHome, .obj_article_summary .published",
        ) {
            for el in doc.select(&sel) {
                let t = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if !t.is_empty() {
                    archive_hints.push(t.clone());
                    year_texts.push(t);
                }
            }
        }
        update_max_year(rec, &year_texts);
    }
}

pub async fn deep_enrich_record(
    client: &reqwest::Client,
    breaker: Arc<DomainBreaker>,
    cache: Arc<Mutex<Option<JournalCache>>>,
    rec: JournalRecord,
    cancel_flag: Arc<AtomicBool>,
) -> JournalRecord {
    deep_enrich_record_inner(client, breaker, cache, rec, cancel_flag, false).await
}

/// Analyze a single user-supplied OJS URL deeply (including `/about/editorialTeam` probe).
pub async fn analyze_single_ojs_url(
    client: &reqwest::Client,
    breaker: Arc<DomainBreaker>,
    cache: Arc<Mutex<Option<JournalCache>>>,
    raw_url: &str,
) -> Result<JournalRecord, String> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err("Masukkan URL website jurnal OJS terlebih dahulu.".to_string());
    }
    let normalized_input = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let parsed =
        Url::parse(&normalized_input).map_err(|e| format!("Format URL tidak valid: {e}"))?;
    let host_fallback = parsed.host_str().unwrap_or("Jurnal OJS").to_string();

    let seed = JournalRecord {
        title: String::new(),
        website_url: normalized_input,
        source_portal: "Cek URL OJS".to_string(),
        ..Default::default()
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let mut enriched = deep_enrich_record_inner(client, breaker, cache, seed, cancel, true).await;
    if enriched.title.trim().is_empty() {
        enriched.title = host_fallback;
    }
    Ok(enriched)
}

async fn deep_enrich_record_inner(
    client: &reqwest::Client,
    breaker: Arc<DomainBreaker>,
    cache: Arc<Mutex<Option<JournalCache>>>,
    mut rec: JournalRecord,
    cancel_flag: Arc<AtomicBool>,
    probe_editorial: bool,
) -> JournalRecord {
    rec.cache_version = CACHE_VERSION.to_string();
    rec.publisher = sanitize_publisher_name(&rec.publisher);

    if !rec.website_url.starts_with("http") || rec.website_url.contains("kemdiktisaintek.go.id") {
        if rec.apc_status.is_empty() {
            rec.apc_status = APC_UNKNOWN.to_string();
        }
        if rec.focus_and_scope.trim().is_empty()
            || rec.focus_and_scope == INFO_NOT_FOUND
            || focus::is_boilerplate_noise(&rec.focus_and_scope)
        {
            rec.focus_and_scope = focus::synthesize_focus_scope(&rec.title, &rec.subject_area);
        }
        rec.issn_valid = heuristics::has_valid_issn_pair(&rec.issn, &rec.eissn);
        rec.refresh_activity_status(false, month::current_local_year());
        return rec;
    }

    // Step 0: Check SQLite WAL cache (incomplete records automatically bypassed inside `cache.get`)
    if let Ok(guard) = cache.lock() {
        if let Some(ref db) = *guard {
            if let Ok(Some(mut cached)) = db.get(&rec.website_url) {
                // Keep authoritative portal SINTA score & portal URLs if fresher
                if cached.sinta_rank_num().is_none() && rec.sinta_rank_num().is_some() {
                    cached.sinta_score = rec.sinta_score.clone();
                }
                if cached.sinta_url.is_empty() && !rec.sinta_url.is_empty() {
                    cached.sinta_url = rec.sinta_url.clone();
                }
                if cached.garuda_url.is_empty() && !rec.garuda_url.is_empty() {
                    cached.garuda_url = rec.garuda_url.clone();
                }
                return cached;
            }
        }
    }

    let base_root = normalize_journal_home_url(&rec.website_url);
    let mut broken_archive = false;
    let crawl_fut = async {
        let mut agg_text = String::new();
        let mut archive_hints = Vec::new();
        let per_req = Duration::from_millis(2600);

        let mut home_res = safe_fetch_html(client, &breaker, &rec.website_url, per_req).await;
        // Fallback to normalized OJS root URL if portal pointed to a 404 sub-path (e.g. /PublicationFees)
        if let Ok(ref resp) = home_res {
            if resp.status == 404 && base_root != rec.website_url && base_root.starts_with("http") {
                rec.website_url = base_root.clone();
                home_res = safe_fetch_html(client, &breaker, &base_root, per_req).await;
            }
        }
        match home_res {
            Err(err) => {
                if err.contains("timeout") {
                    rec.apc_status = TIMEOUT_APC_LABEL.to_string();
                    rec.server_status = "Timeout / Cek Manual".to_string();
                } else {
                    rec.apc_status = DEAD_LINK_TEXT.to_string();
                    rec.server_status = "Non-Aktif".to_string();
                }
                rec.is_incomplete = true;
            }
            Ok(mut home) => {
                if home.cloudflare_blocked || home.status == 403 || home.status == 503 {
                    rec.bot_protected = true;
                    rec.apc_status = "Proteksi Bot (Buka Web)".to_string();
                    rec.server_status = "Proteksi Bot / Cloudflare".to_string();
                    rec.activity_status = "Proteksi Bot (Buka Web)".to_string();
                    rec.is_dormant = false;
                    rec.is_incomplete = true;
                    return;
                }
                if home.status >= 400 {
                    rec.apc_status = DEAD_LINK_TEXT.to_string();
                    rec.is_incomplete = true;
                    return;
                }
                if home.final_url.starts_with("http") && !home.final_url.contains("kemdiktisaintek")
                {
                    rec.website_url = normalize_journal_home_url(&home.final_url);
                } else {
                    rec.website_url = normalize_journal_home_url(&rec.website_url);
                }
                let plain_home = focus::sanitize_html_text(&home.html);
                if let Some(redir_url) =
                    heuristics::extract_html_redirect_url(&home.html, &plain_home, &rec.website_url)
                {
                    rec.website_url = normalize_journal_home_url(&redir_url);
                    if let Ok(redir_out) =
                        safe_fetch_html(client, &breaker, &redir_url, per_req).await
                    {
                        if redir_out.cloudflare_blocked
                            || redir_out.status == 403
                            || redir_out.status == 503
                        {
                            rec.bot_protected = true;
                            rec.apc_status = "Proteksi Bot (Buka Web)".to_string();
                            rec.server_status = "Proteksi Bot / Cloudflare".to_string();
                            rec.activity_status = "Proteksi Bot (Buka Web)".to_string();
                            rec.is_dormant = false;
                            rec.is_incomplete = true;
                            return;
                        }
                        if redir_out.status < 400 {
                            if redir_out.final_url.starts_with("http")
                                && !redir_out.final_url.contains("kemdiktisaintek")
                            {
                                rec.website_url = normalize_journal_home_url(&redir_out.final_url);
                            }
                            home = redir_out;
                        }
                    }
                }
                let effective_base = normalize_journal_home_url(&rec.website_url);
                rec.website_url = effective_base.clone();
                absorb_page(
                    &mut rec,
                    &home.final_url,
                    &home.html,
                    true,
                    &mut agg_text,
                    &mut archive_hints,
                    &mut broken_archive,
                );
                let (m_nums_home, m_names_home, sched_home) =
                    month::resolve_schedule_and_frequency(&agg_text);
                if !m_nums_home.is_empty() {
                    rec.publish_month_nums = m_nums_home;
                    rec.publish_months = m_names_home;
                    rec.publish_schedule = sched_home;
                    rec.next_issue_estimate =
                        month::get_next_issue_estimate_now(&rec.publish_month_nums);
                }

                if (!has_complete_core_data(&rec) || rec.last_issue_year.is_none())
                    && !cancel_flag.load(Ordering::Relaxed)
                {
                    let (fee_links, contact_links, scope_links, archive_links) =
                        discover_ojs_links(&home.html, &effective_base);

                    let mut candidates: Vec<String> = Vec::new();
                    candidates.extend(fee_links.into_iter().take(2));
                    candidates.push(format!("{effective_base}/about/submissions"));
                    candidates.push(format!("{effective_base}/about/editorialPolicies"));
                    candidates.extend(contact_links.into_iter().take(2));
                    candidates.extend(scope_links.into_iter().take(2));
                    candidates.push(format!("{effective_base}/about/contact"));
                    candidates.push(format!("{effective_base}/about"));
                    candidates.push(format!("{effective_base}/about/editorialTeam"));

                    let mut visited = HashSet::new();
                    visited.insert(rec.website_url.clone());

                    let mut visits = 0usize;
                    let max_visits = if probe_editorial { 6 } else { 4 };
                    for sub in candidates {
                        let done_core = has_complete_core_data(&rec)
                            && (!probe_editorial || rec.editor_in_chief.is_some());
                        if visits >= max_visits || done_core || cancel_flag.load(Ordering::Relaxed)
                        {
                            break;
                        }
                        if !visited.insert(sub.clone()) {
                            continue;
                        }
                        visits += 1;
                        if let Ok(sub_out) = safe_fetch_html(client, &breaker, &sub, per_req).await
                        {
                            if sub_out.status == 200 && !sub_out.cloudflare_blocked {
                                absorb_page(
                                    &mut rec,
                                    &sub,
                                    &sub_out.html,
                                    false,
                                    &mut agg_text,
                                    &mut archive_hints,
                                    &mut broken_archive,
                                );
                            }
                        }
                    }

                    if probe_editorial
                        && rec.editor_in_chief.is_none()
                        && !cancel_flag.load(Ordering::Relaxed)
                    {
                        for ed_url in [
                            format!("{effective_base}/about/editorialTeam"),
                            format!("{effective_base}/about/contact"),
                        ] {
                            if !visited.insert(ed_url.clone()) {
                                continue;
                            }
                            if let Ok(ed_out) =
                                safe_fetch_html(client, &breaker, &ed_url, per_req).await
                            {
                                if ed_out.status == 200 && !ed_out.cloudflare_blocked {
                                    absorb_page(
                                        &mut rec,
                                        &ed_url,
                                        &ed_out.html,
                                        false,
                                        &mut agg_text,
                                        &mut archive_hints,
                                        &mut broken_archive,
                                    );
                                    if rec.editor_in_chief.is_some() {
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    // Global aggregated pass across homepage + subpages (Paid Always Beats Free)
                    if rec.apc_category() != "paid" && !agg_text.trim().is_empty() {
                        let (parsed_agg, ev_agg) = apc::parse_apc_with_evidence(&agg_text);
                        apply_apc_with_paid_priority(&mut rec, parsed_agg, ev_agg);
                    }
                    if rec.turnaround_weeks.is_none() && !agg_text.trim().is_empty() {
                        if let Some((w, lbl)) = heuristics::extract_turnaround_time(&agg_text) {
                            rec.turnaround_weeks = Some(w);
                            rec.turnaround_label = lbl;
                        }
                    }
                    if rec.article_languages.is_empty() {
                        rec.article_languages = heuristics::detect_article_languages(
                            &rec.title,
                            &rec.focus_and_scope,
                            &agg_text,
                            rec.is_international,
                        );
                    }
                    if rec.risk_warning.is_none() {
                        rec.risk_warning =
                            heuristics::evaluate_journal_risk(&rec, &agg_text, None, None);
                    }

                    // Resolve publish months & latest issue year from archive when still missing
                    let (mut m_nums, mut m_names, mut sched_str) =
                        month::resolve_schedule_and_frequency(&agg_text);

                    if (m_nums.is_empty() || rec.last_issue_year.is_none())
                        && !cancel_flag.load(Ordering::Relaxed)
                    {
                        if m_nums.is_empty() && !archive_hints.is_empty() {
                            m_nums = month::extract_months_from_archive_titles(&archive_hints);
                        }
                        if m_nums.is_empty() || rec.last_issue_year.is_none() {
                            let arc_url = archive_links
                                .first()
                                .cloned()
                                .unwrap_or_else(|| format!("{effective_base}/issue/archive"));
                            if let Ok(arc_out) =
                                safe_fetch_html(client, &breaker, &arc_url, per_req).await
                            {
                                if arc_out.status == 200 {
                                    if arc_out
                                        .html
                                        .to_ascii_lowercase()
                                        .contains("navigation.archives")
                                    {
                                        broken_archive = true;
                                    }
                                    let doc = Html::parse_document(&arc_out.html);
                                    if let Ok(sel) = Selector::parse(
                                        ".obj_issue_summary .title, .obj_issue_summary .published, .obj_issue_summary h2, .obj_issue_summary h3, a.title, .obj_issue_toc h2, .obj_issue_toc h3, h2, h3, h4",
                                    ) {
                                        let titles: Vec<String> = doc
                                            .select(&sel)
                                            .map(|e| e.text().collect::<Vec<_>>().join(" ").trim().to_string())
                                            .filter(|t| {
                                                let l = t.to_ascii_lowercase();
                                                l.contains("vol") || l.contains("no") || l.contains("issue") || l.contains("202") || l.contains("201")
                                            })
                                            .take(20)
                                            .collect();
                                        update_max_year(&mut rec, &titles);
                                        if m_nums.is_empty() {
                                            m_nums = month::extract_months_from_archive_titles(&titles);
                                        }
                                        if titles.is_empty() && rec.last_issue_year.is_none() {
                                            broken_archive = true;
                                        }
                                    }
                                }
                            }
                        }
                        if !m_nums.is_empty() && m_names.is_empty() {
                            m_names = month::month_nums_to_names_id(&m_nums);
                            let cnt = m_names.len();
                            let joined = if cnt > 1 {
                                format!("{} & {}", m_names[..cnt - 1].join(", "), m_names[cnt - 1])
                            } else {
                                m_names[0].clone()
                            };
                            sched_str = format!("{cnt} kali setahun ({joined})");
                        }
                    }

                    rec.publish_month_nums = m_nums;
                    rec.publish_months = m_names;
                    if !sched_str.is_empty() {
                        rec.publish_schedule = sched_str;
                    }
                    rec.next_issue_estimate =
                        month::get_next_issue_estimate_now(&rec.publish_month_nums);
                } else {
                    let (m_nums, m_names, sched_str) =
                        month::resolve_schedule_and_frequency(&agg_text);
                    if !m_nums.is_empty() {
                        rec.publish_month_nums = m_nums;
                        rec.publish_months = m_names;
                        rec.publish_schedule = sched_str;
                        rec.next_issue_estimate =
                            month::get_next_issue_estimate_now(&rec.publish_month_nums);
                    }
                }
            }
        }
    };

    if tokio::time::timeout(Duration::from_millis(OJS_HARD_TIMEOUT_MS), crawl_fut)
        .await
        .is_err()
    {
        if rec.apc_status.is_empty() || rec.apc_status == APC_UNKNOWN {
            rec.apc_status = TIMEOUT_APC_LABEL.to_string();
        }
        rec.server_status = "Timeout (Batas 5.5 dtk)".to_string();
        rec.is_incomplete = true;
    }

    if rec.apc_status.is_empty() {
        rec.apc_status = APC_UNKNOWN.to_string();
    }
    rec.publisher = sanitize_publisher_name(&rec.publisher);
    if rec.focus_and_scope.trim().is_empty()
        || rec.focus_and_scope == INFO_NOT_FOUND
        || focus::is_boilerplate_noise(&rec.focus_and_scope)
    {
        rec.focus_and_scope = focus::synthesize_focus_scope(&rec.title, &rec.subject_area);
    }
    if rec.article_languages.is_empty() {
        rec.article_languages = heuristics::detect_article_languages(
            &rec.title,
            &rec.focus_and_scope,
            "",
            rec.is_international,
        );
    }
    if rec.risk_warning.is_none() {
        rec.risk_warning = heuristics::evaluate_journal_risk(&rec, "", None, None);
    }
    rec.issn_valid = heuristics::has_valid_issn_pair(&rec.issn, &rec.eissn);
    rec.refresh_activity_status(broken_archive, month::current_local_year());

    if !rec.is_incomplete {
        if let Ok(guard) = cache.lock() {
            if let Some(ref db) = *guard {
                let _ = db.put(&rec.website_url, &rec);
                if base_root != rec.website_url {
                    let _ = db.put(&base_root, &rec);
                }
            }
        }
    }

    rec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_dormant_journal_2020() {
        let mut rec = JournalRecord {
            title: "Jurnal Teknologi Informasi dan Ekonomi (JTIEMB)".to_string(),
            publisher: "Published by Politeknik Negeri Sriwijaya".to_string(),
            issn: "2502-2377".to_string(),
            ..Default::default()
        };
        let html = r#"
        <html><body>
          <nav><a href="/about">navigation.archives</a></nav>
          <div class="cmp_announcements">
            <article class="obj_announcement_summary">
              <h3>Call for Paper Vol. 4 No. 2</h3>
              <div class="date">2020-09-16</div>
            </article>
          </div>
          <div class="current_issue">
            <h2>Vol. 4 No. 1 (2020)</h2>
          </div>
          <footer>Indexed by DOAJ, Copernicus, and Google Scholar</footer>
        </body></html>"#;

        let mut agg = String::new();
        let mut hints = Vec::new();
        let mut broken = false;
        absorb_page(
            &mut rec,
            "https://jurnal.polsri.ac.id/index.php/jtiemb/id",
            html,
            true,
            &mut agg,
            &mut hints,
            &mut broken,
        );
        rec.publisher = sanitize_publisher_name(&rec.publisher);
        rec.issn_valid = heuristics::has_valid_issn_pair(&rec.issn, &rec.eissn);
        rec.refresh_activity_status(broken, 2026);

        assert_eq!(rec.last_issue_year, Some(2020));
        assert!(rec.is_dormant);
        assert!(rec.activity_status.contains("Dorman"));
        assert!(rec.activity_status.contains("2020"));
        assert_eq!(rec.publisher, "Politeknik Negeri Sriwijaya");
        assert!(rec.issn_valid);
        assert!(rec.extra_indexes.contains(&"DOAJ".to_string()));
        assert!(rec.extra_indexes.contains(&"Copernicus".to_string()));
    }

    #[test]
    fn test_bot_protected_does_not_mark_dormant() {
        let mut rec = JournalRecord {
            title: "Jurnal Terproteksi WAF".to_string(),
            bot_protected: true,
            apc_status: "Proteksi Bot (Buka Web)".to_string(),
            ..Default::default()
        };
        rec.refresh_activity_status(false, 2026);
        assert!(!rec.is_dormant);
        assert_eq!(rec.activity_status, "Proteksi Bot (Buka Web)");
    }

    #[test]
    fn test_normalize_journal_home_url_strips_subpaths() {
        assert_eq!(
            normalize_journal_home_url(
                "https://jurnalfebi.uinsa.ac.id/index.php/maliyah/PublicationFees"
            ),
            "https://jurnalfebi.uinsa.ac.id/index.php/maliyah"
        );
        assert_eq!(
            normalize_journal_home_url(
                "https://ejournal.example.ac.id/index.php/jsi/about/editorialPolicies"
            ),
            "https://ejournal.example.ac.id/index.php/jsi"
        );
        assert_eq!(
            normalize_journal_home_url(
                "https://ejournal.example.ac.id/index.php/jsi/about/submissions?lang=id"
            ),
            "https://ejournal.example.ac.id/index.php/jsi"
        );
        assert_eq!(
            normalize_journal_home_url(
                "https://ejournal.example.ac.id/index.php/jsi/issue/current/"
            ),
            "https://ejournal.example.ac.id/index.php/jsi"
        );
    }
}
