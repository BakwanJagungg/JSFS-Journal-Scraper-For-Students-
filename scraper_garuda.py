# =============================================================================
# JSFS - Journal Scraper For Students
# scraper_garuda.py — Modul scraping Portal Garuda (Kemdiktisaintek) v1.3.8
# =============================================================================

import os
import sys
import time
import random
import re
import json
import threading
import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry
import urllib3
from bs4 import BeautifulSoup, Comment
from dataclasses import dataclass
from typing import Optional
from concurrent.futures import ThreadPoolExecutor
from rich.console import Console

import warnings
import month_parser
import ai_engine

# Nonaktifkan peringatan SSL tidak aman untuk server kampus (.ac.id) dan parser warning
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)
warnings.filterwarnings("ignore", category=UserWarning)

# Pastikan console Windows mendukung output UTF-8
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass

from config import (
    GARUDA_JOURNAL_URL,
    HEADERS,
    REQUEST_DELAY_MIN,
    REQUEST_DELAY_MAX,
    MAX_PAGES_LIMIT,
    MAX_RESULTS,
    INFO_NOT_FOUND,
    DEAD_LINK_TEXT,
    USD_TO_IDR_RATE,
    APC_SANITY_MIN_IDR,
    APC_SANITY_MAX_IDR,
    CACHE_DIR,
    OJS_CACHE_FILE,
    CACHE_TTL_SECONDS,
    DEEP_SCRAPE_WORKERS,
    TURBO_SCRAPE_WORKERS,
    NORMAL_SCRAPE_WORKERS,
    HTTP_POOL_CONNECTIONS,
    HTTP_POOL_MAXSIZE,
    HTTP_TIMEOUT,
    DEEP_SCRAPE_TIMEOUT,
    FAST_FAIL_TIMEOUT,
)

console = Console()

# ── PRE-COMPILED REGEX PATTERNS (Hemat ribuan siklus CPU pada multi-threading) ─
RE_EMAIL = re.compile(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
RE_PHONE = re.compile(r"(?:\+62|62|0)(?:[2-9])[0-9\s-]{7,15}[0-9]")
RE_DATE_LIKE = re.compile(r"(?:\d{4}[-/]\d{1,2}[-/]\d{1,2}|\d{1,2}[-/]\d{1,2}[-/]\d{4})")

RE_FREE_APC = re.compile(
    r"(?:"
    r"\b(?:gratis|free\s*of\s*(?:charge|cost)|bebas\s*biaya|tidak\s*(?:dipungut|dikenakan)\s*biaya|tanpa\s*biaya|"
    r"bebas\s*biaya\s*halaman|penyerahan\s*naskah\s*:\s*0|no\s*fee|without\s*any\s*fee|"
    r"free\s*publication|free\s*publishing|free\s*submission|no\s*charge|apc\s*free|apc\s*:\s*0|"
    r"no\s*article\s*processing\s*charge|charges?\s*no\s*(?:author|publication)?\s*fees?)\b|"
    r"(?:(?:rp\.?|idr)\s*0(?:\.00)?(?:\s*\(?idr\)?)?)|"
    r"(?:0(?:\.00)?\s*\(?(?:idr|rp\.?|usd|\$)\)?)|"
    r"\b(?:apc|author\s*fees?|publication\s*fees?|submission\s*fees?|biaya\s*publikasi)\s*:\s*(?:0|free|none|gratis)"
    r")",
    re.IGNORECASE,
)

RE_USD_1 = re.compile(r"(?:\$|USD)\s*([0-9]+(?:[.,][0-9]{1,2})?)\b", re.IGNORECASE)
RE_USD_2 = re.compile(r"\b([0-9]+(?:[.,][0-9]{1,2})?)\s*(?:USD|\$)", re.IGNORECASE)

RE_IDR_1 = re.compile(
    r"(?:(?:Rp\.?|IDR)\s*([0-9]{1,3}(?:[.,][0-9]{3})+)|([0-9]{1,3}(?:[.,][0-9]{3})+)\s*(?:\(?IDR\)?|Rp\.?))",
    re.IGNORECASE,
)
RE_IDR_2 = re.compile(
    r"(?:(?:Rp\.?|IDR)\s*([0-9]{5,8})|([0-9]{5,8})\s*(?:\(?IDR\)?|Rp\.?))",
    re.IGNORECASE,
)

RE_FREQ_SLASH = re.compile(r"\b(\d+)\s*\/\s*(?:year|tahun)\b", re.IGNORECASE)
RE_FREQ_ANNUAL = re.compile(
    r"(\d+)\s*(?:issues?|nomor|edisi|kali|publikasi)?\s*(?:per\s*tahun|setiap\s*tahun|kali\s*setahun|a\s*year|per\s*year|\/\s*year|\/\s*tahun)",
    re.IGNORECASE,
)
RE_FREQ_STANDALONE = re.compile(r"^(\d+)\s*(?:issues?|nomor|edisi|kali|publikasi)?$", re.IGNORECASE)
RE_FREQ_TERBIT = re.compile(r"terbit\s*(\d+)\s*kali", re.IGNORECASE)

RE_ISSN = re.compile(r"ISSN\s*:\s*([0-9a-zA-Z-]+)", re.IGNORECASE)
RE_EISSN = re.compile(r"EISSN\s*:\s*([0-9a-zA-Z-]+)", re.IGNORECASE)
RE_ISSN_FORMAT = re.compile(r"\b([0-9]{4})-?([0-9]{3}[0-9X])\b", re.IGNORECASE)
RE_ARTICLE_OJS = re.compile(r"/article/(?:view|download)/.*$", re.IGNORECASE)

# Pemetaan Bulan Standar Indonesia & Kalender
MONTH_MAP = {
    "januari": "Januari", "january": "Januari", "jan": "Januari",
    "februari": "Februari", "february": "Februari", "feb": "Februari",
    "maret": "Maret", "march": "Maret", "mar": "Maret",
    "april": "April", "apr": "April",
    "mei": "Mei", "may": "Mei",
    "juni": "Juni", "june": "Juni", "jun": "Juni",
    "juli": "Juli", "july": "Juli", "jul": "Juli",
    "agustus": "Agustus", "august": "Agustus", "agu": "Agustus", "aug": "Agustus",
    "september": "September", "sept": "September", "sep": "September",
    "oktober": "Oktober", "october": "Oktober", "okt": "Oktober", "oct": "Oktober",
    "november": "November", "nov": "November",
    "desember": "Desember", "december": "Desember", "des": "Desember", "dec": "Desember",
}

ORDERED_MONTHS = [
    "Januari", "Februari", "Maret", "April", "Mei", "Juni",
    "Juli", "Agustus", "September", "Oktober", "November", "Desember"
]

RE_MONTH_TOKENS = re.compile(
    r"\b(januari|january|jan|februari|february|feb|maret|march|mar|april|apr|"
    r"mei|may|juni|june|jun|juli|july|jul|agustus|august|agu|aug|"
    r"september|sept|sep|oktober|october|okt|oct|november|nov|desember|december|des|dec)\b",
    re.IGNORECASE,
)

# Kamus Kata Kunci Komprehensif
APC_KEYWORDS = [
    "author fees", "publication fee", "biaya publikasi", "article processing charge",
    "processing fee", "submission fee", "page charge", "biaya cetak", "free of charge",
    "gratis", "bebas biaya", "0 idr", "usd", "no charge", "without any fee",
    "penyerahan naskah", "biaya halaman", "biaya pemrosesan", "apc", "biaya penulisan",
]

FREQ_PATTERNS = [
    re.compile(r"(\d+\s*(?:issues?|nomor|kali|edisi|publikasi)\s*(?:per|setiap|a|\/)?\s*(?:year|tahun)?)", re.I),
    re.compile(r"\b(semiannually|biannually|annually|quarterly|bimonthly|monthly|triannually)\b", re.I),
    re.compile(r"(terbit\s*\d+\s*kali\s*(?:setahun|dalam\s*setahun)?)", re.I),
    re.compile(r"(published\s*(?:twice|three\s*times|four\s*times)\s*a\s*year)", re.I),
]

# Thread lock untuk operasi cache file yang aman
_cache_lock = threading.Lock()


@dataclass
class JournalEntry:
    """Model data satu jurnal hasil scraping v1.4.0 (Hybrid AI Engine)."""
    nama_jurnal: str = ""
    penerbit: str = ""
    link_ojs: str = ""
    sinta_profile_url: str = ""
    garuda_profile_url: str = ""
    peringkat_sinta: str = INFO_NOT_FOUND
    issn: str = INFO_NOT_FOUND
    e_issn: str = INFO_NOT_FOUND
    bidang: str = INFO_NOT_FOUND
    apc_status: str = INFO_NOT_FOUND
    apc_amount: Optional[int] = None
    email: str = INFO_NOT_FOUND
    telepon: str = INFO_NOT_FOUND
    frekuensi: str = INFO_NOT_FOUND
    focus_scope: str = INFO_NOT_FOUND
    status_server: str = "Aktif (HTTP)"
    transparansi_kontak: str = "Tidak Transparan"
    sumber: str = "Garuda"
    kesesuaian_naskah: str = ""
    match_score: int = 0
    # Field v1.4.0
    raw_about: str = ""
    raw_submissions: str = ""
    bulan_terbit: list = None
    biaya_submit: Optional[int] = None
    biaya_publikasi: Optional[int] = None
    mata_uang: Optional[str] = None
    sumber_focus: str = ""
    sumber_apc: str = ""

    def __post_init__(self):
        if self.bulan_terbit is None:
            self.bulan_terbit = []

    @property
    def judul(self) -> str:
        return self.nama_jurnal

    @judul.setter
    def judul(self, val: str):
        self.nama_jurnal = val

    @property
    def nama(self) -> str:
        return self.nama_jurnal

    @nama.setter
    def nama(self, val: str):
        self.nama_jurnal = val

    @property
    def link(self) -> str:
        return self.link_ojs

    @link.setter
    def link(self, val: str):
        self.link_ojs = val

    @property
    def peringkat(self) -> str:
        return self.peringkat_sinta

    @peringkat.setter
    def peringkat(self, val: str):
        self.peringkat_sinta = val

    @property
    def link_sinta(self) -> str:
        return self.sinta_profile_url

    @link_sinta.setter
    def link_sinta(self, val: str):
        self.sinta_profile_url = val

    def __getitem__(self, key: str):
        if key == "sinta":
            m = re.search(r"(\d)", str(self.peringkat_sinta))
            return int(m.group(1)) if m else 0
        if key == "judul":
            return self.nama_jurnal
        if hasattr(self, key):
            return getattr(self, key)
        raise KeyError(key)

    def __setitem__(self, key: str, value):
        if key == "judul":
            self.nama_jurnal = value
        elif key == "sinta":
            self.peringkat_sinta = f"S{value}"
        else:
            setattr(self, key, value)

    def get(self, key: str, default=None):
        try:
            return self[key]
        except (KeyError, AttributeError):
            return default

    def update(self, **kwargs):
        for k, v in kwargs.items():
            self[k] = v

    def to_dict(self) -> dict:
        return {
            "nama_jurnal": self.nama_jurnal,
            "penerbit": self.penerbit,
            "link_ojs": self.link_ojs,
            "sinta_profile_url": self.sinta_profile_url,
            "garuda_profile_url": self.garuda_profile_url,
            "peringkat_sinta": self.peringkat_sinta,
            "issn": self.issn,
            "e_issn": self.e_issn,
            "bidang": self.bidang,
            "apc_status": self.apc_status,
            "apc_amount": self.apc_amount,
            "email": self.email,
            "telepon": self.telepon,
            "frekuensi": self.frekuensi,
            "focus_scope": self.focus_scope,
            "status_server": self.status_server,
            "transparansi_kontak": self.transparansi_kontak,
            "sumber": self.sumber,
            "kesesuaian_naskah": self.kesesuaian_naskah,
            "match_score": self.match_score,
            "raw_about": self.raw_about,
            "raw_submissions": self.raw_submissions,
            "bulan_terbit": self.bulan_terbit,
            "biaya_submit": self.biaya_submit,
            "biaya_publikasi": self.biaya_publikasi,
            "mata_uang": self.mata_uang,
            "sumber_focus": self.sumber_focus,
            "sumber_apc": self.sumber_apc,
        }



# ── HIGH-CAPACITY CONNECTION POOLING WITH RETRY (v1.3.6) ──────────────────────
def get_high_capacity_session(is_deep_scrape: bool = False) -> requests.Session:
    """
    Membuat session HTTP berperforma tinggi dengan connection pool besar (v1.3.6).
    Jika is_deep_scrape=True, menggunakan max_retries=1 tanpa backoff lambat
    agar tidak terjadi hanging sockets pada server OJS yang lambat / offline.
    """
    session = requests.Session()
    session.headers.update(HEADERS)
    if is_deep_scrape:
        retries = Retry(
            total=1,
            connect=1,
            read=0,
            redirect=2,
            status=0,
            raise_on_status=False,
        )
    else:
        retries = Retry(
            total=2,
            backoff_factor=0.3,
            status_forcelist=[429, 500, 502, 503, 504],
            raise_on_status=False,
        )
    adapter = HTTPAdapter(
        pool_connections=HTTP_POOL_CONNECTIONS,
        pool_maxsize=HTTP_POOL_MAXSIZE,
        max_retries=retries,
    )
    session.mount("http://", adapter)
    session.mount("https://", adapter)
    return session


# ── OJS ROOT DOMAIN REACHABILITY CIRCUIT BREAKER (v1.3.6) ─────────────────────
_domain_reachability_cache: dict[str, bool] = {}
_domain_reachability_lock = threading.Lock()


def check_ojs_domain_reachable(session: requests.Session, url: str) -> bool:
    """
    OJS Root Domain Reachability Check (2.5s Fast-Fail Circuit Breaker - v1.3.6):
    Memeriksa ketersediaan domain induk OJS menggunakan HEAD/GET cepat dengan timeout ketat
    (connect: 2.0s, read: 2.5s). Jika server domain offline / timeout / refused,
    seluruh request ke sub-halaman OJS tersebut langsung dilewati (skip).
    Hasil disimpan di cache thread-safe untuk mengeliminasi latensi domain yang sama.
    """
    if not url or not url.startswith("http"):
        return False

    if "kemdiktisaintek.go.id" in url:
        return True

    try:
        parsed = requests.compat.urlparse(url)
        netloc = (parsed.netloc or "").lower()
        if not netloc:
            return False

        with _domain_reachability_lock:
            if netloc in _domain_reachability_cache:
                return _domain_reachability_cache[netloc]

        root_url = f"{parsed.scheme}://{netloc}"
        is_alive = False
        try:
            # Uji coba cepat HEAD request
            r = session.head(root_url, timeout=FAST_FAIL_TIMEOUT, verify=False, allow_redirects=True)
            # Respon apapun (200, 301, 302, 403, 404, 405) menandakan server fisik hidup & merespon
            is_alive = True
        except requests.exceptions.RequestException:
            # Fallback stream GET jika server menolak method HEAD
            try:
                r = session.get(root_url, timeout=FAST_FAIL_TIMEOUT, verify=False, stream=True, allow_redirects=True)
                is_alive = True
                r.close()
            except Exception:
                is_alive = False

        with _domain_reachability_lock:
            _domain_reachability_cache[netloc] = is_alive

        return is_alive
    except Exception:
        return False


# ── OJS SERVER HEALTH & SSL STATUS (v1.3.8) ───────────────────────────────────
_domain_ssl_status_cache: dict[str, str] = {}
_domain_ssl_status_lock = threading.Lock()


def get_domain_ssl_health(session: requests.Session, url: str) -> str:
    """
    Indikator Kesehatan Server OJS (OJS Health & SSL Status - v1.3.8):
    Mengklasifikasikan server ke dalam:
    - Sangat Aktif (HTTPS/SSL Valid)
    - Aktif (HTTP)
    - SSL Warning (Bypass Valid)
    - Non-Aktif
    """
    if not url or not url.startswith("http") or url == DEAD_LINK_TEXT:
        return "Non-Aktif"

    if url.startswith("http://"):
        return "Aktif (HTTP)"

    parsed = requests.compat.urlparse(url)
    netloc = (parsed.netloc or "").lower()
    if not netloc:
        return "Non-Aktif"

    with _domain_ssl_status_lock:
        if netloc in _domain_ssl_status_cache:
            return _domain_ssl_status_cache[netloc]

    root_url = f"https://{netloc}"
    try:
        # Coba koneksi HTTPS cepat dengan SSL verification aktif
        r = requests.head(root_url, timeout=(1.5, 2.0), verify=True, headers={"User-Agent": HEADERS["User-Agent"]})
        status = "Sangat Aktif (HTTPS/SSL Valid)"
    except requests.exceptions.SSLError:
        status = "SSL Warning (Bypass Valid)"
    except Exception:
        # Jika timeout saat verify=True tapi session(verify=False) berhasil, anggap aktif
        status = "Sangat Aktif (HTTPS/SSL Valid)"

    with _domain_ssl_status_lock:
        _domain_ssl_status_cache[netloc] = status

    return status


def compute_contact_transparency(email: str, telepon: str) -> str:
    """
    Indikator Transparansi Kontak (Contact Quality Index - v1.3.8):
    - Lengkap (Email & WA/Telp)
    - Sedang (Hanya Email)
    - Terbatas (Hanya WA/Telp)
    - Tidak Transparan
    """
    has_email = bool(email and email not in (INFO_NOT_FOUND, DEAD_LINK_TEXT, "-", "None"))
    has_phone = bool(telepon and telepon not in (INFO_NOT_FOUND, DEAD_LINK_TEXT, "-", "None"))
    if has_email and has_phone:
        return "Lengkap (Email & WA/Telp)"
    elif has_email:
        return "Sedang (Hanya Email)"
    elif has_phone:
        return "Terbatas (Hanya WA/Telp)"
    else:
        return "Tidak Transparan"




# ── THREAD-SAFE ATOMIC CACHING (TTL 7 Hari) ──────────────────────────────────
def _ensure_cache_file():
    """Memastikan folder dan berkas cache JSON tersedia."""
    try:
        os.makedirs(CACHE_DIR, exist_ok=True)
        if not os.path.exists(OJS_CACHE_FILE):
            with open(OJS_CACHE_FILE, "w", encoding="utf-8") as f:
                json.dump({}, f)
    except Exception:
        pass


def _get_from_cache(url: str) -> Optional[dict]:
    """Mengambil data OJS dari cache jika belum kadaluwarsa (TTL 7 hari)."""
    if not url or not url.startswith("http"):
        return None
    _ensure_cache_file()
    try:
        with _cache_lock:
            if not os.path.exists(OJS_CACHE_FILE):
                return None
            with open(OJS_CACHE_FILE, "r", encoding="utf-8") as f:
                cache = json.load(f)
            entry = cache.get(url)
            if entry and isinstance(entry, dict):
                cached_time = entry.get("timestamp", 0)
                if time.time() - cached_time < CACHE_TTL_SECONDS:
                    data = entry.get("data")
                    if data and isinstance(data, dict):
                        # Jangan gunakan cache lama yang belum memiliki raw_about v1.4.0 (kecuali dead link)
                        if data.get("apc_status") != DEAD_LINK_TEXT and not data.get("raw_about"):
                            return None
                        # Jangan gunakan cache jika seluruh field masih bernilai INFO_NOT_FOUND (kegagalan masa lalu)
                        if not (
                            data.get("email") == INFO_NOT_FOUND
                            and data.get("telepon") == INFO_NOT_FOUND
                            and data.get("apc_status") == INFO_NOT_FOUND
                            and data.get("frekuensi") == INFO_NOT_FOUND
                        ):
                            return data
    except Exception:
        pass
    return None


def _save_to_cache(url: str, data: dict):
    """Menyimpan data OJS yang berhasil diekstrak ke dalam cache JSON."""
    if not url or not url.startswith("http") or not data:
        return
    # JANGAN simpan ke cache jika tidak ada satu pun informasi yang berhasil diekstrak
    # (misal situs timeout, Cloudflare 403, atau server mati) agar tidak mengunci kegagalan selama 7 hari
    if (
        data.get("email") == INFO_NOT_FOUND
        and data.get("telepon") == INFO_NOT_FOUND
        and data.get("apc_status") == INFO_NOT_FOUND
        and data.get("frekuensi") == INFO_NOT_FOUND
    ):
        return
    _ensure_cache_file()
    try:
        with _cache_lock:
            cache = {}
            if os.path.exists(OJS_CACHE_FILE):
                try:
                    with open(OJS_CACHE_FILE, "r", encoding="utf-8") as f:
                        cache = json.load(f)
                except Exception:
                    cache = {}
            cache[url] = {
                "timestamp": time.time(),
                "data": data,
            }
            with open(OJS_CACHE_FILE, "w", encoding="utf-8") as f:
                json.dump(cache, f, ensure_ascii=False, indent=2)
    except Exception:
        pass


# ── DOM SANITIZATION PIPELINE (Membersihkan derau script, style, & whitespace) ─
def sanitize_dom(soup: BeautifulSoup) -> str:
    """
    Membersihkan tag script, style, komentar, dan objek non-teks.
    Mempertahankan elemen footer & body tempat informasi kontak dan publisher sering berada.
    """
    for element in soup(["script", "style", "noscript", "svg", "iframe"]):
        element.decompose()
    for comment in soup.find_all(string=lambda text: isinstance(text, Comment)):
        comment.extract()
    raw_text = soup.get_text(" ", strip=True)
    clean_text = raw_text.replace("\xa0", " ").replace("&nbsp;", " ").replace("\u200b", " ")
    return re.sub(r"\s+", " ", clean_text).strip()


def sanitize_text(val: Optional[str]) -> str:
    """
    Standarisasi nilai teks:
    Mengubah null, None, -, N/A, Tidak Diketahui, atau string kosong menjadi
    'Informasi tidak ditemukan dalam situs jurnal'.
    """
    if val is None:
        return INFO_NOT_FOUND
    clean = str(val).strip()
    if clean.lower() in ("", "-", "–", "n/a", "na", "null", "none", "tidak diketahui", "unknown", "undefined"):
        return INFO_NOT_FOUND
    return clean


def normalize_issn(raw: Optional[str]) -> str:
    """
    VALIDASI RIGID ISSN & E-ISSN (v1.3.4):
    Validasi dan standarisasi nomor ISSN/E-ISSN ke format standar XXXX-XXXX (8 digit).
    Otomatis membuang nilai dummy seperti 0, 0000-0000, 1234-5678, none, null.
    """
    if not raw:
        return INFO_NOT_FOUND
    clean = str(raw).strip()
    if clean.lower() in ("", "-", "–", "0", "0000-0000", "1234-5678", "none", "null", "n/a", "tidak diketahui"):
        return INFO_NOT_FOUND

    m = RE_ISSN_FORMAT.search(clean)
    if m:
        part1 = m.group(1)
        part2 = m.group(2).upper()
        res = f"{part1}-{part2}"
        if res in ("0000-0000", "1234-5678"):
            return INFO_NOT_FOUND
        return res
    return INFO_NOT_FOUND


def clean_and_merge_subjects(*raw_sources: str) -> str:
    """
    EKSTRAKSI BIDANG KEILMUAN TERPADU (v1.3.4):
    Membersihkan dan menggabungkan subjek dari berbagai sumber (SINTA, Garuda, OJS meta tags).
    Memfilter stopwords, duplikasi, dan menstandarisasi ke Title Case.
    """
    stopwords = {
        "journal", "jurnal", "ojs", "open access", "pdf", "articles", "article",
        "peer-reviewed", "issn", "vol", "no", "indonesia", "indonesian", "home",
        "about", "view", "index", "current", "archives", "login", "register",
        "none", "null", "n/a", "na", "tidak diketahui", "unknown", "subject area"
    }
    seen = set()
    cleaned_list = []
    for src in raw_sources:
        if not src:
            continue
        clean_src = str(src).strip()
        if not clean_src or clean_src.lower() in ("none", "null", "n/a", "na", "tidak diketahui", "unknown", "-", "–", INFO_NOT_FOUND.lower()):
            continue
        parts = re.split(r"[,;|/\n\r]+", clean_src)
        for p in parts:
            p_clean = re.sub(r"^\W+|\W+$", "", p).strip()
            if not p_clean or len(p_clean) < 3:
                continue
            p_norm = p_clean.lower()
            if p_norm in stopwords or any(p_norm == sw for sw in stopwords):
                continue
            if p_norm in seen:
                continue
            seen.add(p_norm)
            if p_clean.isupper() or p_clean.islower():
                p_clean = p_clean.title()
            cleaned_list.append(p_clean)

    if not cleaned_list:
        return INFO_NOT_FOUND
    return ", ".join(cleaned_list[:6])


def _extract_months_from_context(text: str) -> list[str]:
    """Ekstraksi bulan terbit unik berurutan kalender dari konteks teks."""
    raw_found = RE_MONTH_TOKENS.findall(text.lower())
    seen = set()
    for tok in raw_found:
        std = MONTH_MAP.get(tok)
        if std:
            seen.add(std)
    return [m for m in ORDERED_MONTHS if m in seen]


def format_frequency(raw: str, context_text: str = "") -> str:
    """
    STANDARDISASI MUTLAK FREKUENSI TERBIT (v1.3.5):
    Eliminasi total seluruh istilah generik seperti (Semiannual), (Triwulanan), (Tahunan), (Bulanan).
    Format Output Baku:
    - Jika bulan terbit terdeteksi: 'X kali setahun (Bulan1 & Bulan2)' atau 'X kali setahun (Bulan1, Bulan2, ...)'
    - Jika bulan tidak terdeteksi: 'X kali setahun' (contoh: '2 kali setahun', '4 kali setahun')
    - Jika tidak ada data: 'Informasi tidak ditemukan dalam situs jurnal'
    """
    combined = ((raw or "") + " " + (context_text or "")).lower().strip()
    if not combined or combined in ("-", "–", "n/a", "none", "tidak diketahui"):
        return INFO_NOT_FOUND

    detected_count = None

    # Pemetaan istilah frekuensi internasional ke angka numerik
    if "semiannual" in combined or "biannual" in combined or "twice a year" in combined:
        detected_count = 2
    elif "triannual" in combined or "three times a year" in combined:
        detected_count = 3
    elif "quarterly" in combined or "four times a year" in combined:
        detected_count = 4
    elif "bimonthly" in combined or "six times a year" in combined:
        detected_count = 6
    elif "monthly" in combined or "twelve times a year" in combined:
        detected_count = 12
    elif "annual" in combined or "once a year" in combined:
        detected_count = 1

    if detected_count is None:
        m_t = RE_FREQ_TERBIT.search(combined)
        if m_t:
            detected_count = int(m_t.group(1))
        else:
            m_slash = RE_FREQ_SLASH.search(combined)
            if m_slash:
                detected_count = int(m_slash.group(1))
            else:
                m_annual = RE_FREQ_ANNUAL.search(combined)
                if m_annual:
                    detected_count = int(m_annual.group(1))
                else:
                    m_standalone = RE_FREQ_STANDALONE.match(combined)
                    if m_standalone:
                        detected_count = int(m_standalone.group(1))

    # Ekstraksi bulan terbit spesifik
    months = _extract_months_from_context(combined)
    if detected_count is None and months:
        detected_count = len(months)

    if detected_count and 1 <= detected_count <= 24:
        if months:
            if len(months) == 1:
                months_str = months[0]
            elif len(months) == 2:
                months_str = f"{months[0]} & {months[1]}"
            else:
                months_str = ", ".join(months[:-1]) + f" & {months[-1]}"
            res = f"{detected_count} kali setahun ({months_str})"
        else:
            res = f"{detected_count} kali setahun"
        # Eliminasi mutlak istilah generik dalam kurung atau standalone
        generic_scrub = re.compile(
            r"\s*\((?:Semiannual|Biannual|Triwulanan|Triannual|Dwi-bulanan|Bimonthly|Tahunan|Annual|Bulanan|Monthly)\)",
            re.IGNORECASE,
        )
        res = generic_scrub.sub("", res).strip()
        return res

    return INFO_NOT_FOUND


def normalize_frequency(raw: str, context_text: str = "") -> str:
    """Wrapper untuk format_frequency v1.3.5 (bebas istilah generik)."""
    return format_frequency(raw, context_text)


def parse_apc_from_context(context_text: str) -> tuple[str, Optional[int]]:
    """
    Ekstrak dan validasi biaya APC dari teks yang memiliki konteks author/publication fee.
    Returns: (display_str, numeric_idr_or_none)
    """
    if not context_text:
        return INFO_NOT_FOUND, None

    text = context_text.strip()

    # 1. Cek biaya berbayar USD terlebih dahulu (dengan konversi kurs tetap Rp 17.500 / USD)
    m_usd = RE_USD_1.search(text) or RE_USD_2.search(text)
    if m_usd:
        usd_candidate = m_usd.group(1) or (m_usd.group(2) if m_usd.lastindex and m_usd.lastindex >= 2 else "")
        if usd_candidate:
            num_str = usd_candidate.replace(",", ".")
            try:
                usd_val = float(num_str)
                idr_val = int(round(usd_val * USD_TO_IDR_RATE))
                if APC_SANITY_MIN_IDR <= idr_val <= APC_SANITY_MAX_IDR:
                    usd_repr = f"${int(usd_val)} USD" if usd_val.is_integer() else f"${usd_val:.2f} USD"
                    idr_repr = f"Rp {idr_val:,}".replace(",", ".")
                    return f"{idr_repr} ({usd_repr})", idr_val
            except ValueError:
                pass

    # 2. Cek biaya berbayar IDR / Rupiah
    m_idr = RE_IDR_1.search(text) or RE_IDR_2.search(text)
    if m_idr:
        raw_num = (m_idr.group(1) or m_idr.group(2) or "").replace(".", "").replace(",", "").strip()
        if raw_num:
            try:
                idr_val = int(raw_num)
                if APC_SANITY_MIN_IDR <= idr_val <= APC_SANITY_MAX_IDR:
                    idr_repr = f"Rp {idr_val:,}".replace(",", ".")
                    return idr_repr, idr_val
            except ValueError:
                pass

    # 3. Deteksi Gratis / Free / Bebas Biaya / Rp 0
    if RE_FREE_APC.search(text):
        return "Gratis (Rp 0)", 0

    return INFO_NOT_FOUND, None


def _extract_contacts_from_text(html_text: str, plain_text: str) -> tuple[list[str], list[str]]:
    """
    PENINGKATAN CAKUPAN TELEPON/WA & EMAIL REDAKSI (v1.3.5):
    Mengekstrak seluruh email dan telepon/WhatsApp unik dari teks HTML & polos.
    Penyaringan ketat terhadap format tanggal (YYYY-MM-DD), nomor ISSN 8-digit, derau tahun,
    dan email template/dummy OJS sintetis.
    """
    found_emails = []
    found_phones = []

    clean_html = html_text.replace("&#64;", "@").replace("&commat;", "@")

    # Regex Email pada HTML dan plain text
    raw_emails = RE_EMAIL.findall(clean_html) + RE_EMAIL.findall(plain_text)

    # Ekstraksi eksplisit dari href="mailto:..."
    for m in re.findall(r"mailto:([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})", clean_html, re.IGNORECASE):
        raw_emails.append(m)

    ignored_domains = ("example.com", "domain.com", "sfu.ca", "pkp.sfu.ca", "invalid", "email.com", "sample.com", "test.com", "yoursite.com")
    ignored_prefixes = (
        "user@", "admin@domain", "info@domain", "noreply@", "no-reply@",
        "firstauthor@", "secondauthor@", "thirdauthor@", "first@", "second@", "third@", "fourth@",
        "author@", "authors@", "support@", "john.doe@", "yourname@"
    )

    for em in raw_emails:
        clean_em = em.strip().lower()
        if any(clean_em.endswith(ext) for ext in (".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".css", ".js", ".pdf", ".zip")):
            continue
        if any(clean_em.endswith(f"@{d}") or clean_em.endswith(f".{d}") for d in ignored_domains):
            continue
        if any(clean_em.startswith(p) for p in ignored_prefixes):
            continue
        if any(d in clean_em for d in ("firstauthor", "secondauthor", "thirdauthor", "second@gmail", "third@gmail", "first@gmail", "author@", "pkp.sfu.ca")):
            continue
        if len(clean_em) <= 60 and clean_em not in found_emails:
            found_emails.append(clean_em)

    # Regex Telepon / WhatsApp dengan filter anti-tanggal, anti-tahun & anti-ISSN
    for ph in RE_PHONE.findall(plain_text):
        clean_ph = re.sub(r"\s+", " ", ph).strip()
        # Buang derau pola tanggal (contoh: 2024-05-12, 12-05-2024, 026 2026-04-07, 022-12-28-11050)
        if RE_DATE_LIKE.search(clean_ph):
            continue
        # Buang derau tahun (misal: 2020..2029) di awal/akhir atau setelah pemisah
        if re.search(r"(?:^|[\s\-_/])20[12]\d(?:[\s\-_/]|$)", clean_ph) or re.search(r"\b20[12]\d\b", clean_ph):
            continue
        # Buang pola derau spasi/tanda hubung bercampur tanggal
        if re.search(r"[-/]\d{1,2}[-/]", clean_ph):
            continue
        if clean_ph.count("-") > 2:
            continue
        if clean_ph.startswith("020-") or clean_ph.startswith("020 "):
            continue

        digits = re.sub(r"\D", "", clean_ph)
        # Buang jika panjang digit 8 (kemungkinan besar format ISSN seperti 2527-3027)
        if len(digits) == 8:
            continue
        # Validasi panjang nomor telepon valid Indonesia (10 s.d. 14 digit)
        if not (10 <= len(digits) <= 14):
            continue

        # Cek kode prefix valid Indonesia (+62, 62, 0)
        if clean_ph.startswith("+62") or clean_ph.startswith("62"):
            pref = digits[2:3] if len(digits) > 2 else ""
        elif clean_ph.startswith("0"):
            pref = digits[1:2] if len(digits) > 1 else ""
        else:
            continue

        if pref not in "2345789":
            continue

        # Buang nomor dengan karakter repetitif dummy (seperti 0800000000)
        if len(set(digits)) <= 2:
            continue

        if clean_ph not in found_phones:
            found_phones.append(clean_ph)

    return found_emails, found_phones



def _extract_meta_tags_info(soup: BeautifulSoup) -> dict:
    """
    HTML Meta-Tag Extraction Strategy (v1.3.4):
    Mengekstrak publisher, judul, ISSN/E-ISSN, email, dan subjek/kata kunci bidang ilmu.
    """
    meta_info = {
        "emails": [],
        "publisher": "",
        "title": "",
        "issn": "",
        "e_issn": "",
        "subjects": [],
    }
    for meta in soup.find_all("meta"):
        name = (meta.get("name") or meta.get("property") or "").lower()
        content = (meta.get("content") or "").strip()
        if not content:
            continue

        if name in ("dc.publisher", "citation_publisher") and not meta_info["publisher"]:
            meta_info["publisher"] = content

        if name in ("citation_journal_title", "dc.title") and not meta_info["title"]:
            meta_info["title"] = content

        if name in ("citation_issn", "dc.source.issn", "dc.identifier.issn", "prism.issn"):
            clean_issn = normalize_issn(content)
            if clean_issn != INFO_NOT_FOUND:
                if not meta_info["issn"]:
                    meta_info["issn"] = clean_issn
                elif not meta_info["e_issn"] and clean_issn != meta_info["issn"]:
                    meta_info["e_issn"] = clean_issn

        if name in ("citation_eissn", "prism.eissn"):
            clean_eissn = normalize_issn(content)
            if clean_eissn != INFO_NOT_FOUND:
                meta_info["e_issn"] = clean_eissn

        if name in ("dc.subject", "keywords", "citation_keywords", "dc.subject.classification"):
            meta_info["subjects"].append(content)

        if "email" in name or "@" in content:
            for em in RE_EMAIL.findall(content):
                clean_em = em.strip().lower()
                if any(clean_em.startswith(p) for p in ("firstauthor@", "secondauthor@", "thirdauthor@", "first@", "second@", "third@", "author@", "authors@", "user@", "admin@domain")):
                    continue
                if any(clean_em.endswith(f"@{d}") for d in ("example.com", "domain.com", "sfu.ca", "invalid", "email.com", "test.com")):
                    continue
                if any(d in clean_em for d in ("firstauthor", "secondauthor", "thirdauthor", "second@gmail", "third@gmail", "first@gmail", "pkp.sfu.ca")):
                    continue
                if clean_em not in meta_info["emails"]:
                    meta_info["emails"].append(clean_em)

    return meta_info


def normalize_focus_and_scope(text: str) -> str:
    """
    Normalisasi teks Focus & Scope (v1.3.8):
    Membersihkan tag noise, spasi berlebih, dan prefix 'Focus and Scope:',
    kemudian merangkum menjadi 2-3 kalimat informatif (maksimal 280-300 karakter).
    """
    if not text or len(text.strip()) < 15 or text == INFO_NOT_FOUND:
        return INFO_NOT_FOUND

    # Hapus tag HTML jika tersisa
    clean_txt = re.sub(r"<[^>]+>", " ", text)
    # Hapus prefix umum
    clean_txt = re.sub(
        r"^(?:focus\s*(?:and|&)\s*scope|aims\s*(?:and|&)\s*scope|fokus\s*dan\s*ruang\s*lingkup|ruang\s*lingkup|about\s*the\s*journal|tentang\s*jurnal)[:\s-]*",
        "",
        clean_txt,
        flags=re.IGNORECASE,
    ).strip()
    clean_txt = re.sub(r"\s+", " ", clean_txt).strip()

    # Pisahkan per kalimat (2 - 3 kalimat)
    sentences = re.split(r"(?<=[.!?])\s+", clean_txt)
    short_summary = " ".join(sentences[:3]).strip()
    if not short_summary:
        short_summary = clean_txt

    if len(short_summary) > 300:
        short_summary = short_summary[:297].rsplit(" ", 1)[0] + "..."

    return short_summary if len(short_summary) >= 15 else INFO_NOT_FOUND


def build_tier3_scope_fallback(entry: JournalEntry) -> str:
    """
    Tier 3 Fallback Data Mining Focus & Scope (v1.3.8):
    Sintesis ruang lingkup resmi berbasis profil SINTA (Subject Area) & Garuda.
    Menjamin 100% jurnal memiliki deskripsi Focus & Scope yang representatif.
    """
    sub = entry.bidang if (entry.bidang and entry.bidang != INFO_NOT_FOUND) else ""
    pub = entry.penerbit if (entry.penerbit and entry.penerbit != INFO_NOT_FOUND) else ""
    name = entry.nama_jurnal.strip()
    rank = entry.peringkat_sinta if (entry.peringkat_sinta and entry.peringkat_sinta != INFO_NOT_FOUND) else ""

    if sub and pub:
        raw_text = f"Memuat publikasi artikel penelitian dan kajian ilmiah dalam rumpun bidang {sub}. Dikelola dan diterbitkan secara berkala oleh {pub}."
    elif sub:
        raw_text = f"Fokus dan ruang lingkup mencakup publikasi artikel hasil penelitian dan telaah ilmiah dalam bidang keilmuan {sub}."
    elif pub:
        raw_text = f"Jurnal ilmiah berkala yang mempublikasikan hasil riset dan kajian akademis berkualitas, diterbitkan secara resmi oleh {pub}."
    elif name:
        if rank:
            raw_text = f"Jurnal ilmiah berkala terakreditasi {rank} yang memuat artikel hasil penelitian, kajian teoritis, dan aplikasi empiris dalam bidang terkait."
        else:
            raw_text = f"Jurnal ilmiah berkala yang mempublikasikan artikel hasil penelitian, telaah konseptual, dan kajian keilmuan bereputasi."
    else:
        raw_text = INFO_NOT_FOUND

    return normalize_focus_and_scope(raw_text)


def legacy_extract_focus(raw_text: str) -> str:
    """Ekstraksi Focus & Scope Legacy v1.3.8 berbasis regex dan heuristik OJS."""
    if not raw_text or not raw_text.strip():
        return ""
    text = re.sub(r"\s+", " ", raw_text).strip()
    if "<" in text and ">" in text:
        text = re.sub(r"<[^>]+>", " ", text)
    m = re.search(
        r"(?:focus\s*(?:and|&)\s*scope|aims\s*(?:and|&)\s*scope|fokus\s*dan\s*ruang\s*lingkup|ruang\s*lingkup)[:\s-]*(.{30,600})",
        text,
        re.I,
    )
    if m:
        candidate = m.group(1).strip()
        norm = normalize_focus_and_scope(candidate)
        if norm and norm != INFO_NOT_FOUND:
            return norm
    norm = normalize_focus_and_scope(text)
    return norm if norm != INFO_NOT_FOUND else ""


def legacy_extract_apc_dict(raw_text: str) -> dict:
    """
    Ekstraksi APC Legacy v1.3.8 dengan pemisahan Biaya Submit vs Publikasi.
    Returns: {"biaya_submit": int | None, "biaya_publikasi": int | None, "mata_uang": str | None}
    """
    empty = {"biaya_submit": None, "biaya_publikasi": None, "mata_uang": None}
    if not raw_text or not raw_text.strip():
        return empty
    text = raw_text.strip()

    biaya_submit = None
    biaya_pub = None
    mata_uang = None

    # Deteksi spesifik submission fee
    for sent in re.split(r"(?<=[.!?:;\n])\s+", text):
        sent_l = sent.lower()
        if any(k in sent_l for k in ("submission", "submit", "penyerahan naskah", "processing fee")):
            if any(k in sent_l for k in ("gratis", "free", "no charge", "0 idr", "rp 0", "tanpa biaya")):
                biaya_submit = 0
            else:
                m_usd = RE_USD_1.search(sent) or RE_USD_2.search(sent)
                if m_usd:
                    try:
                        v = float((m_usd.group(1) or m_usd.group(2)).replace(",", "."))
                        biaya_submit = int(v)
                        mata_uang = "USD"
                    except Exception:
                        pass
                m_idr = RE_IDR_1.search(sent) or RE_IDR_2.search(sent)
                if m_idr:
                    try:
                        digits = re.sub(r"[.,]", "", m_idr.group(1) or m_idr.group(2) or "")
                        biaya_submit = int(digits)
                        mata_uang = "IDR"
                    except Exception:
                        pass

    # Deteksi biaya publikasi via parse_apc_from_context legacy
    disp, num = parse_apc_from_context(text)
    if num is not None:
        biaya_pub = num
        if not mata_uang:
            mata_uang = "USD" if ("$" in disp or "USD" in disp) else "IDR"
    elif RE_FREE_APC.search(text):
        biaya_pub = 0
        if not mata_uang:
            mata_uang = "IDR"

    if biaya_submit is None and biaya_pub is None:
        return empty

    return {
        "biaya_submit": biaya_submit,
        "biaya_publikasi": biaya_pub,
        "mata_uang": mata_uang,
    }


# Daftarkan fungsi legacy ke ai_engine
ai_engine.register_legacy(focus_fn=legacy_extract_focus, apc_fn=legacy_extract_apc_dict)



def _extract_focus_and_scope_from_soup(soup: BeautifulSoup, is_homepage: bool = False) -> str:
    """
    Ekstraksi Fokus dan Ruang Lingkup Jurnal (Multi-Tier Mining Engine - v1.3.8):
    Tier 1: Meta-Tags & Targeted DOM Selectors / Headings
    Tier 2: OJS Homepage Context Parsing (Paragraf pembuka substantif)
    """
    if not soup:
        return INFO_NOT_FOUND

    scope_candidate = ""

    # ── TIER 1A: Meta-Tags (DC.Description, description, og:description) ─────
    for m in soup.find_all("meta"):
        name = (m.get("name") or m.get("property") or "").lower()
        content = (m.get("content") or "").strip()
        if name in ("dc.description", "description", "og:description", "dc.subject") and len(content) >= 30:
            if not any(ign in content.lower() for ign in ("open journal systems", "pkp.sfu.ca", "wordpress", "login to", "register")):
                scope_candidate = content
                break

    # ── TIER 1B: Targeted DOM Selectors ─────────────────────────────────────
    if not scope_candidate:
        target_selectors = [
            "#focusAndScope", "#focus_and_scope", ".focusAndScope", ".focus_and_scope",
            "#aimsAndScope", ".aimsAndScope", "div.focus-and-scope", "section.focus_and_scope",
            ".journal-description", "#journalDescription", "div.journal-about", "div.homepage-about",
            "#homepageAbout", "div#content div.sub_item", "div.page_about", "div.article-details"
        ]
        for sel in target_selectors:
            elem = soup.select_one(sel)
            if elem:
                p_tags = elem.find_all("p")
                if p_tags:
                    txt = " ".join(p.get_text(" ", strip=True) for p in p_tags[:3])
                else:
                    txt = elem.get_text(" ", strip=True)
                if len(txt) >= 30 and not any(ign in txt.lower() for ign in ("open journal systems", "pkp", "login to", "register", "editorial team")):
                    scope_candidate = txt
                    break

    # ── TIER 1C: Heading-based Lookup ────────────────────────────────────────
    if not scope_candidate:
        headings = soup.find_all(["h1", "h2", "h3", "h4", "h5", "strong", "b"])
        for h in headings:
            h_text = h.get_text(" ", strip=True).lower()
            if any(term in h_text for term in ("focus and scope", "fokus dan ruang lingkup", "aims and scope", "ruang lingkup", "focus & scope", "aims & scope", "scope of the journal", "about the journal")):
                collected_p = []
                curr = h.next_sibling
                while curr and len(collected_p) < 3:
                    name_attr = getattr(curr, "name", None)
                    if name_attr in ("p", "div", "ul", "ol"):
                        p_t = curr.get_text(" ", strip=True)
                        if len(p_t) > 20:
                            collected_p.append(p_t)
                    elif name_attr in ("h1", "h2", "h3", "h4"):
                        break
                    curr = curr.next_sibling
                if collected_p:
                    scope_candidate = " ".join(collected_p)
                    break
                parent_p = h.parent.find_all("p") if h.parent else []
                if len(parent_p) > 1:
                    scope_candidate = " ".join(p.get_text(" ", strip=True) for p in parent_p[1:3])
                    break

    # ── TIER 2: OJS Homepage Context Parsing ────────────────────────────────
    if not scope_candidate and is_homepage:
        main_containers = soup.select("#content, #main-content, div.page, div.pkp_structure_main, div.page_about, main, body")
        container = main_containers[0] if main_containers else soup
        for p in container.find_all("p"):
            p_text = p.get_text(" ", strip=True)
            if 35 <= len(p_text) <= 800:
                p_low = p_text.lower()
                # Pastikan memuat kata kunci substantif jurnal ilmiah
                if any(k in p_low for k in ("jurnal", "journal", "menerbitkan", "publishes", "mempublikasikan", "penelitian", "research", "bidang", "field", "artikel", "article", "studi", "study", "fokus", "focus", "ruang lingkup", "scope", "cakupan", "ilmiah", "terbit", "kajian")):
                    # Abaikan boilerplate login/PKP
                    if not any(ign in p_low for ign in ("login to", "remember me", "forgot your password", "open journal systems", "pkp.sfu.ca", "developed by")):
                        scope_candidate = p_text
                        break

    return normalize_focus_and_scope(scope_candidate)


def _extract_apc_contextual(soup: BeautifulSoup) -> tuple[str, Optional[int]]:
    """
    Targeted DOM & Meta-Tag Scoping untuk APC:
    Mencari pada blok terstruktur (<div id="authorFees">, <table>, dsb)
    sebelum memindai teks mentah.
    """
    # 1. Cek elemen ber-ID / class spesifik
    targeted_selectors = [
        "#authorFees", ".author_fees", "#submissionFees", "#fees",
        "#contact", "table.listing", "div.sub_item", "#content",
    ]
    for sel in targeted_selectors:
        for block in soup.select(sel):
            block_text = sanitize_dom(BeautifulSoup(str(block), "lxml"))
            if any(kw in block_text.lower() for kw in APC_KEYWORDS):
                disp, amount = parse_apc_from_context(block_text)
                if disp != INFO_NOT_FOUND:
                    return disp, amount

    # 2. Cek tag p, td, div pendek
    for tag in soup.find_all(["p", "div", "td", "li", "section", "article"]):
        tag_text = tag.get_text(" ", strip=True)
        tag_lower = tag_text.lower()
        if any(kw in tag_lower for kw in APC_KEYWORDS) and len(tag_text) <= 800:
            disp, amount = parse_apc_from_context(tag_text)
            if disp != INFO_NOT_FOUND:
                return disp, amount

    # 3. Fallback: scanning jendela teks
    full_plain = sanitize_dom(soup)
    full_lower = full_plain.lower()
    for kw in APC_KEYWORDS:
        idx = full_lower.find(kw)
        while idx != -1:
            start = max(0, idx - 100)
            end = min(len(full_plain), idx + 350)
            snip = full_plain[start:end]
            disp, amount = parse_apc_from_context(snip)
            if disp != INFO_NOT_FOUND:
                return disp, amount
            idx = full_lower.find(kw, idx + len(kw))

    return INFO_NOT_FOUND, None


def extract_ojs_deep_info(session: requests.Session, target_url: str, stop_event: Optional[threading.Event] = None) -> dict:
    """
    QUAD-PASS OJS CRAWLING (v1.3.8 Focus & Scope, Server Health & Speed Acceleration):
    Penelusuran sub-jalur OJS dengan fast-fail root reachability (2.5s),
    ekstraksi Focus & Scope, evaluasi Status Server OJS, dan Indeks Transparansi Kontak.
    """
    info = {
        "ojs_url": target_url,
        "email": INFO_NOT_FOUND,
        "telepon": INFO_NOT_FOUND,
        "frekuensi": INFO_NOT_FOUND,
        "apc_status": INFO_NOT_FOUND,
        "apc_amount": None,
        "publisher": "",
        "issn": INFO_NOT_FOUND,
        "e_issn": INFO_NOT_FOUND,
        "bidang": INFO_NOT_FOUND,
        "focus_scope": INFO_NOT_FOUND,
        "status_server": "Aktif (HTTP)",
        "transparansi_kontak": "Tidak Transparan",
        "raw_about": "",
        "raw_submissions": "",
        "bulan_terbit": [],
    }
    if not target_url or not target_url.startswith("http"):
        return info

    if stop_event and stop_event.is_set():
        return info

    cached = _get_from_cache(target_url)
    if cached:
        return cached

    # OJS Root Domain Reachability Check (2.5s Fast-Fail - v1.3.6)
    if "kemdiktisaintek.go.id" not in target_url:
        if not check_ojs_domain_reachable(session, target_url):
            info["ojs_url"] = target_url
            info["apc_status"] = DEAD_LINK_TEXT
            info["frekuensi"] = DEAD_LINK_TEXT
            info["email"] = DEAD_LINK_TEXT
            info["telepon"] = DEAD_LINK_TEXT
            info["status_server"] = "Non-Aktif"
            info["transparansi_kontak"] = "Tidak Transparan"
            _save_to_cache(target_url, info)
            return info

    all_emails: list[str] = []
    all_phones: list[str] = []
    raw_frequency: str = ""
    effective_ojs_url: str = target_url
    freq_contexts: list[str] = []
    raw_about_parts: list[str] = []
    raw_subs_parts: list[str] = []

    try:
        # Pass 1: Halaman Utama OJS / Detail Garuda (Timeout diperketat)
        resp = session.get(target_url, timeout=DEEP_SCRAPE_TIMEOUT, verify=False, allow_redirects=True)
        if resp.status_code == 200:
            html_text = resp.text
            soup = BeautifulSoup(html_text, "lxml")
            plain_text = sanitize_dom(soup)
            freq_contexts.append(plain_text[:2500])
            raw_about_parts.append(plain_text)

            # Jika ini halaman Garuda (/journal/view/{id}), ambil link OJS asli dari artikel
            if "garuda.kemdiktisaintek.go.id" in target_url:
                for a in soup.find_all("a", href=True):
                    h = a["href"].strip()
                    if "/article/view/" in h or "/article/download/" in h:
                        clean_ojs = RE_ARTICLE_OJS.sub("", h)
                        if clean_ojs.startswith("http"):
                            effective_ojs_url = clean_ojs
                            info["ojs_url"] = clean_ojs
                            break
                    if a.get_text(strip=True).lower() in ("website", "original source") and h.startswith("http"):
                        effective_ojs_url = h
                        info["ojs_url"] = h
                        break

            # Evaluasi Status Server OJS
            info["status_server"] = get_domain_ssl_health(session, effective_ojs_url or target_url)

            # Ekstraksi Focus & Scope Primary Pass (Tier 1 & Tier 2)
            info["focus_scope"] = _extract_focus_and_scope_from_soup(soup, is_homepage=True)

            # Meta-tag extraction
            meta_data = _extract_meta_tags_info(soup)
            for em in meta_data["emails"]:
                if em not in all_emails:
                    all_emails.append(em)
            if meta_data["publisher"]:
                info["publisher"] = meta_data["publisher"]
            if meta_data["issn"]:
                info["issn"] = meta_data["issn"]
            if meta_data["e_issn"]:
                info["e_issn"] = meta_data["e_issn"]
            if meta_data["subjects"]:
                info["bidang"] = clean_and_merge_subjects(info["bidang"], ", ".join(meta_data["subjects"]))

            # Kontak Primary Pass
            e1, p1 = _extract_contacts_from_text(html_text, plain_text)
            for em in e1:
                if em not in all_emails:
                    all_emails.append(em)
            for ph in p1:
                if ph not in all_phones:
                    all_phones.append(ph)

            # Frekuensi Primary Pass
            for pat in FREQ_PATTERNS:
                m_f = pat.search(plain_text)
                if m_f:
                    raw_frequency = m_f.group(0).strip()
                    break

            # APC Primary Pass
            apc_disp, apc_num = _extract_apc_contextual(soup)
            if apc_disp != INFO_NOT_FOUND:
                info["apc_status"] = apc_disp
                info["apc_amount"] = apc_num

            # QUAD-PASS CRAWLING & DYNAMIC LINK HARVESTING (v1.3.8):
            need_more = (
                (len(all_emails) == 0)
                or (info["apc_status"] == INFO_NOT_FOUND)
                or (not raw_frequency)
                or (info["focus_scope"] == INFO_NOT_FOUND)
                or (len(raw_about_parts) < 2)
            )
            if need_more and effective_ojs_url.startswith("http") and not (stop_event and stop_event.is_set()):
                # Fast-Fail Check untuk effective_ojs_url jika baru terungkap dari Garuda
                if effective_ojs_url != target_url and "kemdiktisaintek.go.id" not in effective_ojs_url:
                    if not check_ojs_domain_reachable(session, effective_ojs_url):
                        info["ojs_url"] = effective_ojs_url
                        info["apc_status"] = DEAD_LINK_TEXT
                        info["frekuensi"] = DEAD_LINK_TEXT
                        info["email"] = DEAD_LINK_TEXT
                        info["telepon"] = DEAD_LINK_TEXT
                        info["status_server"] = "Non-Aktif"
                        _save_to_cache(target_url, info)
                        _save_to_cache(effective_ojs_url, info)
                        return info

                clean_base = effective_ojs_url.rstrip("/")
                if clean_base.endswith("/index"):
                    clean_base = clean_base[:-6]

                # Koleksi link dinamis dari homepage untuk APC, Kontak, dan Focus & Scope
                discovered_fee_links = []
                discovered_contact_links = []
                discovered_scope_links = []
                fee_terms = ("author fee", "publication fee", "biaya publikasi", "article processing charge", "processing fee", "apc", "author fees", "biaya")
                contact_terms = ("contact", "kontak", "hubungi", "editorial team", "editorial board", "dewan redaksi", "pengelola")
                scope_terms = ("focus and scope", "focus & scope", "fokus dan ruang lingkup", "ruang lingkup", "aims and scope", "aims & scope", "focus", "scope")

                for a_tag in soup.find_all("a", href=True):
                    h_val = a_tag["href"].strip()
                    txt_val = a_tag.get_text(" ", strip=True).lower()
                    h_low = h_val.lower()

                    if any(h_low.endswith(ext) for ext in (".pdf", ".doc", ".docx", ".jpg", ".png", ".zip", ".rar", ".xml")):
                        continue

                    # Dynamic Focus & Scope links
                    if any(k in txt_val for k in scope_terms) or any(k in h_low for k in ("focus", "scope", "editorialpolicies")):
                        full_h = requests.compat.urljoin(clean_base + "/", h_val)
                        if full_h.startswith("http") and full_h not in discovered_scope_links and "kemdiktisaintek" not in full_h:
                            discovered_scope_links.append(full_h)

                    # Dynamic Fee / APC links
                    if any(k in txt_val for k in fee_terms) or any(k in h_low for k in ("/af", "/apc", "author-fee", "author_fee", "authorfees", "authorcharge", "articleprocessingcharges", "publication-fee")):
                        full_h = requests.compat.urljoin(clean_base + "/", h_val)
                        if full_h.startswith("http") and full_h not in discovered_fee_links and "kemdiktisaintek" not in full_h:
                            discovered_fee_links.append(full_h)

                    # Dynamic Contact links
                    if any(k in txt_val for k in contact_terms) or any(k in h_low for k in ("contact", "kontak", "editorialteam", "editorial-team", "editorial-board", "people")):
                        full_h = requests.compat.urljoin(clean_base + "/", h_val)
                        if full_h.startswith("http") and full_h not in discovered_contact_links and "kemdiktisaintek" not in full_h:
                            discovered_contact_links.append(full_h)

                # Prioritas rute penelusuran OJS terakselerasi
                subpaths = []
                if info["focus_scope"] == INFO_NOT_FOUND and discovered_scope_links:
                    subpaths.extend(discovered_scope_links[:2])
                if info["apc_status"] == INFO_NOT_FOUND and discovered_fee_links:
                    subpaths.extend(discovered_fee_links[:2])
                if not all_emails and discovered_contact_links:
                    subpaths.extend(discovered_contact_links[:2])

                subpaths.extend([
                    f"{clean_base}/about/editorialPolicies#focusAndScope",
                    f"{clean_base}/about/focusAndScope",
                    f"{clean_base}/about",
                    f"{clean_base}/about/submissions",
                    f"{clean_base}/af",
                    f"{clean_base}/about/contact",
                    f"{clean_base}/about/editorialTeam",
                ])

                seen_sub = set()
                ordered_subpaths = []
                for sp in subpaths:
                    if sp not in seen_sub and sp != effective_ojs_url:
                        seen_sub.add(sp)
                        ordered_subpaths.append(sp)

                for sub_url in ordered_subpaths[:4]:
                    if stop_event and stop_event.is_set():
                        break
                    try:
                        sub_resp = session.get(sub_url, timeout=DEEP_SCRAPE_TIMEOUT, verify=False, allow_redirects=True)
                        if sub_resp.status_code == 200:
                            sub_soup = BeautifulSoup(sub_resp.text, "lxml")
                            sub_plain = sanitize_dom(sub_soup)
                            freq_contexts.append(sub_plain[:2000])

                            sub_url_low = sub_url.lower()
                            if any(k in sub_url_low for k in ("submission", "author-fee", "author_fee", "/af", "/apc", "fee")):
                                raw_subs_parts.append(sub_plain)
                            if any(k in sub_url_low for k in ("about", "focus", "scope", "editorialpolicies")):
                                raw_about_parts.append(sub_plain)

                            # Focus & Scope dari subpath
                            if info["focus_scope"] == INFO_NOT_FOUND:
                                s_scope = _extract_focus_and_scope_from_soup(sub_soup)
                                if s_scope != INFO_NOT_FOUND:
                                    info["focus_scope"] = s_scope

                            # Jika di halaman submissions ditemukan link khusus fee (seperti /af), inspeksi langsung
                            if info["apc_status"] == INFO_NOT_FOUND and "submissions" in sub_url:
                                for s_a in sub_soup.find_all("a", href=True):
                                    s_txt = s_a.get_text(" ", strip=True).lower()
                                    s_h = s_a["href"].strip()
                                    if any(k in s_txt for k in fee_terms) or any(k in s_h.lower() for k in ("/af", "/apc", "author-fee", "author_fee", "authorfees")):
                                        target_fee_url = requests.compat.urljoin(clean_base + "/", s_h)
                                        if target_fee_url != sub_url and target_fee_url.startswith("http"):
                                            try:
                                                fee_r = session.get(target_fee_url, timeout=DEEP_SCRAPE_TIMEOUT, verify=False, allow_redirects=True)
                                                if fee_r.status_code == 200:
                                                    f_soup = BeautifulSoup(fee_r.text, "lxml")
                                                    raw_subs_parts.append(sanitize_dom(f_soup))
                                                    f_apc, f_num = _extract_apc_contextual(f_soup)
                                                    if f_apc != INFO_NOT_FOUND:
                                                        info["apc_status"] = f_apc
                                                        info["apc_amount"] = f_num
                                            except Exception:
                                                pass

                            # Ekstraksi meta
                            sub_meta = _extract_meta_tags_info(sub_soup)
                            for em in sub_meta["emails"]:
                                if em not in all_emails:
                                    all_emails.append(em)
                            if sub_meta["issn"] and info["issn"] == INFO_NOT_FOUND:
                                info["issn"] = sub_meta["issn"]
                            if sub_meta["e_issn"] and info["e_issn"] == INFO_NOT_FOUND:
                                info["e_issn"] = sub_meta["e_issn"]
                            if sub_meta["subjects"]:
                                info["bidang"] = clean_and_merge_subjects(info["bidang"], ", ".join(sub_meta["subjects"]))

                            # Kontak
                            sub_e, sub_p = _extract_contacts_from_text(sub_resp.text, sub_plain)
                            for e in sub_e:
                                if e not in all_emails:
                                    all_emails.append(e)
                            for p in sub_p:
                                if p not in all_phones:
                                    all_phones.append(p)

                            # APC
                            if info["apc_status"] == INFO_NOT_FOUND:
                                s_apc, s_num = _extract_apc_contextual(sub_soup)
                                if s_apc != INFO_NOT_FOUND:
                                    info["apc_status"] = s_apc
                                    info["apc_amount"] = s_num

                            # Frekuensi
                            if not raw_frequency:
                                for pat in FREQ_PATTERNS:
                                    m_f = pat.search(sub_plain)
                                    if m_f:
                                        raw_frequency = m_f.group(0).strip()
                                        break
                    except Exception:
                        pass
        else:
            # Respon bukan 200 (mis: 404, 500, dsb.)
            if "kemdiktisaintek.go.id" not in target_url:
                info["apc_status"] = DEAD_LINK_TEXT
                info["status_server"] = "Non-Aktif"
    except Exception:
        # Request error / connection timeout pada Pass 1
        if "kemdiktisaintek.go.id" not in target_url:
            info["apc_status"] = DEAD_LINK_TEXT
            info["status_server"] = "Non-Aktif"
            parsed = requests.compat.urlparse(target_url)
            netloc = (parsed.netloc or "").lower()
            if netloc:
                with _domain_reachability_lock:
                    _domain_reachability_cache[netloc] = False

    info["email"] = ", ".join(all_emails) if all_emails else INFO_NOT_FOUND
    info["telepon"] = ", ".join(all_phones) if all_phones else INFO_NOT_FOUND
    info["frekuensi"] = format_frequency(raw_frequency, " ".join(freq_contexts))

    combined_about = " ".join(raw_about_parts).strip()
    combined_subs = " ".join(raw_subs_parts).strip() or combined_about
    info["raw_about"] = combined_about
    info["raw_submissions"] = combined_subs
    info["bulan_terbit"] = month_parser.parse_bulan_terbit(combined_about)

    if info["apc_status"] == DEAD_LINK_TEXT:
        info["status_server"] = "Non-Aktif"
    elif info["status_server"] == "Non-Aktif":
        info["status_server"] = get_domain_ssl_health(session, effective_ojs_url or target_url)

    info["transparansi_kontak"] = compute_contact_transparency(info["email"], info["telepon"])

    # Simpan ke cache lokal
    _save_to_cache(target_url, info)
    if effective_ojs_url != target_url:
        _save_to_cache(effective_ojs_url, info)

    return info


def merge_cross_source_entries(entries: list[JournalEntry]) -> list[JournalEntry]:
    """
    PENGGABUNGAN DATA LINTAS SUMBER (CROSS-SOURCE FALLBACK MERGING - v1.3.4):
    Jika jurnal muncul di kedua portal (Garuda dan SINTA), gabungkan field-field
    yang saling melengkapi sehingga data tidak kosong / bernilai default:
    - Penerbit, ISSN, E-ISSN, Bidang dari Garuda/SINTA.
    - Peringkat SINTA dan URL Profil dari SINTA.
    - Link OJS, APC, Email, Telepon, dan Frekuensi dari sumber terlengkap.
    """
    merged_map: dict[str, JournalEntry] = {}
    for j in entries:
        key = j.nama_jurnal.lower().strip()
        if not key:
            continue
        if key not in merged_map:
            # Standarisasi nilai awal
            j.issn = normalize_issn(j.issn)
            j.e_issn = normalize_issn(j.e_issn)
            j.bidang = clean_and_merge_subjects(j.bidang)
            merged_map[key] = j
        else:
            existing = merged_map[key]
            # Merge field penerbit
            if existing.penerbit == INFO_NOT_FOUND and j.penerbit != INFO_NOT_FOUND:
                existing.penerbit = j.penerbit

            # Merge ISSN & E-ISSN terstandarisasi
            clean_j_issn = normalize_issn(j.issn)
            clean_j_eissn = normalize_issn(j.e_issn)
            if existing.issn == INFO_NOT_FOUND and clean_j_issn != INFO_NOT_FOUND:
                existing.issn = clean_j_issn
            if existing.e_issn == INFO_NOT_FOUND and clean_j_eissn != INFO_NOT_FOUND:
                existing.e_issn = clean_j_eissn

            # Merge Bidang Keilmuan
            existing.bidang = clean_and_merge_subjects(existing.bidang, j.bidang)

            # Peringkat SINTA
            if existing.peringkat_sinta == INFO_NOT_FOUND and j.peringkat_sinta != INFO_NOT_FOUND:
                existing.peringkat_sinta = j.peringkat_sinta
            if not existing.sinta_profile_url and j.sinta_profile_url:
                existing.sinta_profile_url = j.sinta_profile_url

            # Link OJS Asli
            if (not existing.link_ojs or "kemdiktisaintek" in existing.link_ojs) and (j.link_ojs and "kemdiktisaintek" not in j.link_ojs):
                existing.link_ojs = j.link_ojs

            # APC
            if existing.apc_status == INFO_NOT_FOUND and j.apc_status != INFO_NOT_FOUND:
                existing.apc_status = j.apc_status
                existing.apc_amount = j.apc_amount

            # Email Redaksi
            if existing.email == INFO_NOT_FOUND and j.email != INFO_NOT_FOUND:
                existing.email = j.email
            elif j.email != INFO_NOT_FOUND and existing.email != INFO_NOT_FOUND:
                em_list = [e.strip() for e in existing.email.split(",") if e.strip()]
                for em in [e.strip() for e in j.email.split(",") if e.strip()]:
                    if em not in em_list:
                        em_list.append(em)
                existing.email = ", ".join(em_list)

            # Telepon / WA Redaksi
            if existing.telepon == INFO_NOT_FOUND and j.telepon != INFO_NOT_FOUND:
                existing.telepon = j.telepon
            elif j.telepon != INFO_NOT_FOUND and existing.telepon != INFO_NOT_FOUND:
                ph_list = [p.strip() for p in existing.telepon.split(",") if p.strip()]
                for ph in [p.strip() for p in j.telepon.split(",") if p.strip()]:
                    if ph not in ph_list:
                        ph_list.append(ph)
                existing.telepon = ", ".join(ph_list)

            # Frekuensi Terbit (Prioritaskan format terlengkap yang memiliki bulan terbit)
            if existing.frekuensi == INFO_NOT_FOUND and j.frekuensi != INFO_NOT_FOUND:
                existing.frekuensi = j.frekuensi
            elif "(" in j.frekuensi and "(" not in existing.frekuensi and j.frekuensi != INFO_NOT_FOUND:
                existing.frekuensi = j.frekuensi

            # Focus & Scope (v1.3.8)
            if (existing.focus_scope == INFO_NOT_FOUND or not existing.focus_scope) and j.focus_scope != INFO_NOT_FOUND:
                existing.focus_scope = j.focus_scope

            # Status Server OJS (v1.3.8)
            if existing.status_server in ("Non-Aktif", INFO_NOT_FOUND) and j.status_server not in ("Non-Aktif", INFO_NOT_FOUND):
                existing.status_server = j.status_server

            # Transparansi Kontak (v1.3.8)
            existing.transparansi_kontak = compute_contact_transparency(existing.email, existing.telepon)

            # Kesesuaian Naskah & Match Score (v1.3.5)
            if j.match_score > existing.match_score:
                existing.match_score = j.match_score
                existing.kesesuaian_naskah = j.kesesuaian_naskah

            # Catat sumber
            if j.sumber and j.sumber not in existing.sumber:
                existing.sumber = f"{existing.sumber} + {j.sumber}"

    # Pastikan status server, transparansi kontak, dan Focus & Scope (Tier 3 fallback) terhitung untuk seluruh entri
    for entry in merged_map.values():
        if not entry.transparansi_kontak or entry.transparansi_kontak == "Tidak Transparan":
            entry.transparansi_kontak = compute_contact_transparency(entry.email, entry.telepon)
        if not entry.focus_scope or entry.focus_scope == INFO_NOT_FOUND:
            entry.focus_scope = build_tier3_scope_fallback(entry)

    return list(merged_map.values())


def parse_search_keywords(raw_keyword: str) -> list[str]:
    """
    PARSER KATA KUNCI ADAPTIF MULTI-TOPIK (v1.3.5):
    Memecah kombinasi keyword yang dipisahkan koma atau titik koma (mis: 'bisnis, ekonomi, commerce').
    Menghilangkan duplikasi dan spasi ekstra.
    """
    raw = (raw_keyword or "").strip()
    if not raw:
        return []
    parts = [p.strip() for p in re.split(r"[,;|]+", raw) if p.strip()]
    seen = set()
    unique_parts = []
    for p in parts:
        low = p.lower()
        if low not in seen:
            seen.add(low)
            unique_parts.append(p)
    return unique_parts if unique_parts else [raw]


def resolve_garuda_profile_meta(session: requests.Session, detail_url: str) -> dict:
    """
    Mengambil URL OJS website resmi dan deskripsi Focus & Scope langsung dari halaman detail Garuda (v1.3.8).
    """
    res = {
        "ojs_url": detail_url,
        "focus_scope": INFO_NOT_FOUND,
        "publisher": "",
        "subject": "",
    }
    if not detail_url or not detail_url.startswith("http") or "garuda.kemdiktisaintek.go.id/journal/view" not in detail_url:
        return res
    try:
        r = session.get(detail_url, timeout=HTTP_TIMEOUT, verify=False, allow_redirects=True)
        if r.status_code == 200:
            soup = BeautifulSoup(r.text, "lxml")
            for a in soup.find_all("a", href=True):
                txt = a.get_text(strip=True).lower()
                h = a["href"].strip()
                if h.startswith("//"):
                    h = f"https:{h}"
                if txt in ("home page", "website", "editor url", "portal", "ojs") and "kemdiktisaintek" not in h and h.startswith("http"):
                    res["ojs_url"] = h
                    break

            desc_el = soup.select_one("div.j-meta-desc")
            if desc_el:
                d_txt = desc_el.get_text(" ", strip=True)
                if len(d_txt) >= 20:
                    res["focus_scope"] = normalize_focus_and_scope(d_txt)

            sub_el = soup.select_one("div.j-meta-subject")
            if sub_el:
                res["subject"] = sub_el.get_text(" ", strip=True)

            pub_el = soup.select_one("div.j-meta-pub")
            if pub_el:
                res["publisher"] = pub_el.get_text(" ", strip=True)
    except Exception:
        pass
    return res


def resolve_garuda_ojs_url(session: requests.Session, detail_url: str) -> str:
    """
    Mengambil URL OJS website resmi langsung dari halaman detail Garuda (Home Page / Website).
    """
    return resolve_garuda_profile_meta(session, detail_url).get("ojs_url", detail_url)


def cross_reference_garuda_with_sinta(session: requests.Session, entry: JournalEntry) -> None:
    """
    PENYETARAAN KUALITAS PORTAL GARUDA DENGAN SINTA (v1.3.5):
    Melakukan cross-referencing otomatis data hasil Garuda dengan direktori SINTA
    menggunakan ISSN, E-ISSN, atau Judul Jurnal untuk memperoleh:
    - Peringkat akreditasi SINTA resmi (S1–S6)
    - URL Profil SINTA resmi
    - Metadata Bidang Keilmuan SINTA (Subject Area)
    - URL OJS resmi terverifikasi
    """
    from config import SINTA_JOURNAL_URL

    query_candidates = []
    # 1. Coba pencarian SINTA via E-ISSN (tanpa tanda hubung)
    if entry.e_issn and entry.e_issn != INFO_NOT_FOUND:
        clean_e = re.sub(r"\D", "", entry.e_issn)
        if len(clean_e) == 8:
            query_candidates.append(clean_e)

    # 2. Coba via P-ISSN (tanpa tanda hubung)
    if entry.issn and entry.issn != INFO_NOT_FOUND:
        clean_p = re.sub(r"\D", "", entry.issn)
        if len(clean_p) == 8 and clean_p not in query_candidates:
            query_candidates.append(clean_p)

    # 3. Coba via Judul Singkat / Akronim / Kata Kunci Judul
    clean_title = re.sub(r"[\(\)\[\]:;\-\/]", " ", entry.nama_jurnal).strip()
    words = [w for w in clean_title.split() if len(w) >= 3 and w.lower() not in ("jurnal", "journal", "indonesian", "the", "dan", "of")]
    if words:
        short_title = " ".join(words[:4])
        if short_title not in query_candidates:
            query_candidates.append(short_title)

    for q in query_candidates:
        try:
            resp = session.get(SINTA_JOURNAL_URL, params={"q": q, "page": 1}, timeout=HTTP_TIMEOUT, verify=False)
            if resp.status_code == 200:
                soup = BeautifulSoup(resp.text, "lxml")
                items = soup.select("div.list-item")
                if not items:
                    continue

                for item in items:
                    t_el = item.select_one("div.affil-name a") or item.select_one(".affil-name")
                    if not t_el:
                        continue
                    s_title = t_el.get_text(" ", strip=True)

                    p_id = item.select_one("div.profile-id")
                    p_text = p_id.get_text(" ", strip=True) if p_id else ""

                    is_match = False
                    if q.isdigit() and len(q) == 8:
                        if q in p_text.replace("-", "").replace(" ", ""):
                            is_match = True
                    else:
                        w_entry = set(re.findall(r"\w{3,}", entry.nama_jurnal.lower()))
                        w_sinta = set(re.findall(r"\w{3,}", s_title.lower()))
                        overlap = w_entry & w_sinta
                        if len(overlap) >= min(2, len(w_entry)):
                            is_match = True

                    if is_match:
                        rank_elem = item.select_one("span.num-stat.accredited") or item.select_one(".accredited")
                        if rank_elem:
                            m_rank = re.search(r"S(\d)", rank_elem.get_text())
                            if m_rank:
                                entry.peringkat_sinta = f"Sinta {m_rank.group(1)}"

                        s_href = t_el.get("href", "").strip() if t_el.name == "a" else ""
                        if s_href.startswith("/"):
                            entry.sinta_profile_url = f"https://sinta.kemdiktisaintek.go.id{s_href}"
                        elif s_href.startswith("http"):
                            entry.sinta_profile_url = s_href

                        m_sub = re.search(r"Subject Area\s*:\s*([^|]+)", p_text, re.I)
                        if m_sub:
                            entry.bidang = clean_and_merge_subjects(entry.bidang, m_sub.group(1))

                        m_pi = re.search(r"P-ISSN\s*:\s*([0-9a-zA-Z-]+)", p_text, re.I)
                        if m_pi and entry.issn == INFO_NOT_FOUND:
                            entry.issn = normalize_issn(m_pi.group(1))
                        m_ei = re.search(r"E-ISSN\s*:\s*([0-9a-zA-Z-]+)", p_text, re.I)
                        if m_ei and entry.e_issn == INFO_NOT_FOUND:
                            entry.e_issn = normalize_issn(m_ei.group(1))

                        if "garuda.kemdiktisaintek.go.id" in entry.link_ojs or not entry.link_ojs.startswith("http"):
                            for a_tag in item.select("a[href]"):
                                txt_a = a_tag.get_text(strip=True).lower()
                                href_a = a_tag.get("href", "").strip()
                                if any(t in txt_a for t in ("website", "editor url", "ojs", "portal")) and "kemdiktisaintek" not in href_a and href_a.startswith("http"):
                                    entry.link_ojs = href_a
                                    break
                        return
        except Exception:
            pass


def _parse_garuda_rows(soup: BeautifulSoup, session: requests.Session, deep_detail: bool = False) -> list[JournalEntry]:
    """Parse tabel baris jurnal dari halaman pencarian Garuda dengan penyetaraan SINTA."""
    journals: list[JournalEntry] = []
    rows = soup.select("table.ui.table tr") or soup.select("table tr")

    for row in rows:
        title_el = row.select_one("a.title-journal")
        if not title_el:
            continue

        nama_jurnal = title_el.get_text(" ", strip=True)
        if not nama_jurnal:
            continue

        rel_href = title_el.get("href", "")
        if rel_href.startswith("/"):
            detail_url = f"https://garuda.kemdiktisaintek.go.id{rel_href}"
        elif rel_href.startswith("http"):
            detail_url = rel_href
        else:
            detail_url = f"https://garuda.kemdiktisaintek.go.id/{rel_href}"

        # Penerbit / Institusi
        sub_el = row.select_one("a.subtitle-journal")
        penerbit = sub_el.get_text(" ", strip=True) if sub_el else ""

        # ISSN & E-ISSN
        issn, e_issn = INFO_NOT_FOUND, INFO_NOT_FOUND
        for a_sub in row.select("a.subtitle-journal"):
            txt = a_sub.get_text(" ", strip=True)
            m_issn = RE_ISSN.search(txt)
            if m_issn and m_issn.group(1).strip() not in ("-", ""):
                issn = m_issn.group(1).strip()
            m_eissn = RE_EISSN.search(txt)
            if m_eissn and m_eissn.group(1).strip() not in ("-", ""):
                e_issn = m_eissn.group(1).strip()

        # Bidang Keilmuan
        areas = [a.get_text(" ", strip=True) for a in row.select("a.label-journal")]
        bidang = ", ".join(areas) if areas else INFO_NOT_FOUND

        entry = JournalEntry(
            nama_jurnal=sanitize_text(nama_jurnal),
            penerbit=sanitize_text(penerbit),
            link_ojs=detail_url,
            garuda_profile_url=detail_url if "garuda.kemdiktisaintek.go.id/journal/view" in detail_url else "",
            sinta_profile_url="",
            peringkat_sinta=INFO_NOT_FOUND,
            issn=normalize_issn(issn),
            e_issn=normalize_issn(e_issn),
            bidang=clean_and_merge_subjects(bidang),
            apc_status=INFO_NOT_FOUND,
            apc_amount=None,
            email=INFO_NOT_FOUND,
            telepon=INFO_NOT_FOUND,
            frekuensi=INFO_NOT_FOUND,
            sumber="Garuda",
        )
        journals.append(entry)

    # PENYETARAAN MUTLAK GARUDA KE SINTA (v1.3.5):
    # Resolusi link OJS resmi dan pencocokan peringkat SINTA resmi secara paralel
    if journals:
        try:
            with ThreadPoolExecutor(max_workers=min(len(journals), 16)) as executor:
                def _enhance_garuda_entry(e: JournalEntry):
                    target_g_url = e.garuda_profile_url or (e.link_ojs if "garuda.kemdiktisaintek.go.id/journal/view" in (e.link_ojs or "") else "")
                    if target_g_url:
                        g_meta = resolve_garuda_profile_meta(session, target_g_url)
                        if g_meta.get("ojs_url") and "garuda.kemdiktisaintek.go.id" not in g_meta["ojs_url"]:
                            e.link_ojs = g_meta["ojs_url"]
                        if g_meta.get("focus_scope") and (not e.focus_scope or e.focus_scope == INFO_NOT_FOUND):
                            e.focus_scope = g_meta["focus_scope"]
                        if g_meta.get("subject") and (not e.bidang or e.bidang == INFO_NOT_FOUND):
                            e.bidang = g_meta["subject"]
                        if g_meta.get("publisher") and (not e.penerbit or e.penerbit == INFO_NOT_FOUND):
                            e.penerbit = g_meta["publisher"]
                    cross_reference_garuda_with_sinta(session, e)
                futures = [executor.submit(_enhance_garuda_entry, entry) for entry in journals]
                for f in futures:
                    f.result()
        except Exception:
            pass

    return journals


def _scrape_garuda_single_keyword(
    keyword: str,
    sinta_ranks: Optional[list[int]] = None,
    deep_detail: bool = False,
    max_results: int = MAX_RESULTS,
    turbo_mode: bool = True,
    session: Optional[requests.Session] = None,
    free_only: bool = False,
    include_unknown_apc: bool = True,
) -> list[JournalEntry]:
    """Scraping satu kata kunci pada Portal Garuda dengan paginasi dinamis dan short-circuiting (v1.3.8)."""
    if session is None:
        session = get_high_capacity_session()
    all_journals: list[JournalEntry] = []
    active_workers = TURBO_SCRAPE_WORKERS if turbo_mode else NORMAL_SCRAPE_WORKERS

    console.print(f"\n[bold cyan]🔍 Memulai scraping Portal Garuda (v1.3.8)...[/bold cyan]")
    console.print(f"[dim]   Keyword      : {keyword}[/dim]")
    console.print(f"[dim]   Filter SINTA : Sinta {sinta_ranks if sinta_ranks else 'semua'}[/dim]")
    console.print(f"[dim]   Target Hasil : {max_results} jurnal | Mode Detail OJS: {'Aktif (Quad-Pass)' if deep_detail else 'Mati (Mode Cepat)'}[/dim]")
    console.print(f"[dim]   Hardware Concurrency: {active_workers} workers {'(Turbo Mode)' if turbo_mode else ''}[/dim]")

    max_pages = min(MAX_PAGES_LIMIT, max(10, (max_results // 5) + 5))

    for page in range(1, max_pages + 1):
        params = {"q": keyword, "page": page}
        url = GARUDA_JOURNAL_URL
        console.print(f"[yellow]📄 Garuda Halaman {page}/{max_pages}[/yellow] → {url}?q={keyword}&page={page}")

        try:
            response = session.get(url, params=params, timeout=20, verify=False)

            if response.status_code in (403, 503) or any(
                cf_word in response.text.lower()
                for cf_word in ("just a moment", "cf-browser-verification", "turnstile", "challenge-running")
            ):
                console.print(
                    "[bold red]⚠️  Garuda memblokir request (Cloudflare Challenge Detected).[/bold red]"
                )
                break

            response.raise_for_status()

        except requests.exceptions.RequestException as e:
            console.print(f"[red]✗ Koneksi Garuda gagal: {e}[/red]")
            break

        soup = BeautifulSoup(response.text, "lxml")
        page_journals = _parse_garuda_rows(soup, session, deep_detail=False)
        if not page_journals:
            console.print(f"[dim]   Tidak ada data lagi di halaman {page}, pencarian Garuda selesai.[/dim]")
            break

        console.print(f"[green]   ✓ Ditemukan {len(page_journals)} jurnal di halaman {page}[/green]")
        all_journals.extend(page_journals)

        if len(all_journals) >= max_results:
            all_journals = all_journals[:max_results]
            console.print(f"[dim]   Target {max_results} hasil Garuda tercapai.[/dim]")
            break

        delay = random.uniform(REQUEST_DELAY_MIN, REQUEST_DELAY_MAX)
        time.sleep(delay)

    # Filter peringkat SINTA jika diminta
    if sinta_ranks and all_journals:
        rank_labels = [f"S{r}" for r in sinta_ranks] + [f"Sinta {r}" for r in sinta_ranks] + [str(r) for r in sinta_ranks]
        filtered = [
            j for j in all_journals
            if any(r in j.peringkat_sinta for r in rank_labels)
            or j.peringkat_sinta == INFO_NOT_FOUND
        ]
    else:
        filtered = all_journals

    if len(filtered) > max_results:
        filtered = filtered[:max_results]

    console.print(f"\n[bold]📊 Hasil Garuda: {len(filtered)} jurnal dihimpun[/bold]")

    # Jalankan deep detail pada filtered entries dengan Target Reach Short-Circuiting (v1.3.6)
    if filtered and deep_detail:
        console.print(f"[bold cyan]🔍 Menjalankan Quad-Pass Deep Scraping Garuda ({len(filtered)} jurnal) [{active_workers} workers]...[/bold cyan]")
        stop_event = threading.Event()
        matched_lock = threading.Lock()
        matched_count = 0
        deep_session = get_high_capacity_session(is_deep_scrape=True)

        try:
            with ThreadPoolExecutor(max_workers=active_workers) as executor:
                def _do_garuda_deep(entry: JournalEntry):
                    nonlocal matched_count
                    if stop_event.is_set():
                        return
                    res = extract_ojs_deep_info(deep_session, entry.link_ojs, stop_event=stop_event)
                    entry.link_ojs = res.get("ojs_url", entry.link_ojs)
                    entry.email = res.get("email", INFO_NOT_FOUND)
                    entry.telepon = res.get("telepon", INFO_NOT_FOUND)
                    entry.frekuensi = res.get("frekuensi", INFO_NOT_FOUND)
                    entry.apc_status = res.get("apc_status", INFO_NOT_FOUND)
                    entry.apc_amount = res.get("apc_amount", None)
                    entry.focus_scope = res.get("focus_scope", INFO_NOT_FOUND)
                    if not entry.focus_scope or entry.focus_scope == INFO_NOT_FOUND:
                        entry.focus_scope = build_tier3_scope_fallback(entry)
                    entry.status_server = res.get("status_server", "Aktif (HTTP)")
                    entry.transparansi_kontak = res.get("transparansi_kontak", "Tidak Transparan")
                    if entry.issn == INFO_NOT_FOUND and res.get("issn") and res.get("issn") != INFO_NOT_FOUND:
                        entry.issn = res.get("issn")
                    if entry.e_issn == INFO_NOT_FOUND and res.get("e_issn") and res.get("e_issn") != INFO_NOT_FOUND:
                        entry.e_issn = res.get("e_issn")
                    if res.get("bidang") and res.get("bidang") != INFO_NOT_FOUND:
                        entry.bidang = clean_and_merge_subjects(entry.bidang, res.get("bidang"))
                    if not entry.penerbit and res.get("publisher"):
                        entry.penerbit = res.get("publisher")
                    entry.raw_about = res.get("raw_about", "")
                    entry.raw_submissions = res.get("raw_submissions", "")
                    entry.bulan_terbit = res.get("bulan_terbit", [])

                    with matched_lock:
                        if free_only:
                            apc_low = entry.apc_status.lower()
                            is_free = (entry.apc_amount == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("free" in apc_low)
                            is_unk = (entry.apc_amount is None) or (entry.apc_status == INFO_NOT_FOUND)
                            if is_free or (is_unk and include_unknown_apc):
                                matched_count += 1
                        else:
                            matched_count += 1
                        if matched_count >= max_results:
                            stop_event.set()

                futures = [executor.submit(_do_garuda_deep, entry) for entry in filtered]
                for f in futures:
                    f.result()
        except Exception:
            pass

    return filtered


def scrape_garuda(
    keyword: str,
    sinta_ranks: Optional[list[int]] = None,
    deep_detail: bool = False,
    max_results: int = MAX_RESULTS,
    turbo_mode: bool = True,
    free_only: bool = False,
    include_unknown_apc: bool = True,
) -> list[JournalEntry]:
    """
    Scraping Portal Garuda dengan dukungan multi-keyword adaptif dan short-circuiting (v1.3.6).
    """
    keywords = parse_search_keywords(keyword)
    session = get_high_capacity_session()

    if len(keywords) > 1:
        console.print(f"\n[bold cyan]🔍 Deteksi multi-keyword adaptif ({len(keywords)} topik): {', '.join(keywords)}[/bold cyan]")
        all_res: list[JournalEntry] = []
        buffer_ratio = 1.35
        target_per_kw = max(20, int((max_results * buffer_ratio) / len(keywords)) + 15)
        for kw in keywords:
            current_unique = len(merge_cross_source_entries(all_res))
            if current_unique >= max_results:
                break
            remaining_needed = max_results - current_unique
            fetch_limit = min(max_results, max(target_per_kw, remaining_needed + 15))

            kw_journals = _scrape_garuda_single_keyword(
                kw,
                sinta_ranks=sinta_ranks,
                deep_detail=deep_detail,
                max_results=fetch_limit,
                turbo_mode=turbo_mode,
                session=session,
                free_only=free_only,
                include_unknown_apc=include_unknown_apc,
            )
            all_res.extend(kw_journals)
            all_res = merge_cross_source_entries(all_res)
            if len(all_res) >= max_results:
                break
        merged = merge_cross_source_entries(all_res)
        return merged[:max_results]

    return _scrape_garuda_single_keyword(
        keyword,
        sinta_ranks=sinta_ranks,
        deep_detail=deep_detail,
        max_results=max_results,
        turbo_mode=turbo_mode,
        session=session,
        free_only=free_only,
        include_unknown_apc=include_unknown_apc,
    )

