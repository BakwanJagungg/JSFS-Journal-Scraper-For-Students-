# Arsitektur Sistem — JSFS v4.9.1 (Pure Rust Native Desktop)

Dokumen ini menjelaskan arsitektur teknis **JSFS (Journal Scraper For Students) v4.9.1** yang berjalan sebagai aplikasi desktop **100% Pure Rust** dengan antarmuka GPU native (`eframe` / `egui` + `glow` OpenGL), sepenuhnya bebas dari lapisan WebView2, Tauri, maupun Node.js.

---

## 1. Gambaran Umum Arsitektur

JSFS v4.9.1 memisahkan rendering GPU immediate-mode dari mesin komputasi asinkron (`tokio`) menggunakan batas arsitektur berbasis **`Arc<EngineState>`** dan saluran pesan satu arah **`EventSink`**:

```text
+-----------------------------------------------------------------------+
|  Pure Native GPU Frontend (`src/ui/` — eframe 0.37 + egui + glow GL)   |
|  - `theme.rs`         : Palet JSFS Dark / Daylight + Segoe UI         |
|  - `i18n.rs`          : Kamus dwibahasa ID/EN, `DisplayCurrency` &     |
|                         `AuthorProfile`                               |
|  - `sidebar.rs`       : Navigasi kiri 8 tab + switch tema instan      |
|  - `state.rs`         : State tabel, sort, preset, profil & shortlist |
|  - `search_page.rs`   : Beranda, dropdown Preset/Offline & tabel      |
|  - `detail_page.rs`   : Cek 1 URL OJS, Koleksi Naskah, audit & .docx  |
|  - `dospen_page.rs`   : Shortlist favorit, Dospen Brief & tracker     |
|  - Halaman Lain       : Filters, Match Engine, History, Settings, Guide |
+--------------------------+-----------------------^--------------------+
                           | Dispatch Async Cmd    | Non-blocking Stream
                           v (`engine/*_async`)    | (`EventSink::send`)
+--------------------------+-----------------------+--------------------+
|  Decoupled Rust Engine (`src/engine/` + Tokio Multi-Thread Runtime)   |
|  - `engine/search.rs`     : Orkestrasi 4 fase, Smart Cache & Strict Gate|
|  - `engine/manuscript.rs` : Parsing `.docx`/`.pdf` via `spawn_blocking`|
|  - `engine/ai_ops.rs`     : Status engine, uji Cloud AI & Ollama      |
|  - `engine/file_ops.rs`   : Ekspor `.docx`/`.zip`/`.ics`, cek 1 URL,  |
|                             kurs USD/IDR & peluncur aplikasi OS       |
|  - `engine/cache_ops.rs`  : Statistik, pencarian offline & bersihkan  |
+-----------------------------------------------------------------------+
|  Core Domain Modules                                                  |
|  - `config.rs` : Konfigurasi, matcher & `saved_manuscripts.json`      |
|  - `scraper/`  : SINTA, Garuda, OpenAlex/DOAJ/Crossref, Dedup,        |
|                  Anti-Block Rate Limiter (`mod.rs`), OJS Deep Crawler |
|  - `parser/`   : `apc.rs`, `month.rs`, `contact.rs`, `focus.rs`,      |
|                  `doc_matcher.rs` (BM25/Fit/Topik/Sitasi Paragraf),   |
|                  `heuristics.rs` (ISSN ISO 3297, EIC, CFP, risiko)    |
|  - `ai/`       : Groq / Gemini / OpenAI-Compatible + Ollama Lokal     |
|  - `cache.rs`  : SQLite WAL Cache (14-day TTL, tracker & mode offline)|
|  - `exporter/` : Excel (`.xlsx`), MD, CSV, TXT, `citation.rs`         |
|                  (`.ris`/`.bib`), `templates.rs`, `docx_writer.rs`    |
|                  (`.docx`), `calendar.rs` (`.ics`), dan `bundle.rs`   |
|                  (paket `.zip` siap pengajuan)                        |
+-----------------------------------------------------------------------+
```

Setiap tugas berat (permintaan HTTP konkuren, parsing dokumen OOXML/PDF, dan query SQLite WAL) dijalankan di pool `tokio::runtime::Runtime` atau `tokio::task::spawn_blocking`, sehingga thread render UI 60 FPS tetap responsif tanpa *freeze* meskipun memproses hingga 1.000 jurnal.

---

## 2. Alur Pipeline Pencarian (4 Fase)

### Fase 1 — Discovery Portal Nasional & Internasional (`scraper/sinta.rs`, `scraper/garuda.rs`, `scraper/international.rs`)
- Mengambil daftar jurnal secara konkuren dari portal **SINTA**, **Garuda Kemdikbud**, serta **OpenAlex Sources API**, **DOAJ Journals API**, dan **Crossref Journals API** (untuk jurnal internasional Q1–Q4 & metadata DOI) menggunakan `reqwest` (`rustls-tls`, `gzip`, `hickory-dns`).
- Menerapkan *pre-deep deduplication* (`scraper/dedup.rs`) berdasarkan ISSN dan domain/URL kanonik sehingga jurnal yang muncul di beberapa portal tidak di-crawl dua kali dan bukti berbayar tidak tertimpa metadata Gratis.
- **Filter-Before-Deep & Smart Cache Hit**: Kandidat disaring berdasarkan peringkat SINTA/Q target dan dipartisi dengan cache SQLite segar sebelum memasuki tahap *deep crawl*.

### Fase 2 — Deep OJS Crawler, Anti-Block Shield & Heuristik (`scraper/mod.rs`, `scraper/ojs_deep.rs`, `parser/heuristics.rs`)
- Menelusuri hingga 5 titik halaman OJS (termasuk `/about/submissions` dan `/about/editorialPolicies`) per jurnal dengan batas konkurensi adaptif (`Semaphore(32)`, otomatis `14` pada mesin <= 4 logical CPU), *per-host rate limiter + random jitter*, deteksi redirect OJS 2->3, normalisasi beranda `normalize_journal_home_url` (+ fallback HTTP `404`), dan *hard timeout* `5.5` detik.
- Dilengkapi `DashMap`-backed `DomainBreaker`, deteksi blokir Cloudflare `403`/`503`, serta batas baca stream HTML `2 MB`.
- Mengekstrak tanpa permintaan HTTP tambahan:
  - **APC Zero-Trap (`parser/apc.rs`)**: Memisahkan biaya submit Rp 0 (`Article Submission Charge`), membaca total pajak sesudah `=`, mengabaikan denda penarikan naskah (*withdrawal/retraction fee*), menolak klausa gratis historis serta frasa pembaca Open Access, dan mengonversi nominal seragam ke IDR atau USD.
  - **Bulan Terbit & Audit Keaktifan (`parser/month.rs`, `scraper/ojs_deep.rs`)**: Memindai *Current Issue*, tanggal pengumuman, dan `/issue/archive` untuk menghitung `last_issue_year` serta flag `is_dormant`.
  - **Heuristik Integritas & Produktivitas (`parser/heuristics.rs`)**: Validasi checksum ISSN ISO 3297, deteksi domain kloning/risiko (`evaluate_journal_risk`), indeks tambahan, estimasi kecepatan review (`turnaround_weeks`), bahasa artikel & URL template naskah, gaya sitasi (`APA`/`IEEE`/`Chicago`/`Vancouver`/`Harvard`/`MLA`), batas kata (`word_limit`), serta status CFP (`submission_open`).
  - **Kontak Editor & Email (`parser/contact.rs`)**: Dekode Cloudflare XOR `/cdn-cgi/l/email-protection` serta normalisasi nomor WhatsApp (`wa.me/62...`).
  - **Focus & Scope Higienis (`parser/focus.rs`)**: Menolak teks standar *OJS Privacy Statement* dan boilerplate navigasi.

### Fase 3 — Smart Matcher, Topik Naskah & Pre-Flight Fit (`parser/doc_matcher.rs`, `ai/cloud.rs`, `ai/ollama.rs`)
- **Built-in Smart Matcher (Default Zero-Setup)**: Ekstraksi `.docx`/`.pdf` murni Rust dikombinasikan dengan **Bilingual BM25 + Field-Weighted Scoring + Academic Synonym Dictionary (ID–EN)** untuk menghitung *Match Score* tanpa dependensi tambahan.
- **Multi-Topik, Pre-Flight Fit & Cite-to-Target (`parser/doc_matcher.rs`)**: Mengekstrak daftar chip topik pendek (`recommended_topics`), memvalidasi kecocokan jumlah kata naskah terhadap `word_limit` jurnal (`check_manuscript_word_fit`), dan mencocokkan kecocokan sitasi daftar pustaka.
- **Cloud AI API & Ollama Opsional (`ai/cloud.rs`, `ai/ollama.rs`)**: Pengayaan selektif via **Groq**, **Google Gemini**, **OpenAI-Compatible endpoint**, atau **Ollama** lokal (`http://127.0.0.1:11434`) dengan anggaran fase `45` detik dan *circuit breaker*.

### Fase 4 — Strict Post-Filter, SQLite WAL Cache, Ekspor & Paket Pengajuan (`engine/search.rs`, `cache.rs`, `exporter/`)
- Menegakkan ulang seluruh aturan ketat (*strict SINTA gate*, `only_free`, `min_rp..=max_rp`, `include_na`, target bulan terbit, dan `exclude_dormant`) **setelah** fase pengayaan.
- Menyimpan hasil di **SQLite WAL mode** (`.cache/jsfs_cache.db`, batas cache ~4 MB, TTL 14 hari) untuk pencarian offline cepat (`search_cached_journals`) serta status *Submission Tracker* lokal.
- Mengekspor laporan ke folder `Documents\JSFS Output` (mode terinstal) atau `output\` (mode portabel) dalam format **`.xlsx`**, **`.md`**, **`.csv`**, **`.txt`**, sitasi **`.ris` / `.bib`** (`exporter/citation.rs`), generator draf **Cover Letter**, **Surat Keaslian**, **Inquiry Email**, **Follow-Up Email**, dan **Dosbing Brief** (`exporter/templates.rs`), serta paket bundel **`.zip`** (`exporter/bundle.rs`).

---

## 3. Windows DPI, Resource Embedding & Distribusi (`build.rs`, `installer/jsfs.iss`, `main.rs`)

- Di Windows, `main.rs` menerapkan `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` agar executable rilis berjalan langsung tanpa jendela konsol, dan `.cargo/config.toml` menautkan CRT secara statis (`+crt-static`) agar bebas dari dependensi runtime eksternal.
- `build.rs` mengompilasi `assets/icon.ico`, `VersionInfo` (`4.9.1.0`), *Common Controls v6*, serta manifes **`PerMonitorV2`** DPI awareness melalui `winres` sehingga teks dan layout tetap tajam di layar Full HD, 2K, maupun 4K.
- `installer/jsfs.iss` mengemas installer **Inno Setup 6** (`JSFS-windows-x64-setup.exe`) dengan instalasi user-level tanpa hak Administrator ke `%LOCALAPPDATA%\Programs\JSFS`.
