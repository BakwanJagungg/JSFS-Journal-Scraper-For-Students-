import os
import sys
import time
import urllib3
import pandas as pd

urllib3.disable_warnings()
sys.path.insert(0, os.getcwd())

from scraper_sinta import scrape_sinta
from doc_reader import extract_manuscript_metadata, calculate_match_score
from exporter import export_excel

print("=" * 70)
print("🧪 PENGUJIAN STRESS TEST SKALA 500 JURNAL (Keyword: 'bisnis, ekonomi')")
print("=" * 70)

manuscript_path = os.path.join("scratch", "sample_manuscript.docx")
manuscript_meta = extract_manuscript_metadata(manuscript_path) if os.path.exists(manuscript_path) else None

t0 = time.time()
res_500 = scrape_sinta(
    keyword="bisnis, ekonomi",
    sinta_ranks=[3, 4, 5, 6],
    deep_detail=True,
    max_results=500,
    turbo_mode=True,
)
dur = time.time() - t0
print(f"\n📊 Total Jurnal Diperoleh: {len(res_500)} jurnal dalam {dur:.2f}s ({dur/60:.2f} menit)")
assert len(res_500) >= 300, f"Hasil terlalu sedikit: {len(res_500)}"

if manuscript_meta:
    for entry in res_500:
        res_m = calculate_match_score(manuscript_meta, entry)
        entry.match_score = res_m["score"]
        entry.kesesuaian_naskah = res_m["level"]
    res_500.sort(key=lambda j: j.match_score or 0, reverse=True)

xlsx_file = export_excel(res_500, keyword="stress_test_500")
df = pd.read_excel(xlsx_file)
print(f"📁 Berkas Excel terverifikasi: {xlsx_file} ({len(df)} baris)")

# Verifikasi 0% NaN
for col in df.columns:
    nan_cnt = df[col].isna().sum()
    assert nan_cnt == 0, f"❌ Kolom {col} memiliki {nan_cnt} NaN!"
print("✅ 0% NaN di seluruh kolom.")

# Verifikasi Frekuensi Terbit Baku (0% istilah generik)
generic_words = ["Semiannual", "Triwulanan", "Dwi-bulanan", "Tahunan", "Bulanan"]
for val in df["Frekuensi Terbit"]:
    val_str = str(val)
    for gw in generic_words:
        assert f"({gw})" not in val_str, f"❌ Terdeteksi istilah generik: {val_str}"
print("✅ Frekuensi Terbit 100% Baku & Bebas Istilah Generik.")

print(f"\n🏆 STRESS TEST SKALA 500 JURNAL SELESAI DALAM {dur:.2f}s — LOLOS SEMPURNA!\n")
