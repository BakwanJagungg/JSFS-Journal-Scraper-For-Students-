//! Concurrent Portal Garuda scraper (`garuda.kemdiktisaintek.go.id/journal`).

use crate::config::{CACHE_VERSION, GARUDA_BASE_URL, GARUDA_JOURNAL_URL, INFO_NOT_FOUND};
use crate::models::JournalRecord;
use crate::parser::focus::normalize_focus_and_scope;
use crate::scraper::dedup::normalize_ojs_root_url;
use crate::scraper::{safe_fetch_html, sanitize_publisher_name, DomainBreaker};
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

static RE_PAGE_PARAM: Lazy<Regex> = Lazy::new(|| Regex::new(r"[?&]page=(\d+)").expect("valid regex"));
static RE_ISSN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d{4}[-\s]?[0-9Xx]{4})\b").expect("valid regex"));
static RE_SINTA: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bS(?:inta\s*)?([1-6])\b").expect("valid regex"));

fn clean_issn(raw: &str) -> String {
    let d: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if d.len() == 8 {
        format!("{}-{}", &d[..4], &d[4..])
    } else {
        INFO_NOT_FOUND.to_string()
    }
}

pub fn extract_garuda_last_page(html: &str) -> Option<usize> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a[href*='page=']").ok()?;
    let mut max_p = 1usize;
    let mut saw_any = false;
    for a in doc.select(&sel) {
        if let Some(href) = a.value().attr("href") {
            if let Some(cap) = RE_PAGE_PARAM.captures(href) {
                if let Ok(p) = cap[1].parse::<usize>() {
                    max_p = max_p.max(p);
                    saw_any = true;
                }
            }
        }
    }
    saw_any.then_some(max_p)
}

pub fn parse_garuda_listing(html: &str) -> Vec<JournalRecord> {
    let doc = Html::parse_document(html);
    let sel_row = Selector::parse("table.ui.table tr, table tr").unwrap();
    let sel_td = Selector::parse("td").unwrap();
    let sel_a = Selector::parse("a[href]").unwrap();

    let mut out = Vec::new();
    for row in doc.select(&sel_row) {
        let cells: Vec<_> = row.select(&sel_td).collect();
        if cells.is_empty() {
            continue;
        }
        let mut title = String::new();
        let mut garuda_url = String::new();
        let mut website_url = String::new();
        let mut subjects = Vec::new();

        for a in row.select(&sel_a) {
            let href = a.value().attr("href").unwrap_or("").trim();
            let txt = a.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if href.contains("/journal/view/") && title.is_empty() {
                title = txt;
                garuda_url = if href.starts_with('/') {
                    format!("{GARUDA_BASE_URL}{href}")
                } else {
                    href.to_string()
                };
            } else if href.starts_with("http")
                && !href.contains("kemdiktisaintek.go.id")
                && website_url.is_empty()
            {
                website_url = normalize_ojs_root_url(href);
            } else if href.contains("/subject") && !txt.is_empty() && !subjects.contains(&txt) {
                subjects.push(txt);
            }
        }

        if title.is_empty() {
            continue;
        }

        let row_text = row.text().collect::<Vec<_>>().join(" ");
        let publisher = cells
            .get(1)
            .map(|c| sanitize_publisher_name(&c.text().collect::<Vec<_>>().join(" ")))
            .filter(|s| !s.is_empty() && s.len() < 160)
            .unwrap_or_default();

        let issns: Vec<String> = RE_ISSN
            .captures_iter(&row_text)
            .map(|c| clean_issn(&c[1]))
            .filter(|s| s != INFO_NOT_FOUND)
            .collect();
        let issn = issns
            .first()
            .cloned()
            .unwrap_or_else(|| INFO_NOT_FOUND.to_string());
        let eissn = issns
            .get(1)
            .cloned()
            .unwrap_or_else(|| INFO_NOT_FOUND.to_string());

        let sinta_score = RE_SINTA
            .captures(&row_text)
            .map(|c| format!("Sinta {}", &c[1]))
            .unwrap_or_else(|| INFO_NOT_FOUND.to_string());

        let subject_area = if subjects.is_empty() {
            "Umum".to_string()
        } else {
            subjects.join(", ")
        };

        out.push(JournalRecord {
            title,
            publisher,
            issn,
            eissn,
            garuda_url,
            website_url,
            sinta_score,
            subject_area,
            source_portal: "Garuda".to_string(),
            cache_version: CACHE_VERSION.to_string(),
            ..Default::default()
        });
    }
    out
}

pub async fn resolve_garuda_detail(
    client: &reqwest::Client,
    breaker: &DomainBreaker,
    rec: &mut JournalRecord,
) {
    if !rec.garuda_url.contains("garuda.kemdiktisaintek.go.id/journal/view") {
        return;
    }
    let Ok(out) = safe_fetch_html(client, breaker, &rec.garuda_url, Duration::from_secs(6)).await else {
        return;
    };
    if out.status != 200 || out.cloudflare_blocked {
        return;
    }
    let doc = Html::parse_document(&out.html);
    if let Ok(sel_a) = Selector::parse("a[href]") {
        for a in doc.select(&sel_a) {
            let txt = a.text().collect::<Vec<_>>().join(" ").trim().to_ascii_lowercase();
            let mut h = a.value().attr("href").unwrap_or("").trim().to_string();
            if h.starts_with("//") {
                h = format!("https:{h}");
            }
            if matches!(txt.as_str(), "home page" | "website" | "editor url" | "portal" | "ojs")
                && h.starts_with("http")
                && !h.contains("kemdiktisaintek")
            {
                rec.website_url = normalize_ojs_root_url(&h);
                break;
            }
        }
    }

    if rec.focus_and_scope.trim().is_empty() || rec.focus_and_scope == INFO_NOT_FOUND {
        if let Ok(sel_desc) = Selector::parse("div.j-meta-desc") {
            if let Some(el) = doc.select(&sel_desc).next() {
                let txt = el.text().collect::<Vec<_>>().join(" ");
                let norm = normalize_focus_and_scope(&txt);
                if norm != INFO_NOT_FOUND {
                    rec.focus_and_scope = norm;
                }
            }
        }
    }

    if let Ok(sel_sub) = Selector::parse("div.j-meta-subject") {
        if let Some(el) = doc.select(&sel_sub).next() {
            let subj = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if !subj.is_empty() {
                rec.subject_area = subj;
            }
        }
    }
    if rec.publisher.trim().is_empty() {
        if let Ok(sel_pub) = Selector::parse("div.j-meta-pub") {
            if let Some(el) = doc.select(&sel_pub).next() {
                rec.publisher = sanitize_publisher_name(&el.text().collect::<Vec<_>>().join(" "));
            }
        }
    }
}

pub async fn scrape_garuda<F>(
    client: &reqwest::Client,
    breaker: Arc<DomainBreaker>,
    keyword: &str,
    sinta_ranks: &[u8],
    pool_target: usize,
    cancel_flag: Arc<AtomicBool>,
    on_page: F,
) -> Vec<JournalRecord>
where
    F: Fn(usize, usize, &str) + Send + Sync + 'static,
{
    let on_page = Arc::new(on_page);
    let url1 = format!(
        "{GARUDA_JOURNAL_URL}?q={}&page=1",
        urlencoding::encode(keyword)
    );
    let Ok(p1) = safe_fetch_html(client, &breaker, &url1, Duration::from_secs(12)).await else {
        return Vec::new();
    };
    if p1.status != 200 || p1.cloudflare_blocked {
        return Vec::new();
    }

    let mut all = parse_garuda_listing(&p1.html);
    if all.is_empty() {
        return Vec::new();
    }

    let per_page = all.len().max(5);
    let mut pages_needed = pool_target.div_ceil(per_page) + if sinta_ranks.is_empty() { 2 } else { 4 };
    pages_needed = pages_needed.clamp(3, 80);
    if let Some(last_p) = extract_garuda_last_page(&p1.html) {
        pages_needed = pages_needed.min(last_p.max(1));
    }

    on_page(1, pages_needed, &format!("Garuda '{keyword}' Hal. 1/{pages_needed}"));

    if pages_needed > 1 && all.len() < pool_target && !cancel_flag.load(Ordering::Relaxed) {
        let sem = Arc::new(Semaphore::new(8));
        let done_ctr = Arc::new(AtomicUsize::new(1));
        let mut set = JoinSet::new();

        for page in 2..=pages_needed {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }
            let c = client.clone();
            let br = breaker.clone();
            let kw = keyword.to_string();
            let sem_c = sem.clone();
            let cf_cancel = cancel_flag.clone();
            let ctr = done_ctr.clone();
            let cb = on_page.clone();

            set.spawn(async move {
                let _permit = sem_c.acquire().await.ok()?;
                if cf_cancel.load(Ordering::Relaxed) {
                    return None;
                }
                let url = format!("{GARUDA_JOURNAL_URL}?q={}&page={page}", urlencoding::encode(&kw));
                let res = safe_fetch_html(&c, &br, &url, Duration::from_secs(12)).await.ok()?;
                let d = ctr.fetch_add(1, Ordering::Relaxed) + 1;
                cb(d.min(pages_needed), pages_needed, &format!("Garuda '{kw}' Hal. {d}/{pages_needed}"));
                if res.status == 200 && !res.cloudflare_blocked {
                    Some((page, parse_garuda_listing(&res.html)))
                } else {
                    Some((page, Vec::new()))
                }
            });
        }

        let mut batches = Vec::new();
        while let Some(joined) = set.join_next().await {
            if let Ok(Some((p, rows))) = joined {
                if !rows.is_empty() {
                    batches.push((p, rows));
                }
            }
        }
        batches.sort_by_key(|(p, _)| *p);
        for (_, rows) in batches {
            all.extend(rows);
            if all.len() >= pool_target {
                break;
            }
        }
    }

    // Resolve OJS homepages concurrently for Garuda entries missing official website_url
    let cap = all.len().min(pool_target);
    all.truncate(cap);

    let sem = Arc::new(Semaphore::new(12));
    let mut det_set = JoinSet::new();
    for (idx, mut rec) in all.into_iter().enumerate() {
        let need_detail = !rec.website_url.starts_with("http") && rec.garuda_url.contains("/journal/view/");
        if !need_detail || cancel_flag.load(Ordering::Relaxed) {
            det_set.spawn(async move { (idx, rec) });
            continue;
        }
        let c = client.clone();
        let br = breaker.clone();
        let sem_c = sem.clone();
        det_set.spawn(async move {
            if let Ok(_p) = sem_c.acquire().await {
                resolve_garuda_detail(&c, &br, &mut rec).await;
            }
            (idx, rec)
        });
    }

    let mut resolved = Vec::new();
    while let Some(Ok(pair)) = det_set.join_next().await {
        resolved.push(pair);
    }
    resolved.sort_by_key(|(i, _)| *i);
    resolved.into_iter().map(|(_, r)| r).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_garuda_listing_and_pagination_parser() {
        let html = r#"
        <table class="ui table">
          <tr>
            <td><a href="/journal/view/421">Jurnal Informatika Kampus</a><a href="/subject/5">Computer Science</a></td>
            <td>Institut Teknologi Tes</td>
            <td>ISSN: 2442-1234 E-ISSN: 2548-5678 Sinta 4</td>
          </tr>
        </table>
        <a href="/journal?q=informatika&page=12">Last</a>"#;
        assert_eq!(extract_garuda_last_page(html), Some(12));
        let parsed = parse_garuda_listing(html);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title, "Jurnal Informatika Kampus");
        assert_eq!(parsed[0].sinta_score, "Sinta 4");
        assert_eq!(parsed[0].issn, "2442-1234");
    }
}
