# Arsitektur Sistem — JSFS v4.3.0 (Pure Rust Native Desktop)

Dokumen ini menjelaskan arsitektur teknis **JSFS (Journal Scraper For Students) v4.3.0** yang berjalan sebagai aplikasi desktop **100% Pure Rust** dengan antarmuka GPU native (`eframe` / `egui` + `glow` OpenGL), sepenuhnya bebas dari lapisan WebView2, Tauri, maupun Node.js.

---

## 1. Gambaran Umum Arsitektur

JSFS v4.3.0 memisahkan rendering GPU immediate-mode dari mesin komputasi asinkron (`tokio`) menggunakan batas arsitektur berbasis **`Arc<EngineState>`** dan saluran pesan satu arah **`EventSink`**:

```text
+-----------------------------------------------------------------------+
|  Pure Native GPU Frontend (`src/ui/` — eframe 0.37 + egui + glow GL)   |
|  - `theme.rs`   : Palet JSFS Dark / Daylight + tipografi Segoe UI     |
|  - `i18n.rs`    : Kamus dwibahasa ID/EN & persistensi app_settings    |
|  - `sidebar.rs` : Navigasi kiri 6 halaman + switch tema instan        |
|  - `state.rs`   : State tabel, sort, preset & filter UI deterministik  |
|  - Views        : Search, Filters, Match Engine, History, Settings, Guide |
|  - `detail_modal.rs` : Bukti APC, Focus & Scope, kontak & draft WA    |
+--------------------------+-----------------------^--------------------+
                           | Dispatch Async Cmd    | Non-blocking Stream
                           v (`engine/*_async`)    | (`EventSink::send`)
+--------------------------+-----------------------+--------------------+
|  Decoupled Rust Engine (`src/engine/` + Tokio Multi-Thread Runtime)   |
|  - `engine/search.rs`     : Orkestrasi 4 fase & Strict Post-Filter    |
|  - `engine/manuscript.rs` : Parsing `.docx` / `.pdf` via `spawn_blocking`
|  - `engine/ai_ops.rs`     : Status engine, uji Cloud AI & Ollama      |
|  - `engine/file_ops.rs`   : Ekspor ulang, buka Excel/folder, OS URL   |
|  - `engine/cache_ops.rs`  : Statistik & pembersihan SQLite WAL cache  |
+-----------------------------------------------------------------------+
|  Core Domain Modules                                                  |
|  - `scraper/`  : SINTA, Garuda, OpenAlex/DOAJ (Q1-Q4), Dedup, Breaker |
|  - `parser/`   : APC Zero-Trap, Month/Recency, Contact XOR, Focus, BM25|
|  - `ai/`       : Groq / Gemini / OpenAI-Compatible + Ollama Lokal     |
|  - `cache.rs`  : SQLite WAL Cache (14-day TTL, version invalidation)  |
|  - `exporter/` : Excel 3-sheet (`.xlsx`), Markdown, CSV, Share TXT    |
+-----------------------------------------------------------------------+
```

Setiap tugas berat (permintaan HTTP konkuren, parsing dokumen OOXML/PDF, dan query SQLite WAL) dijalankan di pool `tokio::runtime::Runtime` atau `tokio::task::spawn_blocking`, sehingga thread render UI 60 FPS tetap responsif tanpa *freeze* meskipun memproses hingga 1.000 jurnal.

---

## 2. Alur Pipeline Pencarian (4 Fase)

### Fase 1 — Dual-Source Portal Discovery (`scraper/sinta.rs`, `scraper/garuda.rs`)
- Mengambil daftar jurnal secara konkuren dari portal **SINTA** dan **Garuda Kemdikbud** menggunakan `reqwest` (`rustls-tls`, `gzip`, `hickory-dns`).
- Menerapkan *pre-deep deduplication* (`scraper/dedup.rs`) berdasarkan ISSN dan domain/URL kanonik sehingga jurnal yang muncul di kedua portal tidak di-crawl dua kali.
- **Filter-Before-Deep**: Kandidat disaring berdasarkan peringkat SINTA target (`S1–S6`) dan diurutkan berdasarkan relevansi metadata awal sebelum memasuki tahap *deep crawl*.

### Fase 2 — Deep OJS Crawler (`scraper/ojs_deep.rs`)
- Menelusuri hingga 5 titik halaman OJS per jurnal dengan batas konkurensi adaptif (`Semaphore(32)`, otomatis `14` pada mesin dengan <= 4 logical CPU) dan *hard timeout* `5.5` detik per jurnal.
- Dilengkapi `DashMap`-backed `DomainBreaker` untuk mencegah pengulangan permintaan ke host yang bermasalah serta batas baca stream HTML `1.5 MB`.
- Mengekstrak:
  - **APC Zero-Trap (`parser/apc.rs`)**: Menolak klausa gratis historis serta frasa pembaca Open Access, dan memprioritaskan biaya reguler berbayar di atas *Fast-Track* atau `"IDR 0"` awal.
  - **Bulan Terbit & Audit Keaktifan (`parser/month.rs`, `scraper/ojs_deep.rs`)**: Memindai *Current Issue*, tanggal pengumuman, dan `/issue/archive` untuk menghitung `last_issue_year` serta flag `is_dormant`.
  - **Kontak Editor & Email (`parser/contact.rs`)**: Dekode Cloudflare XOR `/cdn-cgi/l/email-protection` serta normalisasi nomor WhatsApp (`wa.me/62...`).
  - **Focus & Scope Higienis (`parser/focus.rs`)**: Menolak teks standar *OJS Privacy Statement* dan boilerplate navigasi.

### Fase 3 — Multi-Provider Smart Matcher (`parser/doc_matcher.rs`, `ai/cloud.rs`, `ai/ollama.rs`)
- **Built-in Smart Matcher (Default Zero-Setup)**: Ekstraksi `.docx`/`.pdf` murni Rust dikombinasikan dengan **Bilingual BM25 + Field-Weighted Scoring + Academic Synonym Dictionary (ID–EN)** untuk menghitung *Match Score* tanpa dependensi tambahan.
- **Cloud AI API Opsional (`ai/cloud.rs`)**: Pengayaan selektif via **Groq**, **Google Gemini**, atau **OpenAI-Compatible endpoint** menggunakan API Key opsional pengguna (dibatasi maksimal 8 kandidat teratas, anggaran fase `45` detik, dan *circuit breaker*).
- **Ollama Lokal (`ai/ollama.rs`)**: Opsi *power-user* menggunakan `http://127.0.0.1:11434` dengan batas keamanan yang sama.

### Fase 4 — Strict Post-Filter, SQLite WAL Cache & Ekspor (`engine/search.rs`, `cache.rs`, `exporter/`)
- Menegakkan ulang seluruh aturan ketat (*strict SINTA gate*, `only_free`, `min_rp..=max_rp`, `include_na`, target bulan terbit, dan `exclude_dormant`) **setelah** fase pengayaan agar tidak ada hasil melanggar filter.
- Menyimpan hasil di **SQLite WAL mode** (`.cache/jsfs_cache.db`, TTL 14 hari) dengan pembatalan otomatis saat `CACHE_VERSION` berubah (`"4.3.0"`).
- Mengekspor laporan ke folder `output/` dalam 4 format (**`.xlsx`**, **`.md`**, **`.csv`**, **`.txt`**) dan mengirimkan event selesai ke UI native melalui `EventSink`.

---

## 3. Windows DPI & Resource Embedding (`build.rs` & `main.rs`)

- Di Windows, `main.rs` menerapkan `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` agar executable rilis berjalan langsung tanpa jendela konsol.
- `build.rs` mengompilasi `assets/icon.ico`, `VersionInfo` (`4.3.0.0`), *Common Controls v6*, serta manifes **`PerMonitorV2`** DPI awareness melalui `winres` sehingga teks dan layout tetap tajam di layar Full HD, 2K, maupun 4K.
