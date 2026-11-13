# =============================================================================
# JSFS - Journal Scraper For Students
# config.py — Konfigurasi global scraper v2.0.1 (Visual-First WebView2 UI + Selectolax)
# =============================================================================

import os
import re
import sys
from datetime import datetime

APP_VERSION = "2.0.1"


def get_app_dir() -> str:
    """Mengembalikan direktori aplikasi yang aman (di luar _MEIPASS saat frozen)."""
    if getattr(sys, "frozen", False):
        return os.path.dirname(os.path.abspath(sys.executable))
    return os.path.dirname(os.path.abspath(__file__))

# --- Target URL ---
GARUDA_BASE_URL = "https://garuda.kemdiktisaintek.go.id"
GARUDA_JOURNAL_URL = f"{GARUDA_BASE_URL}/journal"
SINTA_BASE_URL = "https://sinta.kemdiktisaintek.go.id"
SINTA_JOURNAL_URL = f"{SINTA_BASE_URL}/journals"

import random

# --- Request Headers (imitasi browser modern Chrome 130+ Windows) ---
USER_AGENTS = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
]

HEADERS = {
    "User-Agent": USER_AGENTS[2],
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
    "Accept-Language": "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7",
    "Connection": "keep-alive",
}


def get_browser_headers(referer: str = "") -> dict:
    """Mengembalikan header browser modern dengan rotasi User-Agent dan Referer opsional."""
    headers = dict(HEADERS)
    headers["User-Agent"] = random.choice(USER_AGENTS)
    if referer:
        headers["Referer"] = referer
    return headers

# --- Delay antar request (detik) ---
REQUEST_DELAY_MIN = 0.1  # detik minimum (akselerasi turbo)
REQUEST_DELAY_MAX = 0.4  # detik maksimum

# --- Batas hasil scraping & Skalabilitas Paginasi ---
MAX_PAGES = 5            # default batas halaman dasar
MAX_PAGES_LIMIT = 200    # batas maksimum penjelajahan halaman untuk target 250 - 1000 jurnal
MAX_RESULTS = 50         # default batas total jurnal yang dikembalikan
TARGET_OPTIONS = ["10", "50", "100", "250", "500", "1000"]

# --- Filter peringkat SINTA yang relevan untuk mahasiswa ---
DEFAULT_SINTA_RANKS = [3, 4, 5, 6]

# --- Standarisasi Nilai Fallback & URL ---
INFO_NOT_FOUND = "Informasi tidak ditemukan dalam situs jurnal"
DEAD_LINK_TEXT = "Situs tidak dapat diakses"

# --- Parameter Konversi Mata Uang & Batas Kewajaran APC ---
# Kurs acuan tetap: 1 USD = Rp 17.500
USD_TO_IDR_RATE = 17500
APC_SANITY_MIN_IDR = 50_000
APC_SANITY_MAX_IDR = 15_000_000

# --- Sistem Caching Metadata OJS Lokal (TTL 7 Hari) ---
CACHE_DIR = os.path.join(get_app_dir(), ".cache")
OJS_CACHE_FILE = os.path.join(CACHE_DIR, "ojs_cache.json")
CACHE_TTL_DAYS = 7
CACHE_TTL_SECONDS = CACHE_TTL_DAYS * 24 * 3600

# --- Alokasi Hardware & Thread Pool Konkuren Maksimal (v1.5.1 Concurrency Throttling) ---
# Disesuaikan agar CPU dan network stack tidak saturasi yang memicu Windows WM_PAINT starvation
TURBO_SCRAPE_WORKERS = min(64, (os.cpu_count() or 4) * 4)
NORMAL_SCRAPE_WORKERS = min(32, (os.cpu_count() or 4) * 2)
DEEP_SCRAPE_WORKERS = TURBO_SCRAPE_WORKERS

# --- High-Capacity Connection Pooling & Adaptive Timeout (v1.5.1 Speed Acceleration) ---
HTTP_POOL_CONNECTIONS = 64
HTTP_POOL_MAXSIZE = 64
HTTP_TIMEOUT = 5  # Toleransi 5 detik untuk penelusuran portal SINTA / Garuda
DEEP_SCRAPE_TIMEOUT = (2.0, 3.5)  # (connect: 2.0s, read: 3.5s) timeout diperketat untuk OJS
FAST_FAIL_TIMEOUT = (2.0, 2.5)    # (connect: 2.0s, read: 2.5s) fast-fail circuit breaker OJS domain

# --- Output Directory & Naming ---
OUTPUT_DIR = "output"
OUTPUT_MD_FILENAME = "hasil_pencarian_jsfs.md"
OUTPUT_CSV_FILENAME = "hasil_pencarian_jsfs.csv"
OUTPUT_EXCEL_FILENAME = "hasil_pencarian_jsfs.xlsx"


def generate_output_filename(keyword: str, ext: str, prefix: str = "JSFS") -> str:
    """
    Membuat nama berkas output unik dengan format:
    JSFS_<keyword_clean>_<YYYYMMDD_HHMMSS>.<ext>
    Contoh: JSFS_bisnis_20261016_221500.xlsx
    """
    clean_kw = re.sub(r"[^\w\s-]", "", keyword or "jurnal").strip()
    clean_kw = re.sub(r"[-\s]+", "_", clean_kw).lower() or "jurnal"
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    clean_ext = ext.lstrip(".")
    return f"{prefix}_{clean_kw}_{timestamp}.{clean_ext}"
