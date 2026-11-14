# =============================================================================
# JSFS - Journal Scraper For Students
# exporter.py — Modul ekspor hasil ke Markdown, CSV, dan Excel v1.4.0 (Hybrid AI Engine)
# =============================================================================

from __future__ import annotations

import os
import re
import threading
from datetime import datetime
from typing import Optional, Union, List
from concurrent.futures import ThreadPoolExecutor
import requests
import urllib3
from rich.console import Console

# Nonaktifkan peringatan SSL tidak aman
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

from config import (
    APP_VERSION,
    OUTPUT_DIR,
    OUTPUT_MD_FILENAME,
    OUTPUT_CSV_FILENAME,
    OUTPUT_EXCEL_FILENAME,
    INFO_NOT_FOUND,
    DEAD_LINK_TEXT,
    HEADERS,
    DEEP_SCRAPE_WORKERS,
    HTTP_TIMEOUT,
    generate_output_filename,
)
from scraper_garuda import (
    JournalEntry,
    sanitize_text,
    clean_contact_list,
    get_whatsapp_link,
)
import month_parser
from month_parser import format_bulan_terbit, get_next_issue_estimate

console = Console()


def _ensure_output_dir():
    """Memastikan direktori penyimpanan output tersedia."""
    os.makedirs(OUTPUT_DIR, exist_ok=True)


def _format_amount(val: Optional[int], currency: Optional[str] = "IDR", fallback: str = "—") -> str:
    """Format nominal angka menjadi format terstandarisasi Rp / USD / Gratis (Rp 0) / fallback."""
    if val is None:
        return fallback
    if val == 0:
        return "Gratis (Rp 0)"
    cur = (currency or "IDR").upper()
    if cur == "USD":
        return f"${val:,} USD"
    return f"Rp {val:,}".replace(",", ".")


def _get_entry_val(obj: any, *keys: str, default: any = None) -> any:
    """Mengambil nilai atribut atau dict key secara aman dari JournalEntry atau dict."""
    if isinstance(obj, dict):
        for k in keys:
            if k in obj and obj[k] is not None:
                return obj[k]
        return default
    for k in keys:
        if hasattr(obj, k):
            v = getattr(obj, k)
            if v is not None:
                return v
    return default


def generate_copypaste_list(journals: list) -> str:
    """
    FITUR COPY-PASTE LIST RINGKAS (FORMAT SHARE GRUP) v1.6.0:
    Menghasilkan daftar bernomor siap salin dengan format standar share WhatsApp / Telegram:
    {No}. {Nama Jurnal} ({SINTA} - {Penerbit})
    Email: ...
    Telepon/WA: ...
    Biaya Publikasi: ... | Terbit: ...
    Link OJS: ...
    """
    if not journals:
        return ""

    entries: list[str] = []
    for idx, j in enumerate(journals, 1):
        name = _get_entry_val(j, "nama_jurnal") or _get_entry_val(j, "judul") or "Jurnal Ilmiah"
        sinta = _get_entry_val(j, "peringkat_sinta", default="Sinta -")
        penerbit = _get_entry_val(j, "penerbit", default="")

        if penerbit and penerbit != INFO_NOT_FOUND and "tidak ditemukan" not in str(penerbit).lower():
            header = f"{idx}. {name} ({sinta} - {penerbit})"
        else:
            header = f"{idx}. {name} ({sinta})"

        lines = [header]

        # Email
        email = _get_entry_val(j, "email", default="")
        if email and email != INFO_NOT_FOUND and "tidak dapat diakses" not in str(email).lower() and "tidak ditemukan" not in str(email).lower():
            lines.append(f"Email: {email}")

        # Telepon / WA
        tel = _get_entry_val(j, "telepon", default="")
        if tel and tel != INFO_NOT_FOUND and "tidak dapat diakses" not in str(tel).lower() and "tidak ditemukan" not in str(tel).lower():
            lines.append(f"Telepon/WA: {tel}")

        # Biaya Publikasi (Smart Fallback Biaya Submit v1.6.2)
        b_pub = _get_entry_val(j, "biaya_publikasi", default=None)
        if b_pub is None:
            b_pub = _get_entry_val(j, "apc_amount", default=None)
        b_sub = _get_entry_val(j, "biaya_submit", default=None)
        if (b_pub is None or b_pub == 0) and (b_sub is not None and b_sub > 50000):
            b_pub = b_sub

        cur = _get_entry_val(j, "mata_uang", default="IDR") or "IDR"
        apc_status = _get_entry_val(j, "apc_status", default="")

        if b_pub is not None:
            if b_pub == 0:
                biaya_str = "Gratis (Rp 0)"
            elif cur.upper() == "USD":
                biaya_str = f"${b_pub:,} USD"
            else:
                biaya_str = f"Rp {b_pub:,}".replace(",", ".")
        elif apc_status and ("gratis" in str(apc_status).lower() or "rp 0" in str(apc_status).lower()):
            biaya_str = "Gratis (Rp 0)"
        elif apc_status and any(c.isdigit() for c in str(apc_status)) and "tidak dapat diakses" not in str(apc_status).lower():
            biaya_str = str(apc_status)
        else:
            biaya_str = "Cek Web / Belum Terdata"

        # Jadwal Terbit (Nama Bulan Lengkap, mis: 'Juni, November')
        months = _get_entry_val(j, "bulan_terbit", default=[])
        norm_m = month_parser._normalize_month_list(months)
        if norm_m:
            terbit_str = ", ".join(month_parser._MONTH_NAMES_FULL_ID.get(m, str(m)) for m in norm_m)
        else:
            freq = _get_entry_val(j, "frekuensi", default="")
            if freq and freq != INFO_NOT_FOUND and "tidak dapat diakses" not in str(freq).lower() and "tidak ditemukan" not in str(freq).lower():
                terbit_str = str(freq)
            else:
                terbit_str = "Belum Terdata"

        lines.append(f"Biaya Publikasi: {biaya_str} | Terbit: {terbit_str}")

        # Fokus Keilmuan (Ringkas, Terjemahan Bersih & Anti-Noise) v1.6.2
        fokus_clean = ""
        f_scope = _get_entry_val(j, "focus_scope", default="")
        try:
            from scraper_garuda import is_focus_scope_noise
        except Exception:
            is_focus_scope_noise = lambda x: False

        # 1. Coba kalimat deskriptif jika bersih dan bebas derau
        if f_scope and f_scope != INFO_NOT_FOUND and "tidak ditemukan" not in str(f_scope).lower() and not is_focus_scope_noise(str(f_scope)):
            f_txt = re.sub(r"\s+", " ", str(f_scope)).strip()
            first_sent = re.split(r"(?<=[.!?])\s+", f_txt)[0].strip()
            if 20 <= len(first_sent) <= 110:
                fokus_clean = first_sent
            elif len(f_txt) <= 110:
                fokus_clean = f_txt

        # 2. Jika belum ada atau terlalu panjang, gunakan Bidang Keilmuan (Subject Area) dengan terjemahan rapi
        if not fokus_clean:
            raw_bidang = _get_entry_val(j, "bidang", default="")
            if raw_bidang and raw_bidang != INFO_NOT_FOUND and "tidak ditemukan" not in str(raw_bidang).lower():
                s_txt = str(raw_bidang)
                # Normalisasi frasa majemuk SINTA sebelum di-split oleh koma
                s_txt = re.sub(r"decision\s*sciences,\s*operations\s*research\s*(?:&|and)\s*management", "Manajemen & Riset Operasi", s_txt, flags=re.I)
                s_txt = re.sub(r"economics,\s*econometrics\s*(?:&|and)\s*finance", "Ekonomi & Keuangan", s_txt, flags=re.I)
                s_txt = re.sub(r"business,\s*management\s*(?:&|and)\s*accounting", "Bisnis & Akuntansi", s_txt, flags=re.I)
                s_txt = re.sub(r"agricultural\s*(?:&|and)\s*biological\s*sciences", "Pertanian & Hayati", s_txt, flags=re.I)
                s_txt = re.sub(r"social\s*sciences", "Ilmu Sosial", s_txt, flags=re.I)
                s_txt = re.sub(r"computer\s*science", "Ilmu Komputer", s_txt, flags=re.I)

                term_map = {
                    "manajemen & riset operasi": "Manajemen & Riset Operasi",
                    "ekonomi & keuangan": "Ekonomi & Keuangan",
                    "bisnis & akuntansi": "Bisnis & Akuntansi",
                    "pertanian & hayati": "Pertanian & Hayati",
                    "ilmu sosial": "Ilmu Sosial",
                    "ilmu komputer": "Ilmu Komputer",
                    "accounting": "Akuntansi",
                    "management": "Manajemen",
                    "finance": "Keuangan",
                    "economics": "Ekonomi",
                    "economy": "Ekonomi",
                    "education": "Pendidikan",
                    "law": "Hukum",
                    "engineering": "Teknik",
                    "health professions": "Kesehatan",
                    "agricultural and biological sciences": "Pertanian",
                    "islamic studies": "Studi Islam",
                    "religious studies": "Agama",
                }

                parts = re.split(r"[,;|/]+", s_txt)
                mapped_terms: list[str] = []
                j_title_low = str(name).lower()
                is_biz_econ = any(k in j_title_low for k in ("bisnis", "business", "ekonomi", "economic", "manajemen", "management", "akuntansi", "accounting", "keuangan", "finance"))

                for p in parts:
                    p_clean = p.strip()
                    if not p_clean or len(p_clean) < 3:
                        continue
                    p_low = p_clean.lower()

                    # Jika jurnal bisnis/ekonomi, buang anomali pertanian/teknik sipil
                    if is_biz_econ and any(ign in p_low for ign in ("agriculture", "biological", "forestry", "civil engineering", "mechanical", "pertanian", "hayati")):
                        continue

                    t_id = term_map.get(p_low)
                    if not t_id:
                        for k_term, v_term in term_map.items():
                            if k_term in p_low:
                                t_id = v_term
                                break
                    final_t = t_id if t_id else p_clean.title()
                    if final_t not in mapped_terms:
                        mapped_terms.append(final_t)

                if mapped_terms:
                    fokus_clean = ", ".join(mapped_terms[:4])

        # 3. Fallback jika masih kosong
        if not fokus_clean and f_scope and f_scope != INFO_NOT_FOUND and not is_focus_scope_noise(str(f_scope)):
            f_txt = re.sub(r"\s+", " ", str(f_scope)).strip()
            if len(f_txt) > 105:
                fokus_clean = f_txt[:102].rsplit(" ", 1)[0].rstrip(" ,.-") + "..."
            else:
                fokus_clean = f_txt

        if fokus_clean:
            if len(fokus_clean) > 110:
                fokus_clean = fokus_clean[:107].rsplit(" ", 1)[0].rstrip(" ,.-") + "..."
            lines.append(f"Fokus: {fokus_clean}")

        # Link OJS
        link = _get_entry_val(j, "link_ojs", default="") or _get_entry_val(j, "link", default="")
        if link and str(link).startswith("http"):
            lines.append(f"Link OJS: {link}")
        else:
            lines.append("Link OJS: —")

        entries.append("\n".join(lines))

    return "\n\n".join(entries)


def validate_urls_async(urls: list[str]) -> dict[str, bool]:
    """
    VALIDASI HYPERLINK SSL-TOLERANT (v1.3.2):
    - Mengabaikan galat sertifikat SSL kampus lokal (.ac.id) dengan verify=False.
    - Mengikuti lokasi redirect (allow_redirects=True).
    - Timeout adaptif untuk server kampus.
    - Menandai 'Situs tidak dapat diakses' HANYA jika server merespons 404, 500, atau DNS unreachable.
    Returns: dict {url: is_accessible (bool)}
    """
    results: dict[str, bool] = {}
    valid_targets = [u for u in set(urls) if u and str(u).startswith("http")]

    if not valid_targets:
        return results

    console.print(f"[dim]🔍 Memvalidasi {len(valid_targets)} link OJS (toleransi SSL & timeout: {HTTP_TIMEOUT}s)...[/dim]")

    def _check_single(u: str) -> tuple[str, bool]:
        try:
            resp = requests.get(
                u,
                timeout=HTTP_TIMEOUT,
                stream=True,
                headers={"User-Agent": HEADERS["User-Agent"]},
                verify=False,
                allow_redirects=True,
            )
            if resp.status_code in (404, 500, 502, 503):
                return u, False
            return u, True
        except (requests.exceptions.SSLError, requests.exceptions.Timeout):
            return u, True
        except Exception:
            return u, False

    with ThreadPoolExecutor(max_workers=DEEP_SCRAPE_WORKERS) as executor:
        futures = [executor.submit(_check_single, u) for u in valid_targets]
        for f in futures:
            u, is_ok = f.result()
            results[u] = is_ok

    return results


def select_top_recommendations(journals: list[JournalEntry], top_n: int = 3) -> list[dict]:
    """
    SISTEM REKOMENDASI JURNAL CERDAS (AI ADVISOR - v1.5.0):
    Memberi peringkat jurnal berdasarkan rumus komposit:
    AdvisorScore = (0.40 * MatchScore) + (0.25 * FreeBonus) + (0.20 * FastPublishBonus) + (0.15 * SintaBonus)
    
    Returns:
    list[dict] berisi:
    {
        "entry": JournalEntry,
        "score": float,
        "reason": str,
        "terbit_terdekat": str,
    }
    """
    if not journals:
        return []

    scored_items = []
    curr_month = datetime.now().month

    def _get_entry_val(obj: any, *keys: str, default: any = None) -> any:
        if isinstance(obj, dict):
            for k in keys:
                if k in obj and obj[k] is not None:
                    return obj[k]
            return default
        for k in keys:
            if hasattr(obj, k):
                v = getattr(obj, k)
                if v is not None:
                    return v
        return default

    for j in journals:
        # 1. MatchScore (0 - 100)
        m_score = _get_entry_val(j, "match_score", default=0) or 0
        if m_score <= 0:
            m_score = 50.0  # Baseline skor netral jika naskah tidak diunggah

        # 2. FreeBonus (0 - 100)
        b_pub = _get_entry_val(j, "biaya_publikasi", "apc_amount")
        b_pub_num = None
        if b_pub is not None:
            try:
                b_pub_num = float(b_pub)
            except (ValueError, TypeError):
                pass

        apc_str = str(_get_entry_val(j, "apc_status", default="")).lower()
        if (b_pub_num is not None and b_pub_num == 0) or "gratis" in apc_str or "free" in apc_str or "rp 0" in apc_str:
            free_bonus = 100.0
        elif b_pub is None or _get_entry_val(j, "apc_status", default="") == INFO_NOT_FOUND:
            free_bonus = 50.0  # Belum Terdata
        else:
            if b_pub_num is not None:
                free_bonus = max(10.0, min(80.0, 100.0 - (b_pub_num / 50000.0)))
            else:
                free_bonus = 50.0

        # 3. FastPublishBonus (0 - 100)
        months = _get_entry_val(j, "bulan_terbit", default=[]) or []
        valid_m = [m for m in months if isinstance(m, int) and 1 <= m <= 12]
        delta = None
        if valid_m:
            cands = [m for m in valid_m if m >= curr_month]
            if cands:
                delta = min(cands) - curr_month
            else:
                delta = (12 - curr_month) + min(valid_m)

            if delta in (0, 1, 2):
                fast_bonus = 100.0
            elif delta in (3, 4):
                fast_bonus = 70.0
            else:
                fast_bonus = 30.0
        else:
            fast_bonus = 40.0

        # 4. SintaBonus (0 - 100)
        rank_str = str(_get_entry_val(j, "peringkat_sinta", "sinta", default="")).upper()
        if any(r in rank_str for r in ("S1", "S2", "SINTA 1", "SINTA 2")):
            sinta_bonus = 100.0
        elif "S3" in rank_str or "SINTA 3" in rank_str:
            sinta_bonus = 85.0
        elif "S4" in rank_str or "SINTA 4" in rank_str:
            sinta_bonus = 75.0
        elif any(r in rank_str for r in ("S5", "S6", "SINTA 5", "SINTA 6")):
            sinta_bonus = 60.0
        else:
            sinta_bonus = 40.0

        advisor_score = (
            (0.40 * m_score)
            + (0.25 * free_bonus)
            + (0.20 * fast_bonus)
            + (0.15 * sinta_bonus)
        )

        terbit_est = _get_entry_val(j, "terbit_terdekat")
        if not terbit_est:
            terbit_est = get_next_issue_estimate(_get_entry_val(j, "bulan_terbit", default=[]))

        # Generator alasan rekomendasi otomatis
        reason_parts = []
        sinta_val = _get_entry_val(j, "peringkat_sinta", "sinta", default="")
        if sinta_val and sinta_val != INFO_NOT_FOUND:
            reason_parts.append(f"berperingkat {sinta_val}")
        if free_bonus == 100.0:
            reason_parts.append("terkonfirmasi Gratis (Rp 0)")
        elif b_pub_num is not None and b_pub_num > 0:
            cur_val = _get_entry_val(j, "mata_uang", "currency", default=None)
            reason_parts.append(f"biaya publikasi {_format_amount(int(b_pub_num), cur_val)}")
        else:
            reason_parts.append("biaya publikasi belum terdata")

        m_sc = _get_entry_val(j, "match_score", default=0)
        if m_sc and float(m_sc) > 0:
            reason_parts.append(f"kecocokan topik {m_sc}%")

        if terbit_est and terbit_est != "—":
            reason_parts.append(f"jadwal terbit terdekat pada {terbit_est}")

        reason = "Direkomendasikan karena " + ", ".join(reason_parts) + "."
        title_val = _get_entry_val(j, "nama_jurnal", "title", default="")

        scored_items.append({
            "entry": j,
            "title": title_val,
            "score": round(advisor_score, 1),
            "advisor_score": round(advisor_score, 1),
            "reason": reason,
            "terbit_terdekat": terbit_est,
        })

    scored_items.sort(key=lambda x: x["score"], reverse=True)
    return scored_items[:top_n]


def _ensure_entry_enrichment(journals: list[JournalEntry], keyword: str = "") -> None:
    """Memastikan match_score, focus_scope, dan penyelarasan APC terisi 100% sebelum diekspor (v1.6.2)."""
    try:
        from doc_reader import calculate_keyword_match_score
    except Exception:
        calculate_keyword_match_score = None

    try:
        from scraper_garuda import synthesize_focus_scope, is_focus_scope_noise
    except Exception:
        synthesize_focus_scope = None
        is_focus_scope_noise = lambda x: False

    for j in journals:
        # 1. Match score & Kesesuaian
        if calculate_keyword_match_score and keyword and (getattr(j, "match_score", 0) <= 0 or not getattr(j, "kesesuaian_naskah", "")):
            try:
                sc = calculate_keyword_match_score(keyword, j)
                j.match_score = sc.get("score", 40)
                j.kesesuaian_naskah = sc.get("level", "Cukup Sesuai")
            except Exception:
                pass

        # 2. Smart Fallback Biaya Submit -> Biaya Publikasi (v1.6.2)
        b_sub = getattr(j, "biaya_submit", None)
        b_pub = getattr(j, "biaya_publikasi", None)
        if (b_sub and b_sub > 50000) and (b_pub is None or b_pub == 0):
            j.biaya_publikasi = b_sub
            if getattr(j, "apc_status", "") in (APC_UNKNOWN, INFO_NOT_FOUND, ""):
                j.apc_status = f"Rp {b_sub:,}".replace(",", ".")

        # 3. Focus & Scope synthesizer (bebas noise)
        f_scope = getattr(j, "focus_scope", "")
        has_noise = is_focus_scope_noise(f_scope) if f_scope else False
        if synthesize_focus_scope and (not f_scope or f_scope == INFO_NOT_FOUND or "tidak ditemukan" in str(f_scope).lower() or has_noise):
            try:
                synth_text, synth_src = synthesize_focus_scope(j)
                j.focus_scope = synth_text
                j.sumber_focus = synth_src
            except Exception:
                pass


def _build_fast_url_status(journals: list[JournalEntry]) -> dict[str, bool]:
    """Menentukan status tautan OJS secara instan dari hasil deep-crawl tanpa probe HTTP ulang."""
    status_map: dict[str, bool] = {}
    for j in journals:
        url = getattr(j, "link_ojs", "") or ""
        if url and str(url).startswith("http"):
            srv_low = str(getattr(j, "status_server", "") or "").lower()
            status_map[url] = not ("non-aktif" in srv_low or "mati" in srv_low)
    return status_map


def _resolve_export_url_status(journals: list[JournalEntry], skip_url_validation: bool) -> dict[str, bool]:
    """Gunakan status instan saat skip_url_validation=True, atau validasi jaringan bila diminta."""
    if skip_url_validation:
        return _build_fast_url_status(journals)
    all_urls = [j.link_ojs for j in journals if j.link_ojs and j.link_ojs.startswith("http")]
    return validate_urls_async(all_urls)


def _clean_entry_value(val: Optional[str]) -> str:
    """Helper untuk memastikan semua nilai kosong diubah ke INFO_NOT_FOUND."""
    return sanitize_text(val)


def _journals_to_dataframe(journals: list[JournalEntry], url_status_map: dict[str, bool]) -> pd.DataFrame:
    """Konversi list JournalEntry ke Pandas DataFrame lengkap v1.5.0."""
    import pandas as pd

    has_match_score = any(getattr(j, "match_score", 0) > 0 for j in journals)
    data = []
    for i, j in enumerate(journals):
        link_val = j.link_ojs
        if not link_val or not str(link_val).startswith("http"):
            resolved_link = INFO_NOT_FOUND
        elif url_status_map.get(link_val, False) is False:
            resolved_link = DEAD_LINK_TEXT
        else:
            resolved_link = link_val

        m_score = getattr(j, "match_score", 0) or 0
        kesesuaian = getattr(j, "kesesuaian_naskah", "") or "—"
        skor_str = f"{int(m_score)}%" if m_score > 0 else "—"

        row = {
            "No": i + 1,
            "Nama Jurnal": _clean_entry_value(j.nama_jurnal),
            "Peringkat SINTA": _clean_entry_value(j.peringkat_sinta),
            "Kesesuaian Naskah": kesesuaian,
            "Match Score (%)": skor_str,
        }

        cur = getattr(j, "mata_uang", None) or "IDR"
        b_sub = getattr(j, "biaya_submit", None)
        b_pub = getattr(j, "biaya_publikasi", None)
        if b_pub is None and getattr(j, "apc_amount", None) is not None:
            b_pub = getattr(j, "apc_amount", None)

        row.update({
            "Penerbit": _clean_entry_value(j.penerbit),
            "Biaya Submit": _format_amount(b_sub, cur),
            "Biaya Publikasi": _format_amount(b_pub, cur),
            "Mata Uang": getattr(j, "mata_uang", None) or "—",
            "Bulan Terbit": format_bulan_terbit(getattr(j, "bulan_terbit", [])),
            "Terbit Terdekat": get_next_issue_estimate(getattr(j, "bulan_terbit", [])),
            "Sumber Focus": getattr(j, "sumber_focus", "Regex") or "Regex",
            "Sumber APC": getattr(j, "sumber_apc", "Regex") or "Regex",
            "Frekuensi Terbit": _clean_entry_value(getattr(j, "frekuensi", INFO_NOT_FOUND)),
            "Email Kontak": _clean_entry_value(getattr(j, "email", INFO_NOT_FOUND)),
            "Telepon / WA": _clean_entry_value(getattr(j, "telepon", INFO_NOT_FOUND)),
            "Transparansi Kontak": _clean_entry_value(getattr(j, "transparansi_kontak", "Tidak Transparan")),
            "Status Server OJS": _clean_entry_value(getattr(j, "status_server", "Aktif (HTTP)")),
            "Focus & Scope": _clean_entry_value(getattr(j, "focus_scope", INFO_NOT_FOUND)),
            "ISSN": _clean_entry_value(j.issn),
            "E-ISSN": _clean_entry_value(j.e_issn),
            "Bidang": _clean_entry_value(j.bidang),
            "Link OJS / Website": resolved_link,
            "Sumber Data": j.sumber,
        })
        data.append(row)
    return pd.DataFrame(data)


def export_markdown(
    journals: list[JournalEntry],
    keyword: str,
    sinta_ranks: Optional[list[int]] = None,
    filename: Optional[str] = None,
    skip_url_validation: bool = True,
) -> str:
    """
    Ekspor hasil scraping ke file Markdown (.md) v2.2.0 dengan tabel rapi,
    statistik peringkat, biaya submit & publikasi, jadwal bulan terbit, serta audit sumber AI/Regex.
    """
    _ensure_output_dir()
    _ensure_entry_enrichment(journals, keyword=keyword)
    final_filename = filename or generate_output_filename(keyword, "md")
    filepath = os.path.join(OUTPUT_DIR, final_filename)

    total = len(journals)
    now = datetime.now().strftime("%d %B %Y, %H:%M:%S")

    url_status = _resolve_export_url_status(journals, skip_url_validation)

    lines: list[str] = []

    # Header Dokumen
    lines.append(f"# 📚 Hasil Pencarian Jurnal Ilmiah (JSFS v{APP_VERSION} Hybrid AI Engine)")
    lines.append("")
    lines.append(f"> Dokumen ini dibuat otomatis oleh **JSFS (Journal Scraper For Students) v{APP_VERSION}**.")
    lines.append(f"> Waktu ekstraksi: **{now}**")
    lines.append("")
    lines.append("---")
    lines.append("")

    # Metadata Pencarian
    lines.append("## 🔍 Parameter Pencarian")
    lines.append(f"- **Kata Kunci**: `{keyword}`")
    sinta_str = ", ".join(f"Sinta {r}" for r in sorted(sinta_ranks)) if sinta_ranks else "Semua Peringkat"
    lines.append(f"- **Filter Peringkat SINTA**: {sinta_str}")
    lines.append(f"- **Total Jurnal Ditemukan**: **{total}** jurnal")
    lines.append("")
    lines.append("---")
    lines.append("")

    # 🏆 Top 3 Rekomendasi Jurnal Terbaik (AI Advisor)
    top_recs = select_top_recommendations(journals, top_n=3)
    if top_recs:
        lines.append("## 🏆 Top 3 Rekomendasi Jurnal Terbaik (AI Advisor)")
        lines.append("")
        lines.append("> 💡 **Rekomendasi Cerdas**: Dipilih secara otomatis berdasarkan skor kecocokan topik, keterjangkauan biaya publikasi (APC), kecepatan jadwal terbit terdekat, dan akreditasi SINTA.")
        lines.append("")
        for idx, rec in enumerate(top_recs, 1):
            rj = rec["entry"]
            r_name = _clean_entry_value(rj.nama_jurnal)
            r_rank = _clean_entry_value(rj.peringkat_sinta)
            r_pub = _format_amount(getattr(rj, "biaya_publikasi", None) or getattr(rj, "apc_amount", None), getattr(rj, "mata_uang", None))
            r_terbit = rec["terbit_terdekat"]
            lines.append(f"### {idx}. {r_name} ({r_rank}) — Skor Advisor: `{rec['score']} pts`")
            lines.append(f"- 🎯 **Alasan**: {rec['reason']}")
            lines.append(f"- 💰 **Biaya Publikasi**: `{r_pub}` | 📅 **Terbit Terdekat**: `{r_terbit}`")

            contact_parts = []
            r_url = rj.link_ojs or ""
            if r_url.startswith("http") and url_status.get(r_url, False):
                contact_parts.append(f"[🔗 Link OJS]({r_url})")
            em_clean = clean_contact_list(getattr(rj, "email", ""), "email")
            if em_clean != INFO_NOT_FOUND and "tidak dapat diakses" not in em_clean.lower():
                contact_parts.append(f"[📧 Email](mailto:{em_clean})")
            wa_url = get_whatsapp_link(getattr(rj, "telepon", ""), rj.nama_jurnal)
            if wa_url:
                contact_parts.append(f"[💬 Chat WA Redaksi]({wa_url})")
            if contact_parts:
                lines.append(f"- 🌐 **Akses Cepat**: {' | '.join(contact_parts)}")
            lines.append("")
        lines.append("---")
        lines.append("")

    # 1. Ringkasan Statistik
    lines.append("## 📈 Ringkasan Statistik")
    lines.append("")

    rank_counts: dict[str, int] = {}
    pub_counts = {"Gratis": 0, "Berbayar": 0, "Belum Teridentifikasi": 0}
    sub_counts = {"Gratis": 0, "Berbayar": 0, "Tidak Ada / Belum Terdata": 0}
    ai_focus_count = 0
    ai_apc_count = 0
    server_counts = {"Sangat Aktif": 0, "Aktif": 0, "SSL Warning": 0, "Non-Aktif": 0}
    transparency_counts = {"Lengkap": 0, "Sedang": 0, "Terbatas": 0, "Tidak Transparan": 0}

    for j in journals:
        rank = _clean_entry_value(j.peringkat_sinta)
        rank_counts[rank] = rank_counts.get(rank, 0) + 1

        # Audit AI
        if getattr(j, "sumber_focus", "") == "AI":
            ai_focus_count += 1
        if getattr(j, "sumber_apc", "") == "AI":
            ai_apc_count += 1

        # Biaya Publikasi
        b_pub = getattr(j, "biaya_publikasi", None)
        if b_pub is None and getattr(j, "apc_amount", None) is not None:
            b_pub = getattr(j, "apc_amount", None)
        if b_pub == 0:
            pub_counts["Gratis"] += 1
        elif b_pub is not None and b_pub > 0:
            pub_counts["Berbayar"] += 1
        else:
            pub_counts["Belum Teridentifikasi"] += 1

        # Biaya Submit
        b_sub = getattr(j, "biaya_submit", None)
        if b_sub == 0:
            sub_counts["Gratis"] += 1
        elif b_sub is not None and b_sub > 0:
            sub_counts["Berbayar"] += 1
        else:
            sub_counts["Tidak Ada / Belum Terdata"] += 1

        srv = getattr(j, "status_server", "Aktif (HTTP)")
        if "Sangat Aktif" in srv:
            server_counts["Sangat Aktif"] += 1
        elif "SSL Warning" in srv:
            server_counts["SSL Warning"] += 1
        elif "Non-Aktif" in srv:
            server_counts["Non-Aktif"] += 1
        else:
            server_counts["Aktif"] += 1

        tr = getattr(j, "transparansi_kontak", "Tidak Transparan")
        if "Lengkap" in tr:
            transparency_counts["Lengkap"] += 1
        elif "Sedang" in tr:
            transparency_counts["Sedang"] += 1
        elif "Terbatas" in tr:
            transparency_counts["Terbatas"] += 1
        else:
            transparency_counts["Tidak Transparan"] += 1

    lines.append("### 📊 Distribusi Peringkat SINTA:")
    for r in sorted(rank_counts.keys()):
        lines.append(f"- **{r}**: {rank_counts[r]} jurnal")
    lines.append("")

    lines.append("### 💰 Analisis Biaya Penulis (Author Fees):")
    lines.append(f"- **Biaya Publikasi (APC)**: {pub_counts['Gratis']} Gratis, {pub_counts['Berbayar']} Berbayar, {pub_counts['Belum Teridentifikasi']} Belum Terdata")
    lines.append(f"- **Biaya Submit**: {sub_counts['Gratis']} Gratis, {sub_counts['Berbayar']} Berbayar, {sub_counts['Tidak Ada / Belum Terdata']} Tidak Ada / Belum Terdata")
    lines.append("")

    lines.append("### 🤖 Transparansi Sumber Ekstraksi Data (AI vs Regex):")
    lines.append(f"- **Focus & Scope**: {ai_focus_count} diekstrak AI (Ollama), {total - ai_focus_count} Regex Legacy")
    lines.append(f"- **Author Fees (APC)**: {ai_apc_count} diekstrak AI (Ollama), {total - ai_apc_count} Regex Legacy")
    lines.append("")

    lines.append("### 🌐 Indikator Server OJS & Transparansi Kontak:")
    lines.append(f"- **Status Server**: {server_counts['Sangat Aktif']} Sangat Aktif (HTTPS), {server_counts['Aktif']} Aktif (HTTP), {server_counts['SSL Warning']} SSL Warning, {server_counts['Non-Aktif']} Non-Aktif")
    lines.append(f"- **Kualitas Kontak**: {transparency_counts['Lengkap']} Lengkap, {transparency_counts['Sedang']} Sedang, {transparency_counts['Terbatas']} Terbatas, {transparency_counts['Tidak Transparan']} Belum Tercantum")
    lines.append("")
    lines.append("---")
    lines.append("")

    # 2. Tabel Utama
    if not journals:
        lines.append("## ⚠️ Tidak ada jurnal yang sesuai dengan kriteria filter.")
        lines.append("")
    else:
        lines.append(f"## 📋 Daftar Hasil Jurnal ({total} Data)")
        lines.append("")
        lines.append("| No | Nama Jurnal | SINTA | Kesesuaian | Skor | Submit | Publikasi | Mata Uang | Bulan Terbit | Terbit Terdekat | Sumber Focus | Sumber APC | Frekuensi | Focus & Scope | Link OJS |")
        lines.append("|:---:|---|:---:|:---:|:---:|---|---|:---:|:---:|:---:|:---:|:---:|---|---|:---:|")

        for i, j in enumerate(journals, 1):
            nama = _clean_entry_value(j.nama_jurnal).replace("|", "\\|")
            rank = _clean_entry_value(j.peringkat_sinta)
            cur = getattr(j, "mata_uang", None) or "—"
            b_sub = _format_amount(getattr(j, "biaya_submit", None), getattr(j, "mata_uang", None))

            b_pub_num = getattr(j, "biaya_publikasi", None)
            if b_pub_num is None and getattr(j, "apc_amount", None) is not None:
                b_pub_num = getattr(j, "apc_amount", None)
            b_pub = _format_amount(b_pub_num, getattr(j, "mata_uang", None))

            bulan = format_bulan_terbit(getattr(j, "bulan_terbit", []))
            terbit_est = get_next_issue_estimate(getattr(j, "bulan_terbit", []))
            s_focus = getattr(j, "sumber_focus", "Regex") or "Regex"
            s_apc = getattr(j, "sumber_apc", "Regex") or "Regex"
            freq = _clean_entry_value(getattr(j, "frekuensi", INFO_NOT_FOUND))
            focus = _clean_entry_value(getattr(j, "focus_scope", INFO_NOT_FOUND)).replace("|", "\\|")
            if len(focus) > 120:
                focus = focus[:117].rsplit(" ", 1)[0] + "..."

            raw_url = j.link_ojs or ""
            if not raw_url.startswith("http"):
                link_col = INFO_NOT_FOUND
            elif url_status.get(raw_url, False):
                link_col = f"[🔗 Buka Situs]({raw_url})"
            else:
                link_col = f"*{DEAD_LINK_TEXT}*"

            m_score = getattr(j, "match_score", 0) or 0
            kesesuaian = getattr(j, "kesesuaian_naskah", "") or "—"
            skor_str = f"{int(m_score)}%" if m_score > 0 else "—"
            lines.append(f"| {i} | **{nama}** | {rank} | **{kesesuaian}** | `{skor_str}` | {b_sub} | {b_pub} | {cur} | {bulan} | {terbit_est} | {s_focus} | {s_apc} | {freq} | {focus} | {link_col} |")

        lines.append("")
        lines.append("---")
        lines.append("")

        # 3. Direktori Kontak Cepat
        contact_journals = [
            j for j in journals
            if clean_contact_list(getattr(j, "email", ""), "email") != INFO_NOT_FOUND
            or clean_contact_list(getattr(j, "telepon", ""), "phone") != INFO_NOT_FOUND
        ]
        if contact_journals:
            lines.append("## 📞 Direktori Kontak Cepat Redaksi Jurnal")
            lines.append("")
            for j in contact_journals:
                lines.append(f"### 📑 {j.nama_jurnal} ({j.peringkat_sinta})")
                if _clean_entry_value(j.penerbit) != INFO_NOT_FOUND:
                    lines.append(f"- 🏛️ **Penerbit**: {j.penerbit}")
                em = clean_contact_list(getattr(j, "email", ""), "email")
                if em != INFO_NOT_FOUND and "tidak dapat diakses" not in em.lower():
                    lines.append(f"- 📧 **Email**: [{em}](mailto:{em})")
                tel = clean_contact_list(getattr(j, "telepon", ""), "phone")
                if tel != INFO_NOT_FOUND and "tidak dapat diakses" not in tel.lower():
                    wa_link = get_whatsapp_link(tel, j.nama_jurnal)
                    if wa_link:
                        lines.append(f"- 📱 **Telepon/WA**: `{tel}` ([💬 Chat WA Redaksi]({wa_link}))")
                    else:
                        lines.append(f"- 📱 **Telepon/WA**: `{tel}`")

                b_pub_num = getattr(j, "biaya_publikasi", None)
                if b_pub_num is not None:
                    lines.append(f"- 💰 **Biaya Publikasi**: `{_format_amount(b_pub_num, getattr(j, 'mata_uang', None))}`")
                b_sub_num = getattr(j, "biaya_submit", None)
                if b_sub_num is not None:
                    lines.append(f"- 📝 **Biaya Submit**: `{_format_amount(b_sub_num, getattr(j, 'mata_uang', None))}`")

                fq = _clean_entry_value(getattr(j, "frekuensi", INFO_NOT_FOUND))
                if fq != INFO_NOT_FOUND:
                    lines.append(f"- ⏱️ **Frekuensi**: `{fq}`")

                raw_u = j.link_ojs or ""
                if raw_u.startswith("http") and url_status.get(raw_u, False):
                    lines.append(f"- 🌐 **Link OJS**: [{raw_u}]({raw_u})")
                elif raw_u.startswith("http"):
                    lines.append(f"- 🌐 **Link OJS**: *{DEAD_LINK_TEXT}* ({raw_u})")
                lines.append("")

    # 📋 Daftar Ringkas Siap Copy-Paste (Untuk Share ke Rekan)
    copypaste_list = generate_copypaste_list(journals)
    if copypaste_list:
        lines.append("## 📋 Daftar Ringkas Siap Copy-Paste (Untuk Share ke Rekan)")
        lines.append("")
        lines.append("> 💡 **Tips Praktis**: Salin blok teks di bawah ini untuk dibagikan langsung ke grup WhatsApp / Telegram rekan tim mahasiswa.")
        lines.append("")
        lines.append("```text")
        lines.append(copypaste_list)
        lines.append("```")
        lines.append("")

    # Footer
    lines.append("---")
    lines.append("")
    lines.append(f"*Dibuat otomatis oleh **JSFS (Journal Scraper For Students) v{APP_VERSION} (Hybrid AI Engine)***")
    lines.append(f"*{now}*")
    lines.append("")
    lines.append(
        "> ⚠️ **Catatan Penting Mahasiswa:** Selalu periksa pedoman penulisan (*Author Guidelines*) "
        "dan hubungi redaksi jurnal sebelum mengirimkan manuskrip untuk memastikan kuota terbit tersedia."
    )

    with open(filepath, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))

    console.print(f"[bold green]✅ Markdown tersimpan → {filepath}[/bold green]")
    return filepath


def export_csv(
    journals: list[JournalEntry],
    keyword: str = "",
    filename: Optional[str] = None,
    skip_url_validation: bool = True,
) -> str:
    """Ekspor hasil ke CSV dengan encoding UTF-8 BOM dan header v2.2.0."""
    _ensure_output_dir()
    _ensure_entry_enrichment(journals, keyword=keyword)
    final_filename = filename or generate_output_filename(keyword, "csv")
    filepath = os.path.join(OUTPUT_DIR, final_filename)

    url_status = _resolve_export_url_status(journals, skip_url_validation)

    df = _journals_to_dataframe(journals, url_status)
    df.to_csv(filepath, index=False, encoding="utf-8-sig")
    console.print(f"[bold green]✅ CSV tersimpan → {filepath}[/bold green]")
    return filepath


def export_excel(
    journals: list[JournalEntry],
    keyword: str = "",
    filename: Optional[str] = None,
    skip_url_validation: bool = True,
) -> str:
    """
    Ekspor hasil ke Excel (.xlsx) dengan visual profesional v2.2.0:
    - Header Navy Blue (#1B365D) dengan teks putih bold.
    - Kolom Biaya Submit, Biaya Publikasi, Mata Uang, Bulan Terbit, Sumber Focus & Sumber APC terpisah rapi.
    - URL OJS aktif sebagai Hyperlink interaktif (=HYPERLINK).
    - Auto-fit lebar kolom dan styling sel halus.
    """
    _ensure_output_dir()
    _ensure_entry_enrichment(journals, keyword=keyword)
    final_filename = filename or generate_output_filename(keyword, "xlsx")
    filepath = os.path.join(OUTPUT_DIR, final_filename)

    url_status = _resolve_export_url_status(journals, skip_url_validation)

    import openpyxl
    from openpyxl.styles import Font, PatternFill, Alignment, Border, Side
    from openpyxl.utils import get_column_letter

    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = "Hasil Jurnal JSFS"

    # Definisi Header Kolom v1.5.1 (Selalu menyertakan Kesesuaian & Skor)
    headers = [
        "No",
        "Nama Jurnal",
        "Peringkat SINTA",
        "Kesesuaian Naskah",
        "Match Score (%)",
        "Penerbit",
        "Biaya Submit",
        "Biaya Publikasi",
        "Mata Uang",
        "Bulan Terbit",
        "Terbit Terdekat",
        "Sumber Focus",
        "Sumber APC",
        "Frekuensi Terbit",
        "Email Kontak",
        "Telepon / WA",
        "Transparansi Kontak",
        "Status Server OJS",
        "Focus & Scope",
        "ISSN",
        "E-ISSN",
        "Bidang Keilmuan",
        "Link OJS / Website",
        "Sumber Data",
    ]
    ws.append(headers)

    # 1. Styling Header
    navy_fill = PatternFill(start_color="1B365D", end_color="1B365D", fill_type="solid")
    header_font = Font(name="Segoe UI", size=11, bold=True, color="FFFFFF")
    thin_border = Border(
        left=Side(style="thin", color="D1D5DB"),
        right=Side(style="thin", color="D1D5DB"),
        top=Side(style="thin", color="D1D5DB"),
        bottom=Side(style="thin", color="D1D5DB"),
    )
    center_align = Alignment(horizontal="center", vertical="center", wrap_text=True)
    left_align = Alignment(horizontal="left", vertical="center")
    wrap_left_align = Alignment(horizontal="left", vertical="center", wrap_text=True)

    for col_idx in range(1, len(headers) + 1):
        cell = ws.cell(row=1, column=col_idx)
        cell.fill = navy_fill
        cell.font = header_font
        cell.alignment = center_align
        cell.border = thin_border
    ws.row_dimensions[1].height = 28

    # Styling Cell untuk Peringkat SINTA, AI Sources & Status
    fill_s1_s2 = PatternFill(start_color="DCFCE7", end_color="DCFCE7", fill_type="solid")
    font_s1_s2 = Font(name="Segoe UI", size=10, bold=True, color="166534")

    fill_s3_s4 = PatternFill(start_color="DBEAFE", end_color="DBEAFE", fill_type="solid")
    font_s3_s4 = Font(name="Segoe UI", size=10, bold=True, color="1E40AF")

    fill_s5_s6 = PatternFill(start_color="F3F4F6", end_color="F3F4F6", fill_type="solid")
    font_s5_s6 = Font(name="Segoe UI", size=10, bold=True, color="374151")

    fill_ai = PatternFill(start_color="FEF3C7", end_color="FEF3C7", fill_type="solid")
    font_ai = Font(name="Segoe UI", size=10, bold=True, color="B45309")

    regular_font = Font(name="Segoe UI", size=10)
    link_font = Font(name="Segoe UI", size=10, color="1D4ED8", underline="single")
    dead_link_font = Font(name="Segoe UI", size=9, italic=True, color="9CA3AF")

    link_col_idx = headers.index("Link OJS / Website") + 1
    rank_col_idx = headers.index("Peringkat SINTA") + 1
    trans_col_idx = headers.index("Transparansi Kontak") + 1
    status_col_idx = headers.index("Status Server OJS") + 1
    focus_col_idx = headers.index("Focus & Scope") + 1
    sumber_focus_idx = headers.index("Sumber Focus") + 1
    sumber_apc_idx = headers.index("Sumber APC") + 1
    terbit_dekat_idx = headers.index("Terbit Terdekat") + 1

    # 2. Tulis Data Jurnal & Terapkan Styling
    for row_num, j in enumerate(journals, start=2):
        raw_url = j.link_ojs or ""
        is_link_alive = raw_url.startswith("http") and url_status.get(raw_url, False)

        cur = getattr(j, "mata_uang", None) or "—"
        b_sub = _format_amount(getattr(j, "biaya_submit", None), getattr(j, "mata_uang", None))

        b_pub_num = getattr(j, "biaya_publikasi", None)
        if b_pub_num is None and getattr(j, "apc_amount", None) is not None:
            b_pub_num = getattr(j, "apc_amount", None)
        b_pub = _format_amount(b_pub_num, getattr(j, "mata_uang", None))

        bulan = format_bulan_terbit(getattr(j, "bulan_terbit", []))
        terbit_est = get_next_issue_estimate(getattr(j, "bulan_terbit", []))
        s_focus = getattr(j, "sumber_focus", "Regex") or "Regex"
        s_apc = getattr(j, "sumber_apc", "Regex") or "Regex"

        m_score = getattr(j, "match_score", 0) or 0
        kesesuaian = getattr(j, "kesesuaian_naskah", "") or "—"
        skor_str = f"{int(m_score)}%" if m_score > 0 else "—"

        row_vals = [
            row_num - 1,
            _clean_entry_value(j.nama_jurnal),
            _clean_entry_value(j.peringkat_sinta),
            kesesuaian,
            skor_str,
            _clean_entry_value(j.penerbit),
            b_sub,
            b_pub,
            cur,
            bulan,
            terbit_est,
            s_focus,
            s_apc,
            _clean_entry_value(getattr(j, "frekuensi", INFO_NOT_FOUND)),
            _clean_entry_value(getattr(j, "email", INFO_NOT_FOUND)),
            _clean_entry_value(getattr(j, "telepon", INFO_NOT_FOUND)),
            _clean_entry_value(getattr(j, "transparansi_kontak", "Tidak Transparan")),
            _clean_entry_value(getattr(j, "status_server", "Aktif (HTTP)")),
            _clean_entry_value(getattr(j, "focus_scope", INFO_NOT_FOUND)),
            _clean_entry_value(j.issn),
            _clean_entry_value(j.e_issn),
            _clean_entry_value(j.bidang),
            raw_url if is_link_alive else (DEAD_LINK_TEXT if raw_url.startswith("http") else INFO_NOT_FOUND),
            j.sumber,
        ]
        ws.append(row_vals)
        ws.row_dimensions[row_num].height = 24

        for col_idx in range(1, len(headers) + 1):
            cell = ws.cell(row=row_num, column=col_idx)
            cell.font = regular_font
            cell.border = thin_border
            cell.alignment = left_align

            if col_idx in (1, rank_col_idx, trans_col_idx, status_col_idx, sumber_focus_idx, sumber_apc_idx, terbit_dekat_idx):
                cell.alignment = center_align

            if col_idx == focus_col_idx:
                cell.alignment = wrap_left_align

            # Pewarnaan SINTA
            if col_idx == rank_col_idx:
                val_str = str(cell.value).upper()
                if "SINTA 1" in val_str or "SINTA 2" in val_str or val_str in ("S1", "S2"):
                    cell.fill = fill_s1_s2
                    cell.font = font_s1_s2
                elif "SINTA 3" in val_str or "SINTA 4" in val_str or val_str in ("S3", "S4"):
                    cell.fill = fill_s3_s4
                    cell.font = font_s3_s4
                elif "SINTA 5" in val_str or "SINTA 6" in val_str or val_str in ("S5", "S6"):
                    cell.fill = fill_s5_s6
                    cell.font = font_s5_s6

            # Highlight AI
            if col_idx in (sumber_focus_idx, sumber_apc_idx) and str(cell.value) == "AI":
                cell.fill = fill_ai
                cell.font = font_ai

            # Link Hyperlink
            if col_idx == link_col_idx:
                if is_link_alive:
                    cell.value = raw_url
                    cell.hyperlink = raw_url
                    cell.font = link_font
                    cell.alignment = left_align
                else:
                    cell.value = DEAD_LINK_TEXT if raw_url.startswith("http") else INFO_NOT_FOUND
                    cell.font = dead_link_font
                    cell.alignment = center_align

    # 3. Auto-fit Lebar Kolom untuk Tabel Utama
    for col in ws.columns:
        max_len = 0
        col_letter = get_column_letter(col[0].column)
        col_name = str(col[0].value or "")
        if col_name == "Focus & Scope":
            ws.column_dimensions[col_letter].width = 46
            continue
        for cell in col:
            val_str = str(cell.value or "")
            if val_str.startswith("="):
                val_str = "🔗 Buka Website"
            max_len = max(max_len, len(val_str))
        ws.column_dimensions[col_letter].width = min(max(max_len + 4, 12), 48)

    # ── Sheet Top 3 Rekomendasi (AI Advisor) ──
    top_recs = select_top_recommendations(journals, top_n=3)
    if top_recs:
        ws_adv = wb.create_sheet(title="🏆 Rekomendasi AI", index=0)
        ws_adv.append(["🏆 TOP 3 REKOMENDASI JURNAL TERBAIK (AI ADVISOR)"])
        ws_adv.append(["Dipilih otomatis berdasarkan Skor Kesesuaian, Keterjangkauan APC, Kecepatan Jadwal Terbit, dan Akreditasi SINTA."])
        ws_adv.append([])

        adv_headers = ["Peringkat", "Nama Jurnal", "SINTA", "Skor Advisor", "Biaya Publikasi", "Terbit Terdekat", "Alasan Rekomendasi", "Link OJS", "Email", "Telepon / WA"]
        ws_adv.append(adv_headers)

        title_font = Font(name="Segoe UI", size=13, bold=True, color="1B365D")
        sub_font = Font(name="Segoe UI", size=10, italic=True, color="4B5563")
        adv_header_font = Font(name="Segoe UI", size=10, bold=True, color="FFFFFF")

        ws_adv.cell(row=1, column=1).font = title_font
        ws_adv.cell(row=2, column=1).font = sub_font

        for c_idx in range(1, len(adv_headers) + 1):
            c = ws_adv.cell(row=4, column=c_idx)
            c.fill = navy_fill
            c.font = adv_header_font
            c.alignment = center_align
            c.border = thin_border
        ws_adv.row_dimensions[4].height = 26

        for r_idx, rec in enumerate(top_recs, start=5):
            rj = rec["entry"]
            r_pub = _format_amount(getattr(rj, "biaya_publikasi", None) or getattr(rj, "apc_amount", None), getattr(rj, "mata_uang", None))
            r_em = clean_contact_list(getattr(rj, "email", ""), "email")
            r_tel = clean_contact_list(getattr(rj, "telepon", ""), "phone")
            r_url = rj.link_ojs or ""

            adv_row = [
                f"#{r_idx - 4}",
                _clean_entry_value(rj.nama_jurnal),
                _clean_entry_value(rj.peringkat_sinta),
                f"{rec['score']} pts",
                r_pub,
                rec["terbit_terdekat"],
                rec["reason"],
                r_url if r_url.startswith("http") else INFO_NOT_FOUND,
                r_em,
                r_tel,
            ]
            ws_adv.append(adv_row)
            ws_adv.row_dimensions[r_idx].height = 26

            for c_idx in range(1, len(adv_headers) + 1):
                cell = ws_adv.cell(row=r_idx, column=c_idx)
                cell.font = regular_font
                cell.border = thin_border
                cell.alignment = left_align
                if c_idx in (1, 3, 4, 5, 6):
                    cell.alignment = center_align
                if c_idx == 8 and r_url.startswith("http"):
                    cell.hyperlink = r_url
                    cell.font = link_font

        for col in ws_adv.columns:
            m_len = 0
            col_letter = get_column_letter(col[0].column)
            col_h = str(col[3].value or "") if len(col) >= 4 else ""
            if col_h == "Alasan Rekomendasi":
                ws_adv.column_dimensions[col_letter].width = 45
                continue
            for cell in col[3:]:
                v_str = str(cell.value or "")
                m_len = max(m_len, len(v_str))
            ws_adv.column_dimensions[col_letter].width = min(max(m_len + 4, 12), 40)

    wb.save(filepath)
    console.print(f"[bold green]✅ Excel tersimpan → {filepath}[/bold green]")
    return filepath


_autosave_lock = threading.Lock()


def export_autosave_checkpoint(
    journals: list,
    keyword: str = "",
) -> tuple[str, str]:
    """
    Menyimpan checkpoint sementara secara thread-safe & atomik ke:
    - output/_AUTOSAVE_TERAKHIR.xlsx
    - output/_AUTOSAVE_TERAKHIR_LIST.txt
    """
    if not journals:
        return "", ""

    snapshot = list(journals)
    xlsx_path = os.path.join(OUTPUT_DIR, "_AUTOSAVE_TERAKHIR.xlsx")
    list_path = os.path.join(OUTPUT_DIR, "_AUTOSAVE_TERAKHIR_LIST.txt")
    tmp_xlsx = "_AUTOSAVE_TERAKHIR.tmp.xlsx"
    tmp_list = list_path + ".tmp"

    with _autosave_lock:
        _ensure_output_dir()
        try:
            written_tmp = export_excel(
                snapshot,
                keyword=keyword,
                filename=tmp_xlsx,
                skip_url_validation=True,
            )
            if os.path.exists(written_tmp):
                os.replace(written_tmp, xlsx_path)
        except Exception as e:
            console.print(f"[yellow]⚠ Auto-save Excel checkpoint gagal: {e}[/yellow]")
            xlsx_path = ""

        try:
            copypaste_text = generate_copypaste_list(snapshot)
            if copypaste_text:
                with open(tmp_list, "w", encoding="utf-8") as f:
                    f.write(copypaste_text)
                os.replace(tmp_list, list_path)
        except Exception as e:
            console.print(f"[yellow]⚠ Auto-save LIST checkpoint gagal: {e}[/yellow]")
            list_path = ""

    return xlsx_path, list_path


def write_exports(
    journals: list[JournalEntry],
    keyword: str,
    formats: list[str],
    sinta_ranks: Optional[list[int]] = None,
    skip_url_validation: bool = True,
) -> list[str]:
    """Ekspor hasil ke beberapa format sekaligus (.md, .xlsx, .csv) dan file teks pendamping _LIST.txt."""
    _ensure_entry_enrichment(journals, keyword=keyword)
    exported: list[str] = []
    if not formats:
        formats = [".xlsx", ".md"]

    for fmt in formats:
        fmt_clean = fmt.lower().lstrip(".")
        if fmt_clean in ("xlsx", "excel"):
            exported.append(export_excel(journals, keyword=keyword, skip_url_validation=skip_url_validation))
        elif fmt_clean == "md":
            exported.append(export_markdown(journals, keyword=keyword, sinta_ranks=sinta_ranks, skip_url_validation=skip_url_validation))
        elif fmt_clean == "csv":
            exported.append(export_csv(journals, keyword=keyword, skip_url_validation=skip_url_validation))

    # File teks pendamping otomatis daftar ringkas siap copy-paste (v1.6.0)
    try:
        copypaste_text = generate_copypaste_list(journals)
        if copypaste_text:
            _ensure_output_dir()
            list_filename = generate_output_filename(keyword, "txt")
            if list_filename.endswith(".txt"):
                list_filename = list_filename[:-4] + "_LIST.txt"
            else:
                list_filename += "_LIST.txt"
            list_path = os.path.join(OUTPUT_DIR, list_filename)
            with open(list_path, "w", encoding="utf-8") as f:
                f.write(copypaste_text)
            exported.append(list_path)
            console.print(f"[bold green]✅ Daftar Copy-Paste tersimpan → {list_path}[/bold green]")
    except Exception as e:
        console.print(f"[yellow]⚠ Gagal membuat file LIST.txt pendamping: {e}[/yellow]")

    return exported


def print_summary_table(journals: list[JournalEntry]):
    """Tampilkan ringkasan hasil di terminal menggunakan Rich."""
    from rich.table import Table

    if not journals:
        console.print("[yellow]⚠ Tidak ada jurnal untuk ditampilkan.[/yellow]")
        return

    table = Table(
        title=f"📚 Hasil JSFS v1.4.0 — {len(journals)} Jurnal Ditemukan",
        show_lines=True,
        highlight=True,
    )
    table.add_column("No", style="dim", width=4, justify="right")
    table.add_column("Nama Jurnal", style="bold", min_width=24)
    table.add_column("SINTA", justify="center", width=8)
    table.add_column("Penerbit", style="cyan", min_width=16)
    table.add_column("Submit", width=12)
    table.add_column("Publikasi", width=14)
    table.add_column("Bulan", width=10)
    table.add_column("Sumber", width=12)
    table.add_column("Link OJS", style="blue underline")

    for i, j in enumerate(journals, 1):
        rank_val = _clean_entry_value(j.peringkat_sinta)
        rank_color = {
            "S1": "bright_green", "S2": "green", "S3": "cyan",
            "S4": "blue", "S5": "yellow", "S6": "white",
        }.get(rank_val.replace("Sinta ", "S"), "white")

        b_sub = _format_amount(getattr(j, "biaya_submit", None), getattr(j, "mata_uang", None))
        b_pub_num = getattr(j, "biaya_publikasi", None)
        if b_pub_num is None and getattr(j, "apc_amount", None) is not None:
            b_pub_num = getattr(j, "apc_amount", None)
        b_pub = _format_amount(b_pub_num, getattr(j, "mata_uang", None))
        bulan = format_bulan_terbit(getattr(j, "bulan_terbit", []))
        src = f"AI" if getattr(j, "sumber_focus", "") == "AI" else "Regex"

        table.add_row(
            str(i),
            _clean_entry_value(j.nama_jurnal),
            f"[{rank_color}]{rank_val}[/{rank_color}]",
            _clean_entry_value(j.penerbit),
            b_sub,
            b_pub,
            bulan,
            src,
            j.link_ojs or INFO_NOT_FOUND,
        )

    console.print(table)
