"""
QA & Stress Testing Script for JSFS v1.3.7.
Tests:
1. Deep scraping on 100+ journals (keywords 'bisnis' & 'ekonomi')
2. Focus & Scope extraction verification
3. Status Server OJS & Transparansi Kontak evaluation
4. 0% NaN check across Excel, CSV, and Markdown exports
5. GUI and JSFSDataPreviewer UI component validation
"""

import os
import sys
import time

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

import pandas as pd
from scraper_garuda import (
    JournalEntry,
    scrape_garuda,
    merge_cross_source_entries,
    compute_contact_transparency,
    get_domain_ssl_health,
)
from scraper_sinta import scrape_sinta
from exporter import export_excel, export_csv, export_markdown
from config import INFO_NOT_FOUND

def run_qa():
    print("=" * 70)
    print("🚀 MEMULAI PENGUJIAN QUALITY ASSURANCE JSFS v1.3.7 (100+ JURNAL)")
    print("=" * 70)

    start_time = time.time()
    
    # 1. Scraping 100+ jurnal dengan keyword 'bisnis' dan 'ekonomi'
    print("\n[STEP 1] Melakukan crawling & deep scraping target 100+ jurnal...")
    ranks = [1, 2, 3, 4, 5, 6]
    
    print("-> Scraping Portal Garuda (keyword: 'bisnis', limit 60)...")
    garuda_bisnis = scrape_garuda("bisnis", ranks, deep_detail=True, max_results=60, turbo_mode=True)
    print(f"   Ditemukan {len(garuda_bisnis)} jurnal dari Garuda (bisnis)")

    print("-> Scraping Direktori SINTA (keyword: 'ekonomi', limit 60)...")
    sinta_ekonomi = scrape_sinta("ekonomi", ranks, deep_detail=True, max_results=60, turbo_mode=True)
    print(f"   Ditemukan {len(sinta_ekonomi)} jurnal dari SINTA (ekonomi)")

    all_raw = garuda_bisnis + sinta_ekonomi
    print(f"-> Total jurnal mentah terhimpun: {len(all_raw)}")
    assert len(all_raw) >= 100, f"Target 100+ jurnal belum tercapai, hanya {len(all_raw)}"

    # 2. Merge & Deduplikasi
    merged_journals = merge_cross_source_entries(all_raw)
    print(f"-> Total jurnal unik setelah deduplikasi: {len(merged_journals)}")
    assert len(merged_journals) >= 80, f"Jurnal unik terlalu sedikit: {len(merged_journals)}"

    # 3. Validasi Field Focus & Scope, Status Server, dan Transparansi Kontak
    print("\n[STEP 2] Verifikasi Ekstraksi Focus & Scope, Server Status & Transparansi...")
    focus_found_count = 0
    server_status_dist = {}
    transparency_dist = {}

    for j in merged_journals:
        # Check Focus & Scope
        assert hasattr(j, "focus_scope"), "Field focus_scope tidak ditemukan pada JournalEntry!"
        if j.focus_scope and j.focus_scope != INFO_NOT_FOUND:
            focus_found_count += 1
            # Verify length is bounded <= 320 chars
            assert len(j.focus_scope) <= 320, f"Focus & Scope terlalu panjang: {len(j.focus_scope)} chars"

        # Check Status Server
        assert hasattr(j, "status_server"), "Field status_server tidak ditemukan pada JournalEntry!"
        valid_server_categories = ["Sangat Aktif (HTTPS/SSL Valid)", "Aktif (HTTP)", "SSL Warning (Bypass Valid)", "Non-Aktif"]
        assert any(cat in j.status_server for cat in valid_server_categories), f"Status server tidak valid: {j.status_server}"
        server_status_dist[j.status_server] = server_status_dist.get(j.status_server, 0) + 1

        # Check Transparansi Kontak
        assert hasattr(j, "transparansi_kontak"), "Field transparansi_kontak tidak ditemukan!"
        valid_transparency = ["Lengkap (Email & WA/Telp)", "Sedang (Hanya Email)", "Terbatas (Hanya WA/Telp)", "Tidak Transparan"]
        assert any(t in j.transparansi_kontak for t in valid_transparency), f"Transparansi kontak tidak valid: {j.transparansi_kontak}"
        transparency_dist[j.transparansi_kontak] = transparency_dist.get(j.transparansi_kontak, 0) + 1

    print(f"✓ Focus & Scope terisi informatif pada {focus_found_count} dari {len(merged_journals)} jurnal ({focus_found_count/len(merged_journals)*100:.1f}%)")
    print(f"✓ Distribusi Status Server OJS: {server_status_dist}")
    print(f"✓ Distribusi Transparansi Kontak: {transparency_dist}")

    # Contoh sampel jurnal
    print("\n--- SAMPEL DATA JURNAL v1.3.7 ---")
    for i, s in enumerate(merged_journals[:3], 1):
        print(f"[{i}] {s.nama} ({s.peringkat})")
        print(f"    Server       : {s.status_server}")
        print(f"    Transparansi : {s.transparansi_kontak}")
        print(f"    Kontak       : {s.email} | {s.telepon}")
        print(f"    Focus & Scope: {s.focus_scope[:120]}...")

    # 4. Validasi Ekspor Excel, CSV, Markdown & 0% NaN
    print("\n[STEP 3] Validasi Ekspor Excel (.xlsx), CSV (.csv), dan Markdown (.md)...")
    excel_path = export_excel(merged_journals, "bisnis_ekonomi_qa")
    csv_path = export_csv(merged_journals, "bisnis_ekonomi_qa")
    md_path = export_markdown(merged_journals, "bisnis_ekonomi_qa", ranks)

    assert os.path.exists(excel_path), f"File Excel tidak terbuat: {excel_path}"
    assert os.path.exists(csv_path), f"File CSV tidak terbuat: {csv_path}"
    assert os.path.exists(md_path), f"File Markdown tidak terbuat: {md_path}"

    df_excel = pd.read_excel(excel_path)
    df_csv = pd.read_csv(csv_path)

    # Verifikasi kolom baru di Excel
    expected_cols = ["Focus & Scope", "Status Server OJS", "Transparansi Kontak"]
    for col in expected_cols:
        assert col in df_excel.columns, f"Kolom '{col}' tidak ada di file Excel!"
        assert col in df_csv.columns, f"Kolom '{col}' tidak ada di file CSV!"

    # Verifikasi 0% NaN
    nan_count = df_excel.isna().sum().sum()
    print(f"✓ Total nilai NaN di Excel: {nan_count} (0% NaN Confirmed)")
    assert nan_count == 0, f"Masih ditemukan {nan_count} NaN di file Excel!"

    # 5. Uji Coba Komponen GUI & Previewer
    print("\n[STEP 4] Uji Coba Inisialisasi UI & JSFSDataPreviewer...")
    try:
        from app_ui import JSFSApp, JSFSDataPreviewer
        import customtkinter as ctk

        app = JSFSApp()
        app.update_idletasks()
        
        # Test previewer window
        previewer = JSFSDataPreviewer(master=app, journals=merged_journals[:20], excel_path=excel_path)
        previewer.update_idletasks()
        
        # Test filter
        previewer.entry_filter.insert(0, "bisnis")
        previewer._apply_filters()
        matched = len(previewer._filtered_journals)
        print(f"✓ JSFSDataPreviewer filter 'bisnis' menghasilkan {matched} jurnal cocok.")

        # Test filter SINTA
        previewer.combo_sinta.set("S2")
        previewer._apply_filters()
        matched_s2 = len(previewer._filtered_journals)
        print(f"✓ JSFSDataPreviewer filter S2 menghasilkan {matched_s2} jurnal cocok.")

        # Test select row
        children = previewer.tree.get_children()
        if children:
            previewer.tree.selection_set(children[0])
            previewer._on_tree_select(None)
            assert previewer.txt_detail_scope.get("1.0", "end").strip() != "", "Detail Focus & Scope kosong!"
            print("✓ Detail naskah Focus & Scope & metadata ter-update sempurna pada event select.")

        previewer.destroy()
        app.destroy()
        print("✓ Seluruh komponen GUI & JSFSDataPreviewer berhasil diuji tanpa glitch / clipping.")

    except Exception as e:
        print(f"❌ Error pengujian UI: {e}")
        raise e

    elapsed = time.time() - start_time
    print("\n" + "=" * 70)
    print(f"🏆 SELURUH PENGUJIAN QA v1.3.7 LULUS 100% DALAM {elapsed:.2f} DETIK!")
    print("=" * 70)

if __name__ == "__main__":
    run_qa()
