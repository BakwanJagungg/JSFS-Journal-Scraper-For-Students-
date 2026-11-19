//! Manuscript parser (.docx / .pdf) and Bilingual BM25 + Academic Synonym Match Scorer.

use crate::error::JsfsError;
use crate::models::{FitStatus, JournalRecord, ManuscriptMeta, PreSubmitChecklist, UiJournalItem};
use once_cell::sync::Lazy;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use zip::ZipArchive;

static RE_TOKEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-zA-Z0-9]{2,}").expect("valid regex"));
static RE_EXPLICIT_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?im)(?:kata\s*kunci|key\s*words?)\s*[:=\-]\s*([^\r\n]{3,240})")
        .expect("valid regex")
});
static RE_BRACKET_CITE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[\s*\d+(?:\s*[-–,]\s*\d+)*\s*\]").expect("valid regex"));
static RE_AUTHOR_YEAR_CITE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\([^()\r\n]{0,80}\b(?:19|20)\d{2}[a-z]?\b[^()\r\n]*\)|[A-Z][A-Za-zÀ-ÖØ-öø-ÿ'’-]+(?:\s+(?:&|and|dan|et\s+al\.?|dkk\.?)\s+[A-Z]?[A-Za-zÀ-ÖØ-öø-ÿ'’-]*)*\s*\((?:19|20)\d{2}[a-z]?\)")
        .expect("valid regex")
});

/// Generic high-frequency hub words that receive lower IDF rarity weight in matching.
const GENERIC_HUB_TERMS: &[&str] = &[
    "sistem",
    "system",
    "systems",
    "informasi",
    "information",
    "ilmu",
    "science",
    "sciences",
    "teknologi",
    "technology",
    "manajemen",
    "management",
    "pendidikan",
    "education",
    "ekonomi",
    "economics",
    "bisnis",
    "business",
    "sosial",
    "social",
    "data",
    "digital",
    "komputer",
    "computer",
    "media",
    "model",
    "aplikasi",
    "application",
    "nasional",
    "national",
    "international",
    "indonesia",
];

fn term_idf_weight(token: &str) -> f64 {
    if GENERIC_HUB_TERMS.contains(&token) {
        0.62
    } else if token.len() >= 8 || token.contains("ecommerce") || token.contains("fintech") {
        1.45
    } else if token.len() >= 5 {
        1.20
    } else {
        1.00
    }
}

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

/// Lightweight rule-based Indonesian stemmer stripping prefixes and suffixes.
pub fn stem_id(word: &str) -> String {
    let w = word.trim().to_ascii_lowercase();
    if w.len() < 5 {
        return w;
    }

    let mut s = w.clone();

    // 1. Strip particles/possessives suffixes: -lah, -kah, -nya, -ku, -mu
    for suf in ["lah", "kah", "nya", "ku", "mu"] {
        if s.len() > suf.len() + 3 && s.ends_with(suf) {
            s.truncate(s.len() - suf.len());
            break;
        }
    }

    // 2. Strip derivational suffixes: -kan, -an, -i
    for suf in ["kan", "an", "i"] {
        if s.len() > suf.len() + 3 && s.ends_with(suf) {
            s.truncate(s.len() - suf.len());
            break;
        }
    }

    // 3. Strip prefixes
    if s.len() >= 6 {
        if let Some(rest) = s.strip_prefix("meny") {
            return format!("s{rest}");
        }
        if let Some(rest) = s.strip_prefix("peny") {
            return format!("s{rest}");
        }
        if let Some(rest) = s.strip_prefix("mem") {
            if rest.starts_with(['b', 'f']) {
                return rest.to_string();
            } else {
                return format!("p{rest}");
            }
        }
        if let Some(rest) = s.strip_prefix("pem") {
            if rest.starts_with(['b', 'f']) {
                return rest.to_string();
            } else {
                return format!("p{rest}");
            }
        }
        if let Some(rest) = s.strip_prefix("meng") {
            if rest.starts_with("embang")
                || rest.starts_with("irim")
                || rest.starts_with("urang")
                || rest.starts_with("ira")
                || rest.starts_with("elola")
            {
                return format!("k{rest}");
            }
            if rest.starts_with(['g', 'h', 'a', 'e', 'i', 'o', 'u']) {
                return rest.to_string();
            } else {
                return format!("k{rest}");
            }
        }
        if let Some(rest) = s.strip_prefix("peng") {
            if rest.starts_with("embang")
                || rest.starts_with("irim")
                || rest.starts_with("urang")
                || rest.starts_with("ira")
                || rest.starts_with("elola")
            {
                return format!("k{rest}");
            }
            if rest.starts_with(['g', 'h', 'a', 'e', 'i', 'o', 'u']) {
                return rest.to_string();
            } else {
                return format!("k{rest}");
            }
        }
        if let Some(rest) = s.strip_prefix("men") {
            if rest.starts_with(['d', 'j', 'c']) {
                return rest.to_string();
            } else {
                return format!("t{rest}");
            }
        }
        if let Some(rest) = s.strip_prefix("pen") {
            if rest.starts_with(['d', 'j', 'c']) {
                return rest.to_string();
            } else {
                return format!("t{rest}");
            }
        }
    }

    // Generic prefixes (up to 2 passes for stacked prefixes like ber-ke-lanjut)
    for _ in 0..2 {
        let mut stripped = false;
        for pref in ["ber", "ter", "per", "pe", "di", "ke", "se"] {
            if s.len() > pref.len() + 3 && s.starts_with(pref) {
                s = s[pref.len()..].to_string();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }

    if s.len() >= 3 {
        s
    } else {
        w
    }
}

/// Lightweight rule-based English stemmer stripping common inflectional suffixes.
pub fn stem_en(word: &str) -> String {
    let w = word.trim().to_ascii_lowercase();
    if w.len() < 4 {
        return w;
    }

    let mut s = w;

    if s.ends_with("ies") && s.len() >= 5 {
        s.truncate(s.len() - 3);
        s.push('y');
        return s;
    }

    if s.ends_with("ication") && s.len() >= 7 {
        s.truncate(s.len() - 7);
        return s;
    }

    if s.ends_with("ation") && s.len() >= 6 {
        s.truncate(s.len() - 5);
        s.push('e');
        return s;
    }

    if s.ends_with("tion") && s.len() >= 6 {
        s.truncate(s.len() - 4);
        s.push('t');
        return s;
    }

    if s.ends_with("ment") && s.len() >= 6 {
        s.truncate(s.len() - 4);
        return s;
    }

    if s.ends_with("ing") && s.len() >= 5 {
        s.truncate(s.len() - 3);
        let bytes = s.as_bytes();
        let blen = bytes.len();
        if blen >= 3
            && bytes[blen - 1] == bytes[blen - 2]
            && !matches!(bytes[blen - 1], b's' | b'l' | b'z')
        {
            s.truncate(blen - 1);
        }
        return s;
    }

    if s.ends_with("ed") && s.len() >= 5 {
        s.truncate(s.len() - 2);
        return s;
    }

    if s.ends_with("es") && s.len() >= 5 {
        s.truncate(s.len() - 2);
        return s;
    }

    if s.ends_with('s') && !s.ends_with("ss") && s.len() >= 4 {
        s.truncate(s.len() - 1);
        return s;
    }

    s
}

/// Academic stemmer handling both ID and EN academic tokens.
pub fn stem_academic_word(word: &str) -> String {
    let low = word.trim().to_ascii_lowercase();
    let s_id = stem_id(&low);
    if s_id != low && s_id.len() >= 3 {
        return s_id;
    }
    let s_en = stem_en(&low);
    if s_en != low && s_en.len() >= 3 {
        return s_en;
    }
    low
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

/// Extract manuscript title, explicit `Kata Kunci:`/`Keywords:`, and Top-6 TF-IDF fallback phrases.
pub fn extract_manuscript_title_and_keywords(text: &str) -> (String, String, Vec<String>) {
    let head: String = text.chars().take(5500).collect();

    // 1) Title extraction: first clean non-boilerplate line in top 12 lines
    let mut extracted_title = String::new();
    for line in head.lines().take(15) {
        let trimmed = line.trim();
        let low = trimmed.to_ascii_lowercase();
        let word_cnt = trimmed.split_whitespace().count();
        if trimmed.len() >= 12
            && trimmed.len() <= 180
            && word_cnt >= 3
            && !low.starts_with("issn")
            && !low.starts_with("e-issn")
            && !low.starts_with("vol.")
            && !low.starts_with("volume")
            && !low.starts_with("abstract")
            && !low.starts_with("abstrak")
            && !low.starts_with("kata kunci")
            && !low.starts_with("keyword")
            && !low.starts_with("http")
            && !low.starts_with("doi")
        {
            extracted_title = trimmed.to_string();
            break;
        }
    }

    // 2) Explicit Keywords line extraction (`Kata Kunci:` / `Keywords:`)
    let mut explicit_terms: Vec<String> = Vec::new();
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    if let Some(cap) = RE_EXPLICIT_KEYWORDS.captures(&head) {
        let mut raw_kw_line = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        for stopper in [
            "1. pendahuluan",
            "pendahuluan",
            "1. introduction",
            "introduction",
            "abstract",
            "abstrak",
            "latar belakang",
        ] {
            if let Some(pos) = raw_kw_line.to_ascii_lowercase().find(stopper) {
                raw_kw_line.truncate(pos);
            }
        }
        for part in raw_kw_line.split([',', ';', '|', '•']) {
            let clean = part
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != ' ')
                .trim();
            let low = clean.to_ascii_lowercase();
            let words: Vec<&str> = clean.split_whitespace().collect();
            if clean.len() >= 2
                && !words.is_empty()
                && words.len() <= 4
                && !stop.contains(low.as_str())
                && !clean.chars().all(|c| c.is_ascii_digit())
                && !explicit_terms
                    .iter()
                    .any(|x: &String| x.eq_ignore_ascii_case(clean))
            {
                explicit_terms.push(clean.to_string());
                if explicit_terms.len() >= 6 {
                    break;
                }
            }
        }
    }

    if !explicit_terms.is_empty() {
        let joined = explicit_terms.join(", ");
        let mut tokens = Vec::new();
        for term in &explicit_terms {
            for t in tokenize_clean(term) {
                if !tokens.contains(&t) {
                    tokens.push(t);
                }
            }
        }
        return (extracted_title, joined, tokens);
    }

    // 3) TF-IDF / Academic Phrase fallback from Title + Abstract (or top 3500 chars)
    let low_head = head.to_ascii_lowercase();
    let abstract_slice = if let Some(abs_start) = low_head
        .find("abstrak")
        .or_else(|| low_head.find("abstract"))
    {
        let tail = &head[abs_start..];
        let end = tail
            .to_ascii_lowercase()
            .find("pendahuluan")
            .or_else(|| tail.to_ascii_lowercase().find("introduction"))
            .unwrap_or_else(|| tail.len().min(2800));
        format!("{extracted_title} {}", &tail[..end])
    } else {
        format!(
            "{extracted_title} {}",
            head.chars().take(2800).collect::<String>()
        )
    };

    let mut phrase_scores: HashMap<String, f64> = HashMap::new();
    let extra_filler = [
        "penulis",
        "author",
        "authors",
        "universitas",
        "university",
        "fakultas",
        "program",
        "studi",
        "mahasiswa",
        "tahun",
        "year",
        "metodologi",
        "pembahasan",
        "kesimpulan",
        "conclusion",
        "keyword",
        "keywords",
        "kunci",
        "kata",
        "introduction",
    ];
    let title_toks: HashSet<String> = tokenize_clean(&extracted_title).into_iter().collect();
    let toks: Vec<String> = tokenize_clean(&abstract_slice)
        .into_iter()
        .filter(|t| t.len() >= 3 && !extra_filler.contains(&t.as_str()))
        .collect();

    for t in &toks {
        let mut w = term_idf_weight(t);
        if title_toks.contains(t) {
            w *= 1.6;
        }
        *phrase_scores.entry(t.clone()).or_insert(0.0) += w;
    }
    for win in toks.windows(2) {
        if win[0] != win[1] {
            let bigram = format!("{} {}", win[0], win[1]);
            let mut w = 0.85 * (term_idf_weight(&win[0]) + term_idf_weight(&win[1]));
            if title_toks.contains(&win[0]) && title_toks.contains(&win[1]) {
                w *= 1.75;
            }
            *phrase_scores.entry(bigram).or_insert(0.0) += w;
        }
    }

    let mut scored: Vec<(String, f64)> = phrase_scores.into_iter().collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut chosen: Vec<String> = Vec::new();
    for (cand, _) in scored {
        let redundant = chosen.iter().any(|existing| {
            existing.contains(&cand) || (cand.contains(existing.as_str()) && cand.contains(' '))
        });
        if !redundant {
            chosen.push(cand);
            if chosen.len() >= 6 {
                break;
            }
        }
    }

    let joined = chosen.join(", ");
    let mut flat_tokens = Vec::new();
    for c in &chosen {
        for t in tokenize_clean(c) {
            if !flat_tokens.contains(&t) {
                flat_tokens.push(t);
            }
        }
    }
    (extracted_title, joined, flat_tokens)
}

pub const MAX_MANUSCRIPT_BYTES: usize = 50 * 1024 * 1024;

pub fn build_manuscript_meta(filename: &str, bytes: &[u8]) -> Result<ManuscriptMeta, JsfsError> {
    if bytes.len() > MAX_MANUSCRIPT_BYTES {
        return Err(JsfsError::InvalidInput(
            "Ukuran file naskah melebihi batas maksimal 50 MB.".to_string(),
        ));
    }
    let low = filename.to_ascii_lowercase();
    if low.ends_with(".docm") {
        return Err(JsfsError::InvalidInput(
            "File .docm (macro-enabled) ditolak demi keamanan; gunakan .docx atau .pdf standar."
                .to_string(),
        ));
    }
    let extracted_text = if low.ends_with(".docx") {
        extract_text_from_docx_bytes(bytes)?
    } else if low.ends_with(".pdf") {
        let pdf_text = extract_text_from_pdf_bytes(bytes)?;
        let trimmed = pdf_text.trim();
        if trimmed.len() < 30 {
            return Err(JsfsError::Document(
                "Berkas PDF tidak memiliki lapisan teks (kemungkinan hasil scan gambar). Silakan unggah berkas .docx asli dari Microsoft Word."
                    .to_string(),
            ));
        }
        pdf_text
    } else {
        return Err(JsfsError::InvalidInput(
            "Hanya file .docx atau .pdf yang didukung".to_string(),
        ));
    };

    let char_count = extracted_text.chars().count();
    let (extracted_title, suggested_keywords, priority_kws) =
        extract_manuscript_title_and_keywords(&extracted_text);
    let (freq_kws, detected_domain) = extract_keywords_and_domain(&extracted_text, 14);
    let mut detected_keywords = priority_kws;
    for kw in freq_kws {
        if !detected_keywords.contains(&kw) {
            detected_keywords.push(kw);
        }
        if detected_keywords.len() >= 14 {
            break;
        }
    }
    let checklist = analyze_manuscript_checklist(&extracted_text);
    let abstract_snippet = extract_abstract_snippet(&extracted_text);
    let cited_journals = extract_cited_journals(&extracted_text);
    let recommended_topics = extract_recommended_topics(
        &extracted_title,
        &suggested_keywords,
        &abstract_snippet,
        &detected_keywords,
        7,
    );
    let detected_citation_style = detect_manuscript_citation_style(&extracted_text);

    Ok(ManuscriptMeta {
        filename: filename.to_string(),
        char_count,
        extracted_text: extracted_text.chars().take(12_000).collect(),
        detected_keywords,
        detected_domain,
        extracted_title,
        suggested_keywords,
        abstract_snippet,
        checklist,
        cited_journals,
        recommended_topics,
        detected_citation_style,
    })
}

fn push_topic_chip(
    topics: &mut Vec<String>,
    stop: &HashSet<&str>,
    raw: &str,
    max_topics: usize,
) -> bool {
    if topics.len() >= max_topics {
        return true;
    }
    let cleaned = raw
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != ' ')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let word_cnt = cleaned.split_whitespace().count();
    if (1..=2).contains(&word_cnt)
        && cleaned.chars().count() >= 3
        && !stop.contains(cleaned.as_str())
        && !topics.iter().any(|t| t == &cleaned)
    {
        topics.push(cleaned);
    }
    topics.len() >= max_topics
}

/// Build 5–7 concise clickable topic chips (1–2 words) from title, explicit keywords, and abstract.
pub fn extract_recommended_topics(
    extracted_title: &str,
    suggested_keywords: &str,
    abstract_snippet: &str,
    detected_keywords: &[String],
    max_topics: usize,
) -> Vec<String> {
    let mut topics: Vec<String> = Vec::new();
    let stop: HashSet<&'static str> = STOPWORDS.iter().copied().collect();

    // 1. Prioritize explicit/TF-IDF suggested phrases (keep 1-2 word phrases whole, split >=3 words)
    for part in suggested_keywords.split([',', ';', '|', '•']) {
        let clean_part = part.trim();
        let wcnt = clean_part.split_whitespace().count();
        if (1..=2).contains(&wcnt) {
            if push_topic_chip(&mut topics, &stop, clean_part, max_topics) {
                return topics;
            }
        } else if wcnt >= 3 {
            let toks = tokenize_clean(clean_part);
            for w in toks.windows(2) {
                let b = format!("{} {}", w[0], w[1]);
                if push_topic_chip(&mut topics, &stop, &b, max_topics) {
                    return topics;
                }
            }
            for t in toks {
                if push_topic_chip(&mut topics, &stop, &t, max_topics) {
                    return topics;
                }
            }
        }
    }

    // 2. Supplement with high-value bigrams/unigrams from Title + Abstract + detected_keywords
    let title_abs = format!("{extracted_title} {abstract_snippet}");
    let toks = tokenize_clean(&title_abs);
    let mut scored: Vec<(String, f64)> = Vec::new();
    for win in toks.windows(2) {
        if win[0] != win[1] && !stop.contains(win[0].as_str()) && !stop.contains(win[1].as_str()) {
            let bg = format!("{} {}", win[0], win[1]);
            let sc = term_idf_weight(&win[0]) + term_idf_weight(&win[1]);
            scored.push((bg, sc));
        }
    }
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (bg, _) in scored {
        if push_topic_chip(&mut topics, &stop, &bg, max_topics) {
            return topics;
        }
    }

    for kw in detected_keywords {
        if push_topic_chip(&mut topics, &stop, kw, max_topics) {
            break;
        }
    }

    topics
}

/// Evaluate manuscript word length against a journal's `(min_words, max_words)` requirement.
pub fn check_manuscript_word_fit(
    manuscript_words: u32,
    word_limit: Option<(u32, u32)>,
) -> Option<FitStatus> {
    if manuscript_words == 0 {
        return None;
    }
    let (min_w, max_w) = word_limit?;
    if min_w > 0 && manuscript_words < min_w {
        Some(FitStatus::TerlaluPendek)
    } else if max_w > 0 && manuscript_words > max_w {
        Some(FitStatus::MelebihiBatas)
    } else {
        Some(FitStatus::Sesuai)
    }
}

fn truncate_clean_words(raw: &str, max_chars: usize) -> String {
    let clean: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= max_chars {
        return clean;
    }
    let mut out = String::new();
    for ch in clean.chars().take(max_chars) {
        out.push(ch);
    }
    if let Some(pos) = out.rfind(' ') {
        if pos > max_chars / 2 {
            out.truncate(pos);
        }
    }
    out.push_str("...");
    out
}

/// Extract a clean 1-2 sentence abstract snippet for auto-filling cover letters & inquiry emails.
pub fn extract_abstract_snippet(text: &str) -> String {
    static RE_ABS_START: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?im)^\s*(?:abstract|abstrak)\b[:\-\s]*").expect("valid regex"));
    static RE_ABS_END: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?im)^\s*(?:kata\s*kunci|key\s*words?|pendahuluan|introduction|background|1\.\s)",
        )
        .expect("valid regex")
    });

    if let Some(m) = RE_ABS_START.find(text) {
        let rest = &text[m.end()..];
        let mut end = rest.len().min(1200);
        while end > 0 && !rest.is_char_boundary(end) {
            end -= 1;
        }
        let window = &rest[..end];
        let slice = if let Some(stop) = RE_ABS_END.find(window) {
            &window[..stop.start()]
        } else {
            window
        };
        let snippet = truncate_clean_words(slice, 240);
        if !snippet.is_empty() {
            return snippet;
        }
    }

    let fallback: String = text
        .lines()
        .map(str::trim)
        .filter(|l| l.len() > 25)
        .skip(1)
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    truncate_clean_words(&fallback, 240)
}

fn normalize_venue_key(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut prev_space = true;
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    out.trim().to_string()
}

fn normalize_issn_token(raw: &str) -> Option<String> {
    let clean: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if clean.len() == 8 && clean[..7].chars().all(|c| c.is_ascii_digit()) {
        Some(format!("{}-{}", &clean[..4], &clean[4..]))
    } else {
        None
    }
}

fn strip_venue_tail(raw: &str) -> &str {
    static RE_VOL_TAIL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)(?:,\s*(?:vol\.?|no\.?|issue|hal\.?|pp\.?|\d)|\s*\(\d|\s+\d{1,3}\s*\(|\s+doi\s*:|\s+https?://)").expect("valid regex")
    });
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| matches!(c, ',' | '.' | ';' | ':' | '"' | '“' | '”' | '\''));
    if let Some(m) = RE_VOL_TAIL.find(trimmed) {
        trimmed[..m.start()]
            .trim()
            .trim_matches(|c: char| matches!(c, ',' | '.' | ';' | ':' | '-'))
    } else {
        trimmed
    }
}

/// Extract cited journal names and ISSNs from the manuscript's bibliography / References section.
pub fn extract_cited_journals(text: &str) -> HashMap<String, u32> {
    static RE_REF_HEADER: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?im)^\s*(?:\d+(?:\.\d+)?\s+)?(?:daftar\s*pustaka|referensi|references|bibliography|pustaka\s*acuan)\s*:?\s*$",
        )
        .expect("valid regex")
    });
    static RE_ISSN_IN_REF: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(\d{4}[-–]\d{3}[\dXx])\b").expect("valid regex"));
    static RE_ENTRY_START: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^(?:\[\d{1,3}\]|\d{1,3}\.\s+[A-Z]|[A-Z][A-Za-z'\-]{1,28},\s+[A-Z])")
            .expect("valid regex")
    });
    static RE_QUOTED_TITLE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"[“"]([^”"\r\n]{8,220})[”"][,\.]?\s*([^.\r\n]{5,120})"#).expect("valid regex")
    });
    static RE_APA_AFTER_YEAR: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?:\(\s*(?:19|20)\d{2}[a-z]?\s*\)\.?|\b(?:19|20)\d{2}\.)\s*[^.\r\n]{8,220}\.\s*([^.\r\n]{5,120})")
            .expect("valid regex")
    });
    static RE_VENUE_PHRASE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b((?:jurnal|journal|prosiding|proceedings|buletin|majalah|transactions\s+on|ieee)\s+[^.;\r\n]{3,95})")
            .expect("valid regex")
    });

    let mut header_match_start = None;
    for m in RE_REF_HEADER.find_iter(text) {
        header_match_start = Some(m.end());
    }
    let ref_slice = if let Some(start) = header_match_start {
        &text[start..]
    } else if text.len() <= 4_500 {
        text
    } else {
        return HashMap::new();
    };

    // Merge wrapped lines into logical bibliography entries.
    let mut entries: Vec<String> = Vec::new();
    let mut cur = String::new();
    for raw_line in ref_slice.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            if !cur.trim().is_empty() {
                entries.push(std::mem::take(&mut cur));
            }
            continue;
        }
        if RE_ENTRY_START.is_match(line) && !cur.trim().is_empty() {
            entries.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(line);
    }
    if !cur.trim().is_empty() {
        entries.push(cur);
    }

    let mut counts: HashMap<String, u32> = HashMap::new();
    for entry in entries {
        let mut entry_keys: HashSet<String> = HashSet::new();

        for cap in RE_ISSN_IN_REF.captures_iter(&entry) {
            if let Some(m) = cap.get(1) {
                if let Some(issn) = normalize_issn_token(m.as_str()) {
                    entry_keys.insert(issn);
                }
            }
        }
        if let Some(cap) = RE_QUOTED_TITLE.captures(&entry) {
            if let Some(v) = cap.get(2) {
                let norm = normalize_venue_key(strip_venue_tail(v.as_str()));
                if norm.len() >= 6 && norm.split_whitespace().count() >= 2 {
                    entry_keys.insert(norm);
                }
            }
        }
        if let Some(cap) = RE_APA_AFTER_YEAR.captures(&entry) {
            if let Some(v) = cap.get(1) {
                let norm = normalize_venue_key(strip_venue_tail(v.as_str()));
                if norm.len() >= 6 && norm.split_whitespace().count() >= 2 {
                    entry_keys.insert(norm);
                }
            }
        }
        for cap in RE_VENUE_PHRASE.captures_iter(&entry) {
            if let Some(v) = cap.get(1) {
                let norm = normalize_venue_key(strip_venue_tail(v.as_str()));
                if norm.len() >= 6 && norm.split_whitespace().count() >= 2 {
                    entry_keys.insert(norm);
                }
            }
        }

        for k in entry_keys {
            *counts.entry(k).or_insert(0) += 1;
        }
    }

    counts
}

/// Count how many times a journal appears in `cited_journals` by ISSN/eISSN or normalized title.
pub fn count_journal_citations(
    cited_journals: &HashMap<String, u32>,
    title: &str,
    issn: &str,
    eissn: &str,
) -> u32 {
    if cited_journals.is_empty() {
        return 0;
    }
    let mut best = 0u32;
    for raw_issn in [issn, eissn] {
        if let Some(norm) = normalize_issn_token(raw_issn) {
            if let Some(&cnt) = cited_journals.get(&norm) {
                best = best.max(cnt);
            }
        }
    }

    let full_norm = normalize_venue_key(title);
    if !full_norm.is_empty() {
        if let Some(&cnt) = cited_journals.get(&full_norm) {
            best = best.max(cnt);
        }
    }

    // Also match title without trailing parenthetical acronym, e.g. "Jurnal X (JX)" -> "jurnal x".
    let before_paren = title.split('(').next().unwrap_or(title);
    let base_norm = normalize_venue_key(before_paren);
    if base_norm.len() >= 8 && base_norm.split_whitespace().count() >= 2 {
        if let Some(&cnt) = cited_journals.get(&base_norm) {
            best = best.max(cnt);
        }
    }

    best
}

/// Populate `cited_count` across in-memory search results from the loaded manuscript's bibliography.
pub fn apply_cited_counts(items: &mut [UiJournalItem], cited_journals: &HashMap<String, u32>) {
    for item in items.iter_mut() {
        item.cited_count =
            count_journal_citations(cited_journals, &item.nama, &item.issn, &item.eissn);
    }
}

/// Analyze manuscript text for IMRAD structure, word count, and reference count (Feature #10).
pub fn analyze_manuscript_checklist(text: &str) -> PreSubmitChecklist {
    let word_count = text.split_whitespace().count();
    let low = text.to_ascii_lowercase();

    let has_abstract = low.contains("abstrak") || low.contains("abstract");
    let has_keywords =
        low.contains("kata kunci") || low.contains("keywords") || low.contains("keyword:");
    let has_intro = low.contains("pendahuluan")
        || low.contains("introduction")
        || low.contains("latar belakang");
    let has_method = low.contains("metode")
        || low.contains("metodologi")
        || low.contains("method")
        || low.contains("materials and methods");
    let has_results = low.contains("hasil")
        || low.contains("pembahasan")
        || low.contains("results")
        || low.contains("discussion");
    let has_references = low.contains("daftar pustaka")
        || low.contains("referensi")
        || low.contains("references")
        || low.contains("bibliography");

    // Estimate reference count from bibliography tail or DOI/year patterns
    let ref_tail = if let Some(idx) = low
        .rfind("daftar pustaka")
        .or_else(|| low.rfind("references"))
        .or_else(|| low.rfind("referensi"))
    {
        &low[idx..]
    } else {
        let start = low.len().saturating_sub(low.len() / 4);
        &low[start..]
    };
    let doi_hits = ref_tail.matches("doi.org").count() + ref_tail.matches("10.").count() / 2;
    let bracket_hits = (1..=60)
        .filter(|n| ref_tail.contains(&format!("[{n}]")))
        .count();
    let year_hits = (2015..=2026)
        .map(|yr| {
            ref_tail.matches(&format!("({yr})")).count()
                + ref_tail.matches(&format!(" {yr}.")).count()
        })
        .sum::<usize>();
    let estimated_refs = doi_hits.max(bracket_hits).max(year_hits).min(150);

    let imrad_complete = has_abstract && has_intro && has_method && has_results && has_references;
    let uncited_long_paragraphs = count_uncited_long_paragraphs(text);
    let mut missing = Vec::new();
    if !has_abstract {
        missing.push("Abstrak");
    }
    if !has_intro {
        missing.push("Pendahuluan");
    }
    if !has_method {
        missing.push("Metode");
    }
    if !has_results {
        missing.push("Hasil");
    }
    if !has_references {
        missing.push("Daftar Pustaka");
    }

    let imrad_str = if imrad_complete {
        "IMRAD Lengkap".to_string()
    } else {
        format!("Belum: {}", missing.join(", "))
    };
    let ref_str = if estimated_refs > 0 {
        format!(" • ~{estimated_refs} ref")
    } else {
        String::new()
    };
    let summary_badge = format!("{word_count} kata • {imrad_str}{ref_str}");

    PreSubmitChecklist {
        word_count,
        has_abstract,
        has_keywords,
        has_intro,
        has_method,
        has_results,
        has_references,
        estimated_refs,
        imrad_complete,
        summary_badge,
        uncited_long_paragraphs,
    }
}

/// Count body paragraphs (`>= 110` words) before the bibliography section that contain
/// neither numeric `[1]` nor `(Author, Year)` in-text citations.
pub fn count_uncited_long_paragraphs(text: &str) -> u32 {
    let normalized = text.replace("\r\n", "\n");
    let low = normalized.to_ascii_lowercase();
    let body_end = low
        .rfind("\ndaftar pustaka")
        .or_else(|| low.rfind("\nreferences"))
        .or_else(|| low.rfind("\nreferensi"))
        .or_else(|| low.rfind("\nbibliography"))
        .unwrap_or(normalized.len());
    let body = &normalized[..body_end];

    let raw_paragraphs: Vec<String> = if body.matches("\n\n").count() >= 3 {
        body.split("\n\n")
            .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    } else {
        body.lines().map(|l| l.trim().to_string()).collect()
    };

    let mut uncited = 0u32;
    let mut in_abstract = false;
    for para in raw_paragraphs {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }
        let p_low = trimmed.to_ascii_lowercase();
        if p_low == "abstrak" || p_low == "abstract" {
            in_abstract = true;
            continue;
        }
        if p_low.contains("pendahuluan") || p_low.contains("introduction") {
            in_abstract = false;
        }
        if in_abstract
            || p_low.starts_with("abstrak:")
            || p_low.starts_with("abstract:")
            || p_low.starts_with("kata kunci")
            || p_low.starts_with("keywords")
        {
            continue;
        }
        let wc = trimmed.split_whitespace().count();
        if wc >= 110 && !RE_BRACKET_CITE.is_match(trimmed) && !RE_AUTHOR_YEAR_CITE.is_match(trimmed)
        {
            uncited = uncited.saturating_add(1);
        }
    }
    uncited
}

/// Detect dominant citation style (IEEE / Numeric vs APA / Author-Year) in manuscript text.
pub fn detect_manuscript_citation_style(text: &str) -> Option<String> {
    let bracket_hits = RE_BRACKET_CITE.find_iter(text).count();
    let author_year_hits = RE_AUTHOR_YEAR_CITE.find_iter(text).count();

    let low = text.to_ascii_lowercase();
    let ref_section = if let Some(idx) = low
        .rfind("daftar pustaka")
        .or_else(|| low.rfind("references"))
        .or_else(|| low.rfind("referensi"))
    {
        &text[idx..]
    } else {
        let start = text.len().saturating_sub(text.len() / 3);
        &text[start..]
    };

    let ref_bracket_hits = (1..=20)
        .filter(|n| ref_section.contains(&format!("[{n}]")))
        .count();
    let total_ieee = bracket_hits + ref_bracket_hits * 3;
    let total_apa = author_year_hits;

    if total_ieee >= 2 && total_ieee > total_apa {
        Some("IEEE (Numeric)".to_string())
    } else if total_apa >= 2 && total_apa >= total_ieee {
        Some("APA (Author-Year)".to_string())
    } else if total_ieee >= 1 {
        Some("IEEE (Numeric)".to_string())
    } else if total_apa >= 1 {
        Some("APA (Author-Year)".to_string())
    } else {
        None
    }
}

/// Auto-Query Expansion: Extracts core academic keywords and bilingual ID-EN equivalents
/// when query is overly specific or returns few results (< 5 journals).
pub fn suggest_expanded_queries(query: &str) -> Vec<String> {
    let clean = query.trim().to_ascii_lowercase();
    if clean.is_empty() {
        return Vec::new();
    }
    let tokens = tokenize_clean(&clean);
    let mut suggestions = Vec::new();

    // 1. Identify core non-stopword tokens
    let core_terms: Vec<&str> = tokens
        .iter()
        .map(|s| s.as_str())
        .filter(|t| !STOPWORDS.contains(t) && t.len() >= 4)
        .collect();

    // 2. Multi-word core combinations
    if core_terms.len() >= 2 {
        for win in core_terms.windows(2) {
            let combined = format!("{} {}", win[0], win[1]);
            if combined != clean && !suggestions.contains(&combined) {
                suggestions.push(combined);
            }
        }
    }

    // 3. Individual core academic terms
    for &term in &core_terms {
        let t_str = term.to_string();
        if t_str != clean && !suggestions.contains(&t_str) {
            suggestions.push(t_str);
        }
    }

    // 4. Bilingual equivalents from BILINGUAL_SYNONYM_GROUPS
    for &term in &core_terms {
        let syns = synonyms_for_token(term);
        for syn in syns {
            let s_str = syn.to_string();
            if s_str != clean && !suggestions.contains(&s_str) {
                suggestions.push(s_str);
            }
        }
    }

    // Return top 2 to 3 suggestions
    suggestions.truncate(3);
    suggestions
}

/// Rank journals in `items` by BM25 similarity to the journal at `target_idx` (Feature #4: Lookalike Finder).
/// Returns `(item_index, similarity_score)` sorted descending, with the target journal first (`100.0`).
pub fn rank_similar_journals(target_idx: usize, items: &[UiJournalItem]) -> Vec<(usize, f64)> {
    let Some(target) = items.get(target_idx) else {
        return Vec::new();
    };
    let seed_text = format!("{} {}", target.nama, target.focus_scope);
    let (mut top_kws, domain) = extract_keywords_and_domain(&seed_text, 10);
    if top_kws.is_empty() {
        for tok in tokenize_clean(&target.nama) {
            if !top_kws.contains(&tok) {
                top_kws.push(tok);
            }
        }
    }
    let synthetic_ms = ManuscriptMeta {
        filename: target.nama.clone(),
        char_count: seed_text.len(),
        extracted_text: seed_text,
        detected_keywords: top_kws.clone(),
        detected_domain: domain,
        extracted_title: target.nama.clone(),
        suggested_keywords: String::new(),
        abstract_snippet: String::new(),
        checklist: PreSubmitChecklist::default(),
        cited_journals: HashMap::new(),
        recommended_topics: Vec::new(),
        detected_citation_style: None,
    };

    let mut ranked: Vec<(usize, f64)> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            if idx == target_idx {
                return (idx, 100.0);
            }
            let rec = JournalRecord {
                title: item.nama.clone(),
                subject_area: item.sumber.clone(),
                focus_and_scope: item.focus_scope.clone(),
                ..Default::default()
            };
            let (sc, _, _) =
                compute_match_score(&rec, &top_kws[..top_kws.len().min(5)], Some(&synthetic_ms));
            (idx, sc)
        })
        .collect();

    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranked
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
    // Morphological stem match (Bilingual ID/EN stemmer)
    let stem = stem_academic_word(term);
    if stem.len() >= 4 && stem != term && raw_low.contains(&stem) {
        return (1, None);
    }
    (0, None)
}

/// Compute discriminative Bilingual BM25 + Field-Weighted (Title 3.0x, Scope 2.0x, Subject 1.5x)
/// + IDF Rarity Weighting + Exact Phrase Bonus match score (`15.0..=96.0`).
pub fn compute_match_score(
    record: &JournalRecord,
    search_keywords: &[String],
    manuscript: Option<&ManuscriptMeta>,
) -> (f64, String, Vec<String>) {
    // Weighted query terms: (token, base_weight * idf_rarity_weight)
    let mut query_weighted: Vec<(String, f64)> = Vec::new();
    let mut raw_phrases: Vec<String> = Vec::new();
    let mut seen_terms: HashSet<String> = HashSet::new();

    for kw in search_keywords {
        let low_kw = kw.trim().to_ascii_lowercase();
        if (low_kw.contains(' ') || low_kw.contains('-')) && low_kw.len() >= 5 {
            raw_phrases.push(low_kw.clone());
            if low_kw.contains("e-commerce") {
                raw_phrases.push("ecommerce".to_string());
            }
        }
        for part in tokenize_clean(&low_kw) {
            if seen_terms.insert(part.clone()) {
                query_weighted.push((part.clone(), 1.60 * term_idf_weight(&part)));
            }
        }
    }

    if let Some(ms) = manuscript {
        if !ms.suggested_keywords.trim().is_empty() {
            for kw_phrase in ms.suggested_keywords.split(',') {
                let lp = kw_phrase.trim().to_ascii_lowercase();
                if (lp.contains(' ') || lp.contains('-'))
                    && lp.len() >= 5
                    && !raw_phrases.contains(&lp)
                {
                    raw_phrases.push(lp);
                }
            }
        }
        let total_ms = ms.detected_keywords.len().max(1) as f64;
        for (rank, kw) in ms.detected_keywords.iter().take(10).enumerate() {
            let clean = kw.trim().to_ascii_lowercase();
            if seen_terms.insert(clean.clone()) {
                let rank_decay = 1.05 - (rank as f64 / total_ms) * 0.55;
                query_weighted.push((clean.clone(), rank_decay * term_idf_weight(&clean)));
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
                if !raw_phrases.iter().any(|p| p == id_p) {
                    raw_phrases.push(id_p.to_string());
                }
                if !raw_phrases.iter().any(|p| p == en_p) {
                    raw_phrases.push(en_p.to_string());
                }
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
    let mut ideal_multi_field_max = 0.0f64;
    let mut total_query_weight = 0.0f64;
    let mut covered_weight = 0.0f64;

    let mut title_matched = Vec::new();
    let mut scope_matched = Vec::new();
    let mut subj_matched = Vec::new();
    let mut synonym_pairs: Vec<String> = Vec::new();

    for (qt, q_weight) in &query_weighted {
        let syns = synonyms_for_token(qt);
        total_query_weight += *q_weight;
        // Balanced denominator across Title (3.0x) + Focus & Scope (2.0x) + Subject (1.5x)
        // prevents premature clamping/saturation on 84%.
        ideal_multi_field_max += q_weight * 6.2;

        let mut best_cov_factor = 0.0f64;

        let (t_cnt, t_syn) = field_term_hit(qt, &syns, &title_tf, &title_low);
        if t_cnt > 0 {
            let mult = if t_syn.is_some() { 0.85 } else { 1.0 };
            raw_bm25 += q_weight * mult * 3.0 * bm25_tf(t_cnt, title_len, 6.0);
            best_cov_factor = best_cov_factor.max(1.0 * mult);
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

        let (f_cnt, f_syn) = field_term_hit(qt, &syns, &scope_tf, &scope_low);
        if f_cnt > 0 {
            let mult = if f_syn.is_some() { 0.85 } else { 1.0 };
            raw_bm25 += q_weight * mult * 2.0 * bm25_tf(f_cnt, scope_len, 45.0);
            best_cov_factor = best_cov_factor.max(0.88 * mult);
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

        let (s_cnt, s_syn) = field_term_hit(qt, &syns, &subj_tf, &subj_low);
        if s_cnt > 0 {
            let mult = if s_syn.is_some() { 0.85 } else { 1.0 };
            raw_bm25 += q_weight * mult * 1.5 * bm25_tf(s_cnt, subj_len, 8.0);
            best_cov_factor = best_cov_factor.max(0.70 * mult);
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

        covered_weight += q_weight * best_cov_factor;
    }

    // Exact multi-word phrase & cross-lingual phrase bonus (differentiates Title vs Scope hits)
    let mut phrase_bonus = 0.0f64;
    for phrase in &raw_phrases {
        if title_low.contains(phrase.as_str()) {
            phrase_bonus += 8.0;
        } else if scope_low.contains(phrase.as_str()) || subj_low.contains(phrase.as_str()) {
            phrase_bonus += 4.5;
        }
        for &(id_p, en_p) in BILINGUAL_PHRASE_PAIRS {
            if phrase == id_p && all_text.contains(en_p) {
                phrase_bonus += if title_low.contains(en_p) { 6.0 } else { 3.5 };
                let lbl = format!("{id_p} ↔ {en_p}");
                if !synonym_pairs.contains(&lbl) {
                    synonym_pairs.push(lbl);
                }
            } else if phrase == en_p && all_text.contains(id_p) {
                phrase_bonus += if title_low.contains(id_p) { 6.0 } else { 3.5 };
                let lbl = format!("{en_p} ↔ {id_p}");
                if !synonym_pairs.contains(&lbl) {
                    synonym_pairs.push(lbl);
                }
            }
        }
    }

    // Bigram phrase boosting (1.5x - 2.0x boost for multi-word phrase matches from manuscript)
    let mut bigram_boost_applied = false;
    if let Some(ms) = manuscript {
        let t_toks = tokenize_clean(&ms.extracted_title);
        let a_toks = tokenize_clean(&ms.abstract_snippet);
        let mut bigrams: Vec<(String, f64)> = Vec::new();
        for win in t_toks.windows(2) {
            if win[0] != win[1] {
                bigrams.push((format!("{} {}", win[0], win[1]), 2.0)); // 2.0x boost for title phrases
            }
        }
        for win in a_toks.windows(2) {
            if win[0] != win[1] {
                let bg = format!("{} {}", win[0], win[1]);
                if !bigrams.iter().any(|(b, _)| b == &bg) {
                    bigrams.push((bg, 1.5)); // 1.5x boost for abstract phrases
                }
            }
        }
        for (bg, mult) in bigrams {
            if title_low.contains(&bg) {
                phrase_bonus += 6.0 * mult;
                bigram_boost_applied = true;
            } else if scope_low.contains(&bg) {
                phrase_bonus += 3.5 * mult;
                bigram_boost_applied = true;
            }
        }
    }

    let coverage_ratio = (covered_weight / total_query_weight.max(0.01)).clamp(0.0, 1.0);
    let field_intensity = (raw_bm25 / ideal_multi_field_max.max(0.01)).clamp(0.0, 1.0);
    let mut score =
        20.0 + 36.0 * coverage_ratio + 26.0 * field_intensity.powf(0.82) + phrase_bonus.min(18.0);

    let mut reasons = Vec::new();
    if bigram_boost_applied {
        reasons.push("Frasa kunci naskah terverifikasi dalam lingkup jurnal".to_string());
    }
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
            "Padanan akademik ID-EN ({})",
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
            score += 4.0;
            reasons.push(format!("Rumpun keilmuan selaras: {}", ms.detected_domain));
        }
    }

    // Small penalty if Focus & Scope is completely empty and no title/subject match
    if scope_low.is_empty() && title_matched.is_empty() && subj_matched.is_empty() {
        score = score.min(34.0);
    }

    let clamped = score.clamp(15.0, 96.0);
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
            match_reasons.iter().any(|r| r.contains("ID-EN")),
            "expected ID-EN synonym rationale, got {match_reasons:?}"
        );
    }

    #[test]
    fn test_manuscript_checklist_and_rank_similar_journals() {
        let sample_paper = "Abstrak: Penelitian ini membahas machine learning. Kata Kunci: AI, Data. \
            1. Pendahuluan Latar belakang masalah. 2. Metode Penelitian eksperimen. \
            3. Hasil dan Pembahasan Akurasi mencapai 95%. Daftar Pustaka [1] Smith (2023) [2] Budi (2024) [3] Ani (2025).";
        let cl = analyze_manuscript_checklist(sample_paper);
        assert!(cl.imrad_complete);
        assert!(
            cl.has_abstract
                && cl.has_keywords
                && cl.has_intro
                && cl.has_method
                && cl.has_results
                && cl.has_references
        );
        assert!(cl.estimated_refs >= 3);
        assert!(cl.summary_badge.contains("IMRAD Lengkap"));

        let items = vec![
            UiJournalItem {
                nama: "Jurnal Rekayasa Perangkat Lunak dan AI".to_string(),
                focus_scope:
                    "Software engineering, machine learning, deep learning, neural network"
                        .to_string(),
                ..Default::default()
            },
            UiJournalItem {
                nama: "Jurnal Peternakan Sapi Perah".to_string(),
                focus_scope: "Nutrisi ternak, pakan hijauan, produksi susu sapi".to_string(),
                ..Default::default()
            },
            UiJournalItem {
                nama: "Journal of Intelligent Systems and Machine Learning".to_string(),
                focus_scope: "Artificial intelligence, deep learning, software engineering"
                    .to_string(),
                ..Default::default()
            },
        ];
        let ranked = rank_similar_journals(0, &items);
        assert_eq!(ranked[0].0, 0);
        assert_eq!(
            ranked[1].0, 2,
            "AI journal should rank closer than dairy farming journal"
        );
    }

    #[test]
    fn test_extract_manuscript_keywords_and_suggestions() {
        let with_explicit_kw = "Implementasi Random Forest untuk Klasifikasi Sentimen Konsumen\n\n\
            Abstrak: Penelitian ini mengevaluasi algoritma random forest pada ulasan produk e-commerce.\n\
            Kata Kunci: random forest, analisis sentimen, klasifikasi teks, e-commerce\n\n\
            1. Pendahuluan";
        let (title1, kw1, _) = extract_manuscript_title_and_keywords(with_explicit_kw);
        assert!(
            title1.contains("Implementasi Random Forest"),
            "unexpected extracted title: {title1}"
        );
        assert!(
            kw1.to_lowercase().contains("random forest")
                && kw1.to_lowercase().contains("analisis sentimen"),
            "expected explicit keywords to be extracted, got: {kw1}"
        );

        let without_explicit_kw = "Optimasi Convolutional Neural Network untuk Deteksi Penyakit Daun Padi\n\n\
            Abstract: This study optimizes convolutional neural network models for rice leaf disease classification \
            using transfer learning and image augmentation on field datasets.\n\
            1. Introduction";
        let (title2, kw2, _) = extract_manuscript_title_and_keywords(without_explicit_kw);
        assert!(
            title2.contains("Convolutional Neural Network"),
            "unexpected fallback title: {title2}"
        );
        let lower_kw2 = kw2.to_lowercase();
        assert!(
            !lower_kw2.is_empty()
                && (lower_kw2.contains("neural")
                    || lower_kw2.contains("rice")
                    || lower_kw2.contains("disease")),
            "expected fallback TF-IDF/bigram keywords from abstract, got: {kw2}"
        );
    }

    #[test]
    fn test_discriminative_match_scoring_not_uniform() {
        let keywords = vec![
            "machine learning".to_string(),
            "sentiment analysis".to_string(),
            "classification".to_string(),
        ];

        let exact_title_and_scope = JournalRecord {
            title: "Journal of Machine Learning and Sentiment Analysis".to_string(),
            subject_area: "Computer Science, Artificial Intelligence".to_string(),
            focus_and_scope: "Publishes machine learning, sentiment analysis, and text classification algorithms.".to_string(),
            ..Default::default()
        };

        let scope_only_partial = JournalRecord {
            title: "Jurnal Teknologi dan Sistem Informasi".to_string(),
            subject_area: "Information Systems".to_string(),
            focus_and_scope: "Covers enterprise systems, software engineering, and occasional classification studies.".to_string(),
            ..Default::default()
        };

        let generic_hub_only = JournalRecord {
            title: "Jurnal Penelitian Multidisiplin Nusantara".to_string(),
            subject_area: "Multidisciplinary".to_string(),
            focus_and_scope:
                "Artikel penelitian ilmu komputer, manajemen, pendidikan, dan teknologi informasi."
                    .to_string(),
            ..Default::default()
        };

        let (s_exact, _, _) = compute_match_score(&exact_title_and_scope, &keywords, None);
        let (s_partial, _, _) = compute_match_score(&scope_only_partial, &keywords, None);
        let (s_generic, _, _) = compute_match_score(&generic_hub_only, &keywords, None);

        assert!(
            (75.0..=96.0).contains(&s_exact),
            "expected strong exact journal in 75..=96, got {s_exact}"
        );
        assert!(
            s_exact > s_partial + 12.0,
            "expected clear gap between exact ({s_exact}) and partial ({s_partial})"
        );
        assert!(
            s_partial > s_generic,
            "expected partial ({s_partial}) > generic/unmatched ({s_generic})"
        );
    }

    #[test]
    fn test_check_manuscript_word_fit() {
        assert_eq!(
            check_manuscript_word_fit(5200, Some((4000, 7000))),
            Some(FitStatus::Sesuai)
        );
        assert_eq!(
            check_manuscript_word_fit(2800, Some((4000, 7000))),
            Some(FitStatus::TerlaluPendek)
        );
        assert_eq!(
            check_manuscript_word_fit(8100, Some((4000, 7000))),
            Some(FitStatus::MelebihiBatas)
        );
        assert_eq!(
            check_manuscript_word_fit(4500, Some((0, 6000))),
            Some(FitStatus::Sesuai)
        );
        assert_eq!(check_manuscript_word_fit(0, Some((4000, 7000))), None);
        assert_eq!(check_manuscript_word_fit(4500, None), None);
    }

    #[test]
    fn test_cite_to_target_extraction_and_matching() {
        let manuscript_text = "\
Judul Artikel: Klasifikasi Sentimen Berbasis IndoBERT\n\
Abstrak\n\
Penelitian ini mengklasifikasikan sentimen ulasan mahasiswa menggunakan model IndoBERT dengan akurasi tinggi.\n\
Kata Kunci: sentimen, indobert\n\
\n\
Daftar Pustaka\n\
[1] A. Budi, \"Analisis Sentimen Akademik,\" Jurnal Sistem Informasi Indonesia, vol. 8, no. 2, pp. 11-20, 2023. ISSN 2548-1234.\n\
[2] R. Sari (2024). Evaluasi Deep Learning pada Data Kampus. Jurnal Sistem Informasi Indonesia, 9(1), 55-64.\n\
[3] T. Putra (2022). Optimasi Neural Network. IEEE Access, 10, 12345-12355.\n";

        let snippet = extract_abstract_snippet(manuscript_text);
        assert!(snippet.contains("mengklasifikasikan sentimen ulasan mahasiswa"));

        let cited = extract_cited_journals(manuscript_text);
        let cnt1 = count_journal_citations(
            &cited,
            "Jurnal Sistem Informasi Indonesia (JSII)",
            "2548-1234",
            "",
        );
        assert_eq!(cnt1, 2);

        let cnt2 = count_journal_citations(&cited, "IEEE Access", "", "");
        assert_eq!(cnt2, 1);

        let cnt3 = count_journal_citations(&cited, "Jurnal Pendidikan Biologi", "9999-0000", "");
        assert_eq!(cnt3, 0);
    }

    #[test]
    fn test_extract_recommended_topics_short_chips() {
        let topics = extract_recommended_topics(
            "Pengaruh Live Streaming E-Commerce terhadap Perilaku Konsumen Gen Z",
            "bisnis, e-commerce, live streaming, commerce, perilaku konsumen, gen z",
            "Penelitian ini mengkaji transaksi live streaming pada platform e-commerce.",
            &[],
            7,
        );
        assert!((5..=7).contains(&topics.len()), "got {:?}", topics);
        assert!(topics.iter().all(|t| t.split_whitespace().count() <= 2));
        assert!(topics.contains(&"e-commerce".to_string()));
        assert!(topics.contains(&"live streaming".to_string()));
    }

    #[test]
    fn test_manuscript_docm_and_size_limit_guards() {
        let err_docm = build_manuscript_meta("naskah_skripsi.docm", b"PK\x03\x04dummy");
        assert!(err_docm.is_err());
        assert!(err_docm.unwrap_err().to_string().contains(".docm"));

        let oversized = vec![b'a'; MAX_MANUSCRIPT_BYTES + 1];
        let err_big = build_manuscript_meta("big_paper.docx", &oversized);
        assert!(err_big.is_err());
        assert!(err_big.unwrap_err().to_string().contains("50 MB"));
    }

    #[test]
    fn test_uncited_long_paragraphs_detection() {
        let cited_120 = format!(
            "{} (Adinata, 2024).",
            std::iter::repeat_n("kata", 118)
                .collect::<Vec<_>>()
                .join(" ")
        );
        let uncited_120 = std::iter::repeat_n("penelitian", 120)
            .collect::<Vec<_>>()
            .join(" ");
        let bracket_120 = format!(
            "{} [12].",
            std::iter::repeat_n("metode", 118)
                .collect::<Vec<_>>()
                .join(" ")
        );
        let short_uncited = std::iter::repeat_n("ringkas", 40)
            .collect::<Vec<_>>()
            .join(" ");

        let doc = format!(
            "Abstrak\n{}\n\nPendahuluan\n{cited_120}\n{uncited_120}\n{bracket_120}\n{short_uncited}\n\nDaftar Pustaka\n{}",
            std::iter::repeat_n("ringkasan", 130).collect::<Vec<_>>().join(" "),
            std::iter::repeat_n("pustaka", 130).collect::<Vec<_>>().join(" ")
        );
        assert_eq!(count_uncited_long_paragraphs(&doc), 1);
    }

    #[test]
    fn test_stemmer_id_and_en() {
        assert_eq!(stem_id("pembelajaran"), "belajar");
        assert_eq!(stem_id("pengembangan"), "kembang");
        assert_eq!(stem_id("berkelanjutan"), "lanjut");
        assert_eq!(stem_id("sistem"), "sistem"); // stem <= 4 stays

        assert_eq!(stem_en("analyzing"), "analyz");
        assert_eq!(stem_en("classification"), "classif");
        assert_eq!(stem_en("systems"), "system");
        assert_eq!(stem_en("running"), "run");

        assert_eq!(stem_academic_word("pengembangan"), "kembang");
        assert_eq!(stem_academic_word("processing"), "process");
    }

    #[test]
    fn test_bigram_phrase_boosting() {
        let rec = JournalRecord {
            title: "Jurnal Rekayasa Perangkat Lunak dan Kecerdasan Buatan".to_string(),
            subject_area: "Informatika".to_string(),
            focus_and_scope:
                "Penelitian mencakup machine learning, rekayasa perangkat lunak, dan deep learning."
                    .to_string(),
            ..Default::default()
        };

        // Manuscript with exact bigrams
        let ms_with_bigrams = ManuscriptMeta {
            extracted_title: "Implementasi Rekayasa Perangkat Lunak pada Sistem Cerdas".to_string(),
            abstract_snippet: "Kajian tentang machine learning dan kecerdasan buatan modern."
                .to_string(),
            detected_keywords: vec!["rekayasa".to_string(), "perangkat".to_string()],
            ..Default::default()
        };

        let (score_boosted, _, reasons) = compute_match_score(&rec, &[], Some(&ms_with_bigrams));
        assert!(score_boosted >= 70.0);
        assert!(reasons.iter().any(|r| r.contains("Frasa kunci naskah")));
    }

    #[test]
    fn test_suggest_expanded_queries_few_results() {
        let long_query =
            "analisis penerapan algoritma machine learning pada sistem deteksi penipuan";
        let suggestions = suggest_expanded_queries(long_query);
        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .any(|s| s.contains("machine") || s.contains("learning")));

        let specific_query = "pendidikan matematika interaktif";
        let sugs2 = suggest_expanded_queries(specific_query);
        assert!(!sugs2.is_empty());
        assert!(sugs2.iter().any(|s| s.contains("pendidikan")
            || s.contains("matematika")
            || s.contains("mathematics")));
    }

    #[test]
    fn test_detect_manuscript_citation_style_apa_vs_ieee() {
        let text_ieee = r#"
            Pendahuluan
            Penelitian sebelumnya [1] menunjukkan akurasi tinggi, sebagaimana dibuktikan oleh penelitian terkait [2, 3].
            
            Daftar Pustaka
            [1] B. Santoso, "Sistem Cerdas," Jurnal Komputer, 2024.
            [2] R. Wijaya, "Pembelajaran Mesin," Jurnal TI, 2025.
        "#;
        assert_eq!(
            detect_manuscript_citation_style(text_ieee).as_deref(),
            Some("IEEE (Numeric)")
        );

        let text_apa = r#"
            Pendahuluan
            Menurut penelitian terbaru (Santoso, 2024), akurasi dapat ditingkatkan secara signifikan. Hal ini sejalan dengan temuan Wijaya et al. (2025).
            
            Daftar Pustaka
            Santoso, B. (2024). Sistem Cerdas. Jurnal Komputer.
            Wijaya, R. (2025). Pembelajaran Mesin. Jurnal TI.
        "#;
        assert_eq!(
            detect_manuscript_citation_style(text_apa).as_deref(),
            Some("APA (Author-Year)")
        );
    }
}
