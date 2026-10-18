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


def calculate_match_score(manuscript_meta: dict, journal_obj: any) -> dict:
    """
    HITUNG SKOR KESESUAIAN NASKAH (MATCH SCORE 0–100%) (v1.3.5):
    Mencocokkan kata kunci, istilah dominan, dan judul naskah terhadap:
    - Bidang Keilmuan jurnal (Subject Area / Kategori)
    - Nama Jurnal (Title)
    - Penerbit / Ruang Lingkup
    
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

    # Ambil atribut jurnal
    nama_jurnal = getattr(journal_obj, "nama_jurnal", "") or ""
    bidang = getattr(journal_obj, "bidang", "") or ""
    penerbit = getattr(journal_obj, "penerbit", "") or ""

    journal_context = f"{nama_jurnal} {bidang} {penerbit}".lower()

    keywords = [k.lower() for k in manuscript_meta.get("keywords", []) if k]
    dominant_terms = [d.lower() for d in manuscript_meta.get("dominant_terms", []) if d]
    title_words = [w.lower() for w in re.findall(r"\b[a-zA-Z]{4,}\b", manuscript_meta.get("title", "")) if w.lower() not in ALL_STOPWORDS]

    matched_terms = []
    score = 0

    # 1. Pencocokan Kata Kunci Naskah (Bobot Utama)
    for kw in keywords:
        kw_clean = kw.strip()
        if not kw_clean:
            continue
        # Cek frasa utuh
        if kw_clean in journal_context:
            # Jika cocok di bidang keilmuan maupun nama jurnal
            in_bidang = kw_clean in bidang.lower()
            in_title = kw_clean in nama_jurnal.lower()
            if in_bidang and in_title:
                score += 40
            elif in_bidang or in_title:
                score += 30
            else:
                score += 20
            matched_terms.append(kw.title())
        else:
            # Cek pecahan kata dalam multi-word keyword
            sub_words = [sw for sw in kw_clean.split() if sw not in ALL_STOPWORDS and len(sw) >= 4]
            sub_matches = [sw for sw in sub_words if sw in journal_context]
            if sub_matches:
                score += 15 * len(sub_matches)
                matched_terms.extend([sm.title() for sm in sub_matches])

    # 2. Pencocokan Istilah Dominan Naskah
    for term in dominant_terms:
        term_clean = term.strip()
        if not term_clean or term_clean in [m.lower() for m in matched_terms]:
            continue
        if term_clean in journal_context:
            score += 15
            matched_terms.append(term.title())

    # 3. Pencocokan Kata dari Judul Naskah
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


# =============================================================================
# Aliases untuk kompatibilitas JSFS v1.4.0
# =============================================================================
parse_manuscript = extract_manuscript_metadata
calculate_manuscript_match = calculate_match_score

