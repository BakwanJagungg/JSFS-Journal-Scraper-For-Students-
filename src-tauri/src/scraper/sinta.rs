//! Concurrent Portal SINTA scraper (`sinta.kemdiktisaintek.go.id/journals`).

use crate::config::{CACHE_VERSION, INFO_NOT_FOUND, SINTA_BASE_URL, SINTA_JOURNAL_URL};
use crate::models::JournalRecord;
use crate::scraper::dedup::normalize_ojs_root_url;
use crate::scraper::{safe_fetch_html, DomainBreaker};
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

static RE_SINTA_RANK: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)S(\d)").expect("valid regex"));
static RE_PISSN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)P-?ISSN\s*:\s*([0-9Xx\-]{4,10})").expect("valid regex")
});
static RE_EISSN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)E-?ISSN\s*:\s*([0-9Xx\-]{4,10})").expect("valid regex")
});
static RE_SUBJ: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)Subject\s*Area\s*:\s*([^|\n]+)").expect("valid regex")
});

/// Strip `"Published by"`, `"Diterbitkan oleh"`, and SINTA link-bar noise from publisher strings.
pub fn sanitize_sinta_publisher(raw: &str) -> String {
    crate::scraper::sanitize_publisher_name(raw)
}

fn format_issn(raw: &str) -> String {
    let d: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if d.len() == 8 {
        format!("{}-{}", &d[..4], &d[4..])
    } else {
        INFO_NOT_FOUND.to_string()
    }
}

pub fn parse_sinta_listing(html: &str) -> Vec<JournalRecord> {
    let doc = Html::parse_document(html);
    let Ok(sel_item) = Selector::parse("div.list-item") else {
        return Vec::new();
    };
    let sel_title = Selector::parse("div.affil-name a, .affil-name").unwrap();
    let sel_affil = Selector::parse("div.affil-loc a[href*='affiliations'], div.affil-loc a, div.affil-loc, .affil-abbrev").unwrap();
    let sel_prof = Selector::parse("div.profile-id").unwrap();
    let sel_rank = Selector::parse("span.num-stat.accredited, .accredited").unwrap();
    let sel_anchor = Selector::parse("a[href]").unwrap();

    let mut out = Vec::new();
    for item in doc.select(&sel_item) {
        let Some(t_el) = item.select(&sel_title).next() else {
            continue;
        };
        let title = t_el
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if title.is_empty() {
            continue;
        }

        let href = t_el.value().attr("href").unwrap_or("").trim();
        let sinta_url = if href.starts_with('/') {
            format!("{SINTA_BASE_URL}{href}")
        } else if href.starts_with("http") {
            href.to_string()
        } else {
            String::new()
        };

        let publisher = item
            .select(&sel_affil)
            .filter_map(|el| {
                let raw = el.text().collect::<Vec<_>>().join(" ");
                let cleaned = sanitize_sinta_publisher(&raw);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .next()
            .unwrap_or_default();

        let prof_text = item
            .select(&sel_prof)
            .map(|el| el.text().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(" ");

        let issn = RE_PISSN
            .captures(&prof_text)
            .map(|c| format_issn(&c[1]))
            .unwrap_or_else(|| INFO_NOT_FOUND.to_string());
        let eissn = RE_EISSN
            .captures(&prof_text)
            .map(|c| format_issn(&c[1]))
            .unwrap_or_else(|| INFO_NOT_FOUND.to_string());
        let subject_area = RE_SUBJ
            .captures(&prof_text)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_else(|| "Umum".to_string());

        let sinta_score = item
            .select(&sel_rank)
            .next()
            .and_then(|el| {
                let txt = el.text().collect::<Vec<_>>().join(" ");
                RE_SINTA_RANK
                    .captures(&txt)
                    .map(|c| format!("Sinta {}", &c[1]))
            })
            .unwrap_or_else(|| INFO_NOT_FOUND.to_string());

        let mut website_url = String::new();
        let mut garuda_url = String::new();
        for a in item.select(&sel_anchor) {
            let txt = a.text().collect::<Vec<_>>().join(" ").to_ascii_lowercase();
            let h = a.value().attr("href").unwrap_or("").trim();
            if h.contains("garuda.kemdiktisaintek") && garuda_url.is_empty() {
                garuda_url = h.to_string();
            } else if h.starts_with("http")
                && !h.contains("kemdiktisaintek.go.id")
                && (txt.contains("website") || txt.contains("editor") || txt.contains("ojs") || website_url.is_empty())
            {
                website_url = normalize_ojs_root_url(h);
                if txt.contains("website") || txt.contains("editor") {
                    break;
                }
            }
        }

        out.push(JournalRecord {
            title,
            publisher,
            issn,
            eissn,
            garuda_url,
            website_url,
            sinta_url,
            sinta_score,
            subject_area,
            source_portal: "SINTA".to_string(),
            cache_version: CACHE_VERSION.to_string(),
            ..Default::default()
        });
    }
    out
}

pub async fn scrape_sinta<F>(
    client: &reqwest::Client,
    breaker: Arc<DomainBreaker>,
    keyword: &str,
    sinta_ranks: &[u8],
    pool_target: usize,
    cancel_flag: Arc<AtomicBool>,
    on_page: F,
) -> (Vec<JournalRecord>, bool)
where
    F: Fn(usize, usize, &str) + Send + Sync + 'static,
{
    let on_page = Arc::new(on_page);
    let url1 = format!(
        "{SINTA_JOURNAL_URL}?q={}&page=1",
        urlencoding::encode(keyword)
    );
    let Ok(p1) = safe_fetch_html(client, &breaker, &url1, Duration::from_secs(10)).await else {
        return (Vec::new(), false);
    };
    if p1.cloudflare_blocked || p1.status != 200 {
        return (Vec::new(), p1.cloudflare_blocked);
    }

    let mut all = parse_sinta_listing(&p1.html);
    if all.is_empty() {
        return (Vec::new(), false);
    }

    let pages_needed = if !sinta_ranks.is_empty() {
        ((pool_target as f64) * 0.45 + 6.0).ceil() as usize
    } else {
        pool_target.div_ceil(10) + 2
    }
    .clamp(3, 60);

    on_page(1, pages_needed, &format!("SINTA '{keyword}' Hal. 1/{pages_needed}"));

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
                let url = format!("{SINTA_JOURNAL_URL}?q={}&page={page}", urlencoding::encode(&kw));
                let res = safe_fetch_html(&c, &br, &url, Duration::from_secs(10)).await.ok()?;
                let d = ctr.fetch_add(1, Ordering::Relaxed) + 1;
                cb(d.min(pages_needed), pages_needed, &format!("SINTA '{kw}' Hal. {d}/{pages_needed}"));
                if res.cloudflare_blocked {
                    return Some((page, Vec::new(), true));
                }
                if res.status == 200 {
                    Some((page, parse_sinta_listing(&res.html), false))
                } else {
                    Some((page, Vec::new(), false))
                }
            });
        }

        let mut page_batches = Vec::new();
        let mut hit_cf = false;
        while let Some(joined) = set.join_next().await {
            if let Ok(Some((p, rows, cf))) = joined {
                if cf {
                    hit_cf = true;
                }
                if !rows.is_empty() {
                    page_batches.push((p, rows));
                }
            }
        }
        page_batches.sort_by_key(|(p, _)| *p);
        for (_, rows) in page_batches {
            all.extend(rows);
            if all.len() >= pool_target {
                break;
            }
        }
        return (all, hit_cf);
    }

    (all, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sinta_listing_html_parser() {
        let html = r##"
        <div class="list-item">
          <div class="affil-name"><a href="/journals/profile/999">Jurnal Riset Mahasiswa</a></div>
          <div class="affil-loc"><a href="#">Universitas Negeri Tes</a></div>
          <div class="profile-id">P-ISSN : 12345678 | E-ISSN : 87654321 | Subject Area : Economy, Management</div>
          <span class="num-stat accredited">S3 Accredited</span>
          <a href="https://ejournal.tes.ac.id/index.php/jrm">Website</a>
        </div>"##;
        let parsed = parse_sinta_listing(html);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].sinta_score, "Sinta 3");
        assert_eq!(parsed[0].issn, "1234-5678");
        assert_eq!(parsed[0].eissn, "8765-4321");
        assert!(parsed[0].website_url.contains("ejournal.tes.ac.id"));
    }

    #[test]
    fn test_sinta_publisher_noise_sanitizer() {
        let noisy_html = r##"
        <div class="list-item">
          <div class="affil-name"><a href="/journals/profile/101">Jurnal Bisnis &amp; Manajemen</a></div>
          <div class="affil-abbrev">Google Scholar   Website   Editor URL</div>
          <div class="affil-loc">Published by Universitas Diponegoro | Google Scholar | Website</div>
          <div class="profile-id">P-ISSN : 11112222 | E-ISSN : 33334444 | Subject Area : Business</div>
          <span class="num-stat accredited">S2 Accredited</span>
        </div>
        <div class="list-item">
          <div class="affil-name"><a href="/journals/profile/102">Jurnal Ekonomi Tanpa Penerbit</a></div>
          <div class="affil-loc"><a href="https://scholar.google.com">Google Scholar</a> <a href="https://ejournal.example.ac.id">Website</a> <a href="#">Editor URL</a></div>
          <span class="num-stat accredited">S4 Accredited</span>
        </div>"##;
        let parsed = parse_sinta_listing(noisy_html);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].publisher, "Universitas Diponegoro");
        assert_eq!(parsed[1].publisher, "");
        assert_eq!(sanitize_sinta_publisher("Google Scholar Website Editor URL"), "");
    }
}
