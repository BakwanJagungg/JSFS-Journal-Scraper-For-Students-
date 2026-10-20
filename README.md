# 📚 JSFS — Journal Scraper For Students (v1.5.1 Hybrid AI Engine)

> **Alat otomatisasi pencarian jurnal SINTA & Garuda untuk mahasiswa & peneliti: Hybrid AI Engine (Ollama qwen2.5:3b + Fallback Regex Legacy), Mode Simpel & 4 Preset Mahasiswa (Target Skripsi, Jurnal Gratis, Terbit Terdekat, Reputasi Tinggi), Top 3 AI Advisor, Next Issue Estimator, Salin Draf Chat Editor, Smart SQLite WAL Cache, dan Eliminasi Total GUI Blackout & MEI Directory Lock.**

---

## 💡 Kenapa Mahasiswa Membutuhkan JSFS?

Mencari jurnal yang tepat di portal **SINTA** dan **Garuda** secara manual sering kali memakan waktu berjam-jam:
- ❌ Harus membuka dan mengklik link puluhan situs jurnal satu per satu.
- ❌ Bingung menentukan jurnal mana yang ruang lingkupnya (*Focus & Scope*) paling cocok dengan topik skripsi/tesis.
- ❌ Sulit memfilter peringkat SINTA yang realistis untuk target publikasi mahasiswa (misal: SINTA 3–5).
- ❌ Informasi biaya naskah sering ambigu antara biaya submit (*submission fee*) dan biaya publikasi (*publication charge / APC*).
- ❌ Sulit mencocokkan jadwal bulan terbit jurnal (*call for papers*) dengan tenggat waktu sidang/kelulusan.
- ❌ Kontak redaksi (Email/WA) dan link OJS sering mati tanpa peringatan sebelumnya.

**JSFS (Journal Scraper For Students) v1.5.1** hadir untuk menyelesaikan seluruh masalah tersebut:
1. ✨ **Mode Simpel & 4 Quick Presets Mahasiswa (Baru di v1.5.1)**:
   - 🎓 **Target Skripsi S1**: SINTA 3–5, biaya murah/gratis, jadwal teratur.
   - 💰 **Jurnal Gratis (Rp 0)**: Hanya jurnal bebas biaya publikasi (Bebas APC).
   - ⚡ **Terbit Terdekat**: Memfilter jurnal dengan edisi terbit 1–3 bulan ke depan agar pas dengan jadwal yudisium.
   - 🏆 **Reputasi Tinggi**: SINTA 1 & 2 (Akreditasi A / Scopus) untuk mahasiswa magister, doktoral, atau hibah.
2. 💬 **Salin Draf Chat Editor (Baru di v1.5.1)**: Satu klik pada pop-up previewer untuk menyalin draf pesan WhatsApp/Email yang sopan ke redaksi menanyakan kuota edisi terdekat.
3. ❓ **Modal Panduan Singkat (Baru di v1.5.1)**: Akses panduan praktis memilih jurnal, memahami SINTA, dan tips publikasi langsung dari tombol di header GUI.
4. 🚀 **Stabilitas Ekstrem & Anti-Blackout (Baru di v1.5.1)**: Arsitektur non-blocking event loop dengan batch queue polling (80 ms) dan memory-bounded log buffer (500 baris) mencegah Windows `WM_PAINT` render starvation.
5. 🛡️ **Zero Lock PyInstaller (Baru di v1.5.1)**: Direktori cache terisolasi pada folder aplikasi dan graceful SQLite checkpoint (`PRAGMA wal_checkpoint(TRUNCATE);`) saat keluar, melenyapkan error `_MEIxxxxxx` lock.
6. 🏆 **Top 3 Rekomendasi Jurnal (AI Advisor)**: Scoring multi-kriteria (Relevansi 40%, Biaya 25%, Kecepatan 20%, SINTA 15%) dengan justifikasi keunggulan jurnal.
7. 🤖 **Hybrid AI Engine (Ollama qwen2.5:3b + Smart Regex Fallback)**: Ekstraksi cerdas Focus & Scope dan pemisahan Author Fees (Submit vs Publikasi) dengan filter anti-halusinasi.
8. ⚡ **Smart SQLite WAL Cache**: Caching lokal metadata jurnal OJS dengan TTL 14 hari, mempercepat pencarian berulang hingga 80-90%.
9. 👁️ **Data Table Previewer Interaktif**: Tabel data responsif untuk memfilter, mencari, dan melihat rincian jurnal sebelum membuka browser.

---

## 🖥️ Pilihan Antarmuka: GUI Desktop & CLI

JSFS hadir dalam 2 mode yang fleksibel:
1. **Mode GUI Desktop (Rekomendasi Utama)**: Tampilan visual modern (*Dark Mode*) berbasis CustomTkinter. Cukup pilih Preset atau atur filter, ketik kata kunci, dan klik tombol!
2. **Mode CLI (Command Line Interface)**: Untuk pengguna tingkat lanjut atau automasi skrip via terminal/prompt.

---

## 📋 Prasyarat Sistem

- **Sistem Operasi**: Windows 10 / 11 (64-bit direkomendasikan).
- **Python**: Python 3.10 atau versi yang lebih baru (3.12+ didukung penuh).
- **Google Chrome** *(Opsional)*: Untuk opsi Selenium / bypass proteksi Cloudflare SINTA bila diperlukan.
- **Tanpa Python**: Jika Anda menggunakan bundel executable (`dist/JSFS.exe`), Anda tidak memerlukan instalasi Python sama sekali!

---

## 🛠️ Panduan Instalasi Dependensi

1. **Buka Terminal / Command Prompt / PowerShell** di direktori proyek JSFS.
2. *(Opsional tapi disarankan)* Buat dan aktifkan *virtual environment*:
   ```bash
   python -m venv venv
   venv\Scripts\activate
   ```
3. **Instal seluruh dependensi**:
   ```bash
   pip install -r requirements.txt
   ```

---

## 🚀 Panduan Menjalankan Aplikasi

### 1. Menjalankan Versi GUI Desktop (Paling Mudah)

```bash
python app_ui.py
```
Atau klik dua kali pada file **`JSFS.exe`** di folder `dist/`.

**Fitur Antarmuka GUI v1.5.1:**
- 💡 **Input Kata Kunci**: Ketik topik riset (misal: `kecerdasan buatan`, `akuntansi publik`, `manajemen sdm`).
- 🎛️ **Mode Simpel vs Mode Pengaturan Lengkap**: Gunakan 4 Quick Preset instan atau atur filter SINTA, rentang biaya, bulan terbit, dan workers secara mandiri.
- 📄 **Analisis Naskah (.docx/.pdf)**: Unggah naskah skripsi untuk mencocokkan kemiripan topik (*Match Score 0–100%*).
- 🌐 **Dual Progress Bar**: Memantau progres crawling portal dan pengayaan AI secara terpisah dan transparan.
- 👁️ **Preview Hasil Data**: Buka tabel previewer interaktif dengan tombol buka OJS, buka Excel, dan **Salin Draf Chat Editor**.
- ❓ **Panduan Singkat**: Panduan langkah demi langkah bagi mahasiswa yang baru pertama kali mengirim jurnal ilmiah.

---

### 2. Menjalankan Versi CLI (Terminal)

#### A. Mode Interaktif
```bash
python jsfs.py --interactive
```

#### B. Perintah Langsung dengan Argumen
```bash
# 1. Cari jurnal topik bisnis, filter SINTA 3-6 (default), output Markdown
python jsfs.py --keyword "bisnis"

# 2. Cari topik e-commerce, filter SINTA 3 & 4, output Markdown dan Excel
python jsfs.py --keyword "e-commerce" --sinta 3 4 --format md excel

# 3. Cari dari SINTA & Garuda sekaligus, output lengkap
python jsfs.py --keyword "manajemen" --source both --format md excel csv

# 4. Gunakan Selenium headless Chrome
python jsfs.py --keyword "sistem informasi" --source sinta --selenium
```

---

## 📦 Hasil Output & Penamaan Berkas

Semua berkas hasil scraping tersimpan secara otomatis di folder **`output/`** dengan penamaan unik:
`JSFS_<keyword>_<YYYYMMDD_HHMMSS>.<ext>` (Contoh: `JSFS_bisnis_20261020_210000.xlsx`)

| Format | Keunggulan & Fitur Visual v1.5.1 |
|:---:|---|
| **Excel (`.xlsx`)** | Header Navy Blue elegan, sheet terpisah `🏆 Rekomendasi AI` (Top 3 Advisor), kolom Kesesuaian & Skor, tautan OJS aktif (`=HYPERLINK`), dan auto-fit lebar kolom. |
| **Markdown (`.md`)** | Tabel terstruktur, seksi Top 3 Rekomendasi Jurnal dengan justifikasi skor, dan Direktori Kontak Cepat Redaksi Jurnal dengan tautan WhatsApp interaktif. |
| **CSV (`.csv`)** | Format tabel standar dengan UTF-8 BOM untuk integrasi ke Google Sheets, database, atau pengolah data statistik. |

---

## 🏗️ Cara Build ke Executable Mandiri (`JSFS.exe`)

```bash
python -m PyInstaller --noconfirm --clean JSFS.spec
```
Setelah build selesai, berkas executable siap pakai berada di direktori **`dist/JSFS.exe`**.

---

## 🗺️ Roadmap Pengembangan

| Versi | Status | Deskripsi Milestone |
|:---:|:---:|---|
| **v1.0.0 (MVP)** | ✅ Selesai | Scraping CLI Garuda + SINTA, filter SINTA, ekspor Markdown/CSV/Excel |
| **v1.1.0 – v1.3.8** | ✅ Selesai | GUI CustomTkinter Dark Mode, Zero Clipping, OJS Deep Scrape, Focus & Scope multi-tier |
| **v1.4.0 (AI Hybrid)** | ✅ Selesai | Hybrid AI Engine (Ollama qwen2.5:3b + Smart Regex), 1-Click AI Setup, Dual Progress |
| **v1.5.0 (Intelligence)** | ✅ Selesai | SQLite WAL Cache, Top 3 AI Advisor, Match Score dwibahasa, Schedule Estimator |
| **v1.5.1 (Stability & UX)** | 🚀 Rilis Ini | Progressive Disclosure (4 Quick Presets), Eliminasi GUI Blackout & MEI lock, Salin Draf Chat Editor, Panduan Singkat |
| **v2.0.0** | 💡 Rencana | Versi Web App Multi-user (FastAPI + React/Vite) untuk akses bersama dalam lab/kampus |

---

*Dikembangkan dengan ❤️ untuk Mahasiswa Pejuang Skripsi, Tesis, dan Publikasi Ilmiah.*  
*Disclaimer: Selalu periksa kembali keaslian jurnal dan pedoman penulisan pada situs resmi OJS masing-masing penerbit sebelum mengirimkan naskah.*
