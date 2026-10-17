# 📚 JSFS — Journal Scraper For Students (v1.3.8)

> **Alat otomatisasi pencarian jurnal SINTA & Garuda untuk mahasiswa: Re-Engineering Mesin Data Mining Focus & Scope (Multi-Tier Mining Engine >90–100% Coverage), Estetika GUI Pixel-Perfect Alignment (Symmetric 2-Column SINTA Grid & Zero Clipping), Indikator Kesehatan Server OJS & Transparansi Kontak, GUI Data Table Previewer Interaktif, Speed Acceleration Engine (Fast-Fail Circuit Breaker), Modul Analisis Naskah (.docx/.pdf) & Match Scoring (0–100%), Hardware Turbo Mode (250–1000 jurnal), dan 0% NaN Data Export.**

---

## 💡 Apa itu JSFS & Kenapa Mahasiswa Membutuhkannya?

Bagi mahasiswa tingkat akhir atau peneliti pemula, mencari jurnal yang tepat di portal **SINTA** dan **Garuda** secara manual sering kali memakan waktu berjam-jam:
- ❌ Harus membuka dan mengklik link puluhan situs jurnal satu per satu.
- ❌ Bingung menentukan jurnal mana yang ruang lingkupnya (*Focus & Scope*) paling cocok dengan topik naskah artikel skripsi/tesis.
- ❌ Sulit memfilter peringkat SINTA yang realistis untuk target publikasi mahasiswa (misal: SINTA 3, 4, 5, atau 6).
- ❌ Informasi status biaya publikasi (*Article Processing Charge* / APC) sering tersembunyi.
- ❌ Kontak redaksi (Email/WA) dan jadwal terbit (*Call for Papers*) sulit ditemukan dengan cepat.
- ❌ Banyak link OJS yang mati atau tidak dapat diakses tanpa peringatan sebelumnya.
- ❌ Menghabiskan waktu berharga yang seharusnya bisa digunakan untuk menyusun naskah dan analisis data.

**JSFS (Journal Scraper For Students) v1.3.8** hadir dengan fitur data mining terlengkap dan estetika GUI terdepan:
- 🎯 **Re-Engineering Mesin Data Mining "Focus & Scope" (Multi-Tier Mining Engine)**: Ekstraksi berjenjang multi-tier: Tier 1 (OJS Meta-Tags & Targeted Selectors), Tier 2 (OJS Homepage Context Parsing), dan Tier 3 (Garuda `j-meta-desc` & SINTA Subject Area Synthesis). Menjamin **>90–100% data Focus & Scope terisi penuh** tanpa "Informasi tidak ditemukan".
- 🖥️ **Tampilan GUI Pixel-Perfect Alignment (Zero Clipping)**: Penataan ulang panel filter SINTA ke dalam grid 2-kolom simetris (Kolom 1: Sinta 1–3, Kolom 2: Sinta 4–6), perataan presisi input Min/Max APC, dan harmonisasi rasio font pada layar 1366x768 maupun 1920x1080.
- 👁️ **GUI Data Table Previewer Interaktif**: Menampilkan data jurnal hasil scraping secara instan di dalam antarmuka desktop (CustomTkinter & ttk.Treeview Dark Theme) dengan fitur live search, filter SINTA/APC/Server, panel pembaca Focus & Scope lengkap, dan dobel-klik untuk membuka URL di browser tanpa perlu Microsoft Excel.
- 🌐 **Indikator Kesehatan Server OJS & SSL**: Deteksi otomatis status server (`Sangat Aktif (HTTPS/SSL Valid)`, `Aktif (HTTP)`, `SSL Warning (Bypass Valid)`, atau `Non-Aktif`).
- 🛡️ **Indikator Transparansi Kontak (Contact Quality Index)**: Mengukur kelengkapan kontak redaksi (`Lengkap (Email & WA/Telp)`, `Sedang (Hanya Email)`, `Terbatas (Hanya WA/Telp)`, atau `Tidak Transparan`).
- 🚀 **Speed Acceleration Engine & Fast-Fail Circuit Breaker**: Pengecekan status domain induk OJS dalam 2.5 detik untuk memangkas thread hanging pada server mati, memangkas durasi pemrosesan 250 jurnal dari 15 menit menjadi < 1.5–2 menit.
- 📄 **Modul Analisis Naskah & Rekomendasi Jurnal (Match Score 0–100%)**: Unggah draf artikel ilmiah (.docx atau .pdf), sistem secara otomatis mengekstrak judul, abstrak, kata kunci, dan istilah dominan, lalu mengalkulasi skor kesesuaian ruang lingkup terhadap setiap jurnal serta mengurutkan hasil secara presisi.
- ⚡ **Hardware Turbo Mode (Throughput Ekstrem)**: Paginasi SINTA via direct concurrent HTTP requests memangkas waktu penelusuran dari belasan detik menjadi sub-detik (<1s), dengan worker paralel dinamis hingga 128 threads.
- 📅 **Standardisasi Mutlak Frekuensi Terbit**: Menghapus total istilah generik seperti `(Semiannual)` atau `(Triwulanan)`. Format baku: `X kali setahun (Bulan1, Bulan2, ...)` atau `X kali setahun`.
- 💰 **Filter Rentang Harga APC & Konversi USD→IDR**: Deteksi kalimat gratis komprehensif, kurs acuan tetap 1 USD = Rp 17.500, dan sanity check kewajaran biaya.
- 📊 **0% NaN Data Export**: Seluruh kolom ekspor Excel (`.xlsx`), CSV (`.csv`), dan Markdown (`.md`) terisi konsisten tanpa missing values.

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
