//! Email & Indonesian phone/WhatsApp extractor with Cloudflare XOR decoder and dummy filtering.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;
use url::Url;

static RE_CFEMAIL_ATTR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"data-cfemail=["']([0-9a-fA-F]{6,})["']"#).expect("valid regex")
});
static RE_CFEMAIL_HREF: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"/cdn-cgi/l/email-protection#([0-9a-fA-F]{6,})"#).expect("valid regex")
});
static RE_MAILTO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)mailto:([^\s"'<>?]+)"#).expect("valid regex")
});
static RE_EMAIL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,12}"#).expect("valid regex")
});
static RE_DEOBF_AT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\s*(?:\[at\]|\(at\)|\{at\}|\s+at\s+)\s*"#).expect("valid regex")
});
static RE_DEOBF_DOT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\s*(?:\[dot\]|\(dot\)|\{dot\})\s*"#).expect("valid regex")
});
static RE_TEL_WA: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(?:tel:|wa\.me/|phone=)(\+?[0-9\-\s()]{8,20})"#).expect("valid regex")
});
static RE_PHONE_CAND: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:\+62|62|0)[0-9][0-9\s\-().]{7,18}"#).expect("valid regex")
});

const DUMMY_PREFIXES: &[&str] = &[
    "firstauthor@",
    "secondauthor@",
    "thirdauthor@",
    "first@",
    "second@",
    "third@",
    "author@",
    "authors@",
    "user@",
    "admin@domain",
    "noreply@",
    "no-reply@",
    "support@pkp",
    "sample@",
];

const DUMMY_DOMAINS: &[&str] = &[
    "example.com",
    "domain.com",
    "sfu.ca",
    "pkp.sfu.ca",
    "email.com",
    "test.com",
    "mail.com",
    "yoursite.com",
    "journal.com",
];

const VALID_AREA_CODES: &[&str] = &[
    "21", "22", "24", "31", "61", "231", "251", "261", "271", "274", "281", "291", "331", "341",
    "351", "361", "370", "380", "411", "431", "451", "511", "541", "561", "711", "721", "741",
    "751", "761", "771", "911", "967",
];

const CONTACT_CUES: &[&str] = &[
    "tel", "tlp", "phone", "hp", "wa", "whatsapp", "fax", "kontak", "contact", "hubungi",
    "mobile", "editor", "redaksi", "sekretariat",
];

/// Decode Cloudflare's hex XOR obfuscated email (`data-cfemail`).
pub fn decode_cfemail(hex_str: &str) -> Option<String> {
    let bytes = hex::decode(hex_str.trim()).ok()?;
    if bytes.len() < 4 {
        return None;
    }
    let key = bytes[0];
    let decoded: Vec<u8> = bytes[1..].iter().map(|&b| b ^ key).collect();
    let s = String::from_utf8(decoded).ok()?;
    let trimmed = s.trim().to_ascii_lowercase();
    if RE_EMAIL.is_match(&trimmed) {
        Some(trimmed)
    } else {
        None
    }
}

pub fn deobfuscate_email_text(text: &str) -> String {
    let step1 = RE_DEOBF_AT.replace_all(text, "@");
    RE_DEOBF_DOT.replace_all(&step1, ".").to_string()
}

fn is_valid_email(email: &str) -> bool {
    let low = email.trim().to_ascii_lowercase();
    if low.len() > 80 || !low.contains('@') {
        return false;
    }
    let Some((_, dom)) = low.split_once('@') else {
        return false;
    };
    if DUMMY_PREFIXES.iter().any(|p| low.starts_with(p)) {
        return false;
    }
    if DUMMY_DOMAINS
        .iter()
        .any(|d| dom == *d || dom.ends_with(&format!(".{d}")))
    {
        return false;
    }
    if matches!(
        dom.rsplit('.').next(),
        Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "css" | "js" | "pdf" | "webp" | "woff")
    ) {
        return false;
    }
    !low.contains("firstauthor") && !low.contains("secondauthor")
}

/// Normalize Indonesian mobile (`08xx` / `628xx` -> `+628xx`) and valid landlines.
pub fn normalize_indonesian_phone(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 9 || digits.len() > 14 {
        return None;
    }
    // Reject trivial sequential/repeated placeholders
    if digits.contains("1234567") || digits.chars().collect::<HashSet<_>>().len() <= 2 {
        return None;
    }

    let national = if let Some(rest) = digits.strip_prefix("62") {
        rest.to_string()
    } else {
        let rest = digits.strip_prefix('0')?;
        rest.to_string()
    };

    if national.starts_with('8') && (9..=12).contains(&national.len()) {
        return Some(format!("+62{national}"));
    }

    // Validate Indonesian landline area codes
    if VALID_AREA_CODES.iter().any(|ac| national.starts_with(ac))
        && (7..=11).contains(&national.len())
    {
        return Some(format!("+62{national}"));
    }

    None
}

/// Extract cleaned, ranked email list and Indonesian phone/WhatsApp list from raw HTML & plain text.
pub fn extract_contacts(html: &str, plain_text: &str, ojs_url: &str) -> (String, String) {
    let mut email_set = Vec::new();
    let mut push_email = |cand: &str| {
        let low = cand.trim().trim_end_matches('.').to_ascii_lowercase();
        if is_valid_email(&low) && !email_set.contains(&low) {
            email_set.push(low);
        }
    };

    for cap in RE_CFEMAIL_ATTR.captures_iter(html) {
        if let Some(decoded) = decode_cfemail(&cap[1]) {
            push_email(&decoded);
        }
    }
    for cap in RE_CFEMAIL_HREF.captures_iter(html) {
        if let Some(decoded) = decode_cfemail(&cap[1]) {
            push_email(&decoded);
        }
    }
    for cap in RE_MAILTO.captures_iter(html) {
        let clean = cap[1].split('?').next().unwrap_or("");
        if let Some(m) = RE_EMAIL.find(clean) {
            push_email(m.as_str());
        }
    }

    let deobf = deobfuscate_email_text(plain_text);
    for m in RE_EMAIL.find_iter(&deobf) {
        push_email(m.as_str());
    }

    // Rank emails: matching OJS root domain first, then `.ac.id`/`.go.id`, then others
    let host_root = Url::parse(ojs_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
        .unwrap_or_default();
    let base_dom = host_root
        .strip_prefix("www.")
        .unwrap_or(&host_root)
        .to_string();

    email_set.sort_by_key(|em| {
        let dom = em.split('@').nth(1).unwrap_or("");
        if !base_dom.is_empty() && (dom == base_dom || base_dom.ends_with(dom) || dom.ends_with(&base_dom)) {
            0
        } else if dom.ends_with(".ac.id") || dom.ends_with(".go.id") || dom.ends_with(".edu") {
            1
        } else {
            2
        }
    });
    email_set.truncate(3);

    // Phones: tel:/wa.me links + context-guarded numbers in plain text
    let mut phones = Vec::new();
    let mut push_phone = |raw: &str| {
        if let Some(norm) = normalize_indonesian_phone(raw) {
            if !phones.contains(&norm) {
                phones.push(norm);
            }
        }
    };

    for cap in RE_TEL_WA.captures_iter(html) {
        push_phone(&cap[1]);
    }
    let plain_low = plain_text.to_ascii_lowercase();
    for m in RE_PHONE_CAND.find_iter(plain_text) {
        let s = m.start();
        let e = m.end();
        let ctx_start = floor_char_boundary(&plain_low, s.saturating_sub(45));
        let ctx_end = ceil_char_boundary(&plain_low, (e + 20).min(plain_low.len()));
        let window = &plain_low[ctx_start..ctx_end];
        if CONTACT_CUES.iter().any(|c| window.contains(c)) {
            push_phone(m.as_str());
        }
    }
    phones.truncate(3);

    (email_set.join(", "), phones.join(", "))
}

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloudflare_email_decoder_and_deobfuscation() {
        // Known Cloudflare hex encoding for "jurnal@kampus.ac.id" using key 0x2a:
        let key: u8 = 0x2a;
        let target = b"jurnal@kampus.ac.id";
        let mut raw = vec![key];
        for b in target {
            raw.push(b ^ key);
        }
        let hex_s = hex::encode(raw);
        assert_eq!(decode_cfemail(&hex_s).as_deref(), Some("jurnal@kampus.ac.id"));

        let text = "Hubungi redaksi [at] universitas [dot] ac [dot] id atau firstauthor@example.com";
        let (emails, _) = extract_contacts("", text, "https://ejournal.universitas.ac.id");
        assert_eq!(emails, "redaksi@universitas.ac.id");
    }

    #[test]
    fn test_phone_normalization_and_noise_rejection() {
        let html = r#"<a href="https://wa.me/6281298765432">WA</a> Telp: (021) 7863456. Tahun 2024 ISSN 123456789."#;
        let plain = "WA Telp: (021) 7863456. Tahun 2024 nomor random 0000000000.";
        let (_, phones) = extract_contacts(html, plain, "");
        assert!(phones.contains("+6281298765432"));
        assert!(phones.contains("+62217863456"));
    }
}
