# 📚 JSFS — Journal Scraper For Students (v1.6.1 High-Yield Recall & Human-Parity Precision)

> **Alat otomatisasi pencarian jurnal SINTA & Garuda untuk mahasiswa & peneliti: High-Yield Recall Multiplier (3x Candidate Pool & Smart APC Filter), Anti-Fake Phone Validator, OJS Profile Table Noise Stripper, Zero-Empty Focus & Scope Synthesizer, Always-On Keyword Match Score, Human-Parity Accuracy Engine, 1-Click Copy-Paste Share List Generator, Hybrid AI Engine (Ollama qwen2.5:3b + Smart Regex), Mode Simpel & 4 Preset Mahasiswa, dan Smart SQLite WAL Cache.**

---

## 💡 Kenapa Mahasiswa Membutuhkan JSFS?

Mencari jurnal yang tepat di portal **SINTA** dan **Garuda** secara manual sering kali memakan waktu berjam-jam:
- ❌ Harus membuka dan mengklik link puluhan situs jurnal satu per satu.
- ❌ Bingung menentukan jurnal mana yang ruang lingkupnya (*Focus & Scope*) paling cocok dengan topik skripsi/tesis.
- ❌ Sulit memfilter peringkat SINTA yang realistis untuk target publikasi mahasiswa (misal: SINTA 3–5).
- ❌ Informasi biaya naskah sering ambigu antara biaya submit (*submission fee*) dan biaya publikasi (*publication charge / APC*).
- ❌ Sulit mencocokkan jadwal bulan terbit jurnal (*call for papers*) dengan tenggat waktu sidang/kelulusan.
- ❌ Kontak redaksi (Email/WA) sering disembunyikan di balik proteksi Cloudflare atau terselubung di menu navigasi.

**JSFS (Journal Scraper For Students) v1.6.1** hadir untuk menyelesaikan seluruh masalah tersebut:
1. 🚀 **High-Yield Recall Engine (Baru di v1.6.1)**:
   - 📈 **Adaptive Candidate Pool Multiplier (3x Over-Fetching)**: Pengambilan kandidat awal `max(target * 3, 60)` menjamin hasil pencarian topik populer (seperti *bisnis*, *akuntansi*, *teknologi*) melimpah (>20 jurnal).
   - 🛡️ **Smart APC Preservation**: Jurnal berstatus `"Cek Web / Belum Terdata"` tidak dibuang secara agresif; otomatis lolos seleksi saat menyetel rentang biaya (`max_rp = 1.500.000`) dan melengkapi kuota target saat memilih *"Hanya Gratis (Rp 0)"*.
2. 🎯 **Presisi Kontak & Metadata Setara Manusia (Baru di v1.6.1)**:
   - 📵 **Anti-Fake Phone Validator**: Menolak mutlak deretan angka acak dari URL, DOI prefix, atau ID artikel yang diawali angka `0` atau `1` setelah kode area (seperti `+6221005162` dan `+62211892256` pada UNESA).
   - ✂️ **OJS Profile Table Noise Stripper**: Memotong dan membuang header tabel biodata OJS (`Journal Title`, `ISSN/E-ISSN`, `DOI Prefix`, `Editor in Chief`, `Publisher`, `Frequency`), hanya menyisakan deskripsi ilmiah intinya.
   - 🧬 **Zero-Empty Focus & Scope Synthesizer (100% Terisi)**: Jika halaman OJS tidak memuat ruang lingkup, otomatis menyintesis deskripsi dari data *Bidang Keilmuan* SINTA/Garuda (`Menerbitkan riset ilmiah di bidang {subject_area}`).
   - 📊 **Always-On Keyword Match Score**: Kolom `Skor` dan `Kesesuaian` selalu aktif dan terisi skor persentase proporsional yang nyata untuk setiap jurnal saat mencari dengan kata kunci tanpa upload file naskah.
3. 🧠 **Human-Parity Accuracy Engine**:
   - 🔍 **Dynamic Menu Anchor Discovery**: Memindai link menu navigasi & sidebar OJS secara dinamis untuk melacak halaman APC tersembunyi (`fee`, `apc`, `biaya`, `tarif`) dan kontak resmi redaksi.
   - 📅 **Verifikasi Jadwal Terbit via Arsip (`/issue/archive`)**: Membaca judul edisi terbitan riil pada arsip jurnal saat halaman About tidak mencantumkan bulan terbit secara eksplisit.
   - 💰 **Akurasi Biaya Publikasi (APC) 3-Status**: Standarisasi status `Gratis (Rp 0)`, `Rp X.XXX.XXX`, dan `Cek Web / Belum Terdata`. Eliminasi mutlak *false positive* Gratis dari biaya submit Rp 0.
   - 🔓 **Dekoder Cloudflare & Email Redaksi Resmi**: Dekripsi tag hex `<a class="__cf_email__">`, deobfuskasi format `[at]`/`[dot]`, prioritas email domain kampus/redaksi, pembersihan otomatis NIM mahasiswa.
4. 📋 **1-Click Copy-Paste List Generator**:
   - Generator daftar bernomor siap *share* ke WhatsApp/Telegram grup rekan mahasiswa/dosen lengkap dengan baris `Fokus:` bidang keilmuan.
   - Tombol instan `📋 Salin Semua Daftar (Format Share)` pada jendela previewer data dengan animasi konfirmasi 2.5 detik.
   - Blok daftar copy-paste di dalam file Markdown (`.md`) dan pembuatan otomatis berkas teks pendamping `JSFS_<keyword>_<timestamp>_LIST.txt`.
5. 🛡️ **Zero-Crash & Anti-Freeze Architecture**:
   - `safe_fetch_html`: Filter Content-Type ketat (hanya memproses HTML) dan pembatas ukuran respons maksimal 1.5 MB untuk mencegah freeze akibat crawler mengunduh file PDF atau ZIP besar.
   - Pelindung multi-layer `try...except Exception:` di seluruh thread scraper dan worker.
6. ✨ **Mode Simpel & 4 Quick Presets Mahasiswa**:
   - 🎓 **Target Skripsi S1**: SINTA 3–5, biaya murah/gratis, jadwal teratur.
   - 💰 **Jurnal Gratis (Rp 0)**: Prioritas jurnal bebas biaya publikasi (Bebas APC) + Cek Web pelengkap kuota.
   - ⚡ **Terbit Terdekat**: Memfilter jurnal dengan edisi terbit 1–3 bulan ke depan agar pas dengan jadwal yudisium.
   - 🏆 **Reputasi Tinggi**: SINTA 1 & 2 (Akreditasi A / Scopus) untuk mahasiswa magister, doktoral, atau hibah.
5. 💬 **Salin Draf Chat Editor**: Satu klik pada pop-up previewer untuk menyalin draf pesan WhatsApp/Email yang sopan ke redaksi menanyakan kuota edisi terdekat.
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

| Format | Keunggulan & Fitur Visual v1.6.0 |
|:---:|---|
| **Text List (`_LIST.txt`)** | Format bernomor 1-klik ringkas (Nama Jurnal, Sinta, Penerbit, Email, Telepon/WA, Biaya Publikasi, Bulan Terbit, Link OJS) yang siap copy-paste langsung ke grup WhatsApp/Telegram rekan mahasiswa. |
| **Markdown (`.md`)** | Tabel terstruktur, seksi Top 3 Rekomendasi Jurnal dengan justifikasi skor, Direktori Kontak Cepat Redaksi Jurnal dengan tautan WhatsApp, dan blok kode khusus `📋 Daftar Ringkas Siap Copy-Paste`. |
| **Excel (`.xlsx`)** | Header Navy Blue elegan, sheet terpisah `🏆 Rekomendasi AI` (Top 3 Advisor), kolom Kesesuaian & Skor, tautan OJS aktif (`=HYPERLINK`), dan auto-fit lebar kolom. |
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
| **v1.5.1 (Stability & UX)** | ✅ Selesai | Progressive Disclosure (4 Quick Presets), Eliminasi GUI Blackout & MEI lock, Salin Draf Chat Editor, Panduan Singkat |
| **v1.6.0 (Human-Parity)** | ✅ Selesai | Human-Parity Accuracy Engine (Dynamic OJS Menu Discovery, CF Email Decoder, Archive Schedule Parser), 3-State APC Accuracy, Zero-Crash Guard, dan 1-Click Copy-Paste Share List |
| **v1.6.1 (High-Yield Recall)** | 🚀 Rilis Ini | High-Yield Recall Multiplier (3x Candidate Pool & Smart APC Filter), Anti-Fake Phone Validator, OJS Profile Table Stripper & Always-On Keyword Match Score |
| **v2.0.0** | 💡 Rencana | Versi Web App Multi-user (FastAPI + React/Vite) untuk akses bersama dalam lab/kampus |

---

*Dikembangkan dengan ❤️ untuk Mahasiswa Pejuang Skripsi, Tesis, dan Publikasi Ilmiah.*  
*Disclaimer: Selalu periksa kembali keaslian jurnal dan pedoman penulisan pada situs resmi OJS masing-masing penerbit sebelum mengirimkan naskah.*
