//! Manuscript parser (.docx / .pdf) and Bilingual Academic Match Scorer.

use crate::error::JsfsError;
use crate::models::{JournalRecord, ManuscriptMeta};
use once_cell::sync::Lazy;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use zip::ZipArchive;

static RE_TOKEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-zA-Z0-9]{3,}").expect("valid regex"));

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "from", "are", "was", "were", "have", "has",
    "using", "based", "study", "analysis", "research", "paper", "article", "journal", "abstract",
    "abstrak", "pendahuluan", "metode", "hasil", "kesimpulan", "dan", "yang", "untuk", "dengan",
    "pada", "dari", "dalam", "atau", "adalah", "terhadap", "tentang", "sebagai", "oleh", "ini",
    "itu", "jurnal", "penelitian", "analisis", "studi", "sistem", "berbasis", "metode",
    "pendekatan", "pengaruh", "implementasi", "penerapan", "perancangan", "pengembangan",
];

/// 8 academic domain synonym clusters (`doc_reader.py` parity).
const BILINGUAL_CLUSTERS: &[(&str, &[&str])] = &[
    (
        "Ekonomi & Bisnis Digital",
        &[
            "bisnis", "business", "commerce", "ecommerce", "manajemen", "management", "akuntansi",
            "accounting", "keuangan", "finance", "ekonomi", "economy", "economic", "pemasaran",
            "marketing", "umkm", "msme", "ewallet", "fintech", "wirausaha", "entrepreneurship",
            "perbankan", "banking", "investasi", "investment", "pajak", "tax",
        ],
    ),
    (
        "Informatika & Komputer",
        &[
            "informatika", "informatics", "komputer", "computer", "perangkat", "software",
            "aplikasi", "application", "machine", "learning", "kecerdasan", "artificial", "data",
            "mining", "jaringan", "network", "keamanan", "security", "siber", "cyber", "cloud",
            "web", "mobile", "android", "iot", "algoritma", "algorithm", "klasifikasi", "nlp",
        ],
    ),
    (
        "Pendidikan",
        &[
            "pendidikan", "education", "pembelajaran", "learning", "pengajaran", "teaching",
            "kurikulum", "curriculum", "siswa", "student", "mahasiswa", "guru", "teacher",
            "pedagogi", "pedagogy", "sekolah", "school", "elearning", "literasi", "literacy",
            "matematika", "bahasa",
        ],
    ),
    (
        "Hukum",
        &[
            "hukum", "law", "legal", "yuridis", "juridical", "pidana", "criminal", "perdata",
            "civil", "konstitusi", "constitution", "regulasi", "regulation", "undang", "statute",
            "pengadilan", "court", "hakim", "ham", "syariah",
        ],
    ),
    (
        "Kesehatan & Kedokteran",
        &[
            "kesehatan", "health", "kedokteran", "medical", "medicine", "keperawatan", "nursing",
            "farmasi", "pharmacy", "gizi", "nutrition", "klinik", "clinical", "pasien", "patient",
            "penyakit", "disease", "puskesmas", "epidemiologi", "epidemiology", "stunting",
        ],
    ),
    (
        "Pertanian & Agribisnis",
        &[
            "pertanian", "agriculture", "agricultural", "agribisnis", "agribusiness", "tanaman",
            "crop", "pangan", "food", "kelautan", "marine", "perikanan", "fisheries", "peternakan",
            "livestock", "kehutanan", "forestry", "agroteknologi", "hortikultura",
        ],
    ),
    (
        "Komunikasi & Media",
        &[
            "komunikasi", "communication", "media", "jurnalistik", "journalism", "penyiaran",
            "broadcasting", "periklanan", "advertising", "humas", "public", "relations", "digital",
            "sosial", "brand", "branding",
        ],
    ),
    (
        "Ilmu Sosial & Pemerintahan",
        &[
            "sosial", "social", "sosiologi", "sociology", "politik", "politics", "political",
            "kebijakan", "policy", "pemerintahan", "government", "administrasi", "administration",
            "masyarakat", "community", "desa", "village", "psikologi", "psychology",
        ],
    ),
];

/// Extract plain text cleanly from `.docx` bytes (`word/document.xml` -> `<w:t>` text nodes).
pub fn extract_text_from_docx_bytes(bytes: &[u8]) -> Result<String, JsfsError> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| JsfsError::Document(format!("DOCX tidak valid: {e}")))?;

    let mut xml_bytes = Vec::new();
    for name in ["word/document.xml", "word/header1.xml", "word/footer1.xml"] {
        if let Ok(mut file) = archive.by_name(name) {
            let mut part = Vec::new();
            if file.read_to_end(&mut part).is_ok() {
                xml_bytes.extend_from_slice(&part);
                xml_bytes.push(b'\n');
            }
        }
    }
    if xml_bytes.is_empty() {
        return Err(JsfsError::Document(
            "Berkas .docx tidak memuat word/document.xml".to_string(),
        ));
    }

    let mut reader = Reader::from_reader(xml_bytes.as_slice());
    reader.config_mut().trim_text(true);
    let mut out = String::new();
    let mut in_text_node = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = e.local_name();
                if local.as_ref() == b"t" {
                    in_text_node = true;
                }
            }
            Ok(Event::End(e)) => {
                let local = e.local_name();
                if local.as_ref() == b"t" {
                    in_text_node = false;
                    out.push(' ');
                } else if local.as_ref() == b"p" {
                    out.push('\n');
                }
            }
            Ok(Event::Text(e)) if in_text_node => {
                if let Ok(txt) = e.unescape() {
                    out.push_str(&txt);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(JsfsError::Document(format!(
                    "Gagal mengurai XML .docx: {e}"
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(out.trim().to_string())
}

/// Extract plain text from `.pdf` bytes using pure-Rust `pdf_extract`.
pub fn extract_text_from_pdf_bytes(bytes: &[u8]) -> Result<String, JsfsError> {
    let text = pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| JsfsError::Document(format!("Gagal membaca PDF: {e}")))?;
    Ok(text.trim().to_string())
}

/// Tokenize text, drop Indonesian/English stopwords, and return frequency-ordered keywords.
pub fn extract_keywords_and_domain(text: &str, top_n: usize) -> (Vec<String>, String) {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    let mut counts: HashMap<String, usize> = HashMap::new();
    // Cap scan window to first ~35,000 chars for fast deterministic extraction
    let sample: String = text.chars().take(35_000).collect();

    for m in RE_TOKEN.find_iter(&sample) {
        let w = m.as_str().to_ascii_lowercase();
        if w.len() >= 3 && !stop.contains(w.as_str()) && !w.chars().all(|c| c.is_ascii_digit()) {
            *counts.entry(w).or_insert(0) += 1;
        }
    }

    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let top_kws: Vec<String> = pairs.into_iter().take(top_n).map(|(k, _)| k).collect();

    // Detect dominant academic cluster
    let low_sample = sample.to_ascii_lowercase();
    let mut best_domain = String::new();
    let mut best_hits = 0usize;
    for &(dom, terms) in BILINGUAL_CLUSTERS {
        let hits: usize = terms.iter().filter(|t| low_sample.contains(**t)).count();
        if hits > best_hits {
            best_hits = hits;
            best_domain = dom.to_string();
        }
    }

    (top_kws, best_domain)
}

pub fn build_manuscript_meta(filename: &str, bytes: &[u8]) -> Result<ManuscriptMeta, JsfsError> {
    let low = filename.to_ascii_lowercase();
    let extracted_text = if low.ends_with(".docx") {
        extract_text_from_docx_bytes(bytes)?
    } else if low.ends_with(".pdf") {
        extract_text_from_pdf_bytes(bytes)?
    } else {
        return Err(JsfsError::InvalidInput(
            "Hanya file .docx atau .pdf yang didukung".to_string(),
        ));
    };

    let char_count = extracted_text.chars().count();
    let (detected_keywords, detected_domain) = extract_keywords_and_domain(&extracted_text, 12);

    Ok(ManuscriptMeta {
        filename: filename.to_string(),
        char_count,
        extracted_text: extracted_text.chars().take(12_000).collect(),
        detected_keywords,
        detected_domain,
    })
}

/// Expand tokens using bilingual academic synonym clusters so Indonesian & English match cleanly.
fn expand_bilingual_tokens(tokens: &[String]) -> HashSet<String> {
    let mut out = HashSet::new();
    for t in tokens {
        let low = t.to_ascii_lowercase();
        if low.len() < 2 {
            continue;
        }
        out.insert(low.clone());
        for &(_, cluster_terms) in BILINGUAL_CLUSTERS {
            if cluster_terms.contains(&low.as_str()) {
                for &syn in cluster_terms {
                    out.insert(syn.to_string());
                }
            }
        }
    }
    out
}

/// Compute weighted semantic/keyword match score (`0.0..=100.0`), label, and reasons.
pub fn compute_match_score(
    record: &JournalRecord,
    search_keywords: &[String],
    manuscript: Option<&ManuscriptMeta>,
) -> (f64, String, Vec<String>) {
    let mut query_terms: Vec<String> = Vec::new();
    for kw in search_keywords {
        for part in kw.split(|c: char| !c.is_alphanumeric()) {
            let p = part.trim().to_ascii_lowercase();
            if p.len() >= 2 && !query_terms.contains(&p) {
                query_terms.push(p);
            }
        }
    }
    if let Some(ms) = manuscript {
        for kw in ms.detected_keywords.iter().take(10) {
            if !query_terms.contains(kw) {
                query_terms.push(kw.clone());
            }
        }
    }

    if query_terms.is_empty() {
        return (50.0, "Cukup Sesuai".to_string(), vec![]);
    }

    let title_low = record.title.to_ascii_lowercase();
    let subj_low = record.subject_area.to_ascii_lowercase();
    let scope_low = record.focus_and_scope.to_ascii_lowercase();
    let all_text = format!("{title_low} {subj_low} {scope_low}");

    let expanded = expand_bilingual_tokens(&query_terms);
    let mut direct_hits = Vec::new();
    let mut cluster_hits = 0usize;

    for qt in &query_terms {
        if title_low.contains(qt) || subj_low.contains(qt) || scope_low.contains(qt) {
            direct_hits.push(qt.clone());
        }
    }
    for exp in &expanded {
        if all_text.contains(exp) {
            cluster_hits += 1;
        }
    }

    let mut score: f64 = 25.0;
    let mut reasons = Vec::new();

    // Direct hits carry heavy weight
    let title_hits = query_terms.iter().filter(|q| title_low.contains(q.as_str())).count();
    let scope_hits = query_terms.iter().filter(|q| scope_low.contains(q.as_str())).count();
    let subj_hits = query_terms.iter().filter(|q| subj_low.contains(q.as_str())).count();

    if title_hits > 0 {
        score += (title_hits as f64 * 18.0).min(36.0);
        reasons.push(format!("Judul jurnal cocok dengan topik ({})", direct_hits.iter().take(2).cloned().collect::<Vec<_>>().join(", ")));
    }
    if subj_hits > 0 {
        score += (subj_hits as f64 * 14.0).min(24.0);
        reasons.push("Bidang ilmu portal selaras dengan kata kunci".to_string());
    }
    if scope_hits > 0 {
        score += (scope_hits as f64 * 12.0).min(28.0);
        reasons.push("Focus & Scope membahas tema riset Anda".to_string());
    }
    if direct_hits.is_empty() && cluster_hits > 0 {
        score += (cluster_hits as f64 * 5.0).min(25.0);
        reasons.push("Rumpun keilmuan bilingual selaras".to_string());
    }
    if let Some(ms) = manuscript {
        if !ms.detected_domain.is_empty()
            && BILINGUAL_CLUSTERS
                .iter()
                .find(|(d, _)| *d == ms.detected_domain)
                .is_some_and(|(_, terms)| terms.iter().any(|t| all_text.contains(t)))
        {
            score += 12.0;
            reasons.push(format!("Domain naskah ({}) cocok", ms.detected_domain));
        }
    }

    let clamped = score.clamp(15.0, 99.0);
    let label = if clamped >= 75.0 {
        "Sangat Sesuai"
    } else if clamped >= 55.0 {
        "Sesuai"
    } else if clamped >= 35.0 {
        "Cukup Sesuai"
    } else {
        "Kurang Sesuai"
    };

    (clamped, label.to_string(), reasons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_docx_extraction_and_bilingual_match() {
        // Construct a minimal valid .docx archive in memory
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("word/document.xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
                <w:document xmlns:w="http://schemas.openxmlformats.org/word/2006/main">
                  <w:body><w:p><w:r><w:t>Penerapan Machine Learning untuk Prediksi Penjualan E-Commerce UMKM</w:t></w:r></w:p></w:body>
                </w:document>"#,
            )
            .unwrap();
            zip.finish().unwrap();
        }

        let meta = build_manuscript_meta("artikel_tes.docx", &buf).unwrap();
        assert!(meta.extracted_text.contains("Machine Learning"));
        assert!(meta.detected_keywords.iter().any(|k| k == "learning" || k == "ecommerce" || k == "commerce" || k == "umkm"));

        let rec = JournalRecord {
            title: "Jurnal Ekonomi Bisnis Digital dan Informatika".to_string(),
            subject_area: "Manajemen, Pemasaran, E-Commerce".to_string(),
            focus_and_scope: "Mempublikasikan riset bisnis digital, machine learning, dan UMKM.".to_string(),
            ..Default::default()
        };
        let (score, label, reasons) = compute_match_score(&rec, &["commerce".to_string()], Some(&meta));
        assert!(score >= 70.0, "expected high match score, got {score}");
        assert!(!reasons.is_empty());
        assert!(matches!(label.as_str(), "Sangat Sesuai" | "Sesuai"));
    }
}
