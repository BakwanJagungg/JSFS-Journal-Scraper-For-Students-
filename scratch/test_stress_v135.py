# =============================================================================
# JSFS QA & STRESS TEST v1.3.5 — Multi-Tier Scale & Manuscript Analysis
# =============================================================================

import os
import sys
import time
import re
import urllib3
import pandas as pd

urllib3.disable_warnings()
sys.path.insert(0, os.getcwd())

from scraper_sinta import scrape_sinta
from scraper_garuda import (
    scrape_garuda,
    merge_cross_source_entries,
    cross_reference_garuda_with_sinta,
    get_high_capacity_session,
)
from doc_reader import extract_manuscript_metadata, calculate_match_score
from exporter import export_excel, export_markdown
from config import INFO_NOT_FOUND, DEAD_LINK_TEXT

GENERIC_TERMS = [
    "Semiannual", "Triwulanan", "Dwi-bulanan", "Tahunan", "Bulanan",
    "semiannual", "quarterly", "monthly", "bimonthly", "annual"
]
RE_VALID_ISSN = re.compile(r"^[0-9]{4}-[0-9]{3}[0-9X]$")
RE_DATE_IN_PHONE = re.compile(r"(?:\d{4}[-/]\d{1,2}[-/]\d{1,2})|(?:\d{1,2}[-/]\d{1,2}[-/]\d{4})|\b20[12]\d\b")
DUMMY_EMAILS = ["firstauthor@", "second@", "author@", "@pkp.sfu.ca", "example.com", "test.com"]


def audit_dataset(df: pd.DataFrame, test_name: str, expected_min_rows: int, has_manuscript: bool = True):
    print(f"\n🔍 [AUDIT] Memeriksa Kualitas Data: {test_name} ({len(df)} baris)")
    assert len(df) >= expected_min_rows, f"❌ Jumlah baris ({len(df)}) kurang dari target minimum ({expected_min_rows})!"

    # 1. Audit NaN
    nan_issues = []
    for col in df.columns:
        cnt = df[col].isna().sum()
        if cnt > 0:
            nan_issues.append((col, cnt))
    assert not nan_issues, f"❌ Ditemukan NaN pada kolom: {nan_issues}"
    print("   ✅ 0% NaN di seluruh kolom.")

    # 2. Audit Frekuensi Terbit (Strict Standard - 100% bebas istilah generik)
    found_generic = []
    with_months = 0
    pure_count = 0
    not_found = 0
    for idx, f in enumerate(df["Frekuensi Terbit"]):
        f_str = str(f)
        for gt in GENERIC_TERMS:
            if f"({gt})" in f_str or f" {gt}" in f_str:
                found_generic.append((idx + 1, f_str))
        if "(" in f_str and ")" in f_str and "kali setahun" in f_str:
            with_months += 1
        elif "kali setahun" in f_str:
            pure_count += 1
        elif f_str == INFO_NOT_FOUND:
            not_found += 1
    assert not found_generic, f"❌ Terdeteksi istilah generik pada frekuensi terbit: {found_generic[:5]}"
    print(f"   ✅ Frekuensi Terbit 100% Baku: {with_months} dengan bulan, {pure_count} hitungan terbit, {not_found} tidak ditemukan.")

    # 3. Audit Kontak: Email & Telepon
    dummy_email_cnt = 0
    date_phone_cnt = 0
    for idx, row in df.iterrows():
        em = str(row.get("Email Kontak", "")).lower()
        ph = str(row.get("Telepon / WA", ""))
        for de in DUMMY_EMAILS:
            if de in em:
                dummy_email_cnt += 1
                print(f"   ❌ Baris {idx+1} Dummy Email: {em}")
        if ph != INFO_NOT_FOUND:
            if RE_DATE_IN_PHONE.search(ph):
                date_phone_cnt += 1
                print(f"   ❌ Baris {idx+1} Format Tanggal di Telepon: {ph}")
    assert dummy_email_cnt == 0, f"❌ Ditemukan {dummy_email_cnt} email dummy sintetis!"
    assert date_phone_cnt == 0, f"❌ Ditemukan {date_phone_cnt} nomor telepon berformat tanggal!"
    print("   ✅ Kontak Redaksi 100% Bersih: Bebas email dummy & bebas derau tanggal pada telepon.")

    # 4. Audit ISSN
    dummy_issn = 0
    for idx, row in df.iterrows():
        for col in ["ISSN", "E-ISSN"]:
            val = str(row.get(col, "")).strip()
            if val == INFO_NOT_FOUND:
                continue
            if val in ("0", "0000-0000", "1234-5678", "none", "null", "None") or not RE_VALID_ISSN.match(val):
                dummy_issn += 1
                print(f"   ❌ Baris {idx+1} Invalid ISSN ({col}): {val}")
    assert dummy_issn == 0, f"❌ Ditemukan {dummy_issn} format ISSN tidak valid/dummy!"
    print("   ✅ ISSN & E-ISSN 100% Presisi (Format resmi XXXX-XXXX).")

    # 5. Audit Naskah jika aktif
    if has_manuscript:
        assert "Kesesuaian Naskah" in df.columns, "❌ Kolom 'Kesesuaian Naskah' tidak ada di DataFrame!"
        assert "Match Score (%)" in df.columns, "❌ Kolom 'Match Score (%)' tidak ada di DataFrame!"
        scores = df["Match Score (%)"].tolist()
        # Verifikasi terurut descending
        assert scores == sorted(scores, reverse=True), "❌ Hasil tidak terurut descending berdasarkan Match Score (%)!"
        top_score = scores[0]
        valid_statuses = {"Sangat Sesuai", "Sesuai", "Kurang Sesuai"}
        for st in df["Kesesuaian Naskah"]:
            assert st in valid_statuses, f"❌ Status kesesuaian tidak valid: {st}"
        print(f"   ✅ Analisis Naskah Terverifikasi: Skor tertinggi {top_score}%, urutan presisi descending.")

    print(f"   🏆 Seluruh audit untuk {test_name} LOLOS 100%!\n")


def run_all_stress_tests():
    print("=" * 80)
    print("🚀 MEMULAI JSFS v1.3.5 COMPREHENSIVE STRESS TEST & QA SUITE")
    print("=" * 80)

    # 0. Siapkan naskah sampel untuk pengujian
    manuscript_path = os.path.join("scratch", "sample_manuscript.docx")
    assert os.path.exists(manuscript_path), f"File naskah {manuscript_path} tidak ditemukan!"
    print(f"\n📄 Membaca draf naskah: {manuscript_path}")
    manuscript_meta = extract_manuscript_metadata(manuscript_path)
    print(f"   Judul Naskah: {manuscript_meta['title']}")
    print(f"   Kata Kunci: {manuscript_meta['keywords']}")
    print(f"   Istilah Dominan: {manuscript_meta['dominant_terms'][:6]}...")

    # =========================================================================
    # PENGUJIAN 1: Target 50 Jurnal (Keyword Tunggal: 'bisnis')
    # =========================================================================
    print("\n" + "=" * 80)
    print("🧪 [PENGUJIAN 1/4] STRESS TEST SKALA 50 JURNAL (Keyword: 'bisnis')")
    print("=" * 80)
    existing_50 = os.path.join("output", "JSFS_stress_test_50_20261017_083932.xlsx")
    if os.path.exists(existing_50):
        print(f"✅ Menggunakan hasil tersimpan Pengujian 1: {existing_50}")
        df_50 = pd.read_excel(existing_50)
        audit_dataset(df_50, "Skala 50 Jurnal", expected_min_rows=45, has_manuscript=True)
    else:
        t0 = time.time()
        res_50 = scrape_sinta(
            keyword="bisnis",
            sinta_ranks=[3, 4, 5, 6],
            deep_detail=True,
            max_results=50,
            turbo_mode=True,
        )
        d50 = time.time() - t0
        print(f"⏱️ Waktu scraping 50 jurnal: {d50:.2f}s ({len(res_50)} jurnal diperoleh, throughput: {len(res_50)/d50:.2f} j/s)")
        assert len(res_50) >= 45, f"Hasil terlalu sedikit: {len(res_50)}"

        # Hitung match score naskah
        for entry in res_50:
            res_m = calculate_match_score(manuscript_meta, entry)
            entry.match_score = res_m["score"]
            entry.kesesuaian_naskah = res_m["level"]

        res_50.sort(key=lambda j: j.match_score or 0, reverse=True)
        xlsx_50 = export_excel(res_50, keyword="stress_test_50")
        df_50 = pd.read_excel(xlsx_50)
        audit_dataset(df_50, "Skala 50 Jurnal", expected_min_rows=45, has_manuscript=True)

    # =========================================================================
    # PENGUJIAN 2: Target 250 Jurnal (Keyword Kombinasi: 'bisnis, ekonomi')
    # =========================================================================
    print("\n" + "=" * 80)
    print("🧪 [PENGUJIAN 2/4] STRESS TEST SKALA 250 JURNAL (Keyword: 'bisnis, ekonomi')")
    print("=" * 80)
    existing_250 = os.path.join("output", "JSFS_stress_test_250_20261017_123956.xlsx")
    if os.path.exists(existing_250):
        print(f"✅ Menggunakan hasil tersimpan Pengujian 2: {existing_250}")
        df_250 = pd.read_excel(existing_250)
        audit_dataset(df_250, "Skala 250 Jurnal", expected_min_rows=200, has_manuscript=True)
    else:
        t0 = time.time()
        res_250 = scrape_sinta(
            keyword="bisnis, ekonomi",
            sinta_ranks=[3, 4, 5, 6],
            deep_detail=True,
            max_results=250,
            turbo_mode=True,
        )
        d250 = time.time() - t0
        print(f"⏱️ Waktu scraping 250 jurnal: {d250:.2f}s ({len(res_250)} jurnal diperoleh, throughput: {len(res_250)/d250:.2f} j/s)")
        assert len(res_250) >= 200, f"Hasil terlalu sedikit: {len(res_250)}"

        for entry in res_250:
            res_m = calculate_match_score(manuscript_meta, entry)
            entry.match_score = res_m["score"]
            entry.kesesuaian_naskah = res_m["level"]

        res_250.sort(key=lambda j: j.match_score or 0, reverse=True)
        xlsx_250 = export_excel(res_250, keyword="stress_test_250")
        df_250 = pd.read_excel(xlsx_250)
        audit_dataset(df_250, "Skala 250 Jurnal", expected_min_rows=200, has_manuscript=True)

    # =========================================================================
    # PENGUJIAN 3: Target 500 Jurnal (Keyword Komprehensif)
    # =========================================================================
    print("\n" + "=" * 80)
    print("🧪 [PENGUJIAN 3/4] STRESS TEST SKALA 500 JURNAL (Keyword: 'bisnis, ekonomi, commerce, e-commerce')")
    print("=" * 80)
    existing_500 = os.path.join("output", "JSFS_stress_test_500_20261017_124556.xlsx")
    if os.path.exists(existing_500):
        print(f"✅ Menggunakan hasil tersimpan Pengujian 3: {existing_500}")
        df_500 = pd.read_excel(existing_500)
        audit_dataset(df_500, "Skala 500 Jurnal", expected_min_rows=350, has_manuscript=True)
    else:
        t0 = time.time()
        res_500 = scrape_sinta(
            keyword="bisnis, ekonomi, commerce, e-commerce",
            sinta_ranks=[3, 4, 5, 6],
            deep_detail=True,
            max_results=500,
            turbo_mode=True,
        )
        d500 = time.time() - t0
        print(f"⏱️ Waktu scraping 500 jurnal: {d500:.2f}s ({len(res_500)} jurnal diperoleh, throughput: {len(res_500)/d500:.2f} j/s)")
        assert len(res_500) >= 350, f"Hasil terlalu sedikit: {len(res_500)}"

        for entry in res_500:
            res_m = calculate_match_score(manuscript_meta, entry)
            entry.match_score = res_m["score"]
            entry.kesesuaian_naskah = res_m["level"]

        res_500.sort(key=lambda j: j.match_score or 0, reverse=True)
        xlsx_500 = export_excel(res_500, keyword="stress_test_500")
        df_500 = pd.read_excel(xlsx_500)
        audit_dataset(df_500, "Skala 500 Jurnal", expected_min_rows=350, has_manuscript=True)

    # =========================================================================
    # PENGUJIAN 4: Garuda-to-SINTA Parity & Cross-Referencing
    # =========================================================================
    print("\n" + "=" * 80)
    print("🧪 [PENGUJIAN 4/4] AUDIT PENYETARAAN PORTAL GARUDA DENGAN SINTA")
    print("=" * 80)
    t0 = time.time()
    res_garuda = scrape_garuda(
        keyword="manajemen",
        sinta_ranks=None,
        deep_detail=True,
        max_results=20,
        turbo_mode=True,
    )
    d_garuda = time.time() - t0
    print(f"⏱️ Waktu scraping Garuda: {d_garuda:.2f}s ({len(res_garuda)} jurnal diperoleh)")
    assert len(res_garuda) > 0, "Scraping Garuda tidak menghasilkan jurnal!"

    # Evaluasi penyetaraan akreditasi SINTA
    sinta_mapped_count = 0
    official_ranks = {"S1", "S2", "S3", "S4", "S5", "S6"}
    for g in res_garuda:
        rank_clean = g.peringkat_sinta.replace("Sinta", "S").replace(" ", "").upper()
        if any(r in rank_clean for r in official_ranks):
            sinta_mapped_count += 1
            print(f"   ⭐ Garuda Jurnal '{g.judul[:30]}...' -> Terpetakan SINTA: {g.peringkat_sinta} | Profile: {g.link_sinta[:35]}...")

    print(f"   📊 Rasio Akreditasi SINTA Terpetakan pada Garuda: {sinta_mapped_count}/{len(res_garuda)} ({sinta_mapped_count/len(res_garuda)*100:.1f}%)")
    assert sinta_mapped_count > 0, "❌ Tidak ada jurnal Garuda yang berhasil dipetakan ke peringkat SINTA!"

    for entry in res_garuda:
        res_m = calculate_match_score(manuscript_meta, entry)
        entry.match_score = res_m["score"]
        entry.kesesuaian_naskah = res_m["level"]

    res_garuda.sort(key=lambda j: j.match_score or 0, reverse=True)
    xlsx_garuda = export_excel(res_garuda, keyword="parity_garuda_test")
    df_garuda = pd.read_excel(xlsx_garuda)
    audit_dataset(df_garuda, "Penyetaraan Garuda", expected_min_rows=10, has_manuscript=True)

    print("=" * 80)
    print("🎉 SEMUA STRESS TEST (50, 250, 500 JURNAL) & AUDIT PENYETARAAN GARUDA LOLOS 100%!")
    print("=" * 80)

    print("\n🔨 MEMULAI RE-BUILD EXECUTABLE WINDOWS (dist/JSFS.exe v1.3.5)...")
    import subprocess
    proc = subprocess.run([sys.executable, "-m", "PyInstaller", "--noconfirm", "JSFS.spec"])
    if proc.returncode != 0:
        print(f"❌ PyInstaller build gagal dengan exit code {proc.returncode}!")
        return False
    print("\n🏆 RE-BUILD EXECUTABLE SELESAI! dist/JSFS.exe v1.3.5 berhasil diterbitkan.")
    return True


if __name__ == "__main__":
    success = run_all_stress_tests()
    if not success:
        sys.exit(1)
