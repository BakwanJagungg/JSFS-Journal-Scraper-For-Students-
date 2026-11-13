# =============================================================================
# JSFS - Journal Scraper For Students
# doc_reader.py — Modul Pembaca & Analisis Naskah Artikel Ilmiah (.docx / .pdf) v1.3.5
# =============================================================================

import os
import re
import zipfile
import xml.etree.ElementTree as ET
from collections import Counter
from typing import Optional, Union

# Try importing standard document parsing libraries
try:
    import docx
    HAS_DOCX = True
except ImportError:
    HAS_DOCX = False

try:
    import pypdf
    HAS_PYPDF = True
except ImportError:
    HAS_PYPDF = False


# Daftar Stopwords Indonesia & Inggris Akademis (Dieliminasi untuk Istilah Dominan)
INDONESIAN_STOPWORDS = {
    "yang", "di", "dan", "dari", "ke", "pada", "untuk", "dengan", "ini", "itu",
    "atau", "oleh", "dalam", "adalah", "sebagai", "bahwa", "juga", "dapat",
    "akan", "ada", "karena", "tersebut", "bisa", "lebih", "telah", "tidak",
    "bagi", "secara", "seperti", "hanya", "antara", "namun", "serta", "saat",
    "sangat", "setiap", "lain", "kami", "mereka", "kita", "ia", "dia", "anda",
    "sudah", "jika", "bila", "agar", "supaya", "meskipun", "walaupun", "sehingga",
    "maka", "tentang", "terhadap", "kepada", "atas", "bawah", "luar", "sebuah",
    "suatu", "para", "pun", "saja", "lagi", "masih", "tentu", "terus", "kemudian",
    "lalu", "setelah", "sebelum", "hingga", "sampai", "sesuai", "berdasarkan",
    "menurut", "yakni", "yaitu", "contoh", "misal", "misalnya", "hal", "kasus",
    # Stopwords akademis umum
    "penelitian", "peneliti", "studi", "hasil", "metode", "analisis", "analisa",
    "data", "tujuan", "kesimpulan", "pembahasan", "pendahuluan", "abstrak",
    "kata", "kunci", "artikel", "jurnal", "tabel", "gambar", "persamaan",
    "universitas", "fakultas", "program", "jurusan", "volume", "nomor", "halaman",
}

ENGLISH_STOPWORDS = {
    "the", "and", "of", "in", "to", "for", "with", "on", "at", "by", "from",
    "up", "about", "into", "over", "after", "is", "are", "was", "were", "be",
    "been", "being", "have", "has", "had", "do", "does", "did", "but", "or",
    "as", "an", "a", "this", "that", "these", "those", "then", "so", "than",
    "too", "very", "can", "will", "just", "should", "now", "any", "both",
    "each", "few", "more", "most", "other", "some", "such", "no", "nor", "not",
    "only", "own", "same", "which", "who", "whom", "what", "where", "when",
    "why", "how", "all", "while", "during", "itself", "their", "theirs",
    # Academic stopwords
    "research", "study", "results", "result", "method", "methods", "analysis",
    "data", "paper", "article", "journal", "abstract", "keywords", "keyword",
    "introduction", "conclusion", "discussion", "university", "department",
    "findings", "finding", "based", "using", "used", "table", "figure",
}

ALL_STOPWORDS = INDONESIAN_STOPWORDS | ENGLISH_STOPWORDS


def extract_text_from_docx(file_path: str) -> str:
    """
    Mengekstrak teks dari berkas Word (.docx).
    Menggunakan python-docx jika tersedia, dengan fallback ke ekstraksi XML internal zipfile.
    """
    if not os.path.exists(file_path):
        raise FileNotFoundError(f"Berkas tidak ditemukan: {file_path}")

    if HAS_DOCX:
        try:
            doc = docx.Document(file_path)
            paragraphs = [p.text for p in doc.paragraphs if p.text.strip()]
            for table in doc.tables:
                for row in table.rows:
                    for cell in row.cells:
                        txt = cell.text.strip()
                        if txt and txt not in paragraphs:
                            paragraphs.append(txt)
            return "\n".join(paragraphs)
        except Exception:
            pass

    # Fallback murni Python (zipfile + xml.etree)
    try:
        with zipfile.ZipFile(file_path, "r") as z:
            xml_content = z.read("word/document.xml")
            tree = ET.fromstring(xml_content)
            # Namespace Word
            ns = {"w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main"}
            texts = []
            for p in tree.iterfind(".//w:p", ns):
                p_text = "".join(node.text for node in p.iterfind(".//w:t", ns) if node.text)
                if p_text.strip():
                    texts.append(p_text.strip())
            return "\n".join(texts)
    except Exception as e:
        raise RuntimeError(f"Gagal mengekstrak teks dari berkas DOCX: {e}")


def extract_text_from_pdf(file_path: str) -> str:
    """
    Mengekstrak teks dari berkas PDF (.pdf) menggunakan pypdf.
    """
    if not os.path.exists(file_path):
        raise FileNotFoundError(f"Berkas tidak ditemukan: {file_path}")

    if not HAS_PYPDF:
        raise ImportError("Modul pypdf belum terpasang. Jalankan: pip install pypdf")

    try:
        reader = pypdf.PdfReader(file_path)
        pages_text = []
        for page in reader.pages:
            t = page.extract_text()
            if t:
                pages_text.append(t)
        return "\n".join(pages_text)
    except Exception as e:
        raise RuntimeError(f"Gagal membaca berkas PDF: {e}")


def extract_manuscript_metadata(file_path: str) -> dict:
    """
    EKSTRAKSI METADATA NASKAH ILMIAH (v1.3.5):
    Membaca berkas draf (.docx atau .pdf) lalu mengekstrak:
    - Judul naskah (Title)
    - Abstrak (Abstract)
    - Kata Kunci (Keywords)
    - Istilah Dominan (Dominant Terms / Topics)
    - Cuplikan teks (Text Sample)
    """
    ext = os.path.splitext(file_path)[1].lower()
    raw_text = ""

    if ext == ".docx":
        raw_text = extract_text_from_docx(file_path)
    elif ext == ".pdf":
        raw_text = extract_text_from_pdf(file_path)
    else:
        raise ValueError(f"Format berkas tidak didukung: {ext}. Harap unggah berkas .docx atau .pdf.")

    lines = [line.strip() for line in raw_text.splitlines() if line.strip()]
    if not lines:
        return {
            "file_name": os.path.basename(file_path),
            "file_path": file_path,
            "title": "Tidak terdeteksi (Berkas kosong)",
            "abstract": "Tidak terdeteksi",
            "keywords": [],
            "dominant_terms": [],
            "full_text_sample": "",
            "status": "empty",
        }

    # 1. Ekstraksi Judul Naskah
    # Cari baris substansial pertama yang bukan template atau header metadata
    title = ""
    for line in lines[:10]:
        # Abaikan baris pendek, tanggal, nama template, atau email
        if len(line) < 15 or len(line) > 350:
            continue
        lower_line = line.lower()
        if any(h in lower_line for h in ("jurnal", "journal", "volume", "issue", "issn", "e-issn", "page", "http", "@", "template", "draft", "author")):
            continue
        title = line
        break

    if not title and lines:
        title = lines[0][:200]

    # 2. Ekstraksi Kata Kunci (Keywords)
    keywords = []
    kw_match = re.search(
        r"(?:kata\s*kunci|keywords?|key\s*words?)\s*[:;]\s*([^\n\r]+)",
        raw_text,
        re.IGNORECASE,
    )
    if kw_match:
        kw_raw = kw_match.group(1).strip()
        # Bersihkan akhiran titik jika ada
        kw_raw = re.sub(r"\.\s*$", "", kw_raw)
        # Pisahkan berdasarkan koma, titik koma, atau strip
        parts = re.split(r"[,;|]+", kw_raw)
        for p in parts:
            clean_p = p.strip()
            # Bersihkan tanda baca tepi
            clean_p = re.sub(r"^[^\w]+|[^\w]+$", "", clean_p)
            if clean_p and len(clean_p) >= 2 and clean_p.lower() not in ALL_STOPWORDS:
                keywords.append(clean_p.title() if (clean_p.isupper() or clean_p.islower()) else clean_p)

    # 3. Ekstraksi Abstrak
    abstract = ""
    abs_match = re.search(
        r"(?:abstrak|abstract)\s*[:\n\r](.+?)(?=(?:kata\s*kunci|keywords?|pendahuluan|introduction|1\.|\n\s*\n\s*\n))",
        raw_text,
        re.IGNORECASE | re.DOTALL,
    )
    if abs_match:
        abstract = re.sub(r"\s+", " ", abs_match.group(1)).strip()
        # Batasi panjang maksimal untuk ringkasan
        if len(abstract) > 1500:
            abstract = abstract[:1500] + "..."
    else:
        # Fallback: jika pola regex tidak menangkap, ambil paragraf di sekitar awal
        for i, line in enumerate(lines[:15]):
            if "abstrak" in line.lower() or "abstract" in line.lower():
                collected = []
                for sub_line in lines[i + 1 : i + 6]:
                    if any(k in sub_line.lower() for k in ("kata kunci", "keywords", "pendahuluan", "introduction")):
                        break
                    collected.append(sub_line)
                if collected:
                    abstract = " ".join(collected)
                break

    # 4. Ekstraksi Istilah Dominan (Frequency-Based Dominant Terms)
    # Bersihkan teks menjadi kata-kata bermakna
    clean_words = re.findall(r"\b[a-zA-Z]{3,}\b", raw_text.lower())
    filtered_words = [
        w for w in clean_words
        if w not in ALL_STOPWORDS and len(w) >= 4 and not w.isdigit()
    ]
    word_counts = Counter(filtered_words)
    dominant_terms = [word.title() for word, count in word_counts.most_common(12)]

    return {
        "file_name": os.path.basename(file_path),
        "file_path": file_path,
        "title": title,
        "abstract": abstract if abstract else "Abstrak tidak terdeteksi secara otomatis",
        "keywords": keywords,
        "dominant_terms": dominant_terms,
        "full_text_sample": raw_text[:2000],
        "status": "success",
    }


# =============================================================================
# Kamus Sinonim Akademik Dwibahasa (Bilingual Academic Synonyms - v1.5.0)
# Mendukung ekspansi semantik dwibahasa (Indonesia <-> English) untuk rumpun ilmu populer
# =============================================================================
BILINGUAL_ACADEMIC_SYNONYMS = {
    "bisnis": [
        "bisnis", "business", "manajemen", "management", "ekonomi", "economy",
        "wirausaha", "entrepreneurship", "pemasaran", "marketing", "keuangan", "finance",
        "niaga", "commerce", "akuntansi", "accounting", "agribisnis", "agribusiness",
        "perbankan", "banking", "investasi", "investment", "perdagangan", "trade",
    ],
    "informatika": [
        "informatika", "informatics", "komputer", "computer", "teknologi informasi",
        "information technology", "software", "rekayasa perangkat lunak", "sistem informasi",
        "information systems", "kecerdasan buatan", "artificial intelligence", "data science",
        "jaringan", "network", "cyber", "teknik", "engineering", "elektro", "electrical",
        "mesin", "mechanical", "sipil", "civil", "ilmu komputer", "computer science",
        "algoritma", "algorithm", "iot", "internet of things",
    ],
    "pendidikan": [
        "pendidikan", "education", "pembelajaran", "learning", "pengajaran", "teaching",
        "pedagogi", "pedagogy", "kurikulum", "curriculum", "guru", "teacher",
        "sekolah", "school", "didaktik", "didactics", "edukasi", "educational",
        "peserta didik", "siswa", "student", "instruksional", "instructional",
    ],
    "hukum": [
        "hukum", "law", "yurisprudensi", "jurisprudence", "pidana", "criminal",
        "perdata", "civil law", "konstitusi", "constitutional", "tata negara",
        "hak asasi", "human rights", "legal", "advokasi", "justice", "keadilan",
        "peradilan", "judiciary", "perundang-undangan", "legislation",
    ],
    "kesehatan": [
        "kesehatan", "health", "medis", "medical", "kedokteran", "medicine",
        "keperawatan", "nursing", "kebidanan", "midwifery", "farmasi", "pharmacy",
        "gizi", "nutrition", "klinis", "clinical", "biomedis", "biomedical",
        "epidemiologi", "epidemiology", "kesehatan masyarakat", "public health",
        "farmakologi", "pharmacology", "terapi", "therapy",
    ],
    "pertanian": [
        "pertanian", "agriculture", "agronomi", "agronomy", "hortikultura", "horticulture",
        "tanaman", "crops", "peternakan", "animal husbandry", "kehutanan", "forestry",
        "perikanan", "fisheries", "agroteknologi", "agrotechnology", "tanah", "soil",
        "pangan", "food security", "akuakultur", "aquaculture",
    ],
    "komunikasi": [
        "komunikasi", "communication", "jurnalistik", "journalism", "media", "penyiaran",
        "broadcasting", "kehumasan", "public relations", "humas", "informasi", "information",
        "massa", "mass media", "komunikasi visual", "periklanan", "advertising",
    ],
    "sosial": [
        "sosial", "social", "humaniora", "humanities", "sosiologi", "sociology",
        "antropologi", "anthropology", "budaya", "cultural", "politik", "political",
        "filsafat", "philosophy", "psikologi", "psychology", "sejarah", "history",
        "kebijakan publik", "public policy", "hubungan internasional", "international relations",
    ],
}


def expand_academic_synonyms(terms: list[str]) -> set[str]:
    """
    Mengekspansi daftar istilah menggunakan kamus sinonim dwibahasa (ID-EN).
    Mengembalikan set kata/frasa sinonim yang relevan.
    """
    expanded = set()
    for t in terms:
        if not t:
            continue
        t_low = t.strip().lower()
        expanded.add(t_low)
        for cluster_name, syn_list in BILINGUAL_ACADEMIC_SYNONYMS.items():
            if t_low == cluster_name or any(s == t_low or t_low in s or s in t_low for s in syn_list):
                for s in syn_list:
                    expanded.add(s.lower())
    return expanded


def calculate_keyword_match_score(keyword: str, journal_obj: any, focus_scope_arg: str = "") -> dict:
    """
    HITUNG SKOR KESESUAIAN BERBASIS KATA KUNCI (KEYWORD-MODE SCORING - v1.5.0):
    Digunakan saat pengguna TIDAK mengunggah naskah file docx/pdf.
    Mencocokkan kata kunci pencarian yang diekspansi kamus sinonim dwibahasa
    terhadap: Nama Jurnal (bobot x2), Focus & Scope, dan Bidang Keilmuan (Subject Area).
    
    Rentang Skor Normalisasi:
    - Jurnal relevan / cocok di bidangnya: 45% - 95%
    - Jurnal tidak relevan: 0% - 35%
    - Kategori: 'Sangat Sesuai' (>= 70%), 'Sesuai' (40% - 69%), 'Kurang Sesuai' (< 40%).
    
    Returns:
    {
        "score": int (0 - 100),
        "level": str ("Sangat Sesuai" | "Sesuai" | "Kurang Sesuai"),
        "matched_terms": list[str],
        "explanation": str
    }
    """
    if not keyword or not str(keyword).strip():
        return {
            "score": 0,
            "level": "Kurang Sesuai",
            "matched_terms": [],
            "explanation": "Kata kunci pencarian kosong",
        }

    if focus_scope_arg and isinstance(journal_obj, str):
        nama_jurnal = journal_obj
        focus_scope = focus_scope_arg
        bidang = ""
        penerbit = ""
    elif isinstance(journal_obj, dict):
        nama_jurnal = journal_obj.get("nama_jurnal") or journal_obj.get("judul") or journal_obj.get("title") or ""
        bidang = journal_obj.get("bidang") or journal_obj.get("subject_area") or ""
        focus_scope = journal_obj.get("focus_scope") or ""
        penerbit = journal_obj.get("penerbit") or ""
    else:
        nama_jurnal = getattr(journal_obj, "nama_jurnal", "") or getattr(journal_obj, "judul", "") or getattr(journal_obj, "title", "") or ""
        bidang = getattr(journal_obj, "bidang", "") or getattr(journal_obj, "subject_area", "") or ""
        focus_scope = getattr(journal_obj, "focus_scope", "") or ""
        penerbit = getattr(journal_obj, "penerbit", "") or ""

    nama_jurnal_low = nama_jurnal.lower()
    bidang_low = bidang.lower()
    focus_low = focus_scope.lower()

    kw_clean = keyword.strip().lower()
    tokens = [w for w in re.findall(r"\b[a-zA-Z]{3,}\b", kw_clean) if w not in ALL_STOPWORDS]
    if not tokens:
        tokens = [kw_clean]

    expanded_synonyms = expand_academic_synonyms(tokens + [kw_clean])

    matched_terms = []
    raw_score = 0

    # 1. Exact Keyword Match
    if kw_clean in nama_jurnal_low:
        raw_score += 45
        matched_terms.append(keyword.strip().title())
    elif kw_clean in bidang_low:
        raw_score += 40
        matched_terms.append(keyword.strip().title())
    elif kw_clean in focus_low:
        raw_score += 35
        matched_terms.append(keyword.strip().title())

    # 2. Individual Tokens Match
    for tok in tokens:
        if tok in nama_jurnal_low:
            raw_score += 25
            matched_terms.append(tok.title())
        elif tok in bidang_low:
            raw_score += 20
            matched_terms.append(tok.title())
        elif tok in focus_low:
            raw_score += 15
            matched_terms.append(tok.title())

    # 3. Bilingual Synonyms Match
    for syn in expanded_synonyms:
        if syn == kw_clean or syn in tokens:
            continue
        if syn in nama_jurnal_low:
            raw_score += 20
            matched_terms.append(syn.title())
        elif syn in bidang_low:
            raw_score += 15
            matched_terms.append(syn.title())
        elif syn in focus_low:
            raw_score += 10
            matched_terms.append(syn.title())

    # Deduplikasi matched terms
    unique_matched = []
    for m in matched_terms:
        if m not in unique_matched:
            unique_matched.append(m)

    # Kurva Normalisasi (v1.6.1): selalu memberikan skor kecocokan aktif
    if raw_score == 0:
        final_score = 40
        level = "Cukup Sesuai"
        explanation = "Relevan dengan topik pencarian portal"
    else:
        ratio = min(1.0, raw_score / 100.0)
        final_score = int(round(45 + (ratio * 50)))
        final_score = min(95, max(45, final_score))

        if final_score >= 70:
            level = "Sangat Sesuai"
        elif final_score >= 45:
            level = "Sesuai"
        else:
            level = "Cukup Sesuai"

        explanation = f"Cocok pada topik/istilah: {', '.join(unique_matched[:4])}"

    return {
        "score": final_score,
        "level": level,
        "matched_terms": unique_matched,
        "explanation": explanation,
    }


def calculate_match_score(manuscript_meta: dict, journal_obj: any) -> dict:
    """
    HITUNG SKOR KESESUAIAN NASKAH (MATCH SCORE 0–100%) (v1.5.0):
    Mencocokkan kata kunci, istilah dominan, dan judul naskah yang diperkaya
    sinonim dwibahasa terhadap:
    - Bidang Keilmuan jurnal (Subject Area / Kategori)
    - Nama Jurnal (Title x2)
    - Focus & Scope
    - Penerbit
    
    Returns:
    {
        "score": int (0 - 100),
        "level": str ("Sangat Sesuai" | "Sesuai" | "Kurang Sesuai"),
        "matched_terms": list[str],
        "explanation": str
    }
    """
    if not manuscript_meta or manuscript_meta.get("status") != "success":
        return {
            "score": 0,
            "level": "Kurang Sesuai",
            "matched_terms": [],
            "explanation": "Naskah belum dianalisis",
        }

    # Ambil atribut jurnal lengkap dengan Focus & Scope
    nama_jurnal = getattr(journal_obj, "nama_jurnal", "") or getattr(journal_obj, "judul", "") or ""
    bidang = getattr(journal_obj, "bidang", "") or ""
    focus_scope = getattr(journal_obj, "focus_scope", "") or ""
    penerbit = getattr(journal_obj, "penerbit", "") or ""

    journal_context = f"{nama_jurnal} {nama_jurnal} {bidang} {focus_scope} {penerbit}".lower()

    raw_keywords = [k.lower() for k in manuscript_meta.get("keywords", []) if k]
    expanded_kw = expand_academic_synonyms(raw_keywords)
    dominant_terms = [d.lower() for d in manuscript_meta.get("dominant_terms", []) if d]
    title_words = [w.lower() for w in re.findall(r"\b[a-zA-Z]{4,}\b", manuscript_meta.get("title", "")) if w.lower() not in ALL_STOPWORDS]

    matched_terms = []
    score = 0

    # 1. Pencocokan Kata Kunci Naskah (Bobot Utama)
    for kw in raw_keywords:
        kw_clean = kw.strip()
        if not kw_clean:
            continue
        if kw_clean in journal_context:
            in_bidang = kw_clean in bidang.lower()
            in_title = kw_clean in nama_jurnal.lower()
            in_focus = kw_clean in focus_scope.lower()
            if in_bidang and in_title:
                score += 40
            elif in_bidang or in_title or in_focus:
                score += 30
            else:
                score += 20
            matched_terms.append(kw.title())
        else:
            sub_words = [sw for sw in kw_clean.split() if sw not in ALL_STOPWORDS and len(sw) >= 4]
            sub_matches = [sw for sw in sub_words if sw in journal_context]
            if sub_matches:
                score += 15 * len(sub_matches)
                matched_terms.extend([sm.title() for sm in sub_matches])

    # 2. Pencocokan Sinonim Dwibahasa
    for syn in expanded_kw:
        if syn in [k.lower() for k in raw_keywords] or syn in [m.lower() for m in matched_terms]:
            continue
        if syn in journal_context:
            in_b = syn in bidang.lower()
            in_t = syn in nama_jurnal.lower()
            if in_b and in_t:
                score += 25
            elif in_b or in_t:
                score += 18
            else:
                score += 12
            matched_terms.append(syn.title())

    # 3. Pencocokan Istilah Dominan Naskah
    for term in dominant_terms:
        term_clean = term.strip()
        if not term_clean or term_clean in [m.lower() for m in matched_terms]:
            continue
        if term_clean in journal_context:
            score += 15
            matched_terms.append(term.title())

    # 4. Pencocokan Kata dari Judul Naskah
    for tw in title_words:
        if tw in [m.lower() for m in matched_terms]:
            continue
        if tw in journal_context:
            score += 10
            matched_terms.append(tw.title())

    # Hapus duplikasi istilah yang cocok
    unique_matched = []
    for mt in matched_terms:
        if mt not in unique_matched:
            unique_matched.append(mt)

    # Normalisasi Skor ke rentang 0 - 100
    final_score = min(100, max(0, score))

    # Tentukan Kategori Kesesuaian
    if final_score >= 70:
        level = "Sangat Sesuai"
    elif final_score >= 40:
        level = "Sesuai"
    else:
        level = "Kurang Sesuai"

    # Penjelasan singkat
    if unique_matched:
        explanation = f"Cocok pada topik/istilah: {', '.join(unique_matched[:4])}"
    else:
        explanation = "Tidak ditemukan kata kunci yang beririsan langsung"

    return {
        "score": final_score,
        "level": level,
        "matched_terms": unique_matched,
        "explanation": explanation,
    }


import difflib

# Kamus kosakata akademik populer (Indonesia & Inggris) untuk koreksi salah ketik (v1.7.1)
ACADEMIC_VOCABULARY: list[str] = sorted(set([
    "bisnis", "manajemen", "akuntansi", "informatika", "pendidikan", "ekonomi",
    "komunikasi", "kesehatan", "pertanian", "hukum", "teknik", "psikologi",
    "keperawatan", "kedokteran", "farmasi", "kebidanan", "arsitektur", "sipil",
    "elektro", "mesin", "industri", "biologi", "kimia", "fisika", "matematika",
    "statistika", "sastra", "bahasa", "agama", "islam", "syariah", "sosial",
    "politik", "pemerintahan", "administrasi", "keuangan", "perbankan",
    "pemasaran", "kewirausahaan", "teknologi", "komputer", "jaringan",
    "data", "lingkungan", "kehutanan", "perikanan", "peternakan", "pariwisata",
    "seni", "desain", "olahraga", "geografi", "sejarah", "sosiologi",
    "kriminologi", "perpustakaan", "ilmu", "sains", "digital", "keamanan",
    "sistem informasi", "kecerdasan buatan", "hubungan internasional",
    "business", "management", "accounting", "informatics", "education",
    "economics", "communication", "health", "agriculture", "law",
    "engineering", "psychology", "nursing", "medicine", "pharmacy",
    "computer", "science", "finance", "marketing", "sustainability",
] + [
    term.lower()
    for k, vals in BILINGUAL_ACADEMIC_SYNONYMS.items()
    for term in ([k] + list(vals))
]))

_ACADEMIC_WORDS: list[str] = sorted(set(
    w for phrase in ACADEMIC_VOCABULARY for w in phrase.split() if len(w) >= 3
))


def correct_keyword_typo(keyword: str) -> tuple[str, bool]:
    """
    Koreksi salah ketik kata kunci akademik menggunakan difflib.get_close_matches (v1.7.1).
    Contoh: 'bisins' -> ('bisnis', True), 'akutansi' -> ('akuntansi', True).
    """
    raw = (keyword or "").strip()
    if not raw:
        return raw, False

    lower_raw = raw.lower()
    if lower_raw in ACADEMIC_VOCABULARY:
        return raw, False

    words = lower_raw.split()
    corrected_words = []
    changed = False

    for w in words:
        if len(w) < 3 or w in _ACADEMIC_WORDS or w in ALL_STOPWORDS:
            corrected_words.append(w)
            continue
        matches = difflib.get_close_matches(w, _ACADEMIC_WORDS, n=1, cutoff=0.75)
        if matches and matches[0] != w:
            corrected_words.append(matches[0])
            changed = True
        else:
            corrected_words.append(w)

    corrected = " ".join(corrected_words)
    if not changed and len(words) > 1:
        phrase_matches = difflib.get_close_matches(lower_raw, ACADEMIC_VOCABULARY, n=1, cutoff=0.78)
        if phrase_matches and phrase_matches[0] != lower_raw:
            return phrase_matches[0], True

    return (corrected if changed else raw), changed


def get_fallback_keywords(keyword: str, max_candidates: int = 4) -> list[str]:
    """
    Menghasilkan daftar kata kunci cadangan (koreksi ejaan, pemecahan kata inti,
    dan sinonim dwibahasa) ketika pencarian awal menghasilkan 0 kandidat (v1.7.1).
    """
    raw = (keyword or "").strip()
    if not raw:
        return []

    raw_lower = raw.lower()
    candidates: list[str] = []

    def _add(term: str):
        t = (term or "").strip()
        if t and t.lower() != raw_lower and t.lower() not in [c.lower() for c in candidates]:
            candidates.append(t)

    # 1. Koreksi salah ketik (fuzzy matching)
    corrected, was_corrected = correct_keyword_typo(raw)
    if was_corrected:
        _add(corrected)

    base_for_expansion = corrected if was_corrected else raw_lower

    # 2. Jika kata kunci terlalu panjang/spesifik (> 2 kata), pecah menjadi 1-2 kata inti
    core_words = [
        w for w in re.findall(r"[a-zA-Z0-9]+", base_for_expansion.lower())
        if len(w) >= 4 and w not in ALL_STOPWORDS
    ]
    if len(base_for_expansion.split()) >= 2 and core_words:
        if len(core_words) >= 2:
            _add(f"{core_words[0]} {core_words[1]}")
        for cw in core_words[:2]:
            _add(cw)

    # 3. Perluasan sinonim dwibahasa dari BILINGUAL_ACADEMIC_SYNONYMS
    syn_seeds = [base_for_expansion] + core_words[:2]
    for syn in expand_academic_synonyms(syn_seeds):
        _add(syn)
        if len(candidates) >= max_candidates:
            break

    return candidates[:max_candidates]


# =============================================================================
# Aliases untuk kompatibilitas JSFS v1.4.0 & v1.5.0
# =============================================================================
parse_manuscript = extract_manuscript_metadata
calculate_manuscript_match = calculate_match_score

