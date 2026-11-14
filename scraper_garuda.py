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
try:
    from selectolax.lexbor import LexborHTMLParser as SelectolaxHTMLParser
    HAS_SELECTOLAX = True
except ImportError:
    SelectolaxHTMLParser = None
    HAS_SELECTOLAX = False
from dataclasses import dataclass
from typing import Optional
from concurrent.futures import ThreadPoolExecutor, as_completed
from rich.console import Console

import warnings
import month_parser
import ai_engine
import cache_engine

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
    get_browser_headers,
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
    MAX_DEEP_SCRAPE_PER_JOURNAL_SEC,
    compute_candidate_pool_limit,
)

console = Console()

# ── PRE-COMPILED REGEX PATTERNS (Hemat ribuan siklus CPU pada multi-threading) ─
RE_EMAIL = re.compile(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
# RE_PHONE v1.6.2: Menangkap nomor seluler Indonesia dan telepon kabel berpemisah/kode area (tanpa rekening/NIP)
RE_PHONE = re.compile(
    r"(?:"
    r"(?:\+62[\s.-]*|62[\s.-]*|0)8[1-9][0-9\s.-]{6,11}[0-9]|"
    r"(?:\+62\s*|62\s*|0)?\(\s*0?[2-9]\d{1,3}\s*\)[\s.-]*[2-9][0-9\s.-]{4,8}[0-9]|"
    r"(?:\+62[\s.-]*|62[\s.-]*|0)[2-9]\d{1,3}[\s.-]+[2-9][0-9\s.-]{4,8}[0-9]"
    r")"
)
RE_DATE_LIKE = re.compile(r"(?:\d{4}[-/]\d{1,2}[-/]\d{1,2}|\d{1,2}[-/]\d{1,2}[-/]\d{4})")

AREA_CODES_2_DIGIT = {"21", "22", "24", "31", "61"}

# DAFTAR PUTIH KODE AREA RESMI INDONESIA (v1.6.2) — Mencegah nomor palsu acak/DOI
VALID_INDONESIAN_AREA_CODES = {
    # 2-digit (tanpa leading 0)
    "21", "22", "24", "31", "61",
    # Jawa Barat & Banten
    "231", "232", "233", "234", "251", "252", "253", "254", "260", "261", "262", "263", "264", "265", "266", "267",
    # Jawa Tengah & DIY
    "271", "272", "273", "274", "275", "276", "281", "282", "283", "284", "285", "286", "287", "289", "291", "292", "293", "294", "295", "296", "297", "298",
    # Jawa Timur
    "321", "322", "323", "324", "325", "327", "328", "331", "332", "333", "334", "335", "338", "341", "342", "343", "351", "352", "353", "354", "355", "356", "357", "358",
    # Bali & Nusa Tenggara
    "361", "362", "363", "365", "366", "368", "370", "371", "372", "373", "374", "380", "381", "382", "383", "384", "385", "386", "387", "388", "389",
    # Sumatra
    "620", "621", "622", "623", "624", "625", "627", "628", "629", "630", "631", "632", "633", "634", "635", "636", "638", "639",
    "641", "642", "643", "644", "645", "646", "650", "651", "652", "653", "654", "655", "656", "657", "658", "659",
    "711", "712", "713", "714", "715", "716", "717", "718", "719", "721", "722", "723", "724", "725", "726", "727", "728", "729",
    "730", "731", "732", "733", "734", "735", "736", "737", "738", "739", "741", "742", "743", "744", "745", "746", "747", "748",
    "751", "752", "753", "754", "755", "756", "757", "759", "760", "761", "762", "763", "764", "765", "767", "768", "769",
    "770", "771", "772", "773", "776", "777", "778", "779",
    # Kalimantan
    "511", "512", "513", "517", "518", "519", "526", "527", "531", "532", "534", "535", "536", "537", "538", "539",
    "541", "542", "543", "545", "548", "549", "551", "552", "553", "554", "556", "561", "562", "563", "564", "565", "567", "568",
    # Sulawesi
    "410", "411", "413", "414", "417", "418", "419", "420", "421", "422", "423", "426", "427", "428", "430", "431", "432", "434", "435", "438",
    "443", "450", "451", "452", "453", "457", "458", "461", "462", "463", "464", "465", "471", "472", "473", "474", "475", "481", "482", "484", "485",
    # Maluku & Papua
    "901", "902", "910", "911", "913", "914", "915", "916", "917", "918", "921", "922", "923", "924", "927", "929",
    "951", "952", "955", "956", "957", "967", "969", "971", "975", "980", "981", "983", "984", "986",
}

CONTACT_CONTEXT_KEYWORDS = (
    "telp", "telepon", "phone", "ph:", "tel:", "call", "wa", "whatsapp", "fax",
    "contact", "kontak", "hubungi", "hp", "mobile", "sekretariat", "office"
)

APC_UNKNOWN = "Cek Web / Belum Terdata"

# Istilah spesifik untuk biaya penerbitan / publikasi (bukan biaya submit semata) v1.7.0
PUB_TERMS = (
    r"(?:article\s*publication|publication(?:\s*fees?)?|article\s*processing\s*charges?|apc|"
    r"publishing(?:\s*fees?)?|biaya\s*(?:publikasi|penerbitan|pemrosesan\s*artikel|artikel|kontribusi)|"
    r"publikasi|penerbitan|kontribusi\s*(?:publikasi|penerbitan)?|tarif\s*(?:publikasi|penerbitan)|"
    r"page\s*charges?|biaya\s*(?:halaman|cetak))"
)

# RE_PUB_FREE_STRICT v1.8.0: Pola gratis eksplisit dengan boundary ketat angka 0 (anti false-positive 1.000,000)
RE_PUB_FREE_STRICT = re.compile(
    r"(?:"
    r"(?:" + PUB_TERMS + r"(?:[^\n\r.,]{0,40}?(?:adalah|sebesar|:|=|-))?\s*"
    r"(?:(?:rp\.?|idr)\s*0(?:[.,]00)?(?=\s|$|\(|[^\d.,])|(?<!\d)0(?:[.,]00)?(?=\s|$|\(|[^\d.,])\s*(?:\(?idr\)?|\(?rp\.?\)|\(?usd\)?|\$)?|"
    r"gratis|free|none|tidak\s*(?:dipungut|dikenakan|memungut|mengenakan)\s*biaya|bebas\s*biaya|tanpa\s*(?:dipungut\s*)?biaya|dibebaskan\s*dari\s*biaya|free\s*of\s*charge)\b)|"
    r"(?:free\s*of\s*charge|no\s*(?:article\s*processing\s*)?fees?|tidak\s*(?:dipungut|dikenakan|memungut)\s*biaya|tanpa\s*biaya|bebas\s*biaya|dibebaskan\s*dari\s*biaya)\s*(?:for|pada|untuk)?\s*(?:publication|publishing|publikasi|penerbitan)|"
    r"charges?\s*no\s*(?:publication|article\s*processing)\s*fees?|"
    r"does\s*not\s*charge\s*(?:any\s*)?(?:fees?|apc|article\s*processing\s*charges?)|"
    r"(?:publication|publishing|biaya\s*publikasi)\s*(?:is|adalah|sebesar)?\s*(?:free|gratis|bebas\s*biaya|free\s*of\s*charge)"
    r")",
    re.IGNORECASE,
)

# RE_PUB_AMOUNT & RE_IDR_1 v1.8.0: Mendukung pemisah ribuan campuran '1.000,000 (IDR)' dan '1,000.000' + desimal 2 digit
RE_PUB_AMOUNT = re.compile(
    PUB_TERMS + r"(?:[^\n\r.]{0,45}?(?:adalah|sebesar|\bis\b|:|=|-))?\s*"
    r"(?:"
    r"(?:Rp\.?|IDR)\s*([0-9]{1,3}(?:[.,][0-9]{3})+(?:[.,][0-9]{2})?|[0-9]{5,8}(?:[.,][0-9]{2})?)(?:,-)?|"
    r"([0-9]{1,3}(?:[.,][0-9]{3})+(?:[.,][0-9]{2})?|[0-9]{5,8}(?:[.,][0-9]{2})?)\s*(?:\(?IDR\)?|\(?Rp\.?\)?)(?:,-)?|"
    r"(?:(?:\$|USD)\s*([0-9]+(?:[.,][0-9]{1,2})?)(?:\s*USD)?)|"
    r"([0-9]+(?:[.,][0-9]{1,2})?)\s*(?:\(?USD\)?|\$)|"
    r"([0-9]+(?:\.[0-9]+)?)\s*(?:rb|ribu|jt|juta)"
    r")",
    re.IGNORECASE,
)

RE_USD_1 = re.compile(r"(?:\$|USD)\s*([0-9]+(?:[.,][0-9]{1,2})?)\b", re.IGNORECASE)
RE_USD_2 = re.compile(r"\b([0-9]+(?:[.,][0-9]{1,2})?)\s*(?:\(?USD\)?|\$)", re.IGNORECASE)

RE_IDR_1 = re.compile(
    r"(?:(?:Rp\.?|IDR)\s*([0-9]{1,3}(?:[.,][0-9]{3})+(?:[.,][0-9]{2})?)|([0-9]{1,3}(?:[.,][0-9]{3})+(?:[.,][0-9]{2})?)\s*(?:\(?IDR\)?|\(?Rp\.?\)?))",
    re.IGNORECASE,
)
RE_IDR_2 = re.compile(
    r"(?:(?:Rp\.?|IDR)\s*([0-9]{5,8})(?:[.,]00)?|([0-9]{5,8})(?:[.,]00)?\s*(?:\(?IDR\)?|\(?Rp\.?\)?))",
    re.IGNORECASE,
)


def _parse_mixed_idr_number(raw_num: str) -> Optional[int]:
    """
    Menormalisasi angka nominal IDR termasuk format pemisah campuran titik-dan-koma:
    - '1.000,000' -> 1000000
    - '1,000.000' -> 1000000
    - '1.000.000,00' / '500,000.00' -> 1000000 / 500000 (membuang 2-digit sen di akhir)
    """
    if not raw_num:
        return None
    s = raw_num.strip()
    s = re.sub(r"(?<=\d{3})[.,]\d{2}$", "", s)
    digits = re.sub(r"[.,\s]", "", s)
    if not digits.isdigit():
        return None
    return int(digits)

# Anti-OA-Trap v1.7.0 (VISION.md 3.1.2):
# Frasa kebijakan akses pembaca yang WAJIB dibuang sebelum pencocokan kata gratis/free
OA_READER_PATTERNS = [
    r"(?:this\s+journal\s+|this\s+article\s+|journal\s+)?(?:provides\s+)?immediate\s+open\s+access[^.\n]*\.?",
    r"(?:making\s+research\s+)?freely\s+available\s+to\s+the\s+public[^.\n]*\.?",
    r"free\s+to\s+read(?:\s+and\s+download)?[^.\n]*\.?",
    r"open\s+access\s+to\s+all\s+users[^.\n]*\.?",
    r"publicly\s+available[^.\n]*\.?",
    r"freely\s+available\s+online[^.\n]*\.?",
    r"open\s+access\s+policy[^.\n]*\.?",
    r"prinsip\s+bahwa\s+membuat\s+penelitian\s+tersedia\s+secara\s+bebas[^.\n]*\.?",
]
OA_READER_STRIP = re.compile(r"(?:" + "|".join(OA_READER_PATTERNS) + r")", re.IGNORECASE)

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

# Kamus Kata Kunci Komprehensif Biaya Publikasi (v1.6.2)
APC_KEYWORDS = [
    "author fees", "author fee", "publication fee", "publishing fee", "biaya publikasi",
    "biaya penerbitan", "article processing charge", "article processing charges", "processing fee",
    "submission fee", "page charge", "biaya cetak", "free of charge", "gratis", "bebas biaya",
    "0 idr", "usd", "no charge", "without any fee", "penyerahan naskah", "biaya halaman",
    "biaya pemrosesan", "apc", "biaya penulisan", "tarif", "payment", "kontribusi",
    "contribution", "tidak dipungut biaya", "tidak dikenakan biaya", "tidak memungut biaya",
    "does not charge", "no article processing"
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
    session.headers.update(get_browser_headers())
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
        pool_connections=max(128, HTTP_POOL_CONNECTIONS),
        pool_maxsize=max(128, HTTP_POOL_MAXSIZE),
        max_retries=retries,
    )
    session.mount("http://", adapter)
    session.mount("https://", adapter)
    return session


_thread_local_sessions = threading.local()


def get_thread_local_session(is_deep_scrape: bool = True) -> requests.Session:
    """Menggunakan thread-local Session keep-alive agar worker OJS tidak mengulang handshake TCP/TLS."""
    attr = "deep_session" if is_deep_scrape else "portal_session"
    sess = getattr(_thread_local_sessions, attr, None)
    if sess is None:
        sess = get_high_capacity_session(is_deep_scrape=is_deep_scrape)
        setattr(_thread_local_sessions, attr, sess)
    return sess


# ── ZERO-CRASH & ANTI-FREEZE HTTP FETCHER (v1.6.0) ───────────────────────────
def safe_fetch_html(
    session: requests.Session,
    url: str,
    timeout=DEEP_SCRAPE_TIMEOUT,
    max_bytes: int = 1_500_000,
) -> tuple[int, str, dict]:
    """
    ZERO-CRASH & ANTI-FREEZE HTTP FETCHER (v1.6.0):
    - Content-Type guard: hanya memproses text/html, application/xhtml+xml, atau text/plain.
    - Response size guard: batasi pembacaan maksimal 1.5 MB per halaman OJS.
    - Exception shield: tangani TooManyRedirects, UnicodeDecodeError, SSLError, ConnectionError tanpa melempar crash.
    Returns: (status_code, html_text, headers)
    """
    if not url or not str(url).startswith("http"):
        return 0, "", {}
    try:
        resp = session.get(
            url,
            timeout=timeout,
            verify=False,
            allow_redirects=True,
            stream=True,
        )
        status_code = resp.status_code
        headers = dict(resp.headers)
        headers["_final_url"] = getattr(resp, "url", "") or url

        # 1. Content-Type Guard
        ct = (headers.get("Content-Type") or "").lower()
        if not any(allowed in ct for allowed in ("text/html", "application/xhtml", "text/plain")):
            resp.close()
            return status_code or 415, "", headers

        # 2. Content-Length preliminary check (> 5 MB skip upfront)
        cl = headers.get("Content-Length")
        if cl and cl.isdigit() and int(cl) > 5_000_000:
            resp.close()
            return 413, "", headers

        # 3. Size Guard (baca max 1.5 MB)
        raw_bytes = resp.raw.read(max_bytes, decode_content=True)
        resp.close()

        # 4. Safe Encoding Decode
        enc = resp.encoding or "utf-8"
        try:
            html_text = raw_bytes.decode(enc, errors="replace")
        except Exception:
            html_text = raw_bytes.decode("utf-8", errors="replace")

        return status_code, html_text, headers
    except requests.exceptions.TooManyRedirects:
        return 310, "", {}
    except (requests.exceptions.RequestException, UnicodeDecodeError, Exception):
        return 0, "", {}


# ── CLOUDFLARE EMAIL & OBFUSCATION DECODER (v1.6.0) ───────────────────────────
def decode_cfemail(cf_hex_string: str) -> str:
    """
    CLOUDFLARE EMAIL PROTECTION DECODER (v1.6.0):
    Mendekode email dari atribut data-cfemail="__cf_email__".
    Formula: 2 karakter hex pertama adalah XOR key, selanjutnya adalah karakter ASCII.
    """
    if not cf_hex_string or len(cf_hex_string) < 4:
        return ""
    try:
        k = int(cf_hex_string[:2], 16)
        return "".join(chr(int(cf_hex_string[i:i+2], 16) ^ k) for i in range(2, len(cf_hex_string), 2))
    except Exception:
        return ""


def deobfuscate_email_text(text: str) -> str:
    """
    DEOBFUSCATION EMAIL REDAKSI (v1.6.0):
    Mengubah format perlindungan teks manusia seperti:
    'jurnal [at] kampus.ac.id' atau 'jurnal(at)kampus[dot]ac[dot]id'
    menjadi email standar 'jurnal@kampus.ac.id'.
    """
    if not text:
        return ""
    t = re.sub(r"[\s\[\(\{<]+(?:at|commat)[\s\]\}\)>]+", "@", text, flags=re.IGNORECASE)
    t = re.sub(r"[\s\[\(\{<]+dot[\s\]\}\)>]+", ".", t, flags=re.IGNORECASE)
    return t


STUDENT_DOMAIN_SUBSTRINGS = (
    "@student.", "@students.", "@mail.ugm.", "@mhs.", "@alumni.",
    "@pasca.", "@pps.", "@mahasiswa."
)

AUTHOR_USER_PREFIXES = (
    "firstauthor", "secondauthor", "thirdauthor", "fourthauthor",
    "author", "authors", "penulis", "correspondence", "korespondensi",
    "first@", "second@", "third@", "fourth@", "user@", "admin@domain",
    "sitinurhaliza", "mahasiswa"
)


def rank_and_filter_emails(emails: list[str], ojs_url: str = "") -> list[str]:
    """
    PRIORITAS EMAIL RESMI & ELIMINASI EMAIL PENULIS / MAHASISWA (v1.6.2):
    - Filter ketat domain mahasiswa (@student., @students., @mail.ugm., @mhs., @alumni., dsb.).
    - Eliminasi mutlak: Menghapus email mahasiswa berformat NIM (>= 6 digit angka di username).
    - Prioritas 1: Email berdomain sama dengan kampus OJS (.ac.id) atau diawali kata kunci redaksi.
    - Jika sudah ditemukan email resmi kampus, buang seluruh email @gmail/@yahoo pribadi penulis.
    - Hard Cap: Maksimal 3 email resmi terbaik per jurnal.
    """
    if not emails:
        return []

    base_domain = ""
    if ojs_url and str(ojs_url).startswith("http"):
        parsed = requests.compat.urlparse(ojs_url)
        netloc = (parsed.netloc or "").lower()
        parts = netloc.split(".")
        if len(parts) >= 3 and parts[-2] == "ac" and parts[-1] == "id":
            base_domain = ".".join(parts[-3:])
        elif len(parts) >= 2:
            base_domain = ".".join(parts[-2:])
        else:
            base_domain = netloc

    ed_keywords = (
        "jurnal", "journal", "editor", "redaksi", "ejournal",
        "publikasi", "ojs", "admin", "contact", "kontak", "secretariat",
        "sekretariat", "pengelola", "dewanredaksi", "chiefeditor", "managingeditor"
    )

    scored: list[tuple[int, bool, str]] = []
    for em in emails:
        em_clean = em.strip().lower()
        if not em_clean or "@" not in em_clean:
            continue

        # 1. Tolak domain mahasiswa / alumni
        if any(sub in em_clean for sub in STUDENT_DOMAIN_SUBSTRINGS):
            continue

        user, _, dom = em_clean.partition("@")

        # 2. Tolak email penulis generik
        if any(user.startswith(p) for p in ("firstauthor", "secondauthor", "thirdauthor", "fourthauthor", "author", "authors", "penulis", "user@")):
            continue

        # 3. Deteksi NIM mahasiswa (>= 6 digit angka dalam username atau berakhiran >= 4 digit)
        has_nim = bool(re.search(r"\d{6,}", user)) or (bool(re.search(r"\d{4,}$", user)) and not any(k in user for k in ed_keywords))

        score = 0
        if has_nim:
            score -= 100
        if base_domain and (base_domain in dom or dom in base_domain):
            score += 60
        if any(user.startswith(k) or k in user for k in ed_keywords):
            score += 50
        if dom.endswith(".ac.id") or dom.endswith(".edu"):
            score += 25
        elif not has_nim:
            score += 5

        scored.append((score, has_nim, em_clean))

    # Cek apakah ada email resmi kampus / redaksi
    has_official = any(s >= 25 for s, is_nim, _ in scored if not is_nim)
    if has_official:
        # Buang semua email ber-NIM
        scored = [x for x in scored if not x[1]]
        # Buang email gmail/yahoo milik pribadi yang tidak mengandung kata redaksi
        filtered_scored = []
        for s, is_nim, em_clean in scored:
            u, _, d = em_clean.partition("@")
            if d in ("gmail.com", "yahoo.com", "yahoo.co.id", "hotmail.com", "outlook.com"):
                if any(k in u for k in ("jurnal", "journal", "editor", "redaksi", "ejournal", "publikasi", "ojs", "admin")):
                    filtered_scored.append((s, is_nim, em_clean))
            else:
                filtered_scored.append((s, is_nim, em_clean))
        if filtered_scored:
            scored = filtered_scored

    scored.sort(key=lambda x: x[0], reverse=True)
    res: list[str] = []
    for _, _, em in scored:
        if em not in res:
            res.append(em)

    # HARD CAP v1.6.2: Maksimal 3 email resmi terbaik
    return res[:3]


# ── VALIDATOR TELEPON/WA, PEMBERSIH TABEL BIODATA & FOCUS SYNTHESIZER (v1.6.2) ──
def is_valid_indonesian_phone(raw_str: str, has_contact_context: bool = False) -> tuple[bool, str]:
    """
    VALIDASI RIGID NOMOR TELEPON & WHATSAPP INDONESIA (v1.6.2):
    - Seluler/WA: Diawali 08 / +628, prefix operator valid (811-819, 821-823, 831-838,
      851-859, 877-878, 881-889, 895-899), panjang 10-13 digit angka setelah '0'.
    - Telepon Kabel: Wajib berada dalam DAFTAR PUTIH KODE AREA RESMI INDONESIA (2-digit atau 3-digit).
      TOLAK MUTLAK:
      * Kode area palsu / acak dari DOI (misal: 0203, 0300, 0491, dsb.).
      * Nomor pelanggan yang diawali '0' atau '1' (misal: +622100..., +622118...).
      * Deretan angka 14–16 digit (nomor rekening bank / NIP / ID artikel).
      * Panjang total digit kabel WAJIB antara 9 hingga 12 digit (termasuk 62).
      * Non-seluler WAJIB memiliki format tanda kurung kode area ATAU konteks kata kunci kontak eksplisit.
    Returns: (is_valid: bool, normalized_phone: str)
    """
    if not raw_str:
        return False, ""
    raw_clean = str(raw_str).strip()
    digits = re.sub(r"\D", "", raw_clean)
    if len(digits) < 9 or len(digits) > 13 or len(set(digits)) <= 2:
        return False, ""

    if digits.startswith("0"):
        full_digits = "62" + digits[1:]
    elif digits.startswith("62"):
        full_digits = digits
    else:
        full_digits = "62" + digits

    # 1. Mobile / WhatsApp (08xx / +628xx)
    if full_digits.startswith("628"):
        m = re.match(r"^628(1[1-9]|2[1-3]|3[1-38]|5[1-35-9]|7[78]|8[1-9]|9[5-9])\d{6,9}$", full_digits)
        if m:
            return True, "+" + full_digits
        return False, ""

    # 2. Landline Kabel (Kode Area Indonesia)
    if not full_digits.startswith("62"):
        return False, ""

    # Panjang total telepon kabel Indonesia (termasuk 62): 9 s.d. 12 digit
    if not (9 <= len(full_digits) <= 12):
        return False, ""

    rest = full_digits[2:]
    if not rest:
        return False, ""

    area_code = ""
    subscriber_num = ""
    if rest[:2] in VALID_INDONESIAN_AREA_CODES:
        area_code = rest[:2]
        subscriber_num = rest[2:]
    elif len(rest) >= 4 and rest[:3] in VALID_INDONESIAN_AREA_CODES:
        area_code = rest[:3]
        subscriber_num = rest[3:]
    else:
        return False, ""

    # Di Indonesia nomor pelanggan lokal telepon kabel TIDAK PERNAH diawali angka 0 atau 1
    if subscriber_num.startswith("0") or subscriber_num.startswith("1"):
        return False, ""

    # Panjang nomor pelanggan lokal telepon kabel umumnya 5 s.d. 8 digit
    if not (5 <= len(subscriber_num) <= 8):
        return False, ""

    # Telepon kabel non-seluler WAJIB berformat eksplisit tanda kurung ATAU berada dekat kata kunci kontak
    has_parentheses = bool(re.search(r"\(\s*0?[2-9]\d{1,3}\s*\)", raw_clean))
    has_separator = bool(re.search(r"[\s.-]", raw_clean))
    if not (has_parentheses or (has_separator and has_contact_context)):
        return False, ""

    return True, "+" + full_digits


# ── FILTER NOISE FOCUS & SCOPE (v1.6.2) ──────────────────────────────────────
FOCUS_NOISE_TERMS = (
    "scopus id", "sinta id", "google scholar", "diindeks oleh", "indexed by",
    "view journal", "current issue", "register", "this issue has been available",
    "all articles in this", "licensed under cc", "creative commons", "copyright ©",
    "copyright (c)", "telah migrasi ke", "p-issn", "e-issn", "abstract :",
    "keywords :", "kata kunci :", "this study aims to", "penelitian ini bertujuan",
    "hasil penelitian menunjukkan", "the results showed", "author guidelines",
    "editorial team", "open journal systems"
)


def is_focus_scope_noise(text: str) -> bool:
    """Mendeteksi apakah teks mengandung derau metadata indeksasi, lisensi, atau abstrak artikel."""
    if not text or not isinstance(text, str):
        return False
    t_clean = text.lower()
    return any(noise in t_clean for noise in FOCUS_NOISE_TERMS)


def canonical_journal_key(title: str, publisher: str = "", issn: str = "") -> str:
    """
    KUNCI KANONIKAL DEDUPLIKASI CERDAS (v1.6.2):
    - Menormalkan simbol '&' dan frasa 'and' menjadi 'dan'
    - Menghapus tanda baca, spasi, simbol.
    - Menghilangkan variasi huruf kapital.
    - Mengabaikan perbedaan tanda titik dua (':') atau strip ('-').
    """
    t = (title or "").lower()
    t = re.sub(r"\s*&\s*", " dan ", t)
    t = re.sub(r"\s+and\s+", " dan ", t)
    clean = re.sub(r"[^a-z0-9]", "", t)
    clean = re.sub(r"^(?:jurnal|journal|ejournal)", "", clean)
    return clean


def strip_ojs_profile_table_noise(text: str) -> str:
    """
    PEMBERSIH HEADER TABEL BIODATA OJS PADA FOCUS & SCOPE (v1.6.1):
    Memotong header tabel metadata profil (Journal Title, ISSN/E-ISSN, DOI Prefix, Editor in Chief,
    Managing Editor, Publisher, Frequency) dan hanya mengambil kalimat deskriptif intinya.
    """
    if not text or not text.strip():
        return ""
    clean = re.sub(r"\s+", " ", text).strip()

    meta_patterns = [
        r"journal\s+title", r"issn(?:/e-issn)?", r"doi\s+prefix", r"editor\s+in\s+chief",
        r"managing\s+editor", r"publisher", r"frequency", r"about\s+the\s+journal",
        r"penerbit", r"frekuensi", r"judul\s+jurnal"
    ]

    has_meta = sum(1 for p in meta_patterns if re.search(r"\b" + p + r"\b", clean, re.IGNORECASE)) >= 2
    if not has_meta:
        return clean

    triggers = [
        r"(?:(?:is\s+a(?:n)?|merupakan|adalah)\s+(?:peer-reviewed|jurnal|journal|scholarly|open\s+access)?)",
        r"(?:publishes|menerbitkan|memuat)\b",
        r"(?:aims\s+to|bertujuan\s+untuk)\b",
        r"(?:fokus|focus(?:es|ing)?|scope|ruang\s+lingkup)\b",
        r"(?:peer-reviewed\s+journal|jurnal\s+ilmiah)\b",
    ]

    best_pos = -1
    for tr in triggers:
        m = re.search(tr, clean, re.IGNORECASE)
        if m:
            pos = m.start()
            if best_pos == -1 or pos < best_pos:
                best_pos = pos

    if best_pos != -1:
        pre_chunk = clean[:best_pos]
        m_title = re.search(r"((?:[A-Z][a-zA-Z0-9]*\s+(?:dan|and|of|in|the|di|pada|untuk|[A-Z][a-zA-Z0-9]*)\s*)+)$", pre_chunk)
        if m_title and not any(re.search(r"\b" + mp + r"\b", m_title.group(1), re.IGNORECASE) for mp in meta_patterns):
            start_idx = pre_chunk.rfind(m_title.group(1).strip())
            candidate = clean[start_idx:].strip()
        else:
            candidate = clean[best_pos:].strip()

        candidate = re.sub(r"^(?:(?:january|february|march|april|may|june|july|august|september|october|november|december|januari|februari|maret|april|mei|juni|juli|agustus|september|oktober|november|desember)\b[,\s]*)+", "", candidate, flags=re.IGNORECASE).strip()
        candidate = candidate[:1].upper() + candidate[1:]
        if len(candidate) >= 20:
            return candidate

    return clean


def synthesize_focus_scope(entry_or_dict: any) -> tuple[str, str]:
    """
    ZERO-EMPTY FOCUS & SCOPE SYNTHESIZER (v1.6.1):
    Menjamin 100% jurnal memiliki Focus & Scope yang representatif.
    Menggunakan data Bidang Keilmuan (subject_area) dari SINTA/Garuda dan judul/penerbit jurnal.
    Returns: (synthesized_text, "SINTA/Garuda")
    """
    if isinstance(entry_or_dict, dict):
        sub = entry_or_dict.get("bidang") or entry_or_dict.get("subject_area") or ""
        pub = entry_or_dict.get("penerbit") or ""
        name = entry_or_dict.get("nama_jurnal") or entry_or_dict.get("judul") or ""
        rank = entry_or_dict.get("peringkat_sinta") or ""
    else:
        sub = getattr(entry_or_dict, "bidang", "") or getattr(entry_or_dict, "subject_area", "") or ""
        pub = getattr(entry_or_dict, "penerbit", "") or ""
        name = getattr(entry_or_dict, "nama_jurnal", "") or getattr(entry_or_dict, "judul", "") or ""
        rank = getattr(entry_or_dict, "peringkat_sinta", "") or ""

    sub = str(sub).strip()
    pub = str(pub).strip()
    if sub == INFO_NOT_FOUND or "tidak ditemukan" in sub.lower():
        sub = ""
    if pub == INFO_NOT_FOUND or "tidak ditemukan" in pub.lower():
        pub = ""

    if sub and pub:
        text = f"Menerbitkan riset ilmiah di bidang {sub}, dikelola dan diterbitkan secara berkala oleh {pub}."
    elif sub:
        text = f"Menerbitkan riset ilmiah di bidang {sub}."
    elif pub:
        text = f"Mempublikasikan artikel hasil riset dan kajian ilmiah berkualitas, diterbitkan secara resmi oleh {pub}."
    elif name:
        if rank and rank != INFO_NOT_FOUND:
            text = f"Jurnal ilmiah berkala terakreditasi {rank} yang memuat artikel hasil penelitian dan kajian teoritis/empiris."
        else:
            text = f"Jurnal ilmiah berkala yang mempublikasikan artikel hasil penelitian dan telaah keilmuan bereputasi."
    else:
        text = "Menerbitkan artikel ilmiah hasil penelitian dan telaah akademis berkala."

    return text, "SINTA/Garuda"


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
    """Mengambil data OJS dari SQLite WAL cache jika belum kadaluwarsa (TTL 14 hari)."""
    if not url or not url.startswith("http"):
        return None
    try:
        cached = cache_engine.cache_engine.get_journal(url)
        if cached:
            # Incomplete Cache Bypass (VISION.md 3.1.4):
            # Paksa crawl ulang jika data biaya publikasi belum terverifikasi
            apc_val = cached.get("apc_publish")
            s_apc = cached.get("source_apc")
            if (
                not apc_val
                or apc_val in ("", "—", "-", "Cek Web / Belum Terdata", INFO_NOT_FOUND)
                or s_apc in ("Kosong", "", None)
            ):
                return None

            email_val = cached.get("email_clean") or INFO_NOT_FOUND
            tel_val = cached.get("phone_clean") or INFO_NOT_FOUND
            if not (
                email_val == INFO_NOT_FOUND
                and tel_val == INFO_NOT_FOUND
                and not cached.get("apc_publish")
                and not cached.get("frequency_str")
                and not cached.get("focus_scope")
            ):
                return {
                    "ojs_url": cached.get("ojs_url_key") or url,
                    "email": email_val,
                    "telepon": tel_val,
                    "frekuensi": cached.get("frequency_str") or INFO_NOT_FOUND,
                    "apc_status": cached.get("apc_publish") or INFO_NOT_FOUND,
                    "apc_amount": cached.get("apc_amount"),
                    "biaya_submit": int(cached["apc_submit"]) if str(cached.get("apc_submit", "")).isdigit() else None,
                    "biaya_publikasi": cached.get("apc_amount"),
                    "mata_uang": cached.get("currency") or "IDR",
                    "focus_scope": cached.get("focus_scope") or INFO_NOT_FOUND,
                    "sumber_focus": cached.get("source_focus") or "Regex",
                    "sumber_apc": cached.get("source_apc") or "Regex",
                    "status_server": cached.get("server_status") or "Aktif (HTTP)",
                    "transparansi_kontak": compute_contact_transparency(email_val, tel_val),
                    "bulan_terbit": cached.get("months") or [],
                    "raw_about": cached.get("focus_scope") or "",
                    "raw_submissions": "",
                }
    except Exception:
        pass
    return None


def _save_to_cache(url: str, data: dict):
    """Menyimpan data OJS yang berhasil diekstrak ke dalam SQLite WAL cache."""
    if not url or not url.startswith("http") or not data:
        return
    if (
        data.get("email") == INFO_NOT_FOUND
        and data.get("telepon") == INFO_NOT_FOUND
        and data.get("apc_status") == INFO_NOT_FOUND
        and data.get("frekuensi") == INFO_NOT_FOUND
    ):
        return
    try:
        cache_engine.cache_engine.upsert_journal(url, data)
    except Exception:
        pass


NOISE_SELECTORS_FOR_SCOPE = [
    "nav", "aside", "header", "footer", "script", "style", "noscript",
    ".pkp_structure_sidebar", ".pkp_navigation_primary", ".pkp_navigation_user",
    "#customblock", ".block_make_submission", ".obj_article_summary",
    ".cmp_article_list", ".cmp_announcements", "#sidebar", ".sidebar",
    ".block_information", ".block_language"
]


def clean_soup_for_scope(soup: BeautifulSoup) -> BeautifulSoup:
    """
    Membersihkan elemen berisik khusus sebelum ekstraksi Focus & Scope:
    Membuang nav, aside, sidebar, dan daftar artikel terkini agar menu navigasi OJS
    dan judul artikel tidak bocor ke teks Focus & Scope.
    """
    if not soup:
        return soup
    cloned = BeautifulSoup(str(soup), "lxml")
    for sel in NOISE_SELECTORS_FOR_SCOPE:
        for el in cloned.select(sel):
            el.decompose()
    return cloned


# ── DOM SANITIZATION PIPELINE (Membersihkan derau script, style, & whitespace) ─
def fast_sanitize_html_text(html: str) -> str:
    """
    C-speed HTML-to-clean-text extractor menggunakan selectolax bila tersedia,
    dengan fallback aman ke BeautifulSoup + lxml.
    """
    if not html:
        return ""
    if HAS_SELECTOLAX and SelectolaxHTMLParser is not None:
        tree = SelectolaxHTMLParser(html)
        tree.strip_tags(["script", "style", "noscript", "svg", "iframe"])
        raw_text = tree.text(separator=" ", strip=True)
        clean_text = raw_text.replace("\xa0", " ").replace("&nbsp;", " ").replace("\u200b", " ")
        return re.sub(r"\s+", " ", clean_text).strip()
    return sanitize_dom(BeautifulSoup(html, "lxml"))


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
    HUMAN-PARITY APC & 3-STATUS ACCURACY (v1.7.0):
    1. 'Rp X.XXX.XXX' -> Prioritas 1 (Paid Priority Rule): Jika ditemukan nominal berbayar
       (Rp 50.000 – Rp 15.000.000 atau USD), tetapkan BERBAYAR meski ada Submission Fee = 0.
    2. 'Gratis (Rp 0)' -> HANYA jika tertulis eksplisit bahwa Publication Fee / APC / Biaya Penerbitan
       adalah Rp 0 / Gratis / Free / Bebas biaya, dan TIDAK ADA nominal berbayar.
       Dilindungi Anti-OA-Trap agar Open Access Policy pembaca tidak menjadi false positive.
    3. 'Cek Web / Belum Terdata' -> Jika halaman tidak mencantumkan konfirmasi keduanya.
    Returns: (display_str, numeric_idr_or_none)
    """
    if not context_text:
        return APC_UNKNOWN, None

    # Anti-OA-Trap (VISION.md 3.1.2):
    # Buang frasa kebijakan akses pembaca sebelum pencocokan kata gratis/free
    cleaned_context = OA_READER_STRIP.sub(" ", context_text)
    text = cleaned_context.strip()
    low = text.lower()

    zero_pub_found = False

    # 1. Prioritas 1: Pindai seluruh kemunculan RE_PUB_AMOUNT dengan Paid Priority Rule
    for m_amt in RE_PUB_AMOUNT.finditer(text):
        # IDR (mendukung pemisah campuran 1.000,000 / 1,000.000 & sen .00 / ,00)
        idr_raw = m_amt.group(1) or m_amt.group(2)
        if idr_raw:
            val = _parse_mixed_idr_number(idr_raw)
            if val is not None:
                if val == 0:
                    zero_pub_found = True
                elif APC_SANITY_MIN_IDR <= val <= APC_SANITY_MAX_IDR:
                    return f"Rp {val:,}".replace(",", "."), val
        # USD
        usd_raw = m_amt.group(3) or m_amt.group(4)
        if usd_raw:
            try:
                uval = float(usd_raw.replace(",", "."))
                if uval == 0:
                    zero_pub_found = True
                else:
                    val = int(round(uval * USD_TO_IDR_RATE))
                    if APC_SANITY_MIN_IDR <= val <= APC_SANITY_MAX_IDR:
                        urepr = f"${int(uval)} USD" if uval.is_integer() else f"${uval:.2f} USD"
                        return f"Rp {val:,}".replace(",", ".") + f" ({urepr})", val
            except ValueError:
                pass
        # Pengali (rb, ribu, jt, juta)
        mult_raw = m_amt.group(5)
        if mult_raw:
            mult_kw = m_amt.group(0).lower()
            mult_factor = 1000 if ("rb" in mult_kw or "ribu" in mult_kw) else 1_000_000
            try:
                val = int(round(float(mult_raw) * mult_factor))
                if APC_SANITY_MIN_IDR <= val <= APC_SANITY_MAX_IDR:
                    return f"Rp {val:,}".replace(",", "."), val
            except ValueError:
                pass

    # 2. Prioritas 1b: Fallback pencarian nominal USD/IDR dalam konteks publikasi
    is_sub_only = bool(re.search(r"(?:submission\s*fee|penyerahan\s*naskah|biaya\s*pendaftaran)\s*[:\w\s]*(?:0|free|gratis)", low))
    has_pub_word = any(w in low for w in ("publication", "publikasi", "penerbitan", "publishing", "apc", "processing charge"))

    if has_pub_word and not is_sub_only:
        m_usd = RE_USD_1.search(text) or RE_USD_2.search(text)
        if m_usd:
            usd_candidate = m_usd.group(1) or (m_usd.group(2) if m_usd.lastindex and m_usd.lastindex >= 2 else "")
            if usd_candidate:
                try:
                    usd_val = float(usd_candidate.replace(",", "."))
                    if usd_val > 0:
                        idr_val = int(round(usd_val * USD_TO_IDR_RATE))
                        if APC_SANITY_MIN_IDR <= idr_val <= APC_SANITY_MAX_IDR:
                            usd_repr = f"${int(usd_val)} USD" if usd_val.is_integer() else f"${usd_val:.2f} USD"
                            return f"Rp {idr_val:,}".replace(",", ".") + f" ({usd_repr})", idr_val
                except ValueError:
                    pass

        m_idr = RE_IDR_1.search(text) or RE_IDR_2.search(text)
        if m_idr:
            idr_val = _parse_mixed_idr_number(m_idr.group(1) or m_idr.group(2) or "")
            if idr_val is not None and APC_SANITY_MIN_IDR <= idr_val <= APC_SANITY_MAX_IDR:
                return f"Rp {idr_val:,}".replace(",", "."), idr_val

    # 3. Prioritas 2: Biaya Publikasi Eksplisit Gratis / Free / Rp 0 (Hanya jika tidak ada nominal berbayar)
    if zero_pub_found or RE_PUB_FREE_STRICT.search(text):
        return "Gratis (Rp 0)", 0

    return APC_UNKNOWN, None


def _extract_contacts_from_text(html_text: str, plain_text: str, ojs_url: str = "") -> tuple[list[str], list[str]]:
    """
    PENINGKATAN CAKUPAN TELEPON/WA & EMAIL REDAKSI (v1.6.0):
    - Dekoder email terproteksi Cloudflare (data-cfemail) dan deobfuscation format manusia ([at], (at), [dot]).
    - Prioritas email resmi kampus/redaksi & eliminasi email artikel/penulis serta NIM mahasiswa.
    - Ekstraksi nomor telepon kampus kabel kode area (021, 0271, dsb.) dan WhatsApp (08xx, +628xx).
    """
    found_emails: list[str] = []
    found_phones: list[str] = []

    clean_html = html_text.replace("&#64;", "@").replace("&commat;", "@")

    # 1. Dekode Cloudflare Protected Email (data-cfemail)
    for cf_hex in re.findall(r'data-cfemail=["\']([a-fA-F0-9]+)["\']', html_text):
        dec = decode_cfemail(cf_hex)
        if dec and "@" in dec:
            clean_em = dec.strip().lower()
            if clean_em not in found_emails:
                found_emails.append(clean_em)

    # 2. Deobfuscation Email Teks (misal: 'jurnal [at] kampus.ac.id')
    deobf_html = deobfuscate_email_text(clean_html)
    deobf_plain = deobfuscate_email_text(plain_text)

    raw_emails = found_emails + RE_EMAIL.findall(deobf_html) + RE_EMAIL.findall(deobf_plain)

    # Ekstraksi eksplisit dari href="mailto:..."
    for m in re.findall(r"mailto:([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})", clean_html, re.IGNORECASE):
        raw_emails.append(m)

    ignored_domains = ("example.com", "domain.com", "sfu.ca", "pkp.sfu.ca", "invalid", "email.com", "sample.com", "test.com", "yoursite.com")
    ignored_prefixes = (
        "user@", "admin@domain", "info@domain", "noreply@", "no-reply@",
        "firstauthor@", "secondauthor@", "thirdauthor@", "first@", "second@", "third@", "fourth@",
        "author@", "authors@", "support@", "john.doe@", "yourname@"
    )

    clean_em_candidates = []
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
        if len(clean_em) <= 60 and clean_em not in clean_em_candidates:
            clean_em_candidates.append(clean_em)

    # Ranking dan eliminasi email mahasiswa (NIM)
    found_emails = rank_and_filter_emails(clean_em_candidates, ojs_url=ojs_url)

    # 3. Regex Telepon / WhatsApp dengan Anti-Fake Phone Validator & Proximity Context (v1.6.1)
    for m in RE_PHONE.finditer(plain_text):
        ph = m.group(0)
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

        # Proximity context v1.6.2: cek label telepon eksplisit dalam rentang 35 karakter tepat di sebelah kiri atau baris yang sama
        start_pos, end_pos = m.start(), m.end()
        line_start = plain_text.rfind("\n", 0, start_pos)
        left_bound = max(0, start_pos - 35, line_start if line_start != -1 else 0)
        left_snippet = plain_text[left_bound:start_pos].lower()
        has_ctx = any(k in left_snippet for k in CONTACT_CONTEXT_KEYWORDS)

        ok, norm_ph = is_valid_indonesian_phone(clean_ph, has_contact_context=has_ctx)
        if not ok:
            continue

        if norm_ph not in found_phones:
            found_phones.append(norm_ph)

    # Hard Cap awal: utamakan mobile, batasi maksimal 3 nomor di raw pass
    found_phones.sort(key=lambda x: (0 if x.startswith("+628") or x.startswith("08") else 1))
    return found_emails[:3], found_phones[:2]


def clean_contact_list(items: list[str] | str, contact_type: str = "email", ojs_url: str = "") -> str:
    """
    Sanitasi Rigid Kontak Redaksi (v1.6.2):
    - Eliminasi mutlak string error/fallback ("tidak dapat diakses", "none", "null", dll).
    - Validasi format email & buang domain dummy (@example.com, @pkp.sfu.ca, dll).
    - Prioritaskan email resmi redaksi/kampus dan buang NIM mahasiswa.
    - Anti-Fake Phone Validator: Tolak nomor palsu/acak dari DOI/ID/Rekening Bank.
    - Urutkan telepon dengan prioritas nomor WhatsApp/seluler (+628...) di posisi pertama.
    - HARD CAP: Maksimal 3 email dan maksimal 2 nomor telepon/WA.
    """
    if isinstance(items, str):
        raw_items = [p.strip() for p in re.split(r"[,;]+", items) if p.strip()]
    elif isinstance(items, (list, tuple, set)):
        raw_items = [str(p).strip() for p in items if p and str(p).strip()]
    else:
        return INFO_NOT_FOUND

    if not raw_items:
        return INFO_NOT_FOUND

    forbidden_exact = {"none", "null", "n/a", "na", "-", "--", "undefined", "unknown", "tidak ada", "kosong"}
    forbidden_subs = (
        "tidak dapat diakses", "tidak ditemukan", "situs tidak", "dead link",
        "contoh@", "test@"
    )

    cleaned = []
    if contact_type.lower() == "email":
        seen_email = set()
        email_pattern = re.compile(r"^[^@\s]+@[^@\s]+\.[a-zA-Z]{2,}$")
        dummy_domains = ("example.com", "domain.com", "pkp.sfu.ca", "invalid", "email.com", "test.com", "gmail.com.com")
        for it in raw_items:
            it_clean = it.strip().lower()
            if it_clean in forbidden_exact or any(sub in it_clean for sub in forbidden_subs):
                continue
            if not email_pattern.match(it_clean):
                continue
            domain = it_clean.split("@")[-1]
            if domain in dummy_domains or any(it_clean.startswith(p) for p in ("firstauthor@", "secondauthor@", "thirdauthor@")):
                continue
            if it_clean not in seen_email:
                seen_email.add(it_clean)
                cleaned.append(it_clean)

        cleaned = rank_and_filter_emails(cleaned, ojs_url=ojs_url)
        if not cleaned:
            return INFO_NOT_FOUND
        # HARD CAP: Maksimal 3 email terbaik
        return ", ".join(cleaned[:3])

    elif contact_type.lower() in ("phone", "telepon"):
        seen_digits = set()
        for it in raw_items:
            it_clean = it.strip()
            if it_clean.lower() in forbidden_exact or any(sub in it_clean.lower() for sub in forbidden_subs):
                continue

            ok, norm_phone = is_valid_indonesian_phone(it_clean, has_contact_context=True)
            if not ok:
                continue

            norm_digits = re.sub(r"\D", "", norm_phone)
            if norm_digits not in seen_digits:
                seen_digits.add(norm_digits)
                cleaned.append(norm_phone)

        if not cleaned:
            return INFO_NOT_FOUND

        # Urutkan: nomor WhatsApp/Seluler (+628...) prioritas pertama
        cleaned.sort(key=lambda x: (0 if x.startswith("+628") or x.startswith("08") else 1))
        # HARD CAP: Maksimal 2 nomor telepon/WA terbaik
        return ", ".join(cleaned[:2])


def get_whatsapp_link(phone: str, journal_name: str = "") -> Optional[str]:
    """
    Membuat tautan WhatsApp Click-to-Chat otomatis jika nomor seluler Indonesia valid.
    """
    if not phone or phone == INFO_NOT_FOUND:
        return None
    phones = [p.strip() for p in re.split(r"[,;]+", phone)]
    for p in phones:
        digits = re.sub(r"\D", "", p)
        if digits.startswith("08") and 10 <= len(digits) <= 13:
            wa_num = "62" + digits[1:]
        elif digits.startswith("628") and 11 <= len(digits) <= 14:
            wa_num = digits
        else:
            continue

        j_name = journal_name or "Jurnal"
        wa_text = requests.compat.quote(
            f"Halo Tim Redaksi {j_name}, saya mahasiswa/peneliti yang tertarik mengirimkan naskah ke jurnal Bapak/Ibu. "
            f"Apakah kuota publikasi untuk terbitan terdekat masih tersedia? Terima kasih."
        )
        return f"https://wa.me/{wa_num}?text={wa_text}"
    return None



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
    Normalisasi teks Focus & Scope (v1.5.0):
    Membersihkan tag noise, spasi berlebih, dan prefix 'Focus and Scope:',
    serta memvalidasi bahwa teks bukan daftar menu navigasi OJS (Anti-Boilerplate).
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

    # Validasi Anti-Boilerplate Noise & Anti-Metadata/Licensing Noise (v1.6.2)
    if ai_engine.is_boilerplate_noise(clean_txt) or is_focus_scope_noise(clean_txt):
        return INFO_NOT_FOUND

    # Pisahkan per kalimat (2 - 3 kalimat)
    sentences = re.split(r"(?<=[.!?])\s+", clean_txt)
    short_summary = " ".join(sentences[:3]).strip()
    if not short_summary:
        short_summary = clean_txt

    if ai_engine.is_boilerplate_noise(short_summary) or is_focus_scope_noise(short_summary):
        return INFO_NOT_FOUND

    if len(short_summary) > 300:
        short_summary = short_summary[:297].rsplit(" ", 1)[0] + "..."

    return short_summary if len(short_summary) >= 15 else INFO_NOT_FOUND


def build_tier3_scope_fallback(entry: JournalEntry) -> str:
    """
    Tier 3 Fallback Data Mining Focus & Scope (v1.3.8 / v1.5.0):
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
    """Ekstraksi Focus & Scope Legacy v1.5.0 berbasis regex dan heuristik OJS dengan Anti-Boilerplate."""
    if not raw_text or not raw_text.strip():
        return ""
    text = re.sub(r"\s+", " ", raw_text).strip()
    if "<" in text and ">" in text:
        text = re.sub(r"<[^>]+>", " ", text)
    if ai_engine.is_boilerplate_noise(text):
        return ""
    m = re.search(
        r"(?:focus\s*(?:and|&)\s*scope|aims\s*(?:and|&)\s*scope|fokus\s*dan\s*ruang\s*lingkup|ruang\s*lingkup)[:\s-]*(.{30,600})",
        text,
        re.I,
    )
    if m:
        candidate = m.group(1).strip()
        if not ai_engine.is_boilerplate_noise(candidate):
            norm = normalize_focus_and_scope(candidate)
            if norm and norm != INFO_NOT_FOUND:
                return norm
    norm = normalize_focus_and_scope(text)
    return norm if norm != INFO_NOT_FOUND else ""


def legacy_extract_apc_dict(raw_text: str) -> dict:
    """
    Ekstraksi APC Legacy v1.5.0 dengan pemisahan Biaya Submit vs Publikasi.
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
                    parsed_sub = _parse_mixed_idr_number(m_idr.group(1) or m_idr.group(2) or "")
                    if parsed_sub is not None:
                        biaya_submit = parsed_sub
                        mata_uang = "IDR"

    # Deteksi biaya publikasi via parse_apc_from_context legacy
    disp, num = parse_apc_from_context(text)
    if num is not None:
        biaya_pub = num
        if not mata_uang:
            mata_uang = "USD" if ("$" in disp or "USD" in disp) else "IDR"

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
    Ekstraksi Fokus dan Ruang Lingkup Jurnal (Multi-Tier Mining Engine - v1.5.0):
    Tier 1: Meta-Tags & Targeted DOM Selectors / Headings (Anti-Boilerplate Sanitization)
    Tier 2: OJS Homepage Context Parsing (Paragraf pembuka substantif bebas derau)
    """
    if not soup:
        return INFO_NOT_FOUND

    # Pra-pembersihan DOM dari elemen berisik: nav, aside, sidebar, article summaries
    clean_soup = clean_soup_for_scope(soup)
    scope_candidate = ""

    # ── TIER 1A: Meta-Tags (DC.Description, description, og:description) ─────
    for m in clean_soup.find_all("meta"):
        name = (m.get("name") or m.get("property") or "").lower()
        content = (m.get("content") or "").strip()
        if name in ("dc.description", "description", "og:description", "dc.subject") and len(content) >= 30:
            if not any(ign in content.lower() for ign in ("open journal systems", "pkp.sfu.ca", "wordpress", "login to", "register")):
                if not ai_engine.is_boilerplate_noise(content):
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
            elem = clean_soup.select_one(sel)
            if elem:
                p_tags = elem.find_all("p")
                if p_tags:
                    txt = " ".join(p.get_text(" ", strip=True) for p in p_tags[:3])
                else:
                    txt = elem.get_text(" ", strip=True)
                if len(txt) >= 30 and not any(ign in txt.lower() for ign in ("open journal systems", "pkp", "login to", "register", "editorial team")):
                    if not ai_engine.is_boilerplate_noise(txt):
                        scope_candidate = txt
                        break

    # ── TIER 1C: Heading-based Lookup ────────────────────────────────────────
    if not scope_candidate:
        headings = clean_soup.find_all(["h1", "h2", "h3", "h4", "h5", "strong", "b"])
        for h in headings:
            h_text = h.get_text(" ", strip=True).lower()
            if any(term in h_text for term in ("focus and scope", "fokus dan ruang lingkup", "aims and scope", "ruang lingkup", "focus & scope", "aims & scope", "scope of the journal", "about the journal")):
                collected_p = []
                curr = h.next_sibling
                while curr and len(collected_p) < 3:
                    name_attr = getattr(curr, "name", None)
                    if name_attr in ("p", "div", "ul", "ol"):
                        p_t = curr.get_text(" ", strip=True)
                        if len(p_t) > 20 and not ai_engine.is_boilerplate_noise(p_t):
                            collected_p.append(p_t)
                    elif name_attr in ("h1", "h2", "h3", "h4"):
                        break
                    curr = curr.next_sibling
                if collected_p:
                    scope_candidate = " ".join(collected_p)
                    break
                parent_p = h.parent.find_all("p") if h.parent else []
                if len(parent_p) > 1:
                    cand_p = " ".join(p.get_text(" ", strip=True) for p in parent_p[1:3])
                    if not ai_engine.is_boilerplate_noise(cand_p):
                        scope_candidate = cand_p
                        break

    # ── TIER 2: OJS Homepage Context Parsing ────────────────────────────────
    if not scope_candidate and is_homepage:
        main_containers = clean_soup.select("#content, #main-content, div.page, div.pkp_structure_main, div.page_about, main, body")
        container = main_containers[0] if main_containers else clean_soup
        for p in container.find_all("p"):
            p_text = p.get_text(" ", strip=True)
            if 35 <= len(p_text) <= 800:
                p_low = p_text.lower()
                # Pastikan memuat kata kunci substantif jurnal ilmiah
                if any(k in p_low for k in ("jurnal", "journal", "menerbitkan", "publishes", "mempublikasikan", "penelitian", "research", "bidang", "field", "artikel", "article", "studi", "study", "fokus", "focus", "ruang lingkup", "scope", "cakupan", "ilmiah", "terbit", "kajian")):
                    # Abaikan boilerplate login/PKP dan menu navigasi
                    if not any(ign in p_low for ign in ("login to", "remember me", "forgot your password", "open journal systems", "pkp.sfu.ca", "developed by")):
                        if not ai_engine.is_boilerplate_noise(p_text):
                            scope_candidate = p_text
                            break

    return normalize_focus_and_scope(scope_candidate)


def _extract_apc_contextual(soup: BeautifulSoup) -> tuple[str, Optional[int]]:
    """
    Targeted DOM & Meta-Tag Scoping untuk APC v1.6.0:
    Mencari pada blok terstruktur (<div id="authorFees">, <table>, dsb)
    sebelum memindai teks mentah.
    """
    # 1. Cek elemen ber-ID / class spesifik (termasuk Sidebar / Custom Block OJS - VISION.md 3.1.1)
    targeted_selectors = [
        "#authorFees", ".author_fees", "#submissionFees", "#fees",
        "#contact", "table.listing", "div.sub_item", "#content",
        "div.pkp_block", "[id*='fee' i]", "[id*='apc' i]", "[class*='customblock' i]",
        "[class*='fee' i]", ".block_custom", ".custom_block",
    ]
    for sel in targeted_selectors:
        for block in soup.select(sel):
            block_text = sanitize_dom(BeautifulSoup(str(block), "lxml"))
            if any(kw in block_text.lower() for kw in APC_KEYWORDS):
                disp, amount = parse_apc_from_context(block_text)
                if disp != APC_UNKNOWN and disp != INFO_NOT_FOUND:
                    return disp, amount

    # 2. Cek tag p, td, div pendek
    for tag in soup.find_all(["p", "div", "td", "li", "section", "article"]):
        tag_text = tag.get_text(" ", strip=True)
        tag_lower = tag_text.lower()
        if any(kw in tag_lower for kw in APC_KEYWORDS) and len(tag_text) <= 800:
            disp, amount = parse_apc_from_context(tag_text)
            if disp != APC_UNKNOWN and disp != INFO_NOT_FOUND:
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
            if disp != APC_UNKNOWN and disp != INFO_NOT_FOUND:
                return disp, amount
            idx = full_lower.find(kw, idx + len(kw))

    return APC_UNKNOWN, None


def extract_ojs_deep_info(session: requests.Session, target_url: str, stop_event: Optional[threading.Event] = None) -> dict:
    """
    HUMAN-PARITY OJS DEEP CRAWLING (v1.6.0):
    - Dynamic Menu Anchor Discovery (pemindaian URL tersembunyi menu navigasi & sidebar).
    - Verifikasi Bulan Terbit via Halaman Arsip (/issue/archive & Current Issue).
    - Zero-Crash & Anti-Freeze: Stream 1.5MB Guard, Content-Type Guard, dan Multi-layered Exception Shield.
    - Dekoder Cloudflare email & pemeringkatan kontak redaksi resmi.
    """
    info = {
        "ojs_url": target_url,
        "email": INFO_NOT_FOUND,
        "telepon": INFO_NOT_FOUND,
        "frekuensi": INFO_NOT_FOUND,
        "apc_status": APC_UNKNOWN,
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
            info["email"] = INFO_NOT_FOUND
            info["telepon"] = INFO_NOT_FOUND
            info["status_server"] = "Non-Aktif"
            info["transparansi_kontak"] = "Tidak Transparan"
            _save_to_cache(target_url, info)
            return info

    t_start = time.monotonic()

    def _deadline_exceeded() -> bool:
        return (time.monotonic() - t_start) >= MAX_DEEP_SCRAPE_PER_JOURNAL_SEC

    def _finalize_scope_fallback():
        if not info.get("focus_scope") or info.get("focus_scope") == INFO_NOT_FOUND:
            try:
                stub = JournalEntry(
                    nama_jurnal=info.get("official_title") or effective_ojs_url or target_url,
                    penerbit=info.get("publisher") or "",
                    bidang=info.get("bidang") or INFO_NOT_FOUND,
                    focus_scope=info.get("focus_scope") or INFO_NOT_FOUND,
                    link_ojs=effective_ojs_url or target_url,
                )
                synth_txt, synth_src = synthesize_focus_scope(stub)
                info["focus_scope"] = synth_txt
                info["sumber_focus"] = synth_src
            except Exception:
                pass

    all_emails: list[str] = []
    all_phones: list[str] = []
    raw_frequency: str = ""
    effective_ojs_url: str = target_url
    freq_contexts: list[str] = []
    raw_about_parts: list[str] = []
    raw_subs_parts: list[str] = []

    try:
        # Pass 1: Halaman Utama OJS / Detail Garuda (Aman dari PDF & file besar via safe_fetch_html)
        status_code, html_text, pass1_headers = safe_fetch_html(session, target_url, timeout=DEEP_SCRAPE_TIMEOUT)
        if status_code == 200 and html_text:
            soup = BeautifulSoup(html_text, "lxml")

            # Follow HTTP redirect (v1.8.0): gunakan domain akhir hasil redirect (mis. journals.ums.ac.id -> journals2.ums.ac.id)
            if "garuda.kemdiktisaintek.go.id" not in target_url:
                final_url = pass1_headers.get("_final_url", "")
                if final_url and final_url.startswith("http"):
                    redir_root = cache_engine.normalize_ojs_root_url(final_url) or final_url
                    if redir_root != effective_ojs_url:
                        effective_ojs_url = redir_root
                        info["ojs_url"] = redir_root

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

                # Jika link OJS asli ditemukan dari Garuda, kunjungi beranda OJS-nya untuk mengikuti redirect & membaca menu Author Fees
                if effective_ojs_url != target_url and effective_ojs_url.startswith("http") and not _deadline_exceeded():
                    ojs_code, ojs_html, ojs_headers = safe_fetch_html(session, effective_ojs_url, timeout=DEEP_SCRAPE_TIMEOUT)
                    if ojs_code == 200 and ojs_html:
                        ojs_final = ojs_headers.get("_final_url", "")
                        if ojs_final and ojs_final.startswith("http"):
                            redir_root = cache_engine.normalize_ojs_root_url(ojs_final) or ojs_final
                            effective_ojs_url = redir_root
                            info["ojs_url"] = redir_root
                        html_text = ojs_html
                        soup = BeautifulSoup(html_text, "lxml")

            plain_text = sanitize_dom(soup)
            freq_contexts.append(plain_text[:2500])
            raw_about_parts.append(sanitize_dom(clean_soup_for_scope(soup)))

            # Evaluasi Status Server OJS
            if not _deadline_exceeded():
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

            # Kontak Primary Pass: Buang blok daftar artikel agar email penulis tidak bocor
            contact_soup = BeautifulSoup(html_text, "lxml")
            for noise_el in contact_soup.select(".obj_article_summary, .cmp_article_list, meta[name='citation_author_email']"):
                noise_el.decompose()
            e1, p1 = _extract_contacts_from_text(str(contact_soup), sanitize_dom(contact_soup), ojs_url=effective_ojs_url or target_url)
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
            if apc_disp not in (APC_UNKNOWN, INFO_NOT_FOUND):
                info["apc_status"] = apc_disp
                info["apc_amount"] = apc_num

            # TAHAP 1.1: DYNAMIC MENU ANCHOR DISCOVERY (Menemukan Halaman Tersembunyi OJS)
            clean_base = effective_ojs_url.rstrip("/")
            if clean_base.endswith("/index"):
                clean_base = clean_base[:-6]

            discovered_fee_links: list[str] = []
            discovered_contact_links: list[str] = []
            discovered_scope_links: list[str] = []
            discovered_archive_links: list[str] = []

            fee_kw = ("fee", "apc", "biaya", "charge", "payment", "tarif", "author fee", "publication fee")
            contact_kw = ("contact", "kontak", "hubungi", "about", "editorial team", "dewan redaksi", "pengelola")
            scope_kw = ("focus and scope", "focus & scope", "fokus dan ruang lingkup", "ruang lingkup", "aims and scope", "focus", "scope")
            archive_kw = ("issue/archive", "archive", "arsip", "terbitan", "issues", "past issues")

            # Pindai menu navigasi, sidebar, dan seluruh link anchor pada DOM
            candidate_anchors = soup.select(
                "nav a, .navbar a, #navMenu a, #navigation a, .pkp_navigation_primary a, "
                "#sidebar a, .pkp_structure_sidebar a, .block a, header a, footer a, a[href]"
            )
            for a_tag in candidate_anchors:
                h_val = (a_tag.get("href") or "").strip()
                txt_val = a_tag.get_text(" ", strip=True).lower()
                h_low = h_val.lower()

                if not h_val or any(h_low.endswith(ext) for ext in (".pdf", ".doc", ".docx", ".jpg", ".png", ".zip", ".rar", ".xml")):
                    continue

                full_h = requests.compat.urljoin(clean_base + "/", h_val)
                if not full_h.startswith("http") or "kemdiktisaintek" in full_h:
                    continue

                # 1. Discovery Biaya (APC)
                if any(k in txt_val for k in fee_kw) or any(k in h_low for k in ("fee", "apc", "biaya", "charge", "payment", "tarif", "/af", "author-fee")):
                    if full_h not in discovered_fee_links:
                        discovered_fee_links.append(full_h)

                # 2. Discovery Kontak
                if any(k in txt_val for k in contact_kw) or any(k in h_low for k in ("contact", "kontak", "hubungi", "about", "editorialteam", "people")):
                    if full_h not in discovered_contact_links:
                        discovered_contact_links.append(full_h)

                # 3. Discovery Scope
                if any(k in txt_val for k in scope_kw) or any(k in h_low for k in ("focus", "scope", "editorialpolicies")):
                    if full_h not in discovered_scope_links:
                        discovered_scope_links.append(full_h)

                # 4. Discovery Archive
                if any(k in txt_val for k in archive_kw) or any(k in h_low for k in ("issue/archive", "issue/current", "/archive")):
                    if full_h not in discovered_archive_links:
                        discovered_archive_links.append(full_h)

            # Prioritas rute penelusuran OJS terakselerasi (Fee & Submissions didahulukan agar tidak terpotong slice)
            subpaths: list[str] = []
            if info["apc_status"] in (APC_UNKNOWN, INFO_NOT_FOUND) and discovered_fee_links:
                subpaths.extend(discovered_fee_links[:2])

            subpaths.extend([
                f"{clean_base}/about/submissions",
                f"{clean_base}/about/editorialPolicies",
                f"{clean_base}/about/editorialPolicies#authorFees",
            ])

            if not all_emails and discovered_contact_links:
                subpaths.extend(discovered_contact_links[:2])
            if info["focus_scope"] == INFO_NOT_FOUND and discovered_scope_links:
                subpaths.extend(discovered_scope_links[:2])

            subpaths.extend([
                f"{clean_base}/about/editorialPolicies#focusAndScope",
                f"{clean_base}/about/focusAndScope",
                f"{clean_base}/about/contact",
                f"{clean_base}/about",
                f"{clean_base}/af",
                f"{clean_base}/about/editorialTeam",
            ])

            seen_sub = set()
            ordered_subpaths = []
            for sp in subpaths:
                if sp not in seen_sub and sp != effective_ojs_url:
                    seen_sub.add(sp)
                    ordered_subpaths.append(sp)

            def _has_complete_core_data() -> bool:
                has_apc = info["apc_status"] not in (APC_UNKNOWN, INFO_NOT_FOUND, "", DEAD_LINK_TEXT)
                has_contact = bool(all_emails or all_phones)
                if not (has_apc and has_contact):
                    return False
                quick_sched = month_parser.resolve_schedule_and_frequency(
                    raw_text=f"{' '.join(freq_contexts)} {raw_frequency}"
                )
                return bool(quick_sched.get("months_num"))

            if _has_complete_core_data() or _deadline_exceeded():
                ordered_subpaths = []

            # Helper untuk memproses hasil HTML subpath secara seragam
            def _ingest_subpath_html(sub_url: str, sub_html: str):
                nonlocal raw_frequency
                sub_soup = BeautifulSoup(sub_html, "lxml")
                sub_plain = sanitize_dom(sub_soup)
                freq_contexts.append(sub_plain[:2000])

                sub_url_low = sub_url.lower()
                if any(k in sub_url_low for k in ("submission", "author-fee", "author_fee", "/af", "/apc", "fee", "biaya", "charge")):
                    raw_subs_parts.append(sub_plain)
                if any(k in sub_url_low for k in ("about", "focus", "scope", "editorialpolicies")):
                    raw_about_parts.append(sanitize_dom(clean_soup_for_scope(sub_soup)))

                if info["focus_scope"] == INFO_NOT_FOUND:
                    s_scope = _extract_focus_and_scope_from_soup(sub_soup)
                    if s_scope != INFO_NOT_FOUND:
                        info["focus_scope"] = s_scope

                # Link fee khusus di halaman submissions (seperti /af)
                if (
                    info["apc_status"] in (APC_UNKNOWN, INFO_NOT_FOUND)
                    and "submissions" in sub_url
                    and not _deadline_exceeded()
                ):
                    for s_a in sub_soup.find_all("a", href=True):
                        if _deadline_exceeded() or (stop_event and stop_event.is_set()):
                            break
                        s_txt = s_a.get_text(" ", strip=True).lower()
                        s_h = s_a["href"].strip()
                        if any(k in s_txt for k in fee_kw) or any(k in s_h.lower() for k in ("/af", "/apc", "author-fee", "author_fee", "authorfees")):
                            target_fee_url = requests.compat.urljoin(clean_base + "/", s_h)
                            if target_fee_url != sub_url and target_fee_url.startswith("http"):
                                try:
                                    f_code, f_html, _ = safe_fetch_html(session, target_fee_url, timeout=DEEP_SCRAPE_TIMEOUT)
                                    if f_code == 200 and f_html:
                                        f_soup = BeautifulSoup(f_html, "lxml")
                                        raw_subs_parts.append(sanitize_dom(f_soup))
                                        f_apc, f_num = _extract_apc_contextual(f_soup)
                                        if f_apc not in (APC_UNKNOWN, INFO_NOT_FOUND):
                                            info["apc_status"] = f_apc
                                            info["apc_amount"] = f_num
                                            break
                                except Exception:
                                    pass

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

                sub_contact_soup = BeautifulSoup(sub_html, "lxml")
                for noise_el in sub_contact_soup.select(".obj_article_summary, .cmp_article_list, meta[name='citation_author_email']"):
                    noise_el.decompose()
                sub_e, sub_p = _extract_contacts_from_text(str(sub_contact_soup), sanitize_dom(sub_contact_soup), ojs_url=effective_ojs_url or target_url)
                for e in sub_e:
                    if e not in all_emails:
                        all_emails.append(e)
                for p in sub_p:
                    if p not in all_phones:
                        all_phones.append(p)

                if info["apc_status"] in (APC_UNKNOWN, INFO_NOT_FOUND):
                    s_apc, s_num = _extract_apc_contextual(sub_soup)
                    if s_apc not in (APC_UNKNOWN, INFO_NOT_FOUND):
                        info["apc_status"] = s_apc
                        info["apc_amount"] = s_num

                if not raw_frequency:
                    for pat in FREQ_PATTERNS:
                        m_f = pat.search(sub_plain)
                        if m_f:
                            raw_frequency = m_f.group(0).strip()
                            break

            # Tahap 2A: Ambil 2 sub-path prioritas (/about/submissions & /about/editorialPolicies atau link biaya) secara paralel
            if ordered_subpaths and not (stop_event and stop_event.is_set()) and not _deadline_exceeded():
                priority_urls = ordered_subpaths[:2]
                extra_urls = ordered_subpaths[2:4]  # Batas maksimum 2 sub-path tambahan setelah rute biaya utama

                def _fetch_sub(u: str):
                    if (stop_event and stop_event.is_set()) or _deadline_exceeded():
                        return u, 0, ""
                    w_sess = get_thread_local_session(is_deep_scrape=True)
                    sc, sh, _ = safe_fetch_html(w_sess, u, timeout=DEEP_SCRAPE_TIMEOUT)
                    return u, sc, sh

                sub_exec = ThreadPoolExecutor(max_workers=len(priority_urls))
                try:
                    sub_futs = [sub_exec.submit(_fetch_sub, u) for u in priority_urls]
                    rem_timeout = max(0.2, MAX_DEEP_SCRAPE_PER_JOURNAL_SEC - (time.monotonic() - t_start))
                    for sf in as_completed(sub_futs, timeout=rem_timeout):
                        if (stop_event and stop_event.is_set()) or _deadline_exceeded():
                            break
                        try:
                            u_done, sc, sh = sf.result()
                            if sc == 200 and sh:
                                _ingest_subpath_html(u_done, sh)
                        except Exception:
                            pass
                except Exception:
                    pass
                finally:
                    sub_exec.shutdown(wait=False, cancel_futures=True)

                # Tahap 2B: Kunjungi maksimum 2 sub-path tambahan hanya jika data inti belum lengkap & belum deadline
                for sub_url in extra_urls:
                    if (stop_event and stop_event.is_set()) or _deadline_exceeded() or _has_complete_core_data():
                        break
                    try:
                        s_code, sub_html, _ = safe_fetch_html(session, sub_url, timeout=DEEP_SCRAPE_TIMEOUT)
                        if s_code == 200 and sub_html:
                            _ingest_subpath_html(sub_url, sub_html)
                    except Exception:
                        pass
        else:
            if "kemdiktisaintek.go.id" not in target_url:
                info["apc_status"] = DEAD_LINK_TEXT
                info["status_server"] = "Non-Aktif"
    except Exception:
        if "kemdiktisaintek.go.id" not in target_url:
            info["apc_status"] = DEAD_LINK_TEXT
            info["status_server"] = "Non-Aktif"
            parsed = requests.compat.urlparse(target_url)
            netloc = (parsed.netloc or "").lower()
            if netloc:
                with _domain_reachability_lock:
                    _domain_reachability_cache[netloc] = False

    info["email"] = clean_contact_list(all_emails, contact_type="email", ojs_url=effective_ojs_url or target_url)
    info["telepon"] = clean_contact_list(all_phones, contact_type="phone")

    combined_about = " ".join(raw_about_parts).strip()
    combined_subs = " ".join(raw_subs_parts).strip() or combined_about
    info["raw_about"] = combined_about
    info["raw_submissions"] = combined_subs

    # TAHAP 1.2: VERIFIKASI BULAN TERBIT VIA HALAMAN ARSIP (/issue/archive)
    sched_info = month_parser.resolve_schedule_and_frequency(
        raw_text=f"{combined_about} {' '.join(freq_contexts)} {raw_frequency}"
    )
    detected_months = sched_info.get("months_num") or []

    # Jika month_parser belum menemukan bulan terbit dari beranda & about, cek arsip edisi
    if not detected_months and effective_ojs_url.startswith("http") and not (stop_event and stop_event.is_set()) and not _deadline_exceeded():
        archive_titles = []
        # 1. Coba ekstrak Current Issue di beranda
        if 'soup' in locals() and soup:
            for el in soup.select(".current_issue, #currentIssue, .obj_issue_toc, .issue-summary, .current"):
                t_c = el.get_text(" ", strip=True)
                if t_c:
                    archive_titles.append(t_c)

        # 2. Lightweight fetch ke {root_ojs_url}/issue/archive
        if not archive_titles:
            clean_b = effective_ojs_url.rstrip("/")
            if clean_b.endswith("/index"):
                clean_b = clean_b[:-6]
            archive_url = discovered_archive_links[0] if ('discovered_archive_links' in locals() and discovered_archive_links) else f"{clean_b}/issue/archive"
            try:
                arc_code, arc_html, _ = safe_fetch_html(session, archive_url, timeout=DEEP_SCRAPE_TIMEOUT)
                if arc_code == 200 and arc_html:
                    arc_soup = BeautifulSoup(arc_html, "lxml")
                    for el in arc_soup.select("a.title, .obj_issue_toc h2, .obj_issue_toc h3, .obj_issue_summary h2, .issue_toc h3, #issues a, .issue a, .series, h2, h3, h4"):
                        t_txt = el.get_text(" ", strip=True)
                        if any(k in t_txt.lower() for k in ("vol", "no.", "no ", "issue", "edisi", "nomor", "202", "201")):
                            archive_titles.append(t_txt)
            except Exception:
                pass

        if archive_titles:
            arc_months = month_parser.extract_months_from_archive_titles(archive_titles[:15])
            if arc_months:
                detected_months = arc_months

    if detected_months:
        info["bulan_terbit"] = detected_months
        full_names = [month_parser._MONTH_NAMES_FULL_ID.get(m, str(m)) for m in detected_months]
        cnt = len(detected_months)
        joined = ", ".join(full_names[:-1]) + f" & {full_names[-1]}" if cnt > 1 else full_names[0]
        info["frekuensi"] = f"{cnt} kali setahun ({joined})"
    else:
        info["bulan_terbit"] = []
        info["frekuensi"] = sched_info.get("frequency_str") or INFO_NOT_FOUND

    # TAHAP 1.3: FINALISASI STATUS APC (3-Status Accuracy & Smart Submit Fallback v1.6.2)
    if (info.get("biaya_submit") and info["biaya_submit"] > 50000) and (not info.get("biaya_publikasi") or info["biaya_publikasi"] == 0):
        info["biaya_publikasi"] = info["biaya_submit"]
        if info.get("apc_status") in (APC_UNKNOWN, INFO_NOT_FOUND, ""):
            info["apc_status"] = f"Rp {info['biaya_publikasi']:,}".replace(",", ".")
            info["apc_amount"] = info["biaya_publikasi"]

    if info["apc_status"] in (INFO_NOT_FOUND, ""):
        info["apc_status"] = APC_UNKNOWN
    if info["apc_status"] == "Gratis (Rp 0)":
        info["biaya_publikasi"] = 0
    elif info["apc_amount"] is not None and info["apc_amount"] > 0:
        info["biaya_publikasi"] = info["apc_amount"]

    if info["apc_status"] == DEAD_LINK_TEXT:
        info["status_server"] = "Non-Aktif"
    elif info["status_server"] == "Non-Aktif":
        info["status_server"] = get_domain_ssl_health(session, effective_ojs_url or target_url)

    info["transparansi_kontak"] = compute_contact_transparency(info["email"], info["telepon"])
    _finalize_scope_fallback()

    # Simpan ke cache lokal
    _save_to_cache(target_url, info)
    if effective_ojs_url != target_url:
        _save_to_cache(effective_ojs_url, info)

    return info


def merge_cross_source_entries(entries: list[JournalEntry]) -> list[JournalEntry]:
    """
    PENGGABUNGAN DATA LINTAS SUMBER & DEDUPLIKASI CERDAS v1.6.2:
    - Mengelompokkan jurnal berdasarkan Kunci Kanonikal (canonical_journal_key),
      ISSN / E-ISSN terstandarisasi, atau Host URL + path yang identik.
    - Menggabungkan variasi URL http:// vs https:// (memilih https://).
    - Memilih data yang paling lengkap (APC, Email, Telepon, Frekuensi, Bulan Terbit).
    - Smart Fallback: Biaya Submit > 50.000 otomatis menjadi Biaya Publikasi jika Biaya Publikasi kosong.
    - Menghapus duplikasi migran (seperti Balance UM Palembang).
    """
    if not entries:
        return []

    merged_list: list[JournalEntry] = []

    def _clean_url_key(u: str) -> str:
        if not u or not str(u).startswith("http"):
            return ""
        clean = re.sub(r"^https?://", "", str(u).lower().strip()).rstrip("/")
        clean = re.sub(r"/index(\.php)?.*$", "", clean)
        return clean

    def _is_article_title(title: str) -> bool:
        if not title:
            return False
        clean = title.strip()
        if len(clean) > 55 and (" " in clean):
            art_words = ("pada", "tinjauan", "analisis", "pengaruh", "studi", "implementasi", "terhadap", "konsep", "dan", "dalam")
            matched = sum(1 for w in art_words if f" {w} " in f" {clean.lower()} ")
            if matched >= 2 or clean.isupper():
                return True
        return False

    for j in entries:
        if not j.nama_jurnal or not j.nama_jurnal.strip():
            continue

        # Standarisasi nilai awal
        j.issn = normalize_issn(j.issn)
        j.e_issn = normalize_issn(j.e_issn)
        j.bidang = clean_and_merge_subjects(j.bidang)

        # Smart fallback Biaya Submit -> Biaya Publikasi
        if (j.biaya_submit and j.biaya_submit > 50000) and (j.biaya_publikasi is None or j.biaya_publikasi == 0):
            j.biaya_publikasi = j.biaya_submit
            if j.apc_status in (APC_UNKNOWN, INFO_NOT_FOUND, ""):
                j.apc_status = f"Rp {j.biaya_publikasi:,}".replace(",", ".")

        j_canon = canonical_journal_key(j.nama_jurnal)
        j_url_key = _clean_url_key(j.link_ojs)
        j_issn = j.issn if j.issn != INFO_NOT_FOUND else ""
        j_eissn = j.e_issn if j.e_issn != INFO_NOT_FOUND else ""

        # Cari existing match
        match_idx = -1
        for idx, ex in enumerate(merged_list):
            ex_canon = canonical_journal_key(ex.nama_jurnal)
            ex_url_key = _clean_url_key(ex.link_ojs)
            ex_issn = ex.issn if ex.issn != INFO_NOT_FOUND else ""
            ex_eissn = ex.e_issn if ex.e_issn != INFO_NOT_FOUND else ""

            # 1. Match ISSN / E-ISSN yang valid
            if (j_issn and j_issn in (ex_issn, ex_eissn)) or (j_eissn and j_eissn in (ex_issn, ex_eissn)):
                match_idx = idx
                break

            # 2. Match Canonical Title (misal: 'balancejurnalakuntansidanbisnis')
            if j_canon and ex_canon and (j_canon == ex_canon):
                match_idx = idx
                break

            # 3. Match URL key (jika host dan path sama, misal migrasi http ke https)
            if j_url_key and ex_url_key and (j_url_key == ex_url_key) and "kemdiktisaintek" not in j_url_key:
                match_idx = idx
                break

        if match_idx == -1:
            merged_list.append(j)
        else:
            existing = merged_list[match_idx]

            # Pilih nama jurnal yang lebih baik (bukan judul artikel)
            if _is_article_title(existing.nama_jurnal) and not _is_article_title(j.nama_jurnal):
                existing.nama_jurnal = j.nama_jurnal
            elif len(j.nama_jurnal) < len(existing.nama_jurnal) and not _is_article_title(j.nama_jurnal) and existing.nama_jurnal.isupper():
                existing.nama_jurnal = j.nama_jurnal

            # Pilih Link OJS https:// dan yang bukan kemdiktisaintek
            if (not existing.link_ojs or "kemdiktisaintek" in existing.link_ojs) and (j.link_ojs and "kemdiktisaintek" not in j.link_ojs):
                existing.link_ojs = j.link_ojs
            elif existing.link_ojs.startswith("http://") and j.link_ojs.startswith("https://"):
                existing.link_ojs = j.link_ojs

            # Penerbit
            if (existing.penerbit == INFO_NOT_FOUND or not existing.penerbit) and j.penerbit != INFO_NOT_FOUND:
                existing.penerbit = j.penerbit

            # ISSN / E-ISSN
            if existing.issn == INFO_NOT_FOUND and j.issn != INFO_NOT_FOUND:
                existing.issn = j.issn
            if existing.e_issn == INFO_NOT_FOUND and j.e_issn != INFO_NOT_FOUND:
                existing.e_issn = j.e_issn

            # Bidang Keilmuan
            existing.bidang = clean_and_merge_subjects(existing.bidang, j.bidang)

            # Peringkat SINTA
            if (existing.peringkat_sinta == INFO_NOT_FOUND or not existing.peringkat_sinta) and j.peringkat_sinta != INFO_NOT_FOUND:
                existing.peringkat_sinta = j.peringkat_sinta
            if not existing.sinta_profile_url and j.sinta_profile_url:
                existing.sinta_profile_url = j.sinta_profile_url

            # Biaya Publikasi / APC
            if (existing.apc_status in (INFO_NOT_FOUND, APC_UNKNOWN) or existing.apc_amount is None) and j.apc_amount is not None:
                existing.apc_status = j.apc_status
                existing.apc_amount = j.apc_amount
                existing.biaya_publikasi = j.biaya_publikasi
                existing.biaya_submit = j.biaya_submit
                existing.sumber_apc = j.sumber_apc

            # Smart Fallback Biaya Submit -> Biaya Publikasi
            if (existing.biaya_submit and existing.biaya_submit > 50000) and (existing.biaya_publikasi is None or existing.biaya_publikasi == 0):
                existing.biaya_publikasi = existing.biaya_submit
                if existing.apc_status in (APC_UNKNOWN, INFO_NOT_FOUND, ""):
                    existing.apc_status = f"Rp {existing.biaya_publikasi:,}".replace(",", ".")

            # Email Redaksi (Gabungkan dan rapikan, max 3)
            all_emails = []
            if existing.email != INFO_NOT_FOUND:
                all_emails.extend([e.strip() for e in existing.email.split(",") if e.strip()])
            if j.email != INFO_NOT_FOUND:
                all_emails.extend([e.strip() for e in j.email.split(",") if e.strip()])
            if all_emails:
                existing.email = clean_contact_list(all_emails, contact_type="email", ojs_url=existing.link_ojs)

            # Telepon / WA (Gabungkan dan rapikan, max 2)
            all_phones = []
            if existing.telepon != INFO_NOT_FOUND:
                all_phones.extend([p.strip() for p in existing.telepon.split(",") if p.strip()])
            if j.telepon != INFO_NOT_FOUND:
                all_phones.extend([p.strip() for p in j.telepon.split(",") if p.strip()])
            if all_phones:
                existing.telepon = clean_contact_list(all_phones, contact_type="phone")

            # Frekuensi & Bulan Terbit
            if not existing.bulan_terbit and j.bulan_terbit:
                existing.bulan_terbit = j.bulan_terbit
                existing.frekuensi = j.frekuensi
            elif existing.frekuensi == INFO_NOT_FOUND and j.frekuensi != INFO_NOT_FOUND:
                existing.frekuensi = j.frekuensi

            # Focus & Scope
            if (not existing.focus_scope or existing.focus_scope == INFO_NOT_FOUND or is_focus_scope_noise(existing.focus_scope)) and j.focus_scope != INFO_NOT_FOUND and not is_focus_scope_noise(j.focus_scope):
                existing.focus_scope = j.focus_scope
                existing.sumber_focus = j.sumber_focus

            # Status Server
            if existing.status_server in ("Non-Aktif", INFO_NOT_FOUND) and j.status_server not in ("Non-Aktif", INFO_NOT_FOUND):
                existing.status_server = j.status_server

            # Transparansi Kontak
            existing.transparansi_kontak = compute_contact_transparency(existing.email, existing.telepon)

            # Match Score
            if getattr(j, "match_score", 0) > getattr(existing, "match_score", 0):
                existing.match_score = j.match_score
                existing.kesesuaian_naskah = j.kesesuaian_naskah

            # Sumber
            if j.sumber and j.sumber not in existing.sumber:
                existing.sumber = f"{existing.sumber} + {j.sumber}"

    # Finalisasi per entri
    for entry in merged_list:
        if not entry.transparansi_kontak or entry.transparansi_kontak == "Tidak Transparan":
            entry.transparansi_kontak = compute_contact_transparency(entry.email, entry.telepon)
        if not entry.focus_scope or entry.focus_scope == INFO_NOT_FOUND or is_focus_scope_noise(entry.focus_scope):
            synth_text, synth_src = synthesize_focus_scope(entry)
            entry.focus_scope = synth_text
            entry.sumber_focus = synth_src

    return merged_list


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
                    res["ojs_url"] = cache_engine.normalize_ojs_root_url(h)
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
                                    entry.link_ojs = cache_engine.normalize_ojs_root_url(href_a)
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
    progress_cb=None,
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
            response = session.get(
                url,
                params=params,
                headers=get_browser_headers(referer="https://garuda.kemdiktisaintek.go.id/"),
                timeout=20,
                verify=False,
            )

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

        # Adaptive Candidate Pool Multiplier (v2.1.0)
        pool_target = compute_candidate_pool_limit(max_results)
        if sinta_ranks:
            rank_labels = [f"S{r}" for r in sinta_ranks] + [f"Sinta {r}" for r in sinta_ranks] + [str(r) for r in sinta_ranks]
            cur_matching = [
                j for j in all_journals
                if any(r in j.peringkat_sinta for r in rank_labels)
                or j.peringkat_sinta == INFO_NOT_FOUND
            ]
            if len(cur_matching) >= max(max_results, 40) or len(all_journals) >= pool_target:
                console.print(f"[dim]   Pool kandidat Garuda memadai ({len(cur_matching)} lolos SINTA / {len(all_journals)} total).[/dim]")
                break
        else:
            if len(all_journals) >= pool_target:
                console.print(f"[dim]   Target pool {pool_target} kandidat Garuda tercapai.[/dim]")
                break

        if callable(progress_cb):
            try:
                progress_cb(
                    phase="fase1_portal",
                    done=page,
                    total=0,
                    current_item=f"Garuda '{keyword}' — Hal. {page} ({len(all_journals)} jurnal)",
                )
            except Exception:
                pass

        if turbo_mode and max_results >= 100:
            time.sleep(0.02)
        else:
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

    # Adaptive Candidate Pool Multiplier (v2.1.0)
    pool_limit = compute_candidate_pool_limit(max_results) if deep_detail else max_results
    if len(filtered) > pool_limit:
        filtered = filtered[:pool_limit]

    console.print(f"\n[bold]📊 Hasil Garuda: {len(filtered)} jurnal dihimpun[/bold]")

    if filtered and deep_detail:
        filtered = deep_enrich_entries(
            filtered,
            keyword=keyword,
            max_results=max_results,
            turbo_mode=turbo_mode,
            free_only=free_only,
            include_unknown_apc=include_unknown_apc,
            progress_cb=progress_cb,
        )
    else:
        try:
            from doc_reader import calculate_keyword_match_score
            for entry in filtered:
                if entry.match_score <= 0 or not entry.kesesuaian_naskah:
                    sc = calculate_keyword_match_score(keyword, entry)
                    entry.match_score = sc.get("score", 40)
                    entry.kesesuaian_naskah = sc.get("level", "Cukup Sesuai")
        except Exception:
            pass

    return filtered


def deep_enrich_entries(
    entries: list[JournalEntry],
    keyword: str = "",
    max_results: int = MAX_RESULTS,
    turbo_mode: bool = True,
    free_only: bool = False,
    include_unknown_apc: bool = True,
    progress_cb=None,
    checkpoint_cb=None,
    cancel_event: Optional[threading.Event] = None,
) -> list[JournalEntry]:
    """
    Deep scraping OJS paralel terpadu (Garuda + SINTA) setelah Pre-Deep Deduplication (v2.1.0).
    Menjamin setiap jurnal unik hanya di-scrape situs OJS-nya SATU KALI dengan 5.5s hard deadline
    dan memicu checkpoint_cb setiap 50 jurnal selesai untuk streaming & incremental auto-save.
    """
    if not entries:
        return []

    active_workers = TURBO_SCRAPE_WORKERS if turbo_mode else NORMAL_SCRAPE_WORKERS
    console.print(
        f"[bold cyan]🔍 Menjalankan Hyper-Speed Deep Scraping ({len(entries)} jurnal unik) "
        f"[{active_workers} workers | Deadline {MAX_DEEP_SCRAPE_PER_JOURNAL_SEC}s/jurnal]...[/bold cyan]"
    )

    stop_event = cancel_event or threading.Event()
    matched_lock = threading.Lock()
    matched_count = 0
    deep_done = 0
    deep_total = len(entries)
    enriched_buffer: list[JournalEntry] = []

    try:
        from scraper_sinta import resolve_sinta_profile_meta, resolve_garuda_profile_meta
    except Exception:
        resolve_sinta_profile_meta = None
        resolve_garuda_profile_meta = None

    try:
        with ThreadPoolExecutor(max_workers=active_workers) as executor:
            def _do_unified_deep(entry: JournalEntry) -> JournalEntry:
                nonlocal matched_count
                if stop_event.is_set():
                    return entry
                try:
                    worker_sess = get_thread_local_session(is_deep_scrape=True)

                    # Resolusi URL OJS bila entri berasal dari SINTA tanpa link_ojs langsung
                    if (
                        resolve_sinta_profile_meta is not None
                        and (not entry.link_ojs or "kemdiktisaintek.go.id" in entry.link_ojs)
                        and getattr(entry, "sinta_profile_url", "")
                    ):
                        p_meta = resolve_sinta_profile_meta(worker_sess, entry.sinta_profile_url)
                        if p_meta.get("ojs_url"):
                            entry.link_ojs = p_meta["ojs_url"]
                        if entry.issn == INFO_NOT_FOUND and p_meta.get("issn") and p_meta.get("issn") != INFO_NOT_FOUND:
                            entry.issn = p_meta["issn"]
                        if entry.e_issn == INFO_NOT_FOUND and p_meta.get("e_issn") and p_meta.get("e_issn") != INFO_NOT_FOUND:
                            entry.e_issn = p_meta["e_issn"]
                        if p_meta.get("subject") and p_meta.get("subject") != INFO_NOT_FOUND:
                            entry.bidang = clean_and_merge_subjects(entry.bidang, p_meta["subject"])

                    res = extract_ojs_deep_info(worker_sess, entry.link_ojs, stop_event=stop_event)
                    entry.link_ojs = res.get("ojs_url", entry.link_ojs)
                    if res.get("official_title") and (len(entry.nama_jurnal) > 55 or entry.nama_jurnal.isupper()):
                        entry.nama_jurnal = res["official_title"]

                    entry.email = res.get("email", INFO_NOT_FOUND)
                    entry.telepon = res.get("telepon", INFO_NOT_FOUND)
                    entry.frekuensi = res.get("frekuensi", INFO_NOT_FOUND)
                    entry.apc_status = res.get("apc_status", APC_UNKNOWN)
                    entry.apc_amount = res.get("apc_amount", None)
                    if res.get("biaya_submit"):
                        entry.biaya_submit = res["biaya_submit"]

                    if (entry.biaya_submit and entry.biaya_submit > 50000) and (entry.biaya_publikasi is None or entry.biaya_publikasi == 0):
                        entry.biaya_publikasi = entry.biaya_submit
                        if entry.apc_status in (APC_UNKNOWN, INFO_NOT_FOUND, ""):
                            entry.apc_status = f"Rp {entry.biaya_publikasi:,}".replace(",", ".")

                    raw_scope = res.get("focus_scope", INFO_NOT_FOUND)
                    if (
                        resolve_garuda_profile_meta is not None
                        and (not raw_scope or raw_scope == INFO_NOT_FOUND)
                        and getattr(entry, "garuda_profile_url", "")
                    ):
                        g_meta = resolve_garuda_profile_meta(worker_sess, entry.garuda_profile_url)
                        if g_meta.get("focus_scope"):
                            raw_scope = g_meta["focus_scope"]

                    if not raw_scope or raw_scope == INFO_NOT_FOUND or "tidak ditemukan" in str(raw_scope).lower() or is_focus_scope_noise(raw_scope):
                        synth_text, synth_src = synthesize_focus_scope(entry)
                        entry.focus_scope = synth_text
                        entry.sumber_focus = synth_src
                    else:
                        cleaned_scope = strip_ojs_profile_table_noise(raw_scope)
                        if is_focus_scope_noise(cleaned_scope):
                            synth_text, synth_src = synthesize_focus_scope(entry)
                            entry.focus_scope = synth_text
                            entry.sumber_focus = synth_src
                        else:
                            entry.focus_scope = cleaned_scope
                            entry.sumber_focus = res.get("sumber_focus", "Regex") or "Regex"

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

                    if entry.apc_status == "Gratis (Rp 0)" or entry.apc_amount == 0:
                        entry.biaya_publikasi = 0
                    elif entry.apc_amount is not None and entry.apc_amount > 0:
                        entry.biaya_publikasi = entry.apc_amount
                    elif (entry.biaya_submit and entry.biaya_submit > 50000) and (entry.biaya_publikasi is None or entry.biaya_publikasi == 0):
                        entry.biaya_publikasi = entry.biaya_submit

                    with matched_lock:
                        if free_only:
                            apc_low = entry.apc_status.lower()
                            is_free = (entry.apc_amount == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("free" in apc_low)
                            is_unk = (entry.apc_amount is None) or (entry.apc_status in (INFO_NOT_FOUND, APC_UNKNOWN))
                            if is_free or (is_unk and include_unknown_apc):
                                matched_count += 1
                        else:
                            matched_count += 1
                        if matched_count >= max_results:
                            stop_event.set()
                except Exception:
                    pass
                return entry

            futures = [executor.submit(_do_unified_deep, entry) for entry in entries]
            for f in as_completed(futures):
                try:
                    done_entry = f.result()
                except Exception:
                    done_entry = None
                deep_done += 1
                if done_entry is not None:
                    enriched_buffer.append(done_entry)
                if callable(progress_cb):
                    try:
                        progress_cb(
                            phase="fase2_deep",
                            done=deep_done,
                            total=deep_total,
                            current_item=(done_entry.nama_jurnal if done_entry else f"Deep crawl {deep_done}/{deep_total}"),
                            entry=done_entry,
                        )
                    except Exception:
                        pass
                if callable(checkpoint_cb) and deep_done % 50 == 0 and enriched_buffer:
                    try:
                        checkpoint_cb(done=deep_done, total=deep_total, entries=list(enriched_buffer))
                    except Exception:
                        pass
    except Exception:
        pass

    try:
        from doc_reader import calculate_keyword_match_score
        for entry in entries:
            if entry.match_score <= 0 or not entry.kesesuaian_naskah:
                sc = calculate_keyword_match_score(keyword, entry)
                entry.match_score = sc.get("score", 40)
                entry.kesesuaian_naskah = sc.get("level", "Cukup Sesuai")
    except Exception:
        pass

    return entries


def scrape_garuda(
    keyword: str,
    sinta_ranks: Optional[list[int]] = None,
    deep_detail: bool = False,
    max_results: int = MAX_RESULTS,
    turbo_mode: bool = True,
    free_only: bool = False,
    include_unknown_apc: bool = True,
    progress_cb=None,
) -> list[JournalEntry]:
    """
    Scraping Portal Garuda dengan Pre-Deep Deduplication lintas multi-keyword (v2.1.0).
    Mengumpulkan metadata dasar seluruh kata kunci lalu menggabungkan duplikat SEBELUM deep scrape.
    """
    keywords = parse_search_keywords(keyword)
    session = get_high_capacity_session()

    if len(keywords) > 1:
        console.print(f"\n[bold cyan]🔍 Deteksi multi-keyword adaptif ({len(keywords)} topik): {', '.join(keywords)}[/bold cyan]")
        all_raw: list[JournalEntry] = []
        pool_limit = compute_candidate_pool_limit(max_results)
        per_kw_limit = max(20, int(pool_limit / max(1, len(keywords))) + 20)
        for kw in keywords:
            if len(merge_cross_source_entries(all_raw)) >= pool_limit:
                break
            kw_raw = _scrape_garuda_single_keyword(
                kw,
                sinta_ranks=sinta_ranks,
                deep_detail=False,
                max_results=per_kw_limit,
                turbo_mode=turbo_mode,
                session=session,
                free_only=free_only,
                include_unknown_apc=include_unknown_apc,
                progress_cb=progress_cb,
            )
            all_raw.extend(kw_raw)
        merged_unique = merge_cross_source_entries(all_raw)[:pool_limit]
        if deep_detail and merged_unique:
            merged_unique = deep_enrich_entries(
                merged_unique,
                keyword=keyword,
                max_results=max_results,
                turbo_mode=turbo_mode,
                free_only=free_only,
                include_unknown_apc=include_unknown_apc,
                progress_cb=progress_cb,
            )
        return merged_unique[:max_results]

    return _scrape_garuda_single_keyword(
        keyword,
        sinta_ranks=sinta_ranks,
        deep_detail=deep_detail,
        max_results=max_results,
        turbo_mode=turbo_mode,
        session=session,
        free_only=free_only,
        include_unknown_apc=include_unknown_apc,
        progress_cb=progress_cb,
    )

