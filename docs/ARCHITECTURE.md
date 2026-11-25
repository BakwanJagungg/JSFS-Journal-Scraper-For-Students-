# Arsitektur Sistem: JSFS v4.16.0 (Pure Rust Native Desktop Lintas Platform)

Dokumen ini menjelaskan arsitektur teknis **JSFS (Journal Scraper For Students) v4.16.0** yang berjalan sebagai aplikasi desktop **100% Pure Rust** lintas platform (**Windows 64/32-bit, Linux, dan macOS**) dengan antarmuka GPU native (`eframe` / `egui` + `glow` OpenGL), sepenuhnya bebas dari lapisan WebView2, Tauri, maupun Node.js.

---

## 1. Gambaran Umum Arsitektur

JSFS v4.16.0 memisahkan rendering GPU immediate-mode dari mesin komputasi asinkron (`tokio`) menggunakan batas arsitektur berbasis **`Arc<EngineState>`** dan saluran pesan satu arah **`EventSink`**:

```text
+-----------------------------------------------------------------------+
|  Pure Native GPU Frontend (src/ui/ : eframe 0.37 + egui + glow GL)    |
|  - theme.rs         : Palet JSFS Dark / Daylight + Segoe UI           |
|  - i18n.rs          : Kamus dwibahasa ID/EN, DisplayCurrency &        |
|                       AuthorProfile                                   |
|  - sidebar.rs       : Navigasi alur kerja bernomor 1-2-3 + tema       |
|  - state.rs         : State tabel, kepadatan, sort, profil & shortlist|
|  - search_page.rs   : Auto-Query Expansion, Beranda panduan pemula    |
|  - detail_page.rs   : Gaya sitasi APA/IEEE, Salin referensi, Turnitin |
|  - dospen_page.rs   : Shortlist permanen, anti-submisi ganda, Word    |
|  - Halaman Lain     : Filters, Match Engine, History, Settings, Guide |
+--------------------------+-----------------------^--------------------+
                           | Dispatch Async Cmd    | Non-blocking Stream
                           v (engine/*_async)      | (EventSink::send)
+--------------------------+-----------------------+--------------------+
|  Decoupled Rust Engine (src/engine/ + Tokio Multi-Thread Runtime)     |
|  - engine/search.rs     : Orkestrasi 4 fase, Smart Cache & Strict Gate|
|  - engine/manuscript.rs : Parsing .docx/.pdf via spawn_blocking       |
|  - engine/ai_ops.rs     : Status engine, uji Cloud AI & Ollama        |
|  - engine/file_ops.rs   : Ekspor .docx/.zip/.ics, cek 1 URL,          |
|                           kurs USD/IDR & peluncur aplikasi OS         |
|  - engine/cache_ops.rs  : Statistik, pencarian offline & bersihkan    |
|  - engine/updater.rs    : Pengecekan update GitHub latar belakang     |
|                           (asinkron, panic-safe & fallback dialog)    |
+-----------------------------------------------------------------------+
|  Core Domain Modules                                                  |
|  - config.rs : Konfigurasi, matcher, naskah & saved_shortlist.json    |
|  - scraper/  : SINTA, Garuda, OpenAlex/DOAJ/Crossref, Dedup,          |
|                Anti-Block Rate Limiter, Slow-Host Deprioritizer,      |
|                Fallback XML OAI-PMH & Atom Feed, OJS Deep Crawler     |
|  - parser/   : apc.rs (Context Window & Print Fee Trap Guard),        |
|                month.rs, contact.rs, focus.rs,                        |
|                doc_matcher.rs (BM25/Fit/Topik/Deteksi Sitasi APA-IEEE)|
|                heuristics.rs (Bilingual Stemmer, ISSN, EIC, CFP, Risk)|
|  - ai/       : Groq / Gemini / OpenAI-Compatible + Ollama Lokal       |
|  - cache.rs  : SQLite WAL Cache + HTTP Conditional Caching (304)      |
|  - exporter/ : Excel (.xlsx), MD, CSV, TXT, citation.rs (.ris/.bib),  |
|                templates.rs, docx_writer.rs (.docx formal pengajuan), |
|                calendar (.ics), dan bundle.rs (paket .zip lengkap)    |
+-----------------------------------------------------------------------+
```

Setiap tugas berat (permintaan HTTP konkuren, parsing dokumen OOXML/PDF, dan query SQLite WAL) dijalankan di pool `tokio::runtime::Runtime` atau `tokio::task::spawn_blocking`, sehingga thread render UI 60 FPS tetap responsif tanpa *freeze* meskipun memproses hingga 1.000 jurnal.

---

## 2. Alur Pipeline Pencarian (4 Fase)

### Fase 1: Discovery Portal Nasional & Internasional (`scraper/sinta.rs`, `scraper/garuda.rs`, `scraper/international.rs`)
- Mengambil daftar jurnal secara konkuren dari portal **SINTA**, **Garuda Kemdikbud**, serta **OpenAlex Sources API**, **DOAJ Journals API**, dan **Crossref Journals API** (untuk jurnal internasional Q1-Q4 & metadata DOI) menggunakan `reqwest` (`rustls-tls`, `gzip`, `hickory-dns`).
- Menerapkan *pre-deep deduplication* (`scraper/dedup.rs`) berdasarkan ISSN dan domain/URL kanonik sehingga jurnal yang muncul di beberapa portal tidak di-crawl dua kali dan bukti berbayar tidak tertimpa metadata Gratis.
- **Triangulasi Metadata ISSN**: Menggunakan OpenAlex, DOAJ, dan Crossref untuk mengisi metadata frekuensi, reputasi, dan status publikasi jurnal berstatus proteksi bot.
- **HTTP Conditional Caching**: Memanfaatkan header ETag dan Last-Modified (HTTP 304 Not Modified) pada `src/cache.rs` untuk menghemat bandwidth data.
- **Filter-Before-Deep & Smart Cache Hit**: Kandidat disaring berdasarkan peringkat SINTA/Q target dan dipartisi dengan cache SQLite segar sebelum memasuki tahap *deep crawl*.

### Fase 2: Deep OJS Crawler, Anti-Block Shield & Heuristik (`scraper/mod.rs`, `scraper/ojs_deep.rs`, `parser/heuristics.rs`)
- **Adaptive Concurrency & Slow-Host Deprioritization (`src/scraper/mod.rs`)**: Pelacakan latensi server kampus (`HostLatencyStats`). Domain yang merespons lambat (> 4.000 ms) otomatis dideprioritaskan ke antrean sekunder sehingga pekerja scraping tetap melaju pada kecepatan puncak.
- **Fallback XML OAI-PMH & Atom Feed (`src/scraper/ojs_deep.rs`)**: Jika halaman HTML OJS kustom tidak memuat arsip artikel standar, crawler melakukan fallback ringan ke `/oai?verb=ListIdentifiers` atau `/gateway/plugin/WebFeedGatewayPlugin/atom`.
- Menelusuri hingga 5 titik halaman OJS (termasuk `/about/submissions` dan `/about/editorialPolicies`) per jurnal dengan batas konkurensi adaptif (`Semaphore(32)`, otomatis `14` pada mesin <= 4 logical CPU), *per-host rate limiter + random jitter*, deteksi redirect OJS 2->3, normalisasi beranda `normalize_journal_home_url` (+ fallback HTTP `404`), dan *hard timeout* `5.5` detik.
- Dilengkapi `DashMap`-backed `DomainBreaker`, deteksi blokir Cloudflare `403`/`503`, serta batas baca stream HTML `2 MB`.
- Mengekstrak tanpa permintaan HTTP tambahan:
  - **APC Zero-Trap & Print Fee Guard (`parser/apc.rs`)**: Memisahkan biaya submit Rp 0 (`Article Submission Charge`), mendeteksi dan mengabaikan jebakan biaya cetak fisik buku/eksemplar (`RE_PRINT_FEE_STRIP`), membaca total pajak sesudah `=`, mengabaikan denda penarikan naskah (*withdrawal/retraction fee*), menolak klausa gratis historis serta frasa pembaca Open Access, dan mengonversi nominal seragam ke IDR atau USD.
  - **Bulan Terbit & Audit Keaktifan (`parser/month.rs`, `scraper/ojs_deep.rs`)**: Memindai *Current Issue*, tanggal pengumuman, dan `/issue/archive` untuk menghitung `last_issue_year` serta flag `is_dormant`.
  - **Heuristik Integritas & Produktivitas (`parser/heuristics.rs`)**: Validasi checksum ISSN ISO 3297, deteksi domain kloning/risiko (`evaluate_journal_risk`), deteksi ketidaksinkronan domain resmi (indikasi *hijacked journal*), indeks tambahan, estimasi kecepatan review (`turnaround_weeks`), bahasa artikel & URL template naskah, gaya sitasi (`APA`/`IEEE`/`Chicago`/`Vancouver`/`Harvard`/`MLA`), batas kata (`word_limit`), batas similarity/plagiarisme (`max_similarity_pct`), serta status CFP (`submission_open`).
  - **Kontak Editor & Email (`parser/contact.rs`)**: Dekode Cloudflare XOR `/cdn-cgi/l/email-protection` serta normalisasi nomor WhatsApp (`wa.me/62...`).
  - **Focus & Scope Higienis (`parser/focus.rs`)**: Menolak teks standar *OJS Privacy Statement* dan boilerplate navigasi.

### Fase 3: Smart Matcher, Topik Naskah & Deteksi Sitasi (`parser/doc_matcher.rs`, `ai/cloud.rs`, `ai/ollama.rs`)
- **Bilingual Stemmer ID-EN & Bigram BM25 (`src/parser/heuristics.rs`, `src/engine/search.rs`)**: Ekstraksi kata dasar dwibahasa (penanganan imbuhan berulang/stacked prefixes bahasa Indonesia dan sufiks akademik bahasa Inggris) dikombinasikan dengan pembobotan bigram frasa 2 kata untuk pencocokan topik naskah yang presisi.
- **Deteksi Gaya Sitasi Naskah Dominan (APA vs IEEE) (`src/parser/doc_matcher.rs`)**: Pindai otomatis format sitasi naskah (angka dalam kurung siku `[1]` untuk IEEE vs nama tahun `(Pratama, 2023)` untuk APA) dan mencocokkan kesesuaian dengan panduan author jurnal target.
- **Salin 5 Referensi Terbaru Jurnal Target (`src/scraper/international.rs`, `src/ui/detail_page.rs`)**: Pengambilan referensi aktual artikel jurnal target via Crossref/metadata OJS untuk disalin dalam 1 klik sebagai penguat daftar pustaka naskah mahasiswa.
- **Multi-Topik, Pre-Flight Fit & Cite-to-Target (`parser/doc_matcher.rs`)**: Mengekstrak daftar chip topik pendek (`recommended_topics`), memvalidasi kecocokan jumlah kata naskah terhadap `word_limit` jurnal (`check_manuscript_word_fit`), dan mencocokkan kecocokan sitasi daftar pustaka.
- **Auto-Query Expansion (`src/parser/heuristics.rs`, `src/ui/search_page.rs`)**: Rekomendasi kueri alternatif saat hasil pencarian kurang dari 5 jurnal.
- **Cloud AI API & Ollama Opsional (`ai/cloud.rs`, `ai/ollama.rs`)**: Pengayaan selektif via **Groq**, **Google Gemini**, **OpenAI-Compatible endpoint**, atau **Ollama** lokal (`http://127.0.0.1:11434`) dengan anggaran fase `45` detik dan *circuit breaker*.

### Fase 4: Strict Post-Filter, SQLite WAL Cache, Ekspor & Paket Pengajuan (`engine/search.rs`, `cache.rs`, `exporter/`)
- Menegakkan ulang seluruh aturan ketat (*strict SINTA gate*, `only_free`, `min_rp..=max_rp`, `include_na`, target bulan terbit, dan `exclude_dormant`) **setelah** fase pengayaan.
- Menyimpan hasil di **SQLite WAL mode** (`.cache/jsfs_cache.db`, batas cache ~4 MB, TTL 14 hari) untuk pencarian offline cepat (`search_cached_journals`) serta status *Submission Tracker* lokal.
- Mengekspor laporan ke folder `Documents\JSFS Output` (mode terinstal) atau `output\` (mode portabel) dalam format **`.xlsx`**, **`.md`**, **`.csv`**, **`.txt`**, sitasi **`.ris` / `.bib`** (`exporter/citation.rs`), generator draf **Cover Letter**, **Surat Keaslian**, **Inquiry Email**, **Follow-Up Email**, **Matriks Revisi Reviewer**, dan **Lembar Persetujuan Dosbing** (`exporter/templates.rs`, `exporter/docx_writer.rs`), serta paket bundel **`.zip`** (`exporter/bundle.rs`).

---

## 3. Isolasi Lintas Platform, DPI & Distribusi (`build.rs`, `installer/jsfs.iss`, `main.rs`)

- **Kompilasi Bersyarat Lintas OS (`Cargo.toml`, `build.rs`, `src/main.rs`)**:
  - `winres` ditempatkan di `[target.'cfg(windows)'.build-dependencies]` dan `build.rs` hanya menyematkan ikon serta manifes Windows ketika target kompilasi adalah Windows.
  - `src/main.rs` menerapkan `#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]`, dialog error native `MessageBoxW` pada Windows, serta *graceful degradation* log standar `eprintln!` saat inisialisasi GUI gagal di Linux/macOS.
  - `src/engine/updater.rs` memisahkan mekanisme penggantian biner `.exe` / `.old` / `setup.exe` khusus Windows dengan jalur verifikasi `SHA256SUMS.txt` untuk arsip `JSFS-linux-x64.tar.gz` dan `JSFS-macos-arm64-preview.tar.gz` pada Unix.
- `.cargo/config.toml` menautkan CRT Windows secara statis (`+crt-static`) untuk target `x86_64-pc-windows-msvc` (64-bit) dan `i686-pc-windows-msvc` (32-bit) agar bebas dari dependensi runtime eksternal.
- `build.rs` mengompilasi `assets/icon.ico`, `VersionInfo` (`4.16.0.0`), *Common Controls v6*, serta manifes **`PerMonitorV2`** DPI awareness melalui `winres` sehingga teks dan layout tetap tajam di layar Full HD, 2K, maupun 4K.
- Distribusi rilis multi-platform melalui `.github/workflows/release.yml` mencakup installer Inno Setup 6 (`JSFS-windows-x64-setup.exe`), arsip portabel Windows x64 (`JSFS-windows-x64-portable.zip`), portabel Windows x86 (`JSFS-windows-x86-portable.zip`), single binary `jsfs.exe`, Linux x64 (`JSFS-linux-x64.tar.gz`), macOS Apple Silicon Preview (`JSFS-macos-arm64-preview.tar.gz`), serta `SHA256SUMS.txt` terpadu.
- `installer/jsfs.iss` mengemas installer **Inno Setup 6** (`JSFS-windows-x64-setup.exe`) dengan instalasi user-level tanpa hak Administrator ke `%LOCALAPPDATA%\Programs\JSFS`.
