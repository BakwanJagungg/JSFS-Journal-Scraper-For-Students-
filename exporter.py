# =============================================================================
# JSFS - Journal Scraper For Students
# exporter.py — Modul ekspor hasil ke Markdown, CSV, dan Excel v1.3.8
# =============================================================================

import os
import re
from datetime import datetime
from typing import Optional
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

console = Console()


def _ensure_output_dir():
    """Memastikan direktori penyimpanan output tersedia."""
    os.makedirs(OUTPUT_DIR, exist_ok=True)


def validate_urls_async(urls: list[str]) -> dict[str, bool]:
    """
    VALIDASI HYPERLINK SSL-TOLERANT (v1.3.2):
    - Mengabaikan galat sertifikat SSL kampus lokal (.ac.id) dengan verify=False.
    - Mengikuti lokasi redirect (allow_redirects=True).
    - Timeout adaptif 6 detik untuk server kampus.
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
            # Server merespons: jika 404/500/502/503 dianggap mati
            if resp.status_code in (404, 500, 502, 503):
                return u, False
            # Status 200..399, 401, 403 (Cloudflare challenge aktif tapi server hidup) dianggap valid
            return u, True
        except (requests.exceptions.SSLError, requests.exceptions.ReadTimeout):
            # SSL warning atau ReadTimeout berarti server aktif merespons koneksi TCP/SSL
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
    """Konversi list JournalEntry ke Pandas DataFrame lengkap (v1.3.8)."""
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

        row.update({
            "Penerbit": _clean_entry_value(j.penerbit),
            "Biaya Publikasi (APC)": _clean_entry_value(j.apc_status),
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
    Ekspor hasil scraping ke file Markdown (.md) dengan tabel rapi,
    ringkasan statistik peringkat & APC, validasi link mati, serta daftar kontak cepat.
    """
    _ensure_output_dir()
    final_filename = filename or generate_output_filename(keyword, "md")
    filepath = os.path.join(OUTPUT_DIR, final_filename)

    now = datetime.now().strftime("%d %B %Y, %H:%M:%S WIB")
    rank_str = f"Sinta {', '.join(map(str, sinta_ranks))}" if sinta_ranks else "Semua Peringkat"
    total = len(journals)

    # Validasi Link OJS secara Asinkronus dengan toleransi SSL
    all_urls = [j.link_ojs for j in journals if j.link_ojs and j.link_ojs.startswith("http")]
    url_status = validate_urls_async(all_urls)

    lines = []
    lines.append("# 📚 JSFS — Hasil Pencarian & Ekstraksi Jurnal")
    lines.append("")
    lines.append("> **Journal Scraper For Students (JSFS v1.3.8)**")
    lines.append("> *Solusi presisi pencarian jurnal terakreditasi SINTA & Garuda untuk mahasiswa & peneliti.*")
    lines.append("")
    lines.append("---")
    lines.append("")

    # 1. Ringkasan Parameter & Statistik
    lines.append("## 🔎 Ringkasan & Parameter Riset")
    lines.append("")
    lines.append(f"| Parameter | Nilai |")
    lines.append(f"|---|---|")
    lines.append(f"| **Kata Kunci (Keyword)** | `{keyword}` |")
    lines.append(f"| **Filter Peringkat SINTA** | {rank_str} |")
    lines.append(f"| **Total Jurnal Ditemukan** | **{total} jurnal** |")
    lines.append(f"| **Waktu Ekstraksi** | {now} |")
    lines.append("")

    # Hitung Statistik Distribusi SINTA & Biaya APC
    rank_counts = {}
    apc_counts = {"Gratis": 0, "Berbayar": 0, "Belum Teridentifikasi": 0}
    server_counts = {"Sangat Aktif": 0, "Aktif": 0, "SSL Warning": 0, "Non-Aktif": 0}
    transparency_counts = {"Lengkap": 0, "Sedang": 0, "Terbatas": 0, "Tidak Transparan": 0}

    for j in journals:
        r = _clean_entry_value(j.peringkat_sinta)
        rank_counts[r] = rank_counts.get(r, 0) + 1

        apc_val = _clean_entry_value(j.apc_status).lower()
        if (j.apc_amount == 0) or ("gratis" in apc_val) or ("free" in apc_val) or ("rp 0" in apc_val):
            apc_counts["Gratis"] += 1
        elif "rp" in apc_val or "usd" in apc_val or "$" in apc_val:
            apc_counts["Berbayar"] += 1
        else:
            apc_counts["Belum Teridentifikasi"] += 1

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
    lines.append("### 💰 Estimasi Biaya Publikasi (APC):")
    lines.append(f"- **Gratis / Free (Rp 0)**: {apc_counts['Gratis']} jurnal")
    lines.append(f"- **Berbayar (Terindikasi Tarif)**: {apc_counts['Berbayar']} jurnal")
    lines.append(f"- **Informasi Belum Tertera di Situs**: {apc_counts['Belum Teridentifikasi']} jurnal")
    lines.append("")
    lines.append("### 🌐 Indikator Server OJS & Transparansi Kontak:")
    lines.append(f"- **Status Server**: {server_counts['Sangat Aktif']} Sangat Aktif (HTTPS), {server_counts['Aktif']} Aktif (HTTP), {server_counts['SSL Warning']} SSL Warning, {server_counts['Non-Aktif']} Non-Aktif")
    lines.append(f"- **Kualitas Kontak**: {transparency_counts['Lengkap']} Lengkap (Email & Telp), {transparency_counts['Sedang']} Sedang (Email Saja), {transparency_counts['Terbatas']} Terbatas, {transparency_counts['Tidak Transparan']} Belum Tercantum")
    lines.append("")
    lines.append("---")
    lines.append("")

    # 2. Tabel Utama
    if not journals:
        lines.append("## ⚠️ Tidak ada jurnal yang sesuai dengan kriteria filter.")
        lines.append("")
        lines.append("Silakan coba kembali dengan kata kunci yang lebih umum atau perluas cakupan filter SINTA.")
    else:
        has_match_score = any(getattr(j, "match_score", 0) > 0 for j in journals)
        lines.append(f"## 📋 Daftar Hasil Jurnal ({total} Data)")
        lines.append("")
        if has_match_score:
            lines.append("| No | Nama Jurnal | SINTA | Kesesuaian | Skor | Biaya APC | Frekuensi | Transparansi | Status Server | Focus & Scope | Link OJS |")
            lines.append("|:---:|---|:---:|:---:|:---:|---|---|:---:|:---:|---|:---:|")
        else:
            lines.append("| No | Nama Jurnal | SINTA | Penerbit | Biaya APC | Frekuensi | Transparansi | Status Server | Focus & Scope | Link OJS |")
            lines.append("|:---:|---|:---:|---|---|---|:---:|:---:|---|:---:|")

        for i, j in enumerate(journals, 1):
            nama = _clean_entry_value(j.nama_jurnal).replace("|", "\\|")
            penerbit = _clean_entry_value(j.penerbit).replace("|", "\\|")
            rank = _clean_entry_value(j.peringkat_sinta)
            apc = _clean_entry_value(j.apc_status)
            freq = _clean_entry_value(getattr(j, "frekuensi", INFO_NOT_FOUND))
            transparansi = _clean_entry_value(getattr(j, "transparansi_kontak", "Tidak Transparan"))
            status_server = _clean_entry_value(getattr(j, "status_server", "Aktif (HTTP)"))
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
                lines.append(f"| {i} | **{nama}** | {rank} | **{kesesuaian}** | `{skor}` | {apc} | {freq} | {transparansi} | {status_server} | {focus} | {link_col} |")
            else:
                lines.append(f"| {i} | **{nama}** | {rank} | {penerbit} | {apc} | {freq} | {transparansi} | {status_server} | {focus} | {link_col} |")

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
                apc_s = _clean_entry_value(j.apc_status)
                if apc_s != INFO_NOT_FOUND:
                    lines.append(f"- 💰 **Biaya Publikasi (APC)**: `{apc_s}`")
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
    lines.append("*Dibuat otomatis oleh **JSFS (Journal Scraper For Students) v1.3.8***")
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
    """Ekspor hasil ke CSV dengan encoding UTF-8 BOM dan validasi link SSL-tolerant."""
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
    Ekspor hasil ke Excel (.xlsx) dengan visual profesional v1.3.8:
    - Header Navy Blue (#1B365D) dengan teks putih bold.
    - URL OJS yang aktif sebagai Hyperlink interaktif (=HYPERLINK).
    - URL OJS yang mati ditampilkan sebagai teks 'Situs tidak dapat diakses' tanpa hyperlink.
    - Warna sel lembut untuk peringkat SINTA (S1/S2 Hijau, S3/S4 Biru, S5/S6 Abu-abu).
    - Border tipis dan auto-fit lebar kolom.
    """
    _ensure_output_dir()
    final_filename = filename or generate_output_filename(keyword, "xlsx")
    filepath = os.path.join(OUTPUT_DIR, final_filename)

    # Validasi link OJS secara asinkronus dengan toleransi SSL
    all_urls = [j.link_ojs for j in journals if j.link_ojs and j.link_ojs.startswith("http")]
    url_status = validate_urls_async(all_urls)

    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = "Hasil Jurnal JSFS"

    has_match_score = any(getattr(j, "match_score", 0) > 0 for j in journals)

    # Definisi Header Kolom
    headers = [
        "No",
        "Nama Jurnal",
        "Peringkat SINTA",
    ]
    if has_match_score:
        headers.extend(["Kesesuaian Naskah", "Match Score (%)"])
    headers.extend([
        "Penerbit",
        "Biaya Publikasi (APC)",
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

    # Definisi Styling Cell untuk Peringkat SINTA, Transparansi & Server
    fill_s1_s2 = PatternFill(start_color="DCFCE7", end_color="DCFCE7", fill_type="solid")  # Hijau Lembut
    font_s1_s2 = Font(name="Segoe UI", size=10, bold=True, color="166534")

    fill_s3_s4 = PatternFill(start_color="DBEAFE", end_color="DBEAFE", fill_type="solid")  # Biru Lembut
    font_s3_s4 = Font(name="Segoe UI", size=10, bold=True, color="1E40AF")

    fill_s5_s6 = PatternFill(start_color="F3F4F6", end_color="F3F4F6", fill_type="solid")  # Abu-abu Lembut
    font_s5_s6 = Font(name="Segoe UI", size=10, bold=True, color="374151")

    fill_warning = PatternFill(start_color="FEF3C7", end_color="FEF3C7", fill_type="solid")  # Kuning / Amber
    font_warning = Font(name="Segoe UI", size=10, bold=True, color="92400E")

    fill_danger = PatternFill(start_color="FEE2E2", end_color="FEE2E2", fill_type="solid")   # Merah Lembut
    font_danger = Font(name="Segoe UI", size=10, bold=True, color="991B1B")

    regular_font = Font(name="Segoe UI", size=10)
    link_font = Font(name="Segoe UI", size=10, color="1D4ED8", underline="single")
    dead_link_font = Font(name="Segoe UI", size=9, italic=True, color="9CA3AF")

    link_col_idx = headers.index("Link OJS / Website") + 1
    rank_col_idx = headers.index("Peringkat SINTA") + 1
    trans_col_idx = headers.index("Transparansi Kontak") + 1
    status_col_idx = headers.index("Status Server OJS") + 1
    focus_col_idx = headers.index("Focus & Scope") + 1
    match_col_idx = headers.index("Kesesuaian Naskah") + 1 if has_match_score else -1

    # 2. Tulis Data Jurnal & Terapkan Styling
    for row_num, j in enumerate(journals, start=2):
        raw_url = j.link_ojs or ""
        is_link_alive = raw_url.startswith("http") and url_status.get(raw_url, False)

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
            _clean_entry_value(j.apc_status),
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

        # Format setiap cell dalam baris
        for col_idx in range(1, len(headers) + 1):
            cell = ws.cell(row=row_num, column=col_idx)
            cell.font = regular_font
            cell.border = thin_border
            cell.alignment = left_align

            # Kolom No, Peringkat, Transparansi, Status Server dibuat rata tengah
            if col_idx in (1, rank_col_idx, trans_col_idx, status_col_idx):
                cell.alignment = center_align

            # Kolom Focus & Scope dibungkus (wrap text)
            if col_idx == focus_col_idx:
                cell.alignment = wrap_left_align

            # Pewarnaan Khusus Peringkat SINTA
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

            # Pewarnaan Khusus Transparansi Kontak
            if col_idx == trans_col_idx:
                val_tr = str(cell.value)
                if "Lengkap" in val_tr:
                    cell.fill = fill_s1_s2
                    cell.font = font_s1_s2
                elif "Sedang" in val_tr:
                    cell.fill = fill_s3_s4
                    cell.font = font_s3_s4
                elif "Terbatas" in val_tr:
                    cell.fill = fill_warning
                    cell.font = font_warning
                else:
                    cell.fill = fill_s5_s6
                    cell.font = font_s5_s6

            # Pewarnaan Khusus Status Server OJS
            if col_idx == status_col_idx:
                val_srv = str(cell.value)
                if "Sangat Aktif" in val_srv:
                    cell.fill = fill_s1_s2
                    cell.font = font_s1_s2
                elif "Aktif (HTTP)" in val_srv:
                    cell.fill = fill_s3_s4
                    cell.font = font_s3_s4
                elif "SSL Warning" in val_srv:
                    cell.fill = fill_warning
                    cell.font = font_warning
                elif "Non-Aktif" in val_srv:
                    cell.fill = fill_danger
                    cell.font = font_danger

            # Pewarnaan Khusus Kesesuaian Naskah
            if col_idx == match_col_idx:
                val_m = str(cell.value)
                if val_m == "Sangat Sesuai":
                    cell.fill = fill_s1_s2
                    cell.font = font_s1_s2
                elif val_m == "Sesuai":
                    cell.fill = fill_s3_s4
                    cell.font = font_s3_s4
                cell.alignment = center_align

            # Hyperlink Interaktif untuk Link OJS / Website
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


def print_summary_table(journals: list[JournalEntry]):
    """Tampilkan ringkasan hasil di terminal menggunakan Rich."""
    from rich.table import Table

    if not journals:
        console.print("[yellow]⚠ Tidak ada jurnal untuk ditampilkan.[/yellow]")
        return

    table = Table(
        title=f"📚 Hasil JSFS v1.3.8 — {len(journals)} Jurnal Ditemukan",
        show_lines=True,
        highlight=True,
    )
    table.add_column("No", style="dim", width=4, justify="right")
    table.add_column("Nama Jurnal", style="bold", min_width=28)
    table.add_column("SINTA", justify="center", width=8)
    table.add_column("Penerbit", style="cyan", min_width=18)
    table.add_column("APC", width=16)
    table.add_column("Frekuensi", width=14)
    table.add_column("Email Kontak", style="magenta", width=20)
    table.add_column("Link OJS", style="blue underline")

    for i, j in enumerate(journals, 1):
        rank_val = _clean_entry_value(j.peringkat_sinta)
        rank_color = {
            "S1": "bright_green", "S2": "green", "S3": "cyan",
            "S4": "blue", "S5": "yellow", "S6": "white",
        }.get(rank_val.replace("Sinta ", "S"), "white")

        table.add_row(
            str(i),
            _clean_entry_value(j.nama_jurnal),
            f"[{rank_color}]{rank_val}[/{rank_color}]",
            _clean_entry_value(j.penerbit),
            _clean_entry_value(j.apc_status),
            _clean_entry_value(getattr(j, "frekuensi", INFO_NOT_FOUND)),
            _clean_entry_value(getattr(j, "email", INFO_NOT_FOUND)),
            j.link_ojs or INFO_NOT_FOUND,
        )

    console.print(table)
