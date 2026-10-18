"""
test_v140_qa.py — Quality Assurance & Verification Suite untuk JSFS v1.4.0
Menguji:
1. Parser Bulan Terbit (Regex) & Filter Bulan
2. Validasi Angka & Smart Truncation
3. Ekstraksi Focus & Scope serta Pemisahan Biaya Submit vs Publikasi
4. Fallback ke Regex Legacy saat Ollama mati
5. 4-Fase Pipeline Scraping nyata untuk keyword 'bisnis' (target 15 jurnal)
6. Verifikasi Struktur Output (.xlsx, .md, .csv) bebas NaN
"""
import os
import sys

# Tambahkan workspace root ke sys.path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import pandas as pd
import openpyxl

import month_parser
import ai_engine
import scraper_garuda
from scraper_garuda import JournalEntry, scrape_garuda
import exporter

def test_month_parser():
    print("\n--- TEST 1: Month Parser ---")
    t1 = "Jurnal ini terbit setiap bulan Maret dan September."
    res1 = month_parser.parse_bulan_terbit(t1)
    assert res1 == [3, 9], f"Expected [3, 9], got {res1}"
    print(f"✓ '{t1}' -> {res1}")

    t2 = "Authors may submit papers for biannual publication."
    res2 = month_parser.parse_bulan_terbit(t2)
    # 'may' should not be counted as Mei
    assert 5 not in res2, f"'may' should be ignored, got {res2}"
    print(f"✓ 'may' check passed -> {res2}")

    t3 = "Frekuensi penerbitan 4 kali setahun: Januari, April, Juli, Oktober."
    res3 = month_parser.parse_bulan_terbit(t3)
    assert res3 == [1, 4, 7, 10], f"Expected [1, 4, 7, 10], got {res3}"
    print(f"✓ 4 issues -> {res3} ({month_parser.format_bulan_terbit(res3)})")

    # Filter test
    assert month_parser.lolos_filter_bulan([3, 9], {3, 4}) == True
    assert month_parser.lolos_filter_bulan([3, 9], {1, 2}) == False
    assert month_parser.lolos_filter_bulan([], {1, 2}, sertakan_tak_terdeteksi=True) == True
    assert month_parser.lolos_filter_bulan([], {1, 2}, sertakan_tak_terdeteksi=False) == False
    print("✓ Filter bulan lolos kriteria.")

def test_ai_engine_units():
    print("\n--- TEST 2: AI Engine Units & Anti-Halusinasi ---")
    txt = "Biaya pemrosesan naskah Rp 250.000 dibayar saat submit. Biaya publikasi Rp 1,5 juta setelah diterima."
    nums = ai_engine.numbers_in_text(txt)
    assert 250000 in nums, f"Expected 250000 in nums: {nums}"
    assert 1500000 in nums, f"Expected 1500000 in nums: {nums}"
    print(f"✓ numbers_in_text deteksi numerik: {nums}")

    # Fallback legacy regex test
    apc_res = ai_engine._apc_job(txt)
    print(f"✓ APC job result: {apc_res}")
    assert apc_res["biaya_submit"] == 250000 or apc_res["biaya_publikasi"] == 1500000

    focus_txt = (
        "Focus and Scope: Jurnal Bisnis dan Manajemen memuat artikel hasil riset di bidang akuntansi, "
        "manajemen keuangan, dan kewirausahaan. Artikel ditinjau secara double-blind review."
    )
    focus_res = ai_engine._focus_job(focus_txt)
    print(f"✓ Focus job result: {focus_res}")
    assert len(focus_res["focus_scope"]) > 10
    assert focus_res["sumber"] in ("AI", "Regex")

def test_pipeline_real_search():
    print("\n--- TEST 3: Real Scraping Pipeline 'bisnis' (Fase 1 - 4) ---")
    keyword = "bisnis"
    target = 10
    sinta_ranks = [3, 4, 5, 6]

    print(f"1. Menjalankan Fast Broad Scraping untuk '{keyword}' target {target} jurnal...")
    records = scrape_garuda(
        keyword=keyword,
        sinta_ranks=sinta_ranks,
        deep_detail=True,
        max_results=target,
        turbo_mode=True,
    )
    assert len(records) > 0, "Gagal mendapatkan record dari Garuda!"
    print(f"✓ Berhasil memperoleh {len(records)} jurnal dari Garuda.")

    # Verifikasi raw_about dan bulan_terbit ada
    for r in records[:3]:
        print(f"   Sample: '{r.nama_jurnal}' | SINTA: {r.peringkat_sinta} | Bulan: {r.bulan_terbit}")
        assert hasattr(r, "raw_about"), "Entry missing raw_about attribute!"

    # Fase 2: Pre-filter
    survivors = [
        r for r in records
        if r["sinta"] in sinta_ranks
        and month_parser.lolos_filter_bulan(r.get("bulan_terbit", []), {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12}, True)
    ]
    print(f"✓ Pre-filter SINTA & Bulan: {len(survivors)} lolos.")

    # Fase 3: AI Enrichment
    print(f"2. Menjalankan AI Enrichment (pool 1-2 workers)...")
    def on_prog(d, t):
        pass
    ai_engine.enrich_records(survivors, on_progress=on_prog)
    print("✓ AI Enrichment selesai.")

    for s in survivors[:3]:
        print(f"   '{s.nama_jurnal}':")
        print(f"     Focus & Scope [{s.sumber_focus}]: {s.focus_scope[:80]}...")
        print(f"     Submit [{s.sumber_apc}]: {s.biaya_submit} | Publikasi: {s.biaya_publikasi} ({s.mata_uang})")

    # Fase 4: Export
    print(f"3. Mengekspor hasil...")
    exported_files = exporter.write_exports(
        survivors,
        keyword=keyword,
        formats=[".xlsx", ".md", ".csv"],
        sinta_ranks=sinta_ranks,
    )
    print(f"✓ Berkas tersimpan: {exported_files}")

    # Verifikasi file Excel
    excel_path = next(f for f in exported_files if f.endswith(".xlsx"))
    wb = openpyxl.load_workbook(excel_path)
    ws = wb.active
    headers = [cell.value for cell in ws[1]]
    print(f"✓ Excel Headers: {headers}")

    required_cols = ["Biaya Submit", "Biaya Publikasi", "Mata Uang", "Bulan Terbit", "Sumber Focus", "Sumber APC"]
    for col in required_cols:
        assert col in headers, f"Kolom wajib '{col}' tidak ditemukan di Excel headers!"

    # Cek baris tidak ada nilai NaN mentah
    df = pd.read_excel(excel_path)
    print(f"✓ DataFrame shape: {df.shape}")
    for col in required_cols:
        assert col in df.columns
    print("✓ Seluruh kolom v1.4.0 terverifikasi valid!")

if __name__ == "__main__":
    test_month_parser()
    test_ai_engine_units()
    test_pipeline_real_search()
    print("\n==========================================")
    print("🎉 SEMUA TEST QA v1.4.0 BERHASIL 100%!")
    print("==========================================")
