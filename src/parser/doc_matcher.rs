//! Manuscript parser (.docx / .pdf) and Bilingual BM25 + Academic Synonym Match Scorer.

use crate::error::JsfsError;
use crate::models::{JournalRecord, ManuscriptMeta};
use once_cell::sync::Lazy;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use zip::ZipArchive;

static RE_TOKEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-zA-Z0-9]{2,}").expect("valid regex"));

const STOPWORDS: &[&str] = &[
    "the",
    "and",
    "for",
    "with",
    "that",
    "this",
    "from",
    "are",
    "was",
    "were",
    "have",
    "has",
    "using",
    "based",
    "study",
    "analysis",
    "research",
    "paper",
    "article",
    "journal",
    "abstract",
    "abstrak",
    "pendahuluan",
    "metode",
    "hasil",
    "kesimpulan",
    "dan",
    "yang",
    "untuk",
    "dengan",
    "pada",
    "dari",
    "dalam",
    "atau",
    "adalah",
    "terhadap",
    "tentang",
    "sebagai",
    "oleh",
    "ini",
    "itu",
    "jurnal",
    "penelitian",
    "analisis",
    "studi",
    "berbasis",
    "pendekatan",
    "pengaruh",
    "implementasi",
    "penerapan",
    "perancangan",
    "pengembangan",
    "effect",
    "impact",
    "role",
    "through",
    "between",
    "among",
    "into",
    "over",
    "under",
    "dapat",
    "akan",
    "serta",
    "secara",
    "bahwa",
    "tersebut",
    "melalui",
    "antara",
];

/// Cross-lingual academic synonym sets (Indonesian <-> English, single words & normalized stems).
const BILINGUAL_SYNONYM_GROUPS: &[&[&str]] = &[
    // UMKM / SMEs & Entrepreneurship
    &[
        "umkm",
        "sme",
        "smes",
        "msme",
        "msmes",
        "ukm",
        "wirausaha",
        "kewirausahaan",
        "entrepreneur",
        "entrepreneurship",
        "mikro",
    ],
    // Business & E-Commerce
    &[
        "bisnis",
        "business",
        "commerce",
        "ecommerce",
        "niaga",
        "perdagangan",
        "trade",
        "dagang",
        "usaha",
    ],
    // Marketing & Consumer
    &[
        "pemasaran",
        "marketing",
        "iklan",
        "advertising",
        "promosi",
        "promotion",
        "konsumen",
        "consumer",
        "pelanggan",
        "customer",
        "brand",
        "branding",
    ],
    // Management & HR
    &[
        "manajemen",
        "management",
        "organisasi",
        "organization",
        "organisational",
        "kepemimpinan",
        "leadership",
        "kinerja",
        "performance",
        "sdm",
        "hrm",
        "karyawan",
        "employee",
    ],
    // Finance & Accounting
    &[
        "keuangan",
        "finance",
        "financial",
        "akuntansi",
        "accounting",
        "akuntan",
        "audit",
        "auditing",
        "pajak",
        "tax",
        "taxation",
        "perbankan",
        "banking",
        "investasi",
        "investment",
        "fintech",
        "ewallet",
    ],
    // Information Systems & Technology
    &[
        "informatika",
        "informatics",
        "komputer",
        "computer",
        "computing",
        "teknologi",
        "technology",
        "perangkat",
        "software",
        "aplikasi",
        "application",
    ],
    &[
        "informasi",
        "information",
        "sisfo",
        "sim",
        "erp",
        "digital",
        "digitalisasi",
        "digitalization",
    ],
    &["sistem", "system", "systems"],
    // AI / ML / Data Science
    &[
        "kecerdasan",
        "artificial",
        "intelligence",
        "neural",
        "algoritma",
        "algorithm",
        "algorithms",
        "klasifikasi",
        "classification",
        "klastering",
        "clustering",
        "prediksi",
        "prediction",
        "forecasting",
        "mining",
        "nlp",
    ],
    // Security & Network
    &[
        "keamanan",
        "security",
        "siber",
        "cyber",
        "cybersecurity",
        "kriptografi",
        "cryptography",
        "jaringan",
        "network",
        "networks",
        "cloud",
        "iot",
    ],
    // Education & Learning
    &[
        "pendidikan",
        "education",
        "educational",
        "pembelajaran",
        "learning",
        "elearning",
        "pengajaran",
        "teaching",
        "kurikulum",
        "curriculum",
        "pedagogi",
        "pedagogy",
        "didaktik",
        "literasi",
        "literacy",
        "siswa",
        "student",
        "students",
        "mahasiswa",
        "guru",
        "teacher",
        "teachers",
        "dosen",
        "sekolah",
        "school",
    ],
    // Health & Medicine
    &[
        "kesehatan",
        "health",
        "healthcare",
        "kedokteran",
        "medical",
        "medicine",
        "klinik",
        "clinical",
        "keperawatan",
        "nursing",
        "farmasi",
        "pharmacy",
        "gizi",
        "nutrition",
        "pasien",
        "patient",
        "penyakit",
        "disease",
        "epidemiologi",
        "epidemiology",
        "puskesmas",
        "stunting",
        "medis",
    ],
    // Law & Regulation
    &[
        "hukum",
        "law",
        "legal",
        "yuridis",
        "juridical",
        "pidana",
        "criminal",
        "perdata",
        "civil",
        "konstitusi",
        "constitution",
        "regulasi",
        "regulation",
        "undang",
        "statute",
        "pengadilan",
        "court",
        "hakim",
        "syariah",
        "sharia",
    ],
    // Agriculture, Food & Marine
    &[
        "pertanian",
        "agriculture",
        "agricultural",
        "agribisnis",
        "agribusiness",
        "agroteknologi",
        "hortikultura",
        "tanaman",
        "crop",
        "pangan",
        "food",
        "kelautan",
        "marine",
        "perikanan",
        "fisheries",
        "peternakan",
        "livestock",
        "kehutanan",
        "forestry",
    ],
    // Communication & Media
    &[
        "komunikasi",
        "communication",
        "communications",
        "media",
        "jurnalistik",
        "journalism",
        "penyiaran",
        "broadcasting",
        "humas",
        "publicity",
    ],
    // Social, Government & Psychology
    &[
        "sosial",
        "social",
        "sosiologi",
        "sociology",
        "politik",
        "politics",
        "political",
        "kebijakan",
        "policy",
        "pemerintahan",
        "government",
        "governance",
        "administrasi",
        "administration",
        "publik",
        "public",
        "masyarakat",
        "community",
        "desa",
        "village",
    ],
    &[
        "psikologi",
        "psychology",
        "psychological",
        "mental",
        "perilaku",
        "behavior",
        "behaviour",
        "kognitif",
        "cognitive",
    ],
    // Environment, Energy & Engineering
    &[
        "lingkungan",
        "environment",
        "environmental",
        "ekologi",
        "ecology",
        "berkelanjutan",
        "sustainability",
        "sustainable",
        "energi",
        "energy",
        "terbarukan",
        "renewable",
        "listrik",
        "electrical",
        "elektro",
        "teknik",
        "engineering",
        "sipil",
        "mesin",
        "mechanical",
        "industri",
        "industrial",
    ],
];

/// Cross-lingual multi-word academic phrases (`id_phrase`, `en_phrase`, `cluster`).
const BILINGUAL_PHRASE_PAIRS: &[(&str, &str)] = &[
    ("sistem informasi", "information system"),
    ("sistem informasi", "information systems"),
    ("kecerdasan buatan", "artificial intelligence"),
    ("pembelajaran mesin", "machine learning"),
    ("pembelajaran mendalam", "deep learning"),
    ("penambangan data", "data mining"),
    ("keamanan siber", "cyber security"),
    ("keamanan informasi", "information security"),
    ("perdagangan elektronik", "electronic commerce"),
    ("bisnis digital", "digital business"),
    ("pemasaran digital", "digital marketing"),
    ("usaha mikro kecil menengah", "small medium enterprise"),
    ("sumber daya manusia", "human resource"),
    ("kesehatan masyarakat", "public health"),
    ("kebijakan publik", "public policy"),
    ("ilmu komputer", "computer science"),
    ("rekayasa perangkat lunak", "software engineering"),
    ("teknologi pendidikan", "educational technology"),
    ("energi terbarukan", "renewable energy"),
];

/// 8 academic domain synonym clusters (`doc_reader.py` parity + expanded coverage).
const BILINGUAL_CLUSTERS: &[(&str, &[&str])] = &[
    (
        "Ekonomi & Bisnis Digital",
        &[
            "bisnis",
            "business",
            "commerce",
            "ecommerce",
            "manajemen",
            "management",
            "akuntansi",
            "accounting",
            "keuangan",
            "finance",
            "ekonomi",
            "economy",
            "economic",
            "pemasaran",
            "marketing",
            "umkm",
            "sme",
            "msme",
            "ewallet",
            "fintech",
            "wirausaha",
            "entrepreneurship",
            "perbankan",
            "banking",
            "investasi",
            "investment",
            "pajak",
            "tax",
        ],
    ),
    (
        "Informatika & Komputer",
        &[
            "informatika",
            "informatics",
            "komputer",
            "computer",
            "perangkat",
            "software",
            "aplikasi",
            "application",
            "machine",
            "learning",
            "kecerdasan",
            "artificial",
            "data",
            "mining",
            "jaringan",
            "network",
            "keamanan",
            "security",
            "siber",
            "cyber",
            "cloud",
            "web",
            "mobile",
            "android",
            "iot",
            "algoritma",
            "algorithm",
            "klasifikasi",
            "nlp",
            "informasi",
            "information",
            "sistem",
            "system",
        ],
    ),
    (
        "Pendidikan",
        &[
            "pendidikan",
            "education",
            "pembelajaran",
            "learning",
            "pengajaran",
            "teaching",
            "kurikulum",
            "curriculum",
            "siswa",
            "student",
            "mahasiswa",
            "guru",
            "teacher",
            "pedagogi",
            "pedagogy",
            "sekolah",
            "school",
            "elearning",
            "literasi",
            "literacy",
            "matematika",
            "bahasa",
        ],
    ),
    (
        "Hukum",
        &[
            "hukum",
            "law",
            "legal",
            "yuridis",
            "juridical",
            "pidana",
            "criminal",
            "perdata",
            "civil",
            "konstitusi",
            "constitution",
            "regulasi",
            "regulation",
            "undang",
            "statute",
            "pengadilan",
            "court",
            "hakim",
            "ham",
            "syariah",
        ],
    ),
    (
        "Kesehatan & Kedokteran",
        &[
            "kesehatan",
            "health",
            "kedokteran",
            "medical",
            "medicine",
            "keperawatan",
            "nursing",
            "farmasi",
            "pharmacy",
            "gizi",
            "nutrition",
            "klinik",
            "clinical",
            "pasien",
            "patient",
            "penyakit",
            "disease",
            "puskesmas",
            "epidemiologi",
            "epidemiology",
            "stunting",
        ],
    ),
    (
        "Pertanian & Agribisnis",
        &[
            "pertanian",
            "agriculture",
            "agricultural",
            "agribisnis",
            "agribusiness",
            "tanaman",
            "crop",
            "pangan",
            "food",
            "kelautan",
            "marine",
            "perikanan",
            "fisheries",
            "peternakan",
            "livestock",
            "kehutanan",
            "forestry",
            "agroteknologi",
            "hortikultura",
        ],
    ),
    (
        "Komunikasi & Media",
        &[
            "komunikasi",
            "communication",
            "media",
            "jurnalistik",
            "journalism",
            "penyiaran",
            "broadcasting",
            "periklanan",
            "advertising",
            "humas",
            "public",
            "relations",
            "digital",
            "sosial",
            "brand",
            "branding",
        ],
    ),
    (
        "Ilmu Sosial & Pemerintahan",
        &[
            "sosial",
            "social",
            "sosiologi",
            "sociology",
            "politik",
            "politics",
            "political",
            "kebijakan",
            "policy",
            "pemerintahan",
            "government",
            "administrasi",
            "administration",
            "masyarakat",
            "community",
            "desa",
            "village",
            "psikologi",
            "psychology",
        ],
    ),
];

/// Extract plain text cleanly from `.docx` bytes (`word/document.xml` -> `<w:t>` text nodes).
pub fn extract_text_from_docx_bytes(bytes: &[u8]) -> Result<String, JsfsError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| JsfsError::Document(format!("DOCX tidak valid: {e}")))?;

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

/// Tokenize text, drop stopwords, and preserve hyphen-merged forms (e.g. `e-commerce` -> `ecommerce`).
fn tokenize_clean(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    let normalized = text
        .to_ascii_lowercase()
        .replace("e-commerce", "ecommerce")
        .replace("e-learning", "elearning")
        .replace("e-wallet", "ewallet");

    let mut tokens = Vec::new();
    for m in RE_TOKEN.find_iter(&normalized) {
        let w = m.as_str();
        if w.len() >= 2 && !stop.contains(w) && !w.chars().all(|c| c.is_ascii_digit()) {
            tokens.push(w.to_string());
        }
    }
    tokens
}

/// Tokenize text, drop Indonesian/English stopwords, and return frequency-ordered keywords.
pub fn extract_keywords_and_domain(text: &str, top_n: usize) -> (Vec<String>, String) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let sample: String = text.chars().take(35_000).collect();

    for w in tokenize_clean(&sample) {
        if w.len() >= 3 {
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
    let (detected_keywords, detected_domain) = extract_keywords_and_domain(&extracted_text, 14);

    Ok(ManuscriptMeta {
        filename: filename.to_string(),
        char_count,
        extracted_text: extracted_text.chars().take(12_000).collect(),
        detected_keywords,
        detected_domain,
    })
}

/// Lookup bilingual synonyms for a given token across `BILINGUAL_SYNONYM_GROUPS`.
fn synonyms_for_token(token: &str) -> Vec<&'static str> {
    let mut syns = Vec::new();
    for &group in BILINGUAL_SYNONYM_GROUPS {
        if group.contains(&token) {
            for &term in group {
                if term != token && !syns.contains(&term) {
                    syns.push(term);
                }
            }
        }
    }
    syns
}

/// Build term-frequency map for a field text.
fn term_freq_map(text: &str) -> (HashMap<String, usize>, usize) {
    let toks = tokenize_clean(text);
    let len = toks.len();
    let mut map = HashMap::with_capacity(len);
    for t in toks {
        *map.entry(t).or_insert(0) += 1;
    }
    (map, len)
}

/// Compute BM25 saturated term score (`k1 = 1.2`, `b = 0.75`).
fn bm25_tf(tf: usize, doc_len: usize, avg_dl: f64) -> f64 {
    if tf == 0 || doc_len == 0 {
        return 0.0;
    }
    const K1: f64 = 1.2;
    const B: f64 = 0.75;
    let tf_f = tf as f64;
    let dl_norm = (doc_len as f64) / avg_dl.max(1.0);
    (tf_f * (K1 + 1.0)) / (tf_f + K1 * (1.0 - B + B * dl_norm))
}

/// Evaluate exact + bilingual synonym term frequency in a field.
fn field_term_hit(
    term: &str,
    syns: &[&'static str],
    tf_map: &HashMap<String, usize>,
    raw_low: &str,
) -> (usize, Option<&'static str>) {
    if let Some(&cnt) = tf_map.get(term) {
        return (cnt, None);
    }
    // Check substring compound match for longer tokens (e.g. `ecommerce` inside `telecommerce` or compound words)
    if term.len() >= 4 && raw_low.contains(term) {
        return (1, None);
    }
    for &syn in syns {
        if let Some(&cnt) = tf_map.get(syn) {
            return (cnt, Some(syn));
        }
        if syn.len() >= 4 && raw_low.contains(syn) {
            return (1, Some(syn));
        }
    }
    (0, None)
}

/// Compute Bilingual BM25 + Field-Weighted + Academic Synonym match score (`0.0..=100.0`).
pub fn compute_match_score(
    record: &JournalRecord,
    search_keywords: &[String],
    manuscript: Option<&ManuscriptMeta>,
) -> (f64, String, Vec<String>) {
    // Weighted query terms: (token, weight)
    let mut query_weighted: Vec<(String, f64)> = Vec::new();
    let mut raw_phrases: Vec<String> = Vec::new();
    let mut seen_terms: HashSet<String> = HashSet::new();

    for kw in search_keywords {
        let low_kw = kw.trim().to_ascii_lowercase();
        if low_kw.contains(' ') && low_kw.len() >= 5 {
            raw_phrases.push(low_kw.clone());
        }
        for part in tokenize_clean(&low_kw) {
            if seen_terms.insert(part.clone()) {
                query_weighted.push((part, 1.45));
            }
        }
    }

    if let Some(ms) = manuscript {
        let total_ms = ms.detected_keywords.len().max(1) as f64;
        for (rank, kw) in ms.detected_keywords.iter().take(12).enumerate() {
            let clean = kw.trim().to_ascii_lowercase();
            if seen_terms.insert(clean.clone()) {
                // Rank-decay TF-IDF proxy weight: top manuscript terms carry higher importance
                let w = 1.15 - (rank as f64 / total_ms) * 0.55;
                query_weighted.push((clean, w));
            }
        }
        let ms_low: String = ms
            .extracted_text
            .chars()
            .take(4000)
            .collect::<String>()
            .to_ascii_lowercase();
        for &(id_p, en_p) in BILINGUAL_PHRASE_PAIRS {
            if ms_low.contains(id_p) || ms_low.contains(en_p) {
                raw_phrases.push(id_p.to_string());
                raw_phrases.push(en_p.to_string());
            }
        }
    }

    if query_weighted.is_empty() {
        return (50.0, "Cukup Sesuai".to_string(), vec![]);
    }

    let title_low = record.title.to_ascii_lowercase();
    let subj_low = record.subject_area.to_ascii_lowercase();
    let scope_low = record.focus_and_scope.to_ascii_lowercase();
    let all_text = format!("{title_low} {subj_low} {scope_low}");

    let (title_tf, title_len) = term_freq_map(&title_low);
    let (subj_tf, subj_len) = term_freq_map(&subj_low);
    let (scope_tf, scope_len) = term_freq_map(&scope_low);

    let mut raw_bm25 = 0.0f64;
    let mut max_possible = 0.0f64;

    let mut title_matched = Vec::new();
    let mut scope_matched = Vec::new();
    let mut subj_matched = Vec::new();
    let mut synonym_pairs: Vec<String> = Vec::new();

    for (qt, q_weight) in &query_weighted {
        let syns = synonyms_for_token(qt);
        // Ideal single-occurrence saturated BM25 ceiling per term
        max_possible += q_weight * 2.2;

        let (t_cnt, t_syn) = field_term_hit(qt, &syns, &title_tf, &title_low);
        if t_cnt > 0 {
            let mult = if t_syn.is_some() { 0.85 } else { 1.0 };
            raw_bm25 += q_weight * mult * 2.4 * bm25_tf(t_cnt, title_len, 6.0);
            if !title_matched.contains(qt) {
                title_matched.push(qt.clone());
            }
            if let Some(s) = t_syn {
                let pair = format!("{qt} ↔ {s}");
                if !synonym_pairs.contains(&pair) {
                    synonym_pairs.push(pair);
                }
            }
        }

        let (s_cnt, s_syn) = field_term_hit(qt, &syns, &subj_tf, &subj_low);
        if s_cnt > 0 {
            let mult = if s_syn.is_some() { 0.85 } else { 1.0 };
            raw_bm25 += q_weight * mult * 1.8 * bm25_tf(s_cnt, subj_len, 8.0);
            if !subj_matched.contains(qt) {
                subj_matched.push(qt.clone());
            }
            if let Some(s) = s_syn {
                let pair = format!("{qt} ↔ {s}");
                if !synonym_pairs.contains(&pair) {
                    synonym_pairs.push(pair);
                }
            }
        }

        let (f_cnt, f_syn) = field_term_hit(qt, &syns, &scope_tf, &scope_low);
        if f_cnt > 0 {
            let mult = if f_syn.is_some() { 0.85 } else { 1.0 };
            raw_bm25 += q_weight * mult * 2.0 * bm25_tf(f_cnt, scope_len, 45.0);
            if !scope_matched.contains(qt) {
                scope_matched.push(qt.clone());
            }
            if let Some(s) = f_syn {
                let pair = format!("{qt} ↔ {s}");
                if !synonym_pairs.contains(&pair) {
                    synonym_pairs.push(pair);
                }
            }
        }
    }

    // Cross-lingual multi-word phrase matching bonus
    let mut phrase_bonus = 0.0f64;
    let mut matched_phrases: Vec<String> = Vec::new();
    for phrase in &raw_phrases {
        if all_text.contains(phrase.as_str()) {
            phrase_bonus += 9.0;
            if !matched_phrases.contains(phrase) {
                matched_phrases.push(phrase.clone());
            }
        }
        for &(id_p, en_p) in BILINGUAL_PHRASE_PAIRS {
            if phrase == id_p && all_text.contains(en_p) {
                phrase_bonus += 8.0;
                let lbl = format!("{id_p} ↔ {en_p}");
                if !synonym_pairs.contains(&lbl) {
                    synonym_pairs.push(lbl);
                }
            } else if phrase == en_p && all_text.contains(id_p) {
                phrase_bonus += 8.0;
                let lbl = format!("{en_p} ↔ {id_p}");
                if !synonym_pairs.contains(&lbl) {
                    synonym_pairs.push(lbl);
                }
            }
        }
    }

    // Normalize BM25 coverage + non-linear saturation for intuitive 15..99 scale
    let ratio = (raw_bm25 / max_possible.max(1.0)).clamp(0.0, 2.5);
    let bm25_component = 68.0 * (1.0 - (-1.45 * ratio).exp());
    let mut score = 18.0 + bm25_component + phrase_bonus.min(18.0);

    let mut reasons = Vec::new();
    if !title_matched.is_empty() {
        reasons.push(format!(
            "Judul jurnal selaras ({})",
            title_matched
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !subj_matched.is_empty() {
        reasons.push("Bidang ilmu portal selaras dengan topik".to_string());
    }
    if !scope_matched.is_empty() {
        reasons.push(format!(
            "Focus & Scope memuat topik ({})",
            scope_matched
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !synonym_pairs.is_empty() {
        reasons.push(format!(
            "Padanan akademik ID–EN ({})",
            synonym_pairs
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    if let Some(ms) = manuscript {
        if !ms.detected_domain.is_empty()
            && BILINGUAL_CLUSTERS
                .iter()
                .find(|(d, _)| *d == ms.detected_domain)
                .is_some_and(|(_, terms)| terms.iter().any(|t| all_text.contains(t)))
        {
            score += 9.0;
            reasons.push(format!("Rumpun keilmuan selaras: {}", ms.detected_domain));
        }
    }

    // Small penalty if Focus & Scope is completely empty and no title/subject match
    if scope_low.is_empty() && title_matched.is_empty() && subj_matched.is_empty() {
        score = score.min(34.0);
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
        assert!(meta
            .detected_keywords
            .iter()
            .any(|k| k == "learning" || k == "ecommerce" || k == "umkm"));

        let rec = JournalRecord {
            title: "Jurnal Ekonomi Bisnis Digital dan Informatika".to_string(),
            subject_area: "Manajemen, Pemasaran, E-Commerce".to_string(),
            focus_and_scope: "Mempublikasikan riset bisnis digital, machine learning, dan UMKM."
                .to_string(),
            ..Default::default()
        };
        let (score, label, reasons) =
            compute_match_score(&rec, &["commerce".to_string()], Some(&meta));
        assert!(score >= 65.0, "expected high match score, got {score}");
        assert!(!reasons.is_empty());
        assert!(matches!(label.as_str(), "Sangat Sesuai" | "Sesuai"));
    }

    #[test]
    fn test_bilingual_bm25_cross_language_synonyms() {
        // Indonesian query against English-only journal metadata
        let english_journal = JournalRecord {
            title: "International Journal of Information Systems and Small Business Learning"
                .to_string(),
            subject_area: "Information System, Electronic Commerce, SME Management".to_string(),
            focus_and_scope: "Publishes peer-reviewed studies on information systems, business commerce, SMEs, and digital learning.".to_string(),
            ..Default::default()
        };
        let unrelated_journal = JournalRecord {
            title: "Jurnal Kedokteran Hewan dan Virologi Ternak".to_string(),
            subject_area: "Veterinary Medicine".to_string(),
            focus_and_scope:
                "Focuses on animal pathology, livestock vaccines, and clinical veterinary surgery."
                    .to_string(),
            ..Default::default()
        };

        let id_keywords = vec![
            "sistem informasi".to_string(),
            "umkm".to_string(),
            "pembelajaran".to_string(),
            "bisnis".to_string(),
        ];

        let (match_sc, _, match_reasons) =
            compute_match_score(&english_journal, &id_keywords, None);
        let (unrel_sc, _, _) = compute_match_score(&unrelated_journal, &id_keywords, None);

        assert!(
            match_sc >= 65.0,
            "expected cross-lingual ID->EN match >= 65, got {match_sc} ({match_reasons:?})"
        );
        assert!(
            match_sc > unrel_sc + 35.0,
            "expected matched journal ({match_sc}) >> unrelated ({unrel_sc})"
        );
        assert!(
            match_reasons.iter().any(|r| r.contains("ID–EN")),
            "expected ID-EN synonym rationale, got {match_reasons:?}"
        );
    }
}
