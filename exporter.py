# =============================================================================
# JSFS - Journal Scraper For Students
# exporter.py — Modul ekspor hasil ke Markdown, CSV, dan Excel v1.4.0 (Hybrid AI Engine)
# =============================================================================

import os
import re
from datetime import datetime
from typing import Optional, Union, List
from concurrent.futures import ThreadPoolExecutor
import requests
import urllib3
import pandas as pd
from rich.console import Console
import openpyxl
from openpyxl.styles import Font, PatternFill, Alignment, Border, Side
from openpyxl.utils import get_column_letter

# Nonaktifkan peringatan SSL tidak aman
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

from config import (
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
from scraper_garuda import JournalEntry, sanitize_text
from month_parser import format_bulan_terbit

console = Console()


def _ensure_output_dir():
    """Memastikan direktori penyimpanan output tersedia."""
    os.makedirs(OUTPUT_DIR, exist_ok=True)


def _format_amount(val: Optional[int], currency: Optional[str] = "IDR") -> str:
    """Format nominal angka menjadi format terstandarisasi Rp / USD / Gratis / —."""
    if val is None:
        return "—"
    if val == 0:
        return "Gratis (0)"
    cur = (currency or "IDR").upper()
    if cur == "USD":
        return f"${val:,} USD"
    return f"Rp {val:,}".replace(",", ".")


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
        except (requests.exceptions.SSLError, requests.exceptions.ReadTimeout):
            return u, True
        except Exception:
            return u, False

    with ThreadPoolExecutor(max_workers=DEEP_SCRAPE_WORKERS) as executor:
        futures = [executor.submit(_check_single, u) for u in valid_targets]
        for f in futures:
            u, is_ok = f.result()
            results[u] = is_ok

    return results


def _clean_entry_value(val: Optional[str]) -> str:
    """Helper untuk memastikan semua nilai kosong diubah ke INFO_NOT_FOUND."""
    return sanitize_text(val)


def _journals_to_dataframe(journals: list[JournalEntry], url_status_map: dict[str, bool]) -> pd.DataFrame:
    """Konversi list JournalEntry ke Pandas DataFrame lengkap v1.4.0."""
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

        row = {
            "No": i + 1,
            "Nama Jurnal": _clean_entry_value(j.nama_jurnal),
            "Peringkat SINTA": _clean_entry_value(j.peringkat_sinta),
        }
        if has_match_score:
            row["Kesesuaian Naskah"] = getattr(j, "kesesuaian_naskah", "Kurang Sesuai") or "Kurang Sesuai"
            row["Match Score (%)"] = getattr(j, "match_score", 0)

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
) -> str:
    """
    Ekspor hasil scraping ke file Markdown (.md) v1.4.0 dengan tabel rapi,
    statistik peringkat, biaya submit & publikasi, jadwal bulan terbit, serta audit sumber AI/Regex.
    """
    _ensure_output_dir()
    final_filename = filename or generate_output_filename(keyword, "md")
    filepath = os.path.join(OUTPUT_DIR, final_filename)

    total = len(journals)
    now = datetime.now().strftime("%d %B %Y, %H:%M:%S")

    all_urls = [j.link_ojs for j in journals if j.link_ojs and j.link_ojs.startswith("http")]
    url_status = validate_urls_async(all_urls)

    lines: list[str] = []

    # Header Dokumen
    lines.append(f"# 📚 Hasil Pencarian Jurnal Ilmiah (JSFS v1.4.0 Hybrid AI Engine)")
    lines.append("")
    lines.append(f"> Dokumen ini dibuat otomatis oleh **JSFS (Journal Scraper For Students) v1.4.0**.")
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
        has_match_score = any(getattr(j, "match_score", 0) > 0 for j in journals)
        lines.append(f"## 📋 Daftar Hasil Jurnal ({total} Data)")
        lines.append("")
        if has_match_score:
            lines.append("| No | Nama Jurnal | SINTA | Kesesuaian | Skor | Submit | Publikasi | Mata Uang | Bulan Terbit | Sumber Focus | Sumber APC | Frekuensi | Focus & Scope | Link OJS |")
            lines.append("|:---:|---|:---:|:---:|:---:|---|---|:---:|:---:|:---:|:---:|---|---|:---:|")
        else:
            lines.append("| No | Nama Jurnal | SINTA | Penerbit | Submit | Publikasi | Mata Uang | Bulan Terbit | Sumber Focus | Sumber APC | Frekuensi | Focus & Scope | Link OJS |")
            lines.append("|:---:|---|:---:|---|---|---|:---:|:---:|:---:|:---:|---|---|:---:|")

        for i, j in enumerate(journals, 1):
            nama = _clean_entry_value(j.nama_jurnal).replace("|", "\\|")
            penerbit = _clean_entry_value(j.penerbit).replace("|", "\\|")
            rank = _clean_entry_value(j.peringkat_sinta)
            cur = getattr(j, "mata_uang", None) or "—"
            b_sub = _format_amount(getattr(j, "biaya_submit", None), getattr(j, "mata_uang", None))
            
            b_pub_num = getattr(j, "biaya_publikasi", None)
            if b_pub_num is None and getattr(j, "apc_amount", None) is not None:
                b_pub_num = getattr(j, "apc_amount", None)
            b_pub = _format_amount(b_pub_num, getattr(j, "mata_uang", None))

            bulan = format_bulan_terbit(getattr(j, "bulan_terbit", []))
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

            if has_match_score:
                kesesuaian = getattr(j, "kesesuaian_naskah", "Kurang Sesuai") or "Kurang Sesuai"
                skor = f"{getattr(j, 'match_score', 0)}%"
                lines.append(f"| {i} | **{nama}** | {rank} | **{kesesuaian}** | `{skor}` | {b_sub} | {b_pub} | {cur} | {bulan} | {s_focus} | {s_apc} | {freq} | {focus} | {link_col} |")
            else:
                lines.append(f"| {i} | **{nama}** | {rank} | {penerbit} | {b_sub} | {b_pub} | {cur} | {bulan} | {s_focus} | {s_apc} | {freq} | {focus} | {link_col} |")

        lines.append("")
        lines.append("---")
        lines.append("")

        # 3. Direktori Kontak Cepat
        contact_journals = [
            j for j in journals
            if _clean_entry_value(getattr(j, "email", INFO_NOT_FOUND)) != INFO_NOT_FOUND
            or _clean_entry_value(getattr(j, "telepon", INFO_NOT_FOUND)) != INFO_NOT_FOUND
        ]
        if contact_journals:
            lines.append("## 📞 Direktori Kontak Cepat Redaksi Jurnal")
            lines.append("")
            for j in contact_journals:
                lines.append(f"### 📑 {j.nama_jurnal} ({j.peringkat_sinta})")
                if _clean_entry_value(j.penerbit) != INFO_NOT_FOUND:
                    lines.append(f"- 🏛️ **Penerbit**: {j.penerbit}")
                em = _clean_entry_value(getattr(j, "email", INFO_NOT_FOUND))
                if em != INFO_NOT_FOUND:
                    lines.append(f"- 📧 **Email**: [{em}](mailto:{em})")
                tel = _clean_entry_value(getattr(j, "telepon", INFO_NOT_FOUND))
                if tel != INFO_NOT_FOUND:
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

    # Footer
    lines.append("---")
    lines.append("")
    lines.append("*Dibuat otomatis oleh **JSFS (Journal Scraper For Students) v1.4.0 (Hybrid AI Engine)***")
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
) -> str:
    """Ekspor hasil ke CSV dengan encoding UTF-8 BOM dan header v1.4.0."""
    _ensure_output_dir()
    final_filename = filename or generate_output_filename(keyword, "csv")
    filepath = os.path.join(OUTPUT_DIR, final_filename)

    all_urls = [j.link_ojs for j in journals if j.link_ojs and j.link_ojs.startswith("http")]
    url_status = validate_urls_async(all_urls)

    df = _journals_to_dataframe(journals, url_status)
    df.to_csv(filepath, index=False, encoding="utf-8-sig")
    console.print(f"[bold green]✅ CSV tersimpan → {filepath}[/bold green]")
    return filepath


def export_excel(
    journals: list[JournalEntry],
    keyword: str = "",
    filename: Optional[str] = None,
) -> str:
    """
    Ekspor hasil ke Excel (.xlsx) dengan visual profesional v1.4.0:
    - Header Navy Blue (#1B365D) dengan teks putih bold.
    - Kolom Biaya Submit, Biaya Publikasi, Mata Uang, Bulan Terbit, Sumber Focus & Sumber APC terpisah rapi.
    - URL OJS aktif sebagai Hyperlink interaktif (=HYPERLINK).
    - Auto-fit lebar kolom dan styling sel halus.
    """
    _ensure_output_dir()
    final_filename = filename or generate_output_filename(keyword, "xlsx")
    filepath = os.path.join(OUTPUT_DIR, final_filename)

    all_urls = [j.link_ojs for j in journals if j.link_ojs and j.link_ojs.startswith("http")]
    url_status = validate_urls_async(all_urls)

    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = "Hasil Jurnal JSFS"

    has_match_score = any(getattr(j, "match_score", 0) > 0 for j in journals)

    # Definisi Header Kolom v1.4.0
    headers = [
        "No",
        "Nama Jurnal",
        "Peringkat SINTA",
    ]
    if has_match_score:
        headers.extend(["Kesesuaian Naskah", "Match Score (%)"])
    headers.extend([
        "Penerbit",
        "Biaya Submit",
        "Biaya Publikasi",
        "Mata Uang",
        "Bulan Terbit",
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
    ])
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
        s_focus = getattr(j, "sumber_focus", "Regex") or "Regex"
        s_apc = getattr(j, "sumber_apc", "Regex") or "Regex"

        row_vals = [
            row_num - 1,
            _clean_entry_value(j.nama_jurnal),
            _clean_entry_value(j.peringkat_sinta),
        ]
        if has_match_score:
            row_vals.append(getattr(j, "kesesuaian_naskah", "Kurang Sesuai") or "Kurang Sesuai")
            row_vals.append(f"{getattr(j, 'match_score', 0)}%")
        row_vals.extend([
            _clean_entry_value(j.penerbit),
            b_sub,
            b_pub,
            cur,
            bulan,
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
        ])
        ws.append(row_vals)
        ws.row_dimensions[row_num].height = 24

        for col_idx in range(1, len(headers) + 1):
            cell = ws.cell(row=row_num, column=col_idx)
            cell.font = regular_font
            cell.border = thin_border
            cell.alignment = left_align

            if col_idx in (1, rank_col_idx, trans_col_idx, status_col_idx, sumber_focus_idx, sumber_apc_idx):
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

    # 3. Auto-fit Lebar Kolom
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

    wb.save(filepath)
    console.print(f"[bold green]✅ Excel tersimpan → {filepath}[/bold green]")
    return filepath


def write_exports(
    journals: list[JournalEntry],
    keyword: str,
    formats: list[str],
    sinta_ranks: Optional[list[int]] = None,
) -> list[str]:
    """Ekspor hasil ke beberapa format sekaligus (.md, .xlsx, .csv)."""
    exported: list[str] = []
    if not formats:
        formats = [".xlsx", ".md"]

    for fmt in formats:
        fmt_clean = fmt.lower().lstrip(".")
        if fmt_clean in ("xlsx", "excel"):
            exported.append(export_excel(journals, keyword=keyword))
        elif fmt_clean == "md":
            exported.append(export_markdown(journals, keyword=keyword, sinta_ranks=sinta_ranks))
        elif fmt_clean == "csv":
            exported.append(export_csv(journals, keyword=keyword))

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
