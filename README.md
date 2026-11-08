# 📚 JSFS — Journal Scraper For Students (v1.7.0 Deep APC Hunter & Modern Bright GUI)

> **Alat otomatisasi pencarian jurnal SINTA & Garuda untuk mahasiswa & peneliti: Deep APC Hunter Engine (Incomplete Cache Bypass, Multi-Path OJS Crawler, Anti-OA-Trap & Paid Priority), Modern Bright GUI (Soft Slate Daylight, Dark/Light Theme Switcher, Live KPI Counter Pills, 1-Click WA Share), Forensic Data Purity, Area Code Whitelist, Issue Archive Year Grouping, dan Smart Canonical Deduplication.**

---

## 💡 Kenapa Mahasiswa Membutuhkan JSFS?

Mencari jurnal yang tepat di portal **SINTA** dan **Garuda** secara manual sering kali memakan waktu berjam-jam:
- ❌ Harus membuka dan mengklik link puluhan situs jurnal satu per satu.
- ❌ Bingung menentukan jurnal mana yang ruang lingkupnya (*Focus & Scope*) paling cocok dengan topik skripsi/tesis.
- ❌ Sulit memfilter peringkat SINTA yang realistis untuk target publikasi mahasiswa (misal: SINTA 3–5).
- ❌ Informasi biaya naskah sering ambigu antara biaya submit (*submission fee*) dan biaya publikasi (*publication charge / APC*).
- ❌ Sulit mencocokkan jadwal bulan terbit jurnal (*call for papers*) dengan tenggat waktu sidang/kelulusan.
- ❌ Kontak redaksi (Email/WA) sering disembunyikan di balik proteksi Cloudflare atau terselubung di menu navigasi.

**JSFS (Journal Scraper For Students) v1.7.0** hadir untuk menyelesaikan seluruh masalah tersebut:
1. 🎯 **Deep APC Hunter Engine (Baru di v1.7.0)**:
   - 🔄 **Incomplete Cache Auto-Bypass & SQLite Auto-Purge**: Entri cache dengan status APC belum terdata otomatis di-bypass untuk deep-scrape ulang saat opsi Deep Scrape diaktifkan, serta auto-purge database SQLite lama.
   - 🌐 **Multi-Path OJS 2 & OJS 3 Crawler**: Memindai secara konsisten 5 titik target: (1) `/about/submissions`, (2) `/about/editorialPolicies` (kritis untuk OJS 2), (3) `/about`, (4) Sidebar & Custom Blocks, dan (5) Dynamic Menu Anchors.
   - 🛡️ **Anti-OA-Trap & Paid Priority Rule**: Menghapus kalimat retorika Open Access pembaca sebelum deteksi kata kunci gratis. Jika ditemukan nominal publikasi positif, status berbayar langsung menang mutlak meskipun pendaftaran berbiaya Rp 0.
2. ☀️ **Modern Bright GUI "Soft Slate Daylight" (Baru di v1.7.0)**:
   - 🎨 **Light Mode Default & Theme Switcher (☀️/🌙)**: Tampilan visual ramah mata dengan palet slate daylight dan switch instan ke mode malam.
   - 📊 **Live KPI Counter Pills**: Header metrik interaktif dengan 4 indikator dinamis (*Total Ditemukan*, *Gratis (Rp 0)*, *Berbayar*, *Terbit Dekat*) yang diperbarui secara thread-safe.
   - 📋 **1-Click WA Share Button**: Tombol aksi cepat *"📋 Salin Daftar (Share WA)"* di action bar utama untuk langsung menyalin daftar ringkas ke clipboard.
3. 🛡️ **Forensic Data Purity & Anti-Email Flood**:
   - 🚫 **Eliminasi Total Email Mahasiswa & Penulis**: Menolak domain mahasiswa (`@student.`, `@mhs.`, `@mail.ugm.`) dan format NIM (>= 6 digit angka). Mengisolasi ekstraksi dari blok artikel agar email penulis tidak bocor ke redaksi.
   - 🔒 **Prioritas Domain Kampus & Hard Cap 3 Email**: Mengutamakan email berdomain resmi kampus OJS atau berawalan kata kunci redaksi, membuang email pribadi gmail/yahoo dosen, dan membatasi maksimal 3 email resmi terbaik.
2. 📞 **Anti-Fake Phone & Whitelist Kode Area Telkom (Baru di v1.6.2)**:
   - 📋 **Daftar Putih Kode Area Resmi Indonesia**: Memvalidasi seluruh kode area 2-digit dan 3-digit resmi Telkom Indonesia, serta menolak kode area palsu dari DOI/HTML (seperti `0203`, `0300`, `0491`).
   - 🛑 **Strict Proximity Context & Length Guard**: Nomor kabel wajib memiliki label kontak dalam rentang 35 karakter kiri, serta membatasi panjang total 9–12 digit (menolak nomor rekening bank / NIP 14–16 digit).
   - 📱 **Hard Cap 2 Telepon/WA**: Mengutamakan WhatsApp/seluler (`+628...`) di urutan teratas dan membatasi maksimal 2 nomor per jurnal.
3. 💰 **Deep Indonesian OJS APC Terminology & Smart Fallback (Baru di v1.6.2)**:
   - 🔍 **Kamus Luas OJS Indonesia**: Mengenali variasi kalimat gratis (`"tidak dikenakan biaya"`, `"tidak memungut biaya"`, `"bebas biaya"`, `"free of charge"`, `"dibebaskan dari biaya"`) dan format nominal naratif uang rupiah.
   - 🔄 **Smart Fallback Biaya Submit -> Biaya Publikasi**: Nilai nominal `> Rp 50.000` pada kolom `Biaya Submit` otomatis dikonversi menjadi `Biaya Publikasi` jika publikasi belum terdata (menyelesaikan kasus MEBIS Rp 600rb dan MUQADDIMAH Rp 500rb).
4. 📅 **Arsip Edisi Berbasis Tahun (Anti-Anomali 7–8 Bulan) (Baru di v1.6.2)**:
   - Mengelompokkan judul arsip edisi berdasarkan tahun dan mengambil bulan hanya dari 1 tahun penuh terbaru (atau 4–6 edisi teratas), mencegah penggabungan historis yang memicu anomali "7–8 kali setahun", serta mengabaikan derau widget kalender bulanan (>= 11 bulan).
5. 🧩 **Smart Canonical Deduplication & OJS Title Protection (Baru di v1.6.2)**:
   - **`canonical_journal_key`**: Deduplikasi cerdas berbasis judul kanonikal bersih (bebas tanda baca, kapitalisasi, spasi, dan titik dua) serta ISSN/E-ISSN dan path URL, menggabungkan jurnal duplikat/migran (seperti kasus *Balance UM Palembang*) menjadi 1 entri tunggal yang utuh.
   - **Koreksi Judul Artikel Menjadi Nama Jurnal**: Mendeteksi jika judul dari Garuda merupakan judul artikel skripsi yang panjang (> 55 karakter seperti `"KONSEP DAN TUJUAN PADA TINJUAN BISNIS SYARIAH"`) dan mengoreksinya menggunakan nama jurnal resmi OJS (*ADZKIYA : Jurnal Hukum dan Ekonomi Syariah*).
6. 🌿 **Pembersihan Baris `Fokus:` & Terjemahan Subjek SINTA (Baru di v1.6.2)**:
   - **Anti-Noise Filter**: Menolak teks Focus & Scope yang memuat metadata pengindeksan (*Scopus ID*, *Google Scholar*, *diindeks oleh*), lisensi (*Creative Commons*, *Copyright ©*), tanggal rilis (*this issue has been available*), atau abstrak artikel (*this study aims to*).
   - **Terjemahan Subjek SINTA ke Bahasa Indonesia**: Menerjemahkan istilah bahasa Inggris SINTA ke Bahasa Indonesia yang rapi (*Manajemen & Riset Operasi*, *Ekonomi & Keuangan*, *Bisnis & Akuntansi*), membuang kategori yang tidak selaras dengan judul jurnal, dan memformat pemotongan teks secara bersih tanpa tanda koma menggantung (`,...`).
7. 🚀 **High-Yield Recall Engine**:
   - 📈 **Adaptive Candidate Pool Multiplier (3x Over-Fetching)**: Pengambilan kandidat awal `max(target * 3, 60)` menjamin hasil pencarian topik populer (seperti *bisnis*, *akuntansi*, *teknologi*) melimpah (>20 jurnal).
   - 🛡️ **Smart APC Preservation**: Jurnal berstatus `"Cek Web / Belum Terdata"` tidak dibuang secara agresif; otomatis lolos seleksi saat menyetel rentang biaya (`max_rp = 1.500.000`) dan melengkapi kuota target saat memilih *"Hanya Gratis (Rp 0)"*.
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
