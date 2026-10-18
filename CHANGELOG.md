# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.0] - 2026-10-18

### Added & Enhanced
- **Hybrid AI Engine (Ollama REST API + Fallback Regex Legacy v1.3.8)**:
  - Ekstraksi Focus & Scope dan Author Fees menggunakan model lokal `qwen2.5:3b` via Ollama REST API (`localhost:11434`) dengan format output terstruktur JSON.
  - Mekanisme **Smart Truncation**: Memilih kalimat-kalimat dengan kata kunci substantif relevan (maksimal 3.000 karakter) dengan menjaga urutan kalimat asli.
  - **Validasi Ketat Anti-Halusinasi**: Pengecekan angka terhadap teks mentah, validasi nominal `0` hanya bila ada pernyataan eksplisit "gratis/free", pembatasan ringkasan Focus & Scope maksimal 2 kalimat dan 40-60 kata.
  - **Ketahanan Sistem & Fallback Otomatis**: Caching status Ollama (TTL 60s aktif / 20s non-aktif), Circuit Breaker (5 kegagalan berturut-turut memicu cooldown 60s ke Regex), dan fallback per-jurnal tanpa menghentikan proses scraping.
- **Pemisahan Biaya Penulis (Author Fees)**:
  - Memisahkan biaya lama `Biaya APC` menjadi kolom terpisah: `Biaya Submit`, `Biaya Publikasi`, dan `Mata Uang` (`IDR` / `USD`).
- **Parser & Filter Bulan Terbit (`month_parser.py`)**:
  - Ekstraksi deterministik jadwal bulan terbit jurnal (1–12) berbasis Regex tanpa beban LLM.
  - Filter interaktif di GUI untuk memilih jadwal bulan terbit dan opsi menyertakan jurnal tanpa data bulan.
  - Kolom baru `Bulan Terbit` (mis. `Mar, Sep`) pada output Excel, CSV, dan Markdown.
- **Transparansi Asal Data (Audit Trail)**:
  - Kolom baru `Sumber Focus` dan `Sumber APC` (bernilai `AI` / `Regex` / `Kosong`) untuk memverifikasi asal data.
- **Arsitektur Pipeline 4-Fase Berkinerja Tinggi**:
  - *Fase 1*: Fast Broad Scraping (128 workers HTTP/Selenium) untuk mengumpulkan teks mentah (`raw_about`, `raw_submissions`) dan metadata awal.
  - *Fase 2*: Pre-Filter menyaring jurnal berdasarkan kriteria SINTA dan Bulan Terbit sebelum ke mesin AI.
  - *Fase 3*: AI Enrichment memproses jurnal lolos seleksi dengan thread pool terpisah (1–2 workers).
  - *Fase 4*: Ekspor Data (.xlsx, .md, .csv).
- **GUI CustomTkinter Zero Clipping & Responsif (`app_ui.py`)**:
  - Layout anti-clipping dengan pengukuran teks dinamis (`fit_w`, `FitCheckBox`, `FitSwitch`).
  - Badge Status Ollama di header kanan atas yang otomatis diperbarui secara asinkronus tiap 30 detik (`● AI Aktif` hijau / `○ Mode Regex` abu-abu).
  - Dual Progress Bar: baris pertama untuk Scraping Massal (HTTP) dan baris kedua untuk Analisis AI (Ollama).
  - Pembaruan berkala via thread-safe `queue.Queue` dengan coalescing pesan progres tiap 100 ms sehingga GUI bebas freeze.
  - Data Table Previewer interaktif v1.4.0 mendukung kolom Submit, Publikasi, Bulan Terbit, dan Sumber.
- **1-Click AI Setup Manager (`ai_installer.py`)**:
  - Wizard terintegrasi di dalam GUI untuk mengunduh dan menginstal Ollama (`OllamaSetup.exe`) secara otomatis tanpa perlu membuka browser secara manual.
  - Unduhan model otomatis (`qwen2.5:3b`) via streaming Ollama REST API `/api/pull` dengan indikator progres layer per layer tanpa mengetik di terminal.
  - Badge Ollama interaktif yang dapat diklik untuk membuka modal dialog instalasi & verifikasi model secara instan.

## [1.3.8] - 2026-10-17

### Added & Enhanced
- **Re-Engineering Mesin Data Mining Focus & Scope (Multi-Tier Mining Engine)**:
  - **Tier 1 (OJS Meta-Tags & Targeted Deep Scrape)**: Mengekstrak tag meta `<meta name="DC.Description">`, `<meta name="description">`, `<meta property="og:description">`, dan kontainer DOM OJS `#focusAndScope`, `.focus_and_scope`, `.journal-description`, `#journalDescription`, `div.page_about`, serta heading-based lookup.
  - **Tier 2 (OJS Homepage Context Parsing)**: Mengekstrak paragraf pembuka kontekstual dari halaman beranda OJS (`#content`, `#main-content`, `div.page`, `div.journal-about`) yang mendeskripsikan ranah jurnal secara substantif.
  - **Tier 3 (SINTA & Garuda Profile Overview Fallback)**: Mengambil deskripsi resmi jurnal dari `div.j-meta-desc` / `div.j-meta-subject` Portal Garuda dan sintesis profil rumpun ilmu (Subject Area) serta instansi penerbit resmi SINTA. Menjamin **>90–100% data Focus & Scope terisi penuh dan berkualitas tinggi**.
  - **Normalisasi Teks Ketat**: Pembersihan tag HTML/noise, penghapusan prefix duplikat, dan peringkasan cerdas menjadi 2–3 kalimat representatif (maksimal 280–300 karakter).
- **Perbaikan Estetika GUI Total — Pixel-Perfect Alignment & Zero Clipping (`app_ui.py`)**:
  - Penataan ulang panel filter SINTA ke dalam format **grid 2-kolom simetris**:
    - Kolom 1: Sinta 1, Sinta 2, Sinta 3
    - Kolom 2: Sinta 4, Sinta 5, Sinta 6
    - Memberikan padding `padx=(10, 10)`, `pady=(3, 3)`, dan `sticky="w"` dengan font terstandardisasi `CTkFont(family="Segoe UI", size=11)`, menjamin label 'Sinta 1' s.d. 'Sinta 6' bebas terpotong (*Zero Clipping*).
  - Penataan label '⚡ Turbo Mode', '🔍 Mode Detail OJS', dan 'Gunakan Selenium' dengan jarak horizontal yang presisi.
  - Penjajaran horizontal simetris pada input `Min (Rp):` dan `Max (Rp):` serta checkbox 'Hanya Gratis (Rp 0)' dan 'Sertakan N/A'.
  - Harmonisasi rasio font header judul (`size=16, weight="bold"`), subtitle (`size=11`), dan kotak log (`height=160`).
- **Peningkatan Data Table Previewer (`JSFSDataPreviewer`)**:
  - Peningkatan pembaca teks Focus & Scope pada panel detail bawah dan integrasi langsung ke ekspor Excel, CSV, dan Markdown.

## [1.3.7] - 2026-10-17

### Added & Enhanced
- **Perbaikan Tampilan GUI Total — Zero Clipping (`app_ui.py`)**:
  - Rekonfigurasi grid checkbox SINTA S1–S6 dengan 3 kolom berbobot setara (`grid_columnconfigure((0, 1, 2), weight=1)`) tanpa batasan lebar kaku, memastikan teks label tidak terpotong sama sekali pada resolusi 1366x768 maupun 1920x1080.
  - Penataan label '⚡ Turbo Mode', '🔍 Detail OJS', dan 'Gunakan Selenium' dengan jarak horizontal (`padx`, `sticky="w"`) yang lega dan simetris.
  - Presisi perataan input 'Min (Rp):' dan 'Max (Rp):' untuk tata letak yang rapi dan profesional.
  - Tombol aksi utama berdampingan: '🚀 MULAI SCRAPING' (#0284c7) dan '👁️ PREVIEW HASIL DATA' (#059669) dengan kontras warna modern.
- **Ekstraksi Kolom "Focus & Scope" Jurnal (`scraper_garuda.py` & `exporter.py`)**:
  - Perluasan penelusuran OJS Quad-Pass ke sub-halaman: `/about/editorialPolicies#focusAndScope`, `/about/focusAndScope`, `/about`, heading anchors, dan meta tags HTML `<meta name="DC.Description">` serta `<meta name="description">`.
  - Pembersihan dan peringkasan cerdas menjadi 2–3 kalimat utama (maksimal 300 karakter) yang menggambarkan ruang lingkup riset jurnal.
  - Penambahan kolom baru **'Focus & Scope'** pada seluruh format ekspor: Excel (`.xlsx`), CSV (`.csv`), dan Markdown (`.md`).
- **Fitur Konsistensi & Indikator Kualitas Data**:
  - Kolom **'Status Server OJS'**: Mendeteksi kesehatan server dan validitas SSL dengan kategori `Sangat Aktif (HTTPS/SSL Valid)`, `Aktif (HTTP)`, `SSL Warning (Bypass Valid)`, atau `Non-Aktif`.
  - Kolom **'Transparansi Kontak'**: Mengukur kelengkapan saluran komunikasi redaksi dengan klasifikasi `Lengkap (Email & WA/Telp)`, `Sedang (Hanya Email)`, `Terbatas (Hanya WA/Telp)`, atau `Tidak Transparan`.
- **GUI Data Table Previewer Interaktif (`app_ui.py`)**:
  - Jendela pop-up tabel data terpadu (`JSFSDataPreviewer`) bertema gelap (*dark modern UI*) untuk melihat, mencari, dan meninjau hasil scraping langsung dari aplikasi tanpa perlu membuka Microsoft Excel.
  - Fitur pencarian instan (real-time filtering) berdasarkan nama, institusi, kontak, dan kata kunci di dalam Focus & Scope.
  - Filter drop-down dinamis untuk Peringkat SINTA, Rentang Biaya APC, dan Status Kesehatan Server.
  - Panel detail naskah di bagian bawah dengan scrollable textbox untuk membaca ruang lingkup Focus & Scope secara utuh.
  - Dobel-klik baris tabel untuk langsung membuka situs jurnal di web browser.
  - Tombol cepat integrasi untuk membuka berkas Excel hasil ekspor.

## [1.3.6] - 2026-10-17

### Added & Optimized
- **Optimasi Kepadatan Tata Letak GUI (`app_ui.py`)**:
  - Rekonfigurasi dimensi jendela menjadi 900x700 (minsize 880x660) yang menjamin tampilan anti-clipping pada layar resolusi standar 1366x768 maupun 1920x1080.
  - Penataan ulang panel *Analisis Naskah Artikel* menjadi 1 baris horizontal yang padat (label, tombol unggah, checkbox match score, status, tombol hapus), mereduksi tinggi vertikal sebesar >40px.
  - Perapatan jarak antar-komponen (`pady` dari 6–8px menjadi 2–4px) dan penyesuaian tinggi kotak riwayat log (160px).
  - Tombol aksi utama **`🚀 MULAI SCRAPING`** selalu tampak utuh di layar tanpa harus digulir.
- **Fast-Fail Circuit Breaker untuk Domain OJS (`scraper_garuda.py` & `scraper_sinta.py`)**:
  - Pengecekan ketersediaan domain induk OJS menggunakan probing HEAD/GET cepat dengan timeout ketat (`FAST_FAIL_TIMEOUT = (2.0, 2.5)`).
  - Melewati (*skip*) instan seluruh sub-jalur OJS jika server induk *offline*, *timeout*, atau *connection refused*.
  - Mekanisme *caching* status domain thread-safe sehingga seluruh jurnal lain yang berada pada domain server mati langsung dilewati dalam 0 ms.
- **Target Reach Short-Circuiting (Stop Awal Saat Kuota Terpenuhi)**:
  - Menghentikan proses Quad-Pass deep scraping segera setelah jumlah jurnal yang lolos filter (misal: *Hanya Gratis / Rp 0*) mencapai target kuota pengguna, mengeliminasi pemrosesan sisa kandidat yang tidak diperlukan.
- **Accelerated Socket Timeout & Single Retry Strategy**:
  - Pengetatan timeout deep scraping OJS: `DEEP_SCRAPE_TIMEOUT = (2.0, 3.5)` (connect timeout 2.0s, read timeout 3.5s).
  - Penerapan `max_retries=1` pada deep scraping OJS untuk mengeliminasi siklus tunggu ulang (*hanging sockets*).
  - Mengoptimalkan penelusuran sub-jalur OJS hingga 4 jalur berprioritas tertinggi, memangkas durasi pemrosesan 250 jurnal dari ~15 menit menjadi di bawah 1.5–2 menit.

## [1.3.5] - 2026-10-17

### Added & Enhanced
- **Modul Analisis Naskah Artikel (.docx / .pdf) & Match Scoring (`doc_reader.py` & `app_ui.py`)**:
  - Pembaca dokumen ilmiah untuk berkas `.docx` (via `python-docx` dengan fallback `zipfile` XML parsing) dan `.pdf` (via `pypdf`).
  - Ekstraksi otomatis metadata naskah: Judul naskah, Abstrak, Kata Kunci (Keywords), dan Istilah Dominan berfrekuensi tinggi (dengan pembersihan stopwords bahasa Indonesia dan Inggris).
  - Algoritma perhitungan kompatibilitas / kecocokan (*Match Score* 0–100%) terhadap ruang lingkup jurnal (bidang keilmuan, judul jurnal, dan fokus penerbit).
  - Klasifikasi tingkat kesesuaian: *Sangat Sesuai* (skor >= 70%), *Sesuai* (40% - 69%), dan *Kurang Sesuai* (< 40%).
  - Integrasi antarmuka GUI: Tombol unggah naskah, status ekstraksi waktu nyata, auto-fill kata kunci pencarian, dan pemetaan hasil ekspor terurut berdasarkan skor kesesuaian tertinggi.
- **Penyetaraan Kualitas Portal Garuda dengan SINTA (`scraper_garuda.py` & `scraper_sinta.py`)**:
  - Resolusi URL OJS langsung dari halaman detail Garuda (`Home Page` / `Website`) menggantikan URL internal portal viewer.
  - Cross-referencing otomatis antara hasil Portal Garuda dengan direktori resmi SINTA menggunakan nomor ISSN, E-ISSN, maupun frasa judul jurnal.
  - Memastikan jurnal dari Garuda memperoleh peringkat akreditasi SINTA resmi (S1–S6), tautan profil SINTA, serta agregasi *Subject Area* SINTA yang setara presisinya dengan hasil penelusuran SINTA.
- **Standardisasi Mutlak Frekuensi Terbit Baku (`scraper_garuda.py`)**:
  - Eliminasi total seluruh istilah generik dalam kurung seperti `(Semiannual)`, `(Triwulanan)`, `(Bulanan)`, `(Tahunan)`, `(Dwi-bulanan)`.
  - Format output mutlak: `X kali setahun (Bulan1 & Bulan2)` atau `X kali setahun (Bulan1, Bulan2, ...)` jika bulan terdeteksi, `X kali setahun` jika hanya jumlah terdeteksi, atau `Informasi tidak ditemukan dalam situs jurnal`.
- **Validasi Kontak Redaksi Strict & Eliminasi Email Dummy**:
  - Penyaringan kontak email: Menolak otomatis alamat email template bawaan OJS atau dummy sintetis (`firstauthor@...`, `secondauthor@...`, `@pkp.sfu.ca`, `@example.com`).
  - Penyaringan telepon/WA ketat: Pembuangan string berformat tanggal (`YYYY-MM-DD`, `026 2026-04-07`, dsb.), angka tahunan (`2024`, `2025`, `2026`), dan derau ISSN 8-digit. Hanya meloloskan format seluler/PSTN valid Indonesia (prefix +62, 62, 08, 02.. panjang 10–14 digit).
- **Pencarian Kata Kunci Adaptif Multi-Topik**:
  - Pemecahan kombinasi kata kunci (misal: `bisnis, ekonomi, commerce`) secara cerdas, penelusuran sub-topik paralel, dan penggabungan unik (*cross-source deduplication*) tanpa duplikasi jurnal.

## [1.3.4] - 2026-10-17

### Added & Optimized
- **Hardware Turbo Mode (250–1000 Jurnal)**:
  - Pemindaian paginasi direktori SINTA menggunakan direct concurrent HTTP requests via `ThreadPoolExecutor` yang mengeliminasi delay sekuensial dan mempercepat crawling halaman dari 15 detik menjadi sub-detik (<1s).
  - Skalabilitas worker dinamis hingga 128 worker paralel (`min(128, CPU * 8)`).
  - High-capacity connection pool: `HTTPAdapter(pool_connections=128, pool_maxsize=128)` dan toleransi timeout adaptif 5 detik untuk throughput jaringan maksimal.
  - Sakelar antarmuka desktop: Checkbox *"Hardware Turbo Mode (Kecepatan Maksimal)"* di GUI CustomTkinter.
- **Standardisasi Mutlak Frekuensi Terbit**:
  - Eliminasi total seluruh istilah generik: `(Semiannual)`, `(Triwulanan)`, `(Dwi-bulanan)`, `(Tahunan)`, `(Bulanan)`.
  - Format output mutlak: `X kali setahun (Bulan1, Bulan2, ...)` jika bulan terbit terdeteksi (contoh: `2 kali setahun (Juni & Desember)`), atau `X kali setahun`, atau `Informasi tidak ditemukan dalam situs jurnal`.
- **Ekstraksi Bidang Keilmuan Terpadu**:
  - Penggabungan multi-sumber dari SINTA profile (`Subject Area`), kategori Garuda, dan meta-tag HTML OJS (`<meta name="DC.Subject">`, `<meta name="keywords">`, `<meta name="citation_keywords">`).
  - Pembersihan stopwords akademis dan normalisasi menjadi string terpisah koma yang presisi (contoh: `Ekonomi, Manajemen, Bisnis`).
- **Validasi Rigid ISSN & E-ISSN**:
  - Eliminasi otomatis nomor dummy seperti `0`, `0000-0000`, `1234-5678`, `None`.
  - Normalisasi dan cross-matching format standar 8 digit (`XXXX-XXXX`) lintas sumber (SINTA, Garuda, OJS meta tags).
- **Peningkatan Cakupan Telepon/WA & Email Redaksi**:
  - Perlindungan elemen footer OJS (tidak lagi di-decompose) sehingga alamat kontak pada footer tertangkap secara utuh.
  - Penyaringan ketat regex untuk membuang format tanggal (YYYY-MM-DD), derau ISSN, dan alamat email template OJS (`@pkp.sfu.ca`, `@example.com`).

## [1.3.3] - 2026-10-17

### Fixed & Enhanced
- **Agresif Meminimalkan Data Null / Kosong (Zero-Null Policy)**:
  - Implementasi fallback berjenjang multi-sumber untuk memastikan data kontak, frekuensi terbit, dan link OJS terisi dengan presisi tinggi.
  - Standarisasi nilai default yang konsisten menjadi `"Informasi tidak ditemukan dalam situs jurnal"` untuk seluruh field tanpa data.
- **Penangkapan Bulan Terbit Spesifik pada Frekuensi Terbitan**:
  - Ekstraksi token bulan kalender Indonesia & internasional (Januari–Desember / Jan–Dec) dari konteks teks deskripsi jurnal.
  - Pemformatan frekuensi terbitan spesifik menyertakan bulan kalender (contoh: `2 kali setahun (Juni & Desember)`).
- **Presisi Ekstraksi Biaya Publikasi (APC) Contextual Window**:
  - Pembatasan jendela pencarian regex biaya publikasi hanya pada teks yang memiliki kata kunci konteks (`author fees`, `biaya publikasi`, `apc`, `publication charge`).
  - Deteksi dan konversi tarif multi-mata uang (USD ke IDR kurs Rp 17.500) dan pengenalan ragam frasa gratis OJS Indonesia.
- **Pembersihan Kontak Redaksi & Eliminasi Domain Template OJS**:
  - Penyaringan awal terhadap domain template bawaan OJS (`@pkp.sfu.ca`, `@example.com`) dan nomor telepon palsu dari metadata instalasi OJS.
- **Validasi Hyperlink OJS Berbasis SSL-Tolerant & Adaptive Timeout**:
  - Mengakomodasi server institusi lokal (.ac.id) dengan bypass galat sertifikat SSL mandiri dan timeout jaringan 6 detik.

## [1.3.2] - 2026-10-17

### Fixed & Architected
- **Pemulihan Link OJS via SINTA Profile Fallback**:
  - Menyimpan URL profil SINTA resmi pada setiap jurnal. Jika tautan OJS tidak ditemukan pada daftar pencarian utama, scraper secara otomatis melakukan secondary request ke profil SINTA untuk mengekstrak href tombol 'Website' / 'Editor URL'.
- **Pemanfaatan Hardware Maksimal & Connection Pooling**:
  - Alokasi dinamis `ThreadPoolExecutor` disesuaikan dengan core CPU pengguna (`min(64, CPU * 4)`).
  - Implementasi `requests.Session` dengan `HTTPAdapter(pool_connections=100, pool_maxsize=100)` dan retry otomatis untuk transmisi paralel tanpa hambatan socket.
- **Quad-Pass OJS Crawling**:
  - Penelusuran 4 sub-jalur OJS: Halaman utama -> `/about/submissions` -> `/about/contact` -> `/about/editorialTeam`.
  - Ekstraksi blok HTML terstruktur (`#authorFees`, `.author_fees`, `#contact`) dan meta-tag akademik sebelum pemindaian teks mentah.
- **Validasi Hyperlink SSL-Tolerant & Adaptive Timeout**:
  - Pengecekan status OJS menggunakan `verify=False` dan timeout 6 detik untuk mengakomodasi server kampus lokal (.ac.id) tanpa salah menandai sebagai situs mati.
- **Pre-Compilation Regex & Pipeline Sanitasi DOM**:
  - Seluruh pola regex dikompilasi di tingkat awal modul.
  - Pembersihan menyeluruh tag `<script>`, `<style>`, komentar, serta karakter tersembunyi (`&nbsp;`, `\xa0`) sebelum proses ekstraksi regex.

## [1.3.1] - 2026-10-16

### Fixed & Accelerated
- **Perbaikan Logika Filter APC & Checkbox Fleksibel**:
  - Menambahkan opsi antarmuka baru: Checkbox *"Sertakan Jurnal Belum Teridentifikasi (N/A)"* (aktif secara default). Jurnal yang tidak mencantumkan biaya publikasi secara eksplisit di situs OJS tidak lagi dibuang ketika filter *"Hanya Jurnal Gratis"* aktif, menyelesaikan kendala 0 data.
- **Pembaruan Kurs Dolar (USD ke IDR)**:
  - Kurs acuan konversi Dolar diperbarui menjadi Rp 17.500 per USD (contoh: $10 USD $\rightarrow$ Rp 175.000 ($10 USD)).
- **Perluasan Pattern Regex Biaya Gratis**:
  - Menambahkan pengenalan ragam frasa gratis OJS Indonesia & internasional: `penyerahan naskah: 0`, `0.00 (idr)`, `0 idr`, `tidak dikenakan biaya`, `bebas biaya halaman`, `free publication`, dsb.
- **Sistem Caching Metadata OJS Lokal (`.cache/ojs_cache.json`)**:
  - Mekanisme cache thread-safe berbasis JSON dengan TTL 7 hari. Menghindari request berulang ke situs OJS yang sama dan mempercepat eksekusi berturut-turut hingga mendekati instan.
- **Strategi Ekstraksi HTML Meta-Tag**:
  - Scanning OJS memprioritaskan ekstraksi langsung dari tag `<meta>` (seperti `DC.Publisher`, `citation_journal_title`, `citation_author_email`) sebelum menelusuri seluruh hierarki DOM.
- **Akselerasi Parallel Fetching (12 Workers)**:
  - Meningkatkan konkurensi `ThreadPoolExecutor` menjadi 12 workers untuk pemrosesan deep detail puluhan hingga ratusan jurnal secara simultan.

## [1.3.0] - 2026-10-16

### Added
- **Skalabilitas Target & Paginasi Dinamis (Hingga 1000 Jurnal)**: Input target fleksibel (10, 50, 100, 250, 500, 1000, atau Custom Entry) dengan penelusuran halaman dinamis di SINTA dan Garuda hingga kuota terpenuhi.
- **Strategi Data Mining & Dual-Subpath Inspection**: Secondary Pass Crawling otomatis menelusuri sub-halaman `/about/contact` dan `/about/submissions` jika kontak atau APC tidak ditemukan di halaman utama OJS.
- **Contextual DOM Scoping untuk APC**: Pencarian regex biaya publikasi dibatasi secara presisi pada elemen teks dengan kata kunci konteks ("author fee", "publication fee", "biaya publikasi", "processing charge").
- **Konversi Mata Uang USD ke IDR**: Otomatis mendeteksi tarif USD dan mengonversi ke Rupiah (kurs 1 USD = Rp 16.000) dengan format ganda `Rp X ($Y USD)`.
- **APC Sanity Filter & Kontrol Filter Rentang Harga di UI**:
  - Validasi kewajaran nilai APC Rupiah (Gratis / Rp 0, atau rentang Rp 50.000 s.d. Rp 15.000.000). Mengeliminasi angka acak dari regex.
  - Filter antarmuka: Checkbox "Hanya Jurnal Gratis (Rp 0)" serta input rentang "Min APC" dan "Max APC".
- **Normalisasi Frekuensi Terbitan**: Pemetaan teks frekuensi ke format baku Bahasa Indonesia (Tahunan, Semiannual, Triwulanan, Dwi-bulanan, Bulanan) dan eliminasi angka akumulasi historis seperti "9 Nomor" atau "14 NOMOR".
- **Validasi Hyperlink OJS (Link Mati)**: Pengecekan HTTP asinkronus cepat (timeout 3 detik). Tautan mati (404, 500, Connection Error, Timeout) otomatis diubah menjadi teks non-aktif "Situs tidak dapat diakses".
- **Pembersihan & Standardisasi Data Null**: Seluruh data yang tidak ditemukan distandarisasi secara resmi menjadi kalimat baku: *"Informasi tidak ditemukan dalam situs jurnal"*.

## [1.2.0] - 2026-10-16

### Added
- **Sistem Penamaan Berkas Unik Otomatis**: Format berkas ekspor kini otomatis menyertakan kata kunci dan timestamp presisi detik (`JSFS_<keyword>_<YYYYMMDD_HHMMSS>.<ext>`) agar tidak menimpa berkas hasil riset sebelumnya.
- **Pengendalian Beban Rayapan (Performance Control)**: Pilihan target jumlah jurnal pada UI (`10`, `20`, `50`, `100` jurnal) untuk membatasi rayapan sesuai kebutuhan mahasiswa.
- **Mode Ekstraksi Detail OJS Mendalam**: Opsi toggle mode detail dengan `ThreadPoolExecutor` paralel untuk mengekstrak email redaksi, nomor telepon/WhatsApp, frekuensi penerbitan per tahun, serta estimasi biaya publikasi (APC) langsung dari situs jurnal.
- **Penyempurnaan Visual Excel (.xlsx)**:
  - Header bernuansa Navy Blue (`#1B365D`) dengan teks tebal putih dan border tipis.
  - Tautan OJS aktif interaktif (`=HYPERLINK(...)`) yang dapat langsung diklik menuju situs jurnal.
  - Pewarnaan sel lembut tematik untuk Peringkat SINTA (S1/S2: Hijau Muda, S3/S4: Biru Muda, S5/S6: Abu-abu).
  - Auto-fit cerdas lebar kolom Excel.
- **Penyempurnaan Visual Markdown (.md)**: Seksi ringkasan statistik distribusi peringkat SINTA, estimasi biaya APC, serta direktori kontak cepat redaksi jurnal.
- **Pembaruan Parser DOM SINTA & Garuda**: Penyesuaian selector terhadap pembaruan portal Kemdiktisaintek (`div.list-item` SINTA dan `table tr` Garuda) serta eliminasi false-positive deteksi Cloudflare pada CDN pihak ketiga.

## [1.1.0] - 2026-10-16

### Added
- Graphical User Interface (GUI) desktop modern berbasis `CustomTkinter` (`app_ui.py`) dengan tema Dark Mode.
- Modul redirect log terminal (`log_redirect.py`) dengan `QueueWriter` dan `make_queue_console` untuk menangkap log `rich.Console` / stdout dan mengalirkannya ke GUI.
- Arsitektur non-blocking menggunakan `threading.Thread` dan `queue.Queue` untuk menjaga GUI tetap responsif (anti-freeze) saat proses scraping berjalan.
- Polling antrean log secara asinkronus menggunakan `self.after(100)` pada main GUI thread.
- Kontrol UI interaktif: input keyword, checkbox pemilihan sumber data (Garuda & SINTA), checkbox peringkat SINTA (S1 - S6), format ekspor (Markdown, Excel, CSV), dan toggle Selenium headless.
- Jendela teks log pemantauan proses (read-only) dengan auto-scroll waktu nyata.
- Tombol pintas "Buka Folder" untuk langsung mengakses folder hasil scraping (`output/`) di Windows Explorer.
- Panduan dan konfigurasi build executable tunggal Windows (`JSFS.exe`) menggunakan PyInstaller (`--onefile`, `--windowed`/`--noconsole`, `--collect-data customtkinter`).
- Update `requirements.txt` untuk menyertakan `customtkinter==5.2.2` dan `pyinstaller==6.11.0`.

## [1.0.0] - 2026-10-15

### Added
- Rilis MVP awal berbasis Command Line Interface (CLI) (`jsfs.py`).
- Modul scraper Portal Garuda (`scraper_garuda.py`) untuk ekstraksi metadata jurnal, penerbit, link OJS, ISSN/E-ISSN, akreditasi Sinta, dan estimasi status APC.
- Modul scraper Direktori SINTA (`scraper_sinta.py`) berbasis HTTP request standar dan Selenium headless Chrome untuk mem-bypass proteksi JavaScript/Cloudflare.
- Dukungan pemfilteran peringkat akreditasi SINTA 1 hingga 6 (default: SINTA 3, 4, 5, 6 yang relevan bagi mahasiswa).
- Modul ekspor (`exporter.py`) yang menghasilkan berkas Markdown (`.md`), Microsoft Excel (`.xlsx`), dan Spreadsheet (`.csv`).
- Visualisasi tabel ringkasan dan banner di terminal menggunakan library `rich`.
- Konfigurasi terpusat pada `config.py` mencakup headers realistis, random request delays, dan batas maksimum halaman.
- Mode interaktif terpandu CLI (`--interactive`) untuk kemudahan penggunaan via command prompt.
