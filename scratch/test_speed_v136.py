# =============================================================================
# JSFS QA & SPEED OPTIMIZATION TEST v1.3.6
# =============================================================================

import os
import sys
import time
import urllib3
import pandas as pd

urllib3.disable_warnings()
sys.path.insert(0, os.getcwd())

from config import (
    DEEP_SCRAPE_TIMEOUT,
    FAST_FAIL_TIMEOUT,
    DEAD_LINK_TEXT,
    INFO_NOT_FOUND,
)
from scraper_garuda import (
    get_high_capacity_session,
    check_ojs_domain_reachable,
    extract_ojs_deep_info,
    scrape_garuda,
    merge_cross_source_entries,
    JournalEntry,
)
from scraper_sinta import scrape_sinta
from exporter import export_excel


def test_circuit_breaker():
    print("\n" + "=" * 70)
    print("🧪 1. PENGUJIAN FAST-FAIL CIRCUIT BREAKER (OJS REACHABILITY)")
    print("=" * 70)

    session = get_high_capacity_session(is_deep_scrape=True)

    # 1. Domain valid / aktif
    t0 = time.time()
    alive = check_ojs_domain_reachable(session, "https://garuda.kemdiktisaintek.go.id")
    dur_alive = time.time() - t0
    print(f"   ✓ Domain Aktif: reachability={alive} (Durasi: {dur_alive:.3f}s)")
    assert alive is True, "❌ Domain resmi kemdiktisaintek harus terdeteksi aktif!"

    # 2. Domain mati / dummy (Simulasi server OJS down)
    dead_url = "http://dead-ojs-server-unknown-domain-test99.ac.id/index.php"
    t1 = time.time()
    dead_check_1 = check_ojs_domain_reachable(session, dead_url)
    dur_dead_1 = time.time() - t1
    print(f"   ✓ Domain Mati (First Probe): reachability={dead_check_1} (Durasi: {dur_dead_1:.3f}s)")
    assert dead_check_1 is False, "❌ Domain fiktif harus terdeteksi mati!"
    assert dur_dead_1 <= 3.0, f"❌ Fast-fail timeout melebihi toleransi ({dur_dead_1:.2f}s > 3.0s)!"

    # 3. Domain mati yang sama (Harus 0ms via Thread-Safe Cache)
    t2 = time.time()
    dead_check_2 = check_ojs_domain_reachable(session, dead_url)
    dur_dead_2 = time.time() - t2
    print(f"   ✓ Domain Mati (Cached): reachability={dead_check_2} (Durasi: {dur_dead_2*1000:.2f}ms)")
    assert dead_check_2 is False, "❌ Cache harus konsisten False!"
    assert dur_dead_2 < 0.05, f"❌ Cache harus merespon sub-detik ({dur_dead_2:.4f}s)!"

    # 4. Verifikasi extract_ojs_deep_info langsung skip pada domain mati
    t3 = time.time()
    res_dead = extract_ojs_deep_info(session, dead_url)
    dur_extract = time.time() - t3
    print(f"   ✓ extract_ojs_deep_info pada Domain Mati: {res_dead['apc_status']} (Durasi: {dur_extract*1000:.2f}ms)")
    assert res_dead["apc_status"] == DEAD_LINK_TEXT, "❌ Deep info harus langsung menetapkan DEAD_LINK_TEXT!"
    assert dur_extract < 0.05, "❌ Deep info tidak boleh hanging pada domain mati yang tercatat!"

    print("   🏆 FAST-FAIL CIRCUIT BREAKER LOLOS 100%!\n")


def test_gui_compactness():
    print("=" * 70)
    print("🧪 2. PENGUJIAN KEPADATAN TATA LETAK GUI (app_ui.py)")
    print("=" * 70)

    from app_ui import JSFSApp

    app = JSFSApp()
    # Verifikasi geometri
    geo = app.geometry()
    print(f"   ✓ Window Geometry: {geo} (Compact 900x700 Anti-Clipping Target)")
    assert "900x700" in geo, f"❌ Geometri tidak sesuai target 900x700: {geo}"

    # Verifikasi 1-row manuscript action layout
    assert hasattr(app, "ms_action_row"), "❌ ms_action_row tidak ditemukan!"
    assert hasattr(app, "btn_upload_ms"), "❌ btn_upload_ms tidak ditemukan!"
    assert hasattr(app, "chk_match_scoring"), "❌ chk_match_scoring tidak ditemukan!"
    assert hasattr(app, "lbl_ms_status"), "❌ lbl_ms_status tidak ditemukan!"
    assert hasattr(app, "btn_clear_ms"), "❌ btn_clear_ms tidak ditemukan!"
    assert hasattr(app, "btn_scrape"), "❌ btn_scrape tidak ditemukan!"

    # Uji aksi upload & reset naskah tanpa freeze
    dummy_doc = os.path.join("scratch", "sample_manuscript.docx")
    if os.path.exists(dummy_doc):
        import doc_reader
        meta = doc_reader.extract_manuscript_metadata(dummy_doc)
        app._manuscript_meta = meta
        app.chk_match_scoring.select()
        app.btn_clear_ms.pack(side="right")
        assert app._manuscript_meta is not None, "❌ Metadata naskah gagal diset!"
        app._on_clear_manuscript()
        assert app._manuscript_meta is None, "❌ Reset naskah gagal!"
        print("   ✓ Siklus Unggah & Hapus Naskah Kompak Berfungsi Normal.")

    app.destroy()
    print("   🏆 PENGUJIAN GUI COMPACTNESS LOLOS 100%!\n")


def test_speed_and_target_short_circuiting():
    print("=" * 70)
    print("🧪 3. PENGUJIAN SPEED ACCELERATION & TARGET SHORT-CIRCUITING (Target 250 Jurnal)")
    print("=" * 70)

    keyword = "bisnis"
    target_count = 250

    start_time = time.time()
    print(f"⏱️ Memulai penelusuran skala besar: keyword='{keyword}', target={target_count} jurnal...")

    # Jalankan scraping SINTA dengan turbo mode dan short-circuiting
    sinta_entries = scrape_sinta(
        keyword=keyword,
        sinta_ranks=[3, 4, 5, 6],
        deep_detail=True,
        max_results=target_count,
        turbo_mode=True,
        free_only=False,
    )

    elapsed = time.time() - start_time
    print(f"\n📊 Total Jurnal Diperoleh: {len(sinta_entries)} jurnal")
    print(f"⏱️ Total Durasi Ekstraksi: {elapsed:.2f} detik ({elapsed/60:.2f} menit)")

    assert len(sinta_entries) >= 50, f"❌ Hasil ({len(sinta_entries)}) terlalu sedikit!"
    assert elapsed < 120.0, f"❌ Waktu ekstraksi {elapsed:.2f}s melebihi batas toleransi 2 menit (120 detik)!"

    # Audit integritas data hasil
    xlsx_file = export_excel(sinta_entries, keyword="speed_test_250")
    df = pd.read_excel(xlsx_file)
    print(f"📁 Berkas Excel terverifikasi: {xlsx_file} ({len(df)} baris)")

    # 1. Verifikasi 0% NaN
    for col in df.columns:
        nan_cnt = df[col].isna().sum()
        assert nan_cnt == 0, f"❌ Kolom {col} memiliki {nan_cnt} NaN!"
    print("   ✅ 0% NaN di seluruh kolom.")

    # 2. Verifikasi Frekuensi Terbit Baku (0% istilah generik)
    generic_words = ["Semiannual", "Triwulanan", "Dwi-bulanan", "Tahunan", "Bulanan"]
    for val in df["Frekuensi Terbit"]:
        val_str = str(val)
        for gw in generic_words:
            assert f"({gw})" not in val_str, f"❌ Terdeteksi istilah generik: {val_str}"
    print("   ✅ Frekuensi Terbit 100% Baku & Bebas Istilah Generik.")

    print(f"\n🏆 SPEED ACCELERATION SUITE SELESAI DALAM {elapsed:.2f} DETIK (< 2 MENIT) — LOLOS SEMPURNA!\n")


if __name__ == "__main__":
    test_circuit_breaker()
    test_gui_compactness()
    test_speed_and_target_short_circuiting()
    print("=" * 70)
    print("🎉 SEMUA TEST QA & SPEED ENGINE v1.3.6 LOLOS 100%!")
    print("=" * 70)
