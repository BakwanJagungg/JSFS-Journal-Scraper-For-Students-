"""
cache_engine.py — Smart SQLite WAL Cache Engine v1.7.0
Menyediakan penyimpanan cache lokal persisten berbasis SQLite mode WAL
untuk mempercepat scraping berulang, menghemat bandwidth OJS,
dan mengurangi beban inferensi LLM Ollama.
Dilengkapi Incomplete Cache Bypass & Auto-Purge Versioning.
"""
from __future__ import annotations

import atexit
import json
import logging
import os
import re
import sqlite3
import sys
import threading
import time
from typing import Any, Dict, Optional
from urllib.parse import urlparse

log = logging.getLogger("jsfs.cache")

CACHE_VERSION = "2.0.0"


def _ver_tuple(v: Optional[str]) -> tuple[int, ...]:
    """Konversi string versi 'X.Y.Z' menjadi tuple integer untuk perbandingan aman."""
    if not v:
        return (0,)
    try:
        return tuple(int(p) for p in str(v).strip().split(".") if p.isdigit())
    except Exception:
        return (0,)


def get_app_dir() -> str:
    """Mengembalikan direktori aplikasi yang aman (di luar _MEIPASS saat frozen)."""
    if getattr(sys, "frozen", False):
        return os.path.dirname(os.path.abspath(sys.executable))
    return os.path.dirname(os.path.abspath(__file__))


DEFAULT_CACHE_DIR = os.path.join(get_app_dir(), ".cache")
DEFAULT_DB_FILE = os.path.join(DEFAULT_CACHE_DIR, "jsfs_cache.db")
DEFAULT_CACHE_TTL_DAYS = 14
DEFAULT_CACHE_TTL_SECONDS = DEFAULT_CACHE_TTL_DAYS * 24 * 3600


def normalize_ojs_root_url(url: str) -> str:
    """
    Normalisasi URL OJS untuk mengembalikan root path jurnal yang bersih:
    Memotong sub-halaman seperti /about/..., /issue/..., /article/..., /submissions/...,
    /editorialTeam, /editorialPolicies, /contact, /login, /user/..., /announcement/..., /search/...
    sehingga selalu mengarah ke beranda/root jurnal yang sesungguhnya.
    Contoh:
    http://ejournal.iainkendari.ac.id/index.php/lifalah/about/editorialTeam
    -> http://ejournal.iainkendari.ac.id/index.php/lifalah
    """
    if not url or not isinstance(url, str):
        return ""
    u = url.strip()
    if not u.startswith("http://") and not u.startswith("https://"):
        u = f"http://{u}"

    try:
        parsed = urlparse(u)
        scheme = parsed.scheme.lower()
        netloc = parsed.netloc.lower()
        path = parsed.path.rstrip("/")

        # Pola sub-path OJS yang harus dipotong dari root URL
        patterns = [
            r"/about(?:/.*)?$",
            r"/issue(?:/.*)?$",
            r"/article(?:/.*)?$",
            r"/submissions(?:/.*)?$",
            r"/submission(?:/.*)?$",
            r"/editorialTeam(?:/.*)?$",
            r"/editorialPolicies(?:/.*)?$",
            r"/contact(?:/.*)?$",
            r"/login(?:/.*)?$",
            r"/user(?:/.*)?$",
            r"/announcement(?:/.*)?$",
            r"/search(?:/.*)?$",
            r"/af(?:/.*)?$",
            r"/index$",
        ]
        for pat in patterns:
            path = re.sub(pat, "", path, flags=re.IGNORECASE).rstrip("/")

        return f"{scheme}://{netloc}{path}"
    except Exception:
        return u.rstrip("/")


def normalize_ojs_url(url: str) -> str:
    """
    Normalisasi URL OJS untuk dijadikan kunci unik di tabel cache:
    - Lowercase scheme dan host (netloc).
    - Hapus trailing slash.
    - Hapus sub-halaman OJS umum sehingga variasi URL dari satu jurnal mengarah ke 1 baris cache.
    """
    root = normalize_ojs_root_url(url)
    if not root:
        return ""
    try:
        parsed = urlparse(root)
        scheme = parsed.scheme.lower()
        netloc = parsed.netloc.lower()
        path = parsed.path.rstrip("/")
        # Bersihkan juga index.php jika di akhir path untuk key cache seragam
        path = re.sub(r"/index\.php$", "", path, flags=re.IGNORECASE).rstrip("/")
        return f"{scheme}://{netloc}{path}"
    except Exception:
        return root.lower().rstrip("/")


class JSFSCacheEngine:
    """
    Mesin Cache SQLite WAL untuk JSFS v1.5.0:
    - Thread-safe dengan threading.Lock() pada operasi tulis.
    - journal_mode=WAL dan synchronous=NORMAL untuk throughput tinggi tanpa lock contention.
    - TTL 14 hari untuk data jurnal OJS.
    """

    def __init__(self, db_path: str = DEFAULT_DB_FILE):
        self.db_path = db_path
        self._lock = threading.RLock()
        self._initialized = False

    def _open_raw_connection(self) -> sqlite3.Connection:
        """Membuka koneksi SQLite terisolasi per thread tanpa memicu inisialisasi berulang."""
        conn = sqlite3.connect(
            self.db_path,
            timeout=10.0,
            check_same_thread=False,
        )
        conn.row_factory = sqlite3.Row
        return conn

    def _ensure_ready(self) -> None:
        """Inisialisasi direktori & skema SQLite secara malas (lazy) pada akses pertama."""
        if not self._initialized:
            with self._lock:
                if not self._initialized:
                    self._ensure_db()
                    self._initialized = True

    def _get_connection(self) -> sqlite3.Connection:
        """Membuka koneksi SQLite terisolasi per thread."""
        self._ensure_ready()
        return self._open_raw_connection()

    def _ensure_db(self) -> None:
        """Membuat folder cache dan skema tabel jika belum ada."""
        db_dir = os.path.dirname(self.db_path)
        if db_dir:
            os.makedirs(db_dir, exist_ok=True)

        with self._lock:
            try:
                conn = self._open_raw_connection()
                try:
                    conn.execute("PRAGMA journal_mode=WAL;")
                    conn.execute("PRAGMA synchronous=NORMAL;")
                    conn.execute("""
                        CREATE TABLE IF NOT EXISTS journal_cache (
                            ojs_url_key TEXT PRIMARY KEY,
                            journal_name TEXT,
                            sinta_rank TEXT,
                            focus_scope TEXT,
                            source_focus TEXT,
                            apc_submit TEXT,
                            apc_publish TEXT,
                            currency TEXT,
                            source_apc TEXT,
                            months_json TEXT,
                            frequency_str TEXT,
                            email_clean TEXT,
                            phone_clean TEXT,
                            server_status TEXT,
                            updated_at INTEGER
                        );
                    """)
                    conn.execute("""
                        CREATE INDEX IF NOT EXISTS idx_journal_cache_name
                        ON journal_cache(journal_name);
                    """)
                    conn.execute("""
                        CREATE TABLE IF NOT EXISTS cache_metadata (
                            key TEXT PRIMARY KEY,
                            value TEXT
                        );
                    """)
                    # Mekanisme Auto-Purge jika versi cache sebelumnya < CACHE_VERSION (v1.8.0)
                    cur = conn.cursor()
                    cur.execute("SELECT value FROM cache_metadata WHERE key = 'cache_version'")
                    v_row = cur.fetchone()
                    installed_version = v_row[0] if v_row else None
                    if installed_version is None or _ver_tuple(installed_version) < _ver_tuple(CACHE_VERSION):
                        log.info("Auto-purge cache SQLite OJS: upgrade dari versi %s ke %s...", installed_version, CACHE_VERSION)
                        conn.execute("DELETE FROM journal_cache;")
                        conn.execute("INSERT OR REPLACE INTO cache_metadata (key, value) VALUES ('cache_version', ?);", (CACHE_VERSION,))
                    conn.commit()
                finally:
                    conn.close()
            except Exception as e:
                log.error("Gagal menginisialisasi SQLite cache: %s", e)

    def get_journal(self, url: str, max_age_days: int = DEFAULT_CACHE_TTL_DAYS, allow_incomplete: bool = False) -> Optional[dict]:
        """
        Mengambil entri jurnal dari cache jika belum kedaluwarsa.
        Jika allow_incomplete=False (default), entri dengan biaya_publikasi kosong
        atau sumber_apc 'Kosong' akan di-bypass untuk di-deep scrape ulang (Incomplete Cache Bypass).
        Mengembalikan dict data jurnal atau None jika cache miss / kedaluwarsa / incomplete.
        """
        url_key = normalize_ojs_url(url)
        if not url_key:
            return None

        ttl_seconds = max_age_days * 24 * 3600
        now = int(time.time())

        try:
            conn = self._get_connection()
            try:
                cur = conn.cursor()
                cur.execute(
                    "SELECT * FROM journal_cache WHERE ojs_url_key = ?",
                    (url_key,),
                )
                row = cur.fetchone()
                if not row:
                    return None

                updated_at = row["updated_at"] or 0
                if (now - updated_at) > ttl_seconds:
                    log.debug("Cache expired untuk %s (umur: %d hari)", url_key, (now - updated_at) // 86400)
                    return None

                # Incomplete Cache Bypass (VISION.md 3.1.4):
                # Jurnal dianggap belum lengkap dan WAJIB di-bypass jika biaya belum terdata
                apc_pub = row["apc_publish"]
                s_apc = row["source_apc"]
                if not allow_incomplete:
                    if (
                        not apc_pub
                        or apc_pub in ("", "—", "-", "Cek Web / Belum Terdata", "None", "Informasi tidak ditemukan dalam situs jurnal")
                        or s_apc in ("Kosong", "", "None", None)
                    ):
                        log.debug("Cache incomplete untuk %s (APC: %s, Source: %s) -> bypass cache", url_key, apc_pub, s_apc)
                        return None

                months = []
                if row["months_json"]:
                    try:
                        months = json.loads(row["months_json"])
                    except Exception:
                        months = []

                # Konversi angka APC bila valid
                apc_amount = None
                if apc_pub:
                    if apc_pub in ("0", "Gratis (0)", "Free"):
                        apc_amount = 0
                    else:
                        digits = re.sub(r"\D", "", apc_pub)
                        if digits:
                            try:
                                apc_amount = int(digits)
                            except ValueError:
                                pass

                return {
                    "ojs_url_key": row["ojs_url_key"],
                    "journal_name": row["journal_name"] or "",
                    "sinta_rank": row["sinta_rank"] or "",
                    "focus_scope": row["focus_scope"] or "",
                    "source_focus": row["source_focus"] or "Regex",
                    "apc_submit": row["apc_submit"] or "",
                    "apc_publish": row["apc_publish"] or "",
                    "currency": row["currency"] or "IDR",
                    "source_apc": row["source_apc"] or "Regex",
                    "months": months,
                    "frequency_str": row["frequency_str"] or "",
                    "email_clean": row["email_clean"] or "",
                    "phone_clean": row["phone_clean"] or "",
                    "server_status": row["server_status"] or "Aktif (HTTP)",
                    "updated_at": updated_at,
                    "apc_amount": apc_amount,
                }
            finally:
                conn.close()
        except Exception as e:
            log.warning("Galat saat membaca SQLite cache (%s): %s", url_key, e)
            return None

    def upsert_journal(self, url: str, data: dict) -> bool:
        """
        Menyimpan atau memperbarui data jurnal ke dalam SQLite cache.
        Aman dipanggil secara konkuren dari multi-thread worker.
        """
        url_key = normalize_ojs_url(url)
        if not url_key or not isinstance(data, dict):
            return False

        # Jangan simpan jika data kosong sama sekali
        if not data.get("journal_name") and not data.get("email") and not data.get("apc_status"):
            return False

        months_list = data.get("bulan_terbit") or data.get("months") or []
        months_json = json.dumps(sorted(list(set(months_list))))

        apc_submit = data.get("apc_submit")
        if apc_submit is None and data.get("biaya_submit") is not None:
            apc_submit = str(data.get("biaya_submit"))

        apc_publish = data.get("apc_publish")
        if apc_publish is None:
            if data.get("biaya_publikasi") is not None:
                apc_publish = str(data.get("biaya_publikasi"))
            elif data.get("apc_amount") is not None:
                apc_publish = str(data.get("apc_amount"))
            elif data.get("apc_status"):
                apc_publish = str(data.get("apc_status"))

        now = int(time.time())

        with self._lock:
            try:
                conn = self._get_connection()
                try:
                    conn.execute("""
                        INSERT INTO journal_cache (
                            ojs_url_key, journal_name, sinta_rank, focus_scope,
                            source_focus, apc_submit, apc_publish, currency,
                            source_apc, months_json, frequency_str, email_clean,
                            phone_clean, server_status, updated_at
                        ) VALUES (
                            ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                        )
                        ON CONFLICT(ojs_url_key) DO UPDATE SET
                            journal_name=excluded.journal_name,
                            sinta_rank=excluded.sinta_rank,
                            focus_scope=excluded.focus_scope,
                            source_focus=excluded.source_focus,
                            apc_submit=excluded.apc_submit,
                            apc_publish=excluded.apc_publish,
                            currency=excluded.currency,
                            source_apc=excluded.source_apc,
                            months_json=excluded.months_json,
                            frequency_str=excluded.frequency_str,
                            email_clean=excluded.email_clean,
                            phone_clean=excluded.phone_clean,
                            server_status=excluded.server_status,
                            updated_at=excluded.updated_at;
                    """, (
                        url_key,
                        str(data.get("nama_jurnal") or data.get("journal_name") or ""),
                        str(data.get("peringkat_sinta") or data.get("sinta_rank") or ""),
                        str(data.get("focus_scope") or ""),
                        str(data.get("sumber_focus") or data.get("source_focus") or "Regex"),
                        str(apc_submit or ""),
                        str(apc_publish or ""),
                        str(data.get("mata_uang") or data.get("currency") or "IDR"),
                        str(data.get("sumber_apc") or data.get("source_apc") or "Regex"),
                        months_json,
                        str(data.get("frekuensi") or data.get("frequency_str") or ""),
                        str(data.get("email") or data.get("email_clean") or ""),
                        str(data.get("telepon") or data.get("phone_clean") or ""),
                        str(data.get("status_server") or data.get("server_status") or "Aktif (HTTP)"),
                        now,
                    ))
                    conn.commit()
                    return True
                finally:
                    conn.close()
            except Exception as e:
                log.warning("Gagal menulis SQLite cache (%s): %s", url_key, e)
                return False

    def cleanup_expired(self, max_age_days: int = DEFAULT_CACHE_TTL_DAYS) -> int:
        """Menghapus entri cache yang kedaluwarsa (> max_age_days) dan mengembalikan jumlah terhapus."""
        cutoff = int(time.time()) - max(1, int(max_age_days)) * 24 * 3600
        with self._lock:
            try:
                conn = self._get_connection()
                try:
                    cur = conn.execute("DELETE FROM journal_cache WHERE updated_at < ?", (cutoff,))
                    deleted = cur.rowcount if cur.rowcount and cur.rowcount > 0 else 0
                    conn.commit()
                    return deleted
                finally:
                    conn.close()
            except Exception as e:
                log.warning("Gagal membersihkan entri kedaluwarsa: %s", e)
                return 0

    def get_stats(self) -> dict:
        """Mengembalikan statistik ukuran dan isi database cache."""
        try:
            conn = self._get_connection()
            try:
                cur = conn.cursor()
                cur.execute("SELECT COUNT(*) FROM journal_cache")
                total = cur.fetchone()[0]
                cur.execute("SELECT COUNT(*) FROM journal_cache WHERE source_focus = 'AI'")
                ai_count = cur.fetchone()[0]
                size_bytes = os.path.getsize(self.db_path) if os.path.exists(self.db_path) else 0
                return {
                    "total_entries": total,
                    "valid_entries": total,
                    "ai_enriched_count": ai_count,
                    "db_path": self.db_path,
                    "size_bytes": size_bytes,
                }
            finally:
                conn.close()
        except Exception as e:
            return {"error": str(e), "total_entries": 0, "valid_entries": 0}

    def clear_cache(self) -> bool:
        """Menghapus seluruh entri dalam database cache."""
        with self._lock:
            try:
                conn = self._get_connection()
                try:
                    conn.execute("DELETE FROM journal_cache;")
                    conn.commit()
                    conn.execute("VACUUM;")
                    return True
                finally:
                    conn.close()
            except Exception as e:
                log.error("Gagal membersihkan cache: %s", e)
                return False


# Singleton instans default
cache_engine = JSFSCacheEngine()


def close_all_connections():
    """
    Menutup dan melakukan checkpoint TRUNCATE pada SQLite WAL
    agar tidak ada file lock (-wal / -shm) tersisa saat aplikasi ditutup.
    """
    try:
        if os.path.exists(cache_engine.db_path):
            conn = sqlite3.connect(cache_engine.db_path, timeout=5.0)
            try:
                conn.execute("PRAGMA wal_checkpoint(TRUNCATE);")
                conn.commit()
            finally:
                conn.close()
    except Exception as e:
        log.warning("Gagal menutup koneksi SQLite cache: %s", e)


atexit.register(close_all_connections)

