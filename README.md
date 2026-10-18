# 📚 JSFS — Journal Scraper For Students (v1.4.0 Hybrid AI Engine)

> **Alat otomatisasi pencarian jurnal SINTA & Garuda untuk mahasiswa: Hybrid AI Engine (Ollama qwen2.5:3b + Fallback Regex Legacy v1.3.8), Pemisahan Biaya Submit & Publikasi, Filter & Deteksi Bulan Terbit, Audit Trail Asal Data, Estetika GUI Zero Clipping & Responsif, Dual Progress Bar, Indikator Kesehatan Server OJS & Transparansi Kontak, GUI Data Table Previewer Interaktif, Analisis Naskah (.docx/.pdf), dan 0% NaN Data Export.**

---

## 💡 Apa itu JSFS & Kenapa Mahasiswa Membutuhkannya?

Bagi mahasiswa tingkat akhir atau peneliti pemula, mencari jurnal yang tepat di portal **SINTA** dan **Garuda** secara manual sering kali memakan waktu berjam-jam:
- ❌ Harus membuka dan mengklik link puluhan situs jurnal satu per satu.
- ❌ Bingung menentukan jurnal mana yang ruang lingkupnya (*Focus & Scope*) paling cocok dengan topik naskah artikel skripsi/tesis.
- ❌ Sulit memfilter peringkat SINTA yang realistis untuk target publikasi mahasiswa (misal: SINTA 3, 4, 5, atau 6).
- ❌ Informasi status biaya naskah sering ambigu antara biaya submit (*submission fee*) dan biaya publikasi (*publication charge / APC*).
- ❌ Sulit mencocokkan jadwal bulan terbit jurnal (*call for papers*) dengan tenggat waktu sidang/kelulusan.
- ❌ Kontak redaksi (Email/WA) dan link OJS sering mati tanpa peringatan sebelumnya.

**JSFS (Journal Scraper For Students) v1.4.0** hadir dengan arsitektur **Hybrid AI Engine**:
- 🤖 **Hybrid AI Engine (Ollama REST API + Fallback Regex Legacy)**: Ekstraksi Focus & Scope dan rincian biaya naskah secara cerdas menggunakan model open-weights lokal `qwen2.5:3b` via Ollama (`localhost:11434`). Dilengkapi smart truncation, circuit breaker, dan fallback otomatis ke Regex v1.3.8 jika Ollama tidak aktif.
- 💰 **Pemisahan Biaya Submit vs Publikasi**: Kolom terpisah `Biaya Submit`, `Biaya Publikasi`, dan `Mata Uang` (`IDR` / `USD`).
- 📅 **Parser & Filter Bulan Terbit**: Deteksi jadwal bulan terbit jurnal (1–12) dengan filter checkbox bulan di GUI untuk mencocokkan periode wisuda/sidang.
- 🔍 **Transparansi Asal Data (Audit Trail)**: Kolom `Sumber Focus` dan `Sumber APC` (`AI` / `Regex` / `Kosong`) untuk audit kualitas ekstraksi.
- 🖥️ **Tampilan GUI Zero Clipping & Responsif**: Pengukuran teks dinamis, scrollable frame, badge status Ollama real-time (`● AI Aktif` / `○ Mode Regex`), dan Dual Progress Bar (Scrape Massal + Analisis AI).
- 👁️ **GUI Data Table Previewer Interaktif**: Pop-up previewer v1.4.0 untuk memfilter, mencari, dan membaca detail Focus & Scope serta Author Fees langsung di aplikasi.
- 🌐 **Indikator Kesehatan Server OJS & SSL**: Deteksi otomatis status server (`Sangat Aktif`, `Aktif`, `SSL Warning`, `Non-Aktif`).
- 🛡️ **Indikator Transparansi Kontak**: Mengukur kelengkapan kontak redaksi (`Lengkap`, `Sedang`, `Terbatas`, `Tidak Transparan`).
- 📄 **Modul Analisis Naskah & Rekomendasi Jurnal (Match Score 0–100%)**: Unggah draf artikel ilmiah (.docx atau .pdf) untuk menghitung skor kecocokan topik terhadap setiap jurnal.
- ⚙️ **1-Click AI Setup Manager**: Wizard mandiri di dalam aplikasi GUI untuk mengunduh Ollama dan menarik model AI secara otomatis tanpa perlu membuka browser atau terminal manual.
- ⚡ **Arsitektur Pipeline 4-Fase Berkecepatan Tinggi**: Fase 1 Scraping Massal (128 workers), Fase 2 Pre-Filter (SINTA & Bulan), Fase 3 AI Enrichment (pool 1–2 workers), Fase 4 Export Data (.xlsx, .md, .csv).

---

## 🖥️ Pilihan Antarmuka: GUI Desktop & CLI

JSFS kini hadir dalam 2 mode yang fleksibel:
1. **Mode GUI Desktop (Rekomendasi Utama)**: Tampilan visual modern (*Dark Mode*) berbasis CustomTkinter. Tanpa perlu ketik perintah di terminal — cukup ketik kata kunci, atur target jumlah jurnal (10 - 1000), tentukan rentang APC, centang filter, dan klik tombol!
2. **Mode CLI (Command Line Interface)**: Untuk pengguna tingkat lanjut atau automasi skrip via terminal/prompt.

---

## 📋 Prasyarat Sistem

- **Sistem Operasi**: Windows 10 / 11 (64-bit direkomendasikan).
- **Python**: Python 3.10 atau versi yang lebih baru (3.12+ didukung penuh).
- **Google Chrome** *(Opsional)*: Hanya jika Anda ingin menggunakan opsi Selenium / undetected-chromedriver untuk mem-bypass proteksi JavaScript/Cloudflare SINTA.
- **Tanpa Python**: Jika Anda menggunakan versi bundel executable (`dist/JSFS.exe`), Anda tidak memerlukan instalasi Python sama sekali!

---

## 🛠️ Panduan Instalasi Dependensi

1. **Buka Terminal / Command Prompt / PowerShell** di direktori proyek JSFS.
2. *(Opsional tapi disarankan)* Buat dan aktifkan *virtual environment*:
   ```bash
   python -m venv venv
   venv\Scripts\activate
   ```
3. **Instal seluruh dependensi yang diperlukan**:
   ```bash
   pip install -r requirements.txt
   ```

---

## 🚀 Panduan Menjalankan Aplikasi

### 1. Menjalankan Versi GUI Desktop (Paling Mudah)

Cukup jalankan perintah berikut di terminal:
```bash
python app_ui.py
```
Atau jika Anda sudah memiliki file executable:
- Cukup klik dua kali (double-click) pada file **`JSFS.exe`** di folder `dist/`.

**Fitur Antarmuka GUI v1.2:**
- 🔍 **Input Keyword**: Masukkan kata kunci topik riset Anda (misal: `bisnis`, `e-commerce`, `machine learning`).
- 🎯 **Target Jumlah Jurnal**: Pilih batas hasil yang ingin dikumpulkan (`10`, `20`, `50`, atau `100` jurnal) untuk mengontrol beban rayapan dan kecepatan scraping.
- 📡 **Sumber Data**: Centang sumber yang diinginkan (**Garuda**, **SINTA**, atau keduanya).
- 📊 **Filter Akreditasi SINTA**: Pilih peringkat yang diinginkan (S1 sampai S6). Secara default, S3 hingga S6 aktif karena memiliki peluang penerimaan yang lebih realistis bagi mahasiswa.
- 🔍 **Mode Detail OJS (Kontak, APC, Frekuensi)**:
  - *Dimatikan (Mode Cepat)*: Mengumpulkan metadata judul, peringkat SINTA, penerbit, dan tautan OJS dalam hitungan detik.
  - *Dinyalakan (Mode Detail)*: Melakukan kunjungan mendalam secara paralel ke situs OJS untuk menarik Email redaksi, Nomor Telepon/WhatsApp, Frekuensi Terbitan per tahun, dan Estimasi Biaya Publikasi (APC).
- 💾 **Format Output**: Centang format berkas yang Anda butuhkan (**Markdown**, **Excel**, dan/atau **CSV**).
- 🤖 **Toggle Selenium**: Aktifkan jika portal SINTA memerlukan bypass proteksi Cloudflare via browser automation.
- 📜 **Log Pemantauan Real-time**: Memantau perkembangan scraping halaman demi halaman tanpa membuat aplikasi *freeze* (*anti-hang* berkat arsitektur *multithreading*).
- 📂 **Tombol Buka Folder**: Membuka folder `output/` secara langsung di Windows Explorer.

---

### 2. Menjalankan Versi CLI (Terminal)

#### A. Mode Interaktif (Dipandu Langkah demi Langkah)
```bash
python jsfs.py --interactive
# atau cukup:
python jsfs.py
```

#### B. Perintah Langsung dengan Argumen
```bash
# 1. Cari jurnal topik bisnis, filter SINTA 3-6 (default), output Markdown
python jsfs.py --keyword "bisnis"

# 2. Cari topik e-commerce, filter SINTA 3 & 4, output Markdown dan Excel
python jsfs.py --keyword "e-commerce" --sinta 3 4 --format md excel

# 3. Cari dari SINTA & Garuda sekaligus, output lengkap
python jsfs.py --keyword "manajemen" --source both --format md excel csv

# 4. Gunakan Selenium headless Chrome (jika portal SINTA memblokir request biasa)
python jsfs.py --keyword "sistem informasi" --source sinta --selenium
```

---

## 📦 Hasil Output & Penamaan Berkas Unik

Semua berkas hasil scraping akan disimpan secara otomatis di folder **`output/`** dengan penamaan unik berformat:
`JSFS_<keyword>_<YYYYMMDD_HHMMSS>.<ext>` (Contoh: `JSFS_bisnis_20261016_221500.xlsx`)

| Format | Keunggulan & Fitur Visual v1.2 |
|:---:|---|
| **Excel (`.xlsx`)** | Header Navy Blue elegan, tautan OJS aktif yang dapat diklik (`=HYPERLINK`), pewarnaan lembut tematik untuk peringkat SINTA (S1/S2 Hijau, S3/S4 Biru, S5/S6 Abu-abu), serta auto-fit lebar kolom. |
| **Markdown (`.md`)** | Tabel terstruktur, seksi ringkasan parameter & statistik peringkat, serta direktori kontak cepat redaksi jurnal (Email, Telepon, APC, Frekuensi). |
| **CSV (`.csv`)** | Format tabel data standar dengan UTF-8 BOM untuk integrasi ke Google Sheets, database, atau pengolah data. |

Setiap baris data menyertakan:
- **Nama Jurnal** & **Penerbit / Institusi**
- **Peringkat Akreditasi SINTA**
- **Biaya Publikasi (APC)** (Gratis / Terindikasi Tarif)
- **Frekuensi Terbit** (Jumlah nomor / kali terbit setahun)
- **Email Kontak Redaksi** & **Nomor Telepon / WA**
- **Nomor ISSN / E-ISSN** & **Bidang Keilmuan**
- **Tautan Langsung ke Situs OJS** (bisa langsung diklik untuk submisi)

---

## 🏗️ Cara Build ke Executable Mandiri (`JSFS.exe`)

Untuk membuat file `.exe` yang bisa dibagikan ke teman atau dosen tanpa perlu menginstal Python:

```bash
python -m PyInstaller --noconfirm --onefile --windowed --noconsole --name "JSFS" --icon="assets/icon.ico" --collect-data customtkinter --collect-all numpy --collect-all pandas --hidden-import=lxml.etree --hidden-import=lxml._elementpath --hidden-import=bs4 --hidden-import=openpyxl --hidden-import=undetected_chromedriver app_ui.py
```

Setelah proses build selesai:
- Berkas siap pakai berada di direktori **`dist/JSFS.exe`**.

---

## 🗺️ Roadmap Pengembangan

| Versi | Status | Deskripsi Milestone |
|:---:|:---:|---|
| **v1.0.0 (MVP)** | ✅ Selesai | Scraping CLI Garuda + SINTA, filter SINTA, ekspor Markdown/CSV/Excel |
| **v1.1.0 (Desktop)** | ✅ Selesai | Antarmuka GUI CustomTkinter Dark Mode, anti-freeze worker thread, packaging PyInstaller `.exe` |
| **v1.2.0 (Pro)** | 🚀 Rilis Ini | Penamaan berkas ber-timestamp unik, kontrol target rayapan, mode detail OJS (kontak redaksi, frekuensi, APC), styling Excel & Markdown profesional |
| **v2.0.0** | 💡 Ide Masa Depan | Aplikasi Web Multi-user (FastAPI + React/Vite) agar dapat diakses teman sekelas via browser |
| **v3.0.0** | 💡 Ide Masa Depan | Bot Notifikasi Telegram / Email otomatis saat Call for Papers jurnal sasaran dibuka |

---

*Dikembangkan dengan ❤️ untuk Mahasiswa Pejuang Skripsi, Tesis, dan Publikasi Ilmiah.*  
*Disclaimer: Selalu periksa kembali keaslian jurnal dan pedoman penulisan pada situs resmi OJS masing-masing penerbit sebelum mengirimkan naskah.*
