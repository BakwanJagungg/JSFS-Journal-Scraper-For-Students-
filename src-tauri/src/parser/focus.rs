//! DOM sanitization, boilerplate noise filter, and Multi-Tier Focus & Scope extractor.

use crate::config::INFO_NOT_FOUND;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};

static RE_SCRIPT_STYLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<(?:script|style|noscript|svg|nav|aside|footer)[^>]*>.*?</(?:script|style|noscript|svg|nav|aside|footer)>")
        .expect("valid regex")
});
static RE_OJS_ARTICLE_LIST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)<(?:div|ul|section)[^>]*class="[^"]*(?:obj_article_summary|cmp_article_list|pkp_structure_sidebar|sidebar|block_)[^"]*"[^>]*>.*?</(?:div|ul|section)>"#)
        .expect("valid regex")
});
static RE_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").expect("valid regex"));
static RE_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").expect("valid regex"));
static RE_BOILERPLATE_PROFILE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)(?:Home\s+User\s+Home|User\s+Home\s+Journals|Navigation\s+Search\s+Current|Open\s+Journal\s+Systems\s+Help)")
        .expect("valid regex")
});

pub const BOILERPLATE_TOKENS: &[&str] = &[
    "open access policy",
    "peer review process",
    "publication ethics",
    "author guidelines",
    "submission guidelines",
    "editorial team",
    "plagiarism policy",
    "copyright notice",
    "section policies",
    "publication frequency",
    "archiving policy",
    "allegations of misconduct",
    "online submissions",
    "privacy statement",
    "the names and email addresses entered in this journal site",
    "used exclusively for the stated purposes of this journal",
    "will not be made available for any other purpose",
    "nama dan alamat surel yang dimasukkan di situs jurnal ini",
    "focus and scope",
    "about the journal",
    "editor-in-chief",
    "editorial board",
    "managing editor",
    "dewan redaksi",
    "reviewer",
];

pub fn sanitize_html_text(html: &str) -> String {
    let stripped_blocks = RE_SCRIPT_STYLE.replace_all(html, " ");
    let no_tags = RE_TAG.replace_all(&stripped_blocks, " ");
    let unescaped = no_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");
    RE_WS.replace_all(&unescaped, " ").trim().to_string()
}

pub fn clean_html_for_scope(html: &str) -> String {
    let s1 = RE_SCRIPT_STYLE.replace_all(html, " ");
    RE_OJS_ARTICLE_LIST.replace_all(&s1, " ").into_owned()
}

pub fn is_boilerplate_noise(text: &str) -> bool {
    let clean = RE_WS.replace_all(text, " ").trim().to_ascii_lowercase();
    if clean.is_empty() {
        return false;
    }
    if RE_BOILERPLATE_PROFILE.is_match(&clean) {
        return true;
    }
    const PRIVACY_BOILERPLATE: &[&str] = &[
        "the names and email addresses entered in this journal site",
        "used exclusively for the stated purposes of this journal",
        "will not be made available for any other purpose",
        "nama dan alamat surel yang dimasukkan di situs jurnal ini",
        "digunakan secara eksklusif untuk tujuan",
        "tidak akan tersedia untuk tujuan lain",
    ];
    if PRIVACY_BOILERPLATE.iter().any(|p| clean.contains(p)) {
        return true;
    }
    let editorial_tokens = [
        "editor-in-chief",
        "dewan redaksi",
        "editorial board",
        "managing editor",
        "chief editor",
    ];
    if editorial_tokens.iter().any(|t| clean.contains(t))
        && !["focus", "scope", "ruang lingkup", "bidang riset", "aims"]
            .iter()
            .any(|k| clean.contains(k))
    {
        return true;
    }

    let matched = BOILERPLATE_TOKENS
        .iter()
        .filter(|tok| clean.contains(*tok))
        .count();
    if matched >= 3 {
        let words = clean.split_whitespace().count();
        if clean.len() < 400 || (words > 0 && (matched as f64 * 2.5 / words as f64) > 0.25) {
            return true;
        }
    }
    false
}

pub fn normalize_focus_and_scope(raw: &str) -> String {
    let stripped = strip_ojs_profile_table_noise(raw);
    let clean = RE_WS.replace_all(&stripped, " ").trim().to_string();
    if clean.len() < 25 || is_boilerplate_noise(&clean) {
        return INFO_NOT_FOUND.to_string();
    }
    // Truncate cleanly to ~420 chars on word boundary
    if clean.chars().count() <= 420 {
        clean
    } else {
        let trunc: String = clean.chars().take(415).collect();
        if let Some(pos) = trunc.rfind(' ') {
            format!("{}...", &trunc[..pos])
        } else {
            format!("{trunc}...")
        }
    }
}

pub fn strip_ojs_profile_table_noise(text: &str) -> String {
    let mut out = text.to_string();
    for pat in [
        "Home User Home",
        "Open Journal Systems",
        "Journal Help",
        "Remember me",
        "Forgot your password?",
        "Developed by",
    ] {
        if let Some(idx) = out.find(pat) {
            if idx > 30 {
                out.truncate(idx);
            }
        }
    }
    out.trim().to_string()
}

pub fn extract_focus_and_scope_from_html(html: &str, is_homepage: bool) -> String {
    let cleaned_html = clean_html_for_scope(html);
    let doc = Html::parse_document(&cleaned_html);

    // Tier 1A: meta tags
    if let Ok(sel_meta) = Selector::parse("meta") {
        for m in doc.select(&sel_meta) {
            let name = m
                .value()
                .attr("name")
                .or_else(|| m.value().attr("property"))
                .unwrap_or("")
                .to_ascii_lowercase();
            let content = m.value().attr("content").unwrap_or("").trim();
            if matches!(
                name.as_str(),
                "dc.description" | "description" | "og:description" | "dc.subject"
            ) && content.len() >= 30
            {
                let low = content.to_ascii_lowercase();
                if !["open journal systems", "pkp.sfu.ca", "wordpress", "login to", "register"]
                    .iter()
                    .any(|ign| low.contains(ign))
                    && !is_boilerplate_noise(content)
                {
                    let norm = normalize_focus_and_scope(content);
                    if norm != INFO_NOT_FOUND {
                        return norm;
                    }
                }
            }
        }
    }

    // Tier 1B: targeted OJS selectors
    let target_selectors = [
        "#focusAndScope",
        "#focus_and_scope",
        ".focusAndScope",
        ".focus_and_scope",
        "#aimsAndScope",
        ".aimsAndScope",
        "div.focus-and-scope",
        ".journal-description",
        "#journalDescription",
        "div.journal-about",
        "div.homepage-about",
        "#homepageAbout",
        "div.page_about",
    ];
    for sel_str in target_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            if let Some(el) = doc.select(&sel).next() {
                let txt = RE_WS
                    .replace_all(&el.text().collect::<Vec<_>>().join(" "), " ")
                    .trim()
                    .to_string();
                let low = txt.to_ascii_lowercase();
                if txt.len() >= 30
                    && !["open journal systems", "pkp", "login to", "register", "editorial team"]
                        .iter()
                        .any(|ign| low.contains(ign))
                    && !is_boilerplate_noise(&txt)
                {
                    let norm = normalize_focus_and_scope(&txt);
                    if norm != INFO_NOT_FOUND {
                        return norm;
                    }
                }
            }
        }
    }

    // Tier 1C & Tier 2: substantive <p> paragraphs (especially on OJS homepage/about)
    let p_sel_str = if is_homepage {
        "#content p, #main-content p, div.pkp_structure_main p, div.page_about p, main p, p"
    } else {
        "p"
    };
    if let Ok(p_sel) = Selector::parse(p_sel_str) {
        let research_cues = [
            "jurnal",
            "journal",
            "menerbitkan",
            "publishes",
            "mempublikasikan",
            "penelitian",
            "research",
            "bidang",
            "field",
            "artikel",
            "article",
            "studi",
            "study",
            "fokus",
            "focus",
            "ruang lingkup",
            "scope",
            "cakupan",
            "ilmiah",
            "kajian",
        ];
        for p in doc.select(&p_sel) {
            let p_text = RE_WS
                .replace_all(&p.text().collect::<Vec<_>>().join(" "), " ")
                .trim()
                .to_string();
            if (35..=900).contains(&p_text.len()) {
                let p_low = p_text.to_ascii_lowercase();
                if research_cues.iter().any(|k| p_low.contains(k))
                    && !["login to", "remember me", "forgot your password", "open journal systems", "pkp.sfu.ca", "developed by"]
                        .iter()
                        .any(|ign| p_low.contains(ign))
                    && !is_boilerplate_noise(&p_text)
                {
                    let norm = normalize_focus_and_scope(&p_text);
                    if norm != INFO_NOT_FOUND {
                        return norm;
                    }
                }
            }
        }
    }

    INFO_NOT_FOUND.to_string()
}

pub fn synthesize_focus_scope(title: &str, subject_area: &str) -> String {
    let subj = subject_area.trim();
    if !subj.is_empty() && subj != INFO_NOT_FOUND && !subj.eq_ignore_ascii_case("umum") {
        format!("Jurnal ilmiah yang berfokus pada bidang {subj} ({title}).")
    } else {
        INFO_NOT_FOUND.to_string()
    }
}

pub fn build_tier3_scope_fallback(title: &str, subject_area: &str) -> String {
    synthesize_focus_scope(title, subject_area)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_boilerplate_rejection_and_scope_extraction() {
        let nav_noise = "Open Access Policy Peer Review Process Publication Ethics Author Guidelines Editorial Team Plagiarism Policy Copyright Notice";
        assert!(is_boilerplate_noise(nav_noise));

        let html = r#"
        <html><body>
          <div id="focusAndScope">
            <p>Jurnal Ekonomi & Bisnis Digital menerbitkan artikel penelitian empiris dan konseptual dalam bidang e-commerce, pemasaran digital, fintech, dan manajemen UMKM.</p>
          </div>
        </body></html>"#;
        let scope = extract_focus_and_scope_from_html(html, false);
        assert!(scope.contains("e-commerce"));
        assert!(scope.contains("UMKM"));
    }

    #[test]
    fn test_strip_ojs_privacy_statement_from_focus() {
        let privacy_html = r#"
        <html><body>
          <div id="focusAndScope">
            <p>The names and email addresses entered in this journal site will be used exclusively for the stated purposes of this journal and will not be made available for any other purpose or to any other party.</p>
          </div>
        </body></html>"#;
        let extracted = extract_focus_and_scope_from_html(privacy_html, false);
        assert_eq!(extracted, INFO_NOT_FOUND);

        let fallback = synthesize_focus_scope(
            "Jurnal Inovasi Bisnis Manajemen dan Akuntansi",
            "Business, Management, Accounting",
        );
        assert!(fallback.contains("Business, Management, Accounting"));
        assert!(fallback.contains("Jurnal Inovasi Bisnis Manajemen dan Akuntansi"));
    }
}
