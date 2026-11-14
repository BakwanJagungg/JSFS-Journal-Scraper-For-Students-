# Arsitektur Sistem — JSFS v3.0.0

Dokumen ini menjelaskan arsitektur teknis **JSFS (Journal Scraper For Students)** berbasis **Rust + Tauri v2**.

---

## 1. Gambaran Umum Arsitektur

JSFS memisahkan antarmuka pengguna dan mesin komputasi ke dalam dua lapisan terisolasi yang berkomunikasi melalui **Tauri IPC (Commands & Events)**:

```text
+-------------------------------------------------------------------+
|  Frontend (WebView2 / Vanilla HTML5 + CSS3 + ES6)                 |
|  - ui/index.html, ui/style.css, ui/app.js                         |
|  - Invoke async commands & listen real-time Tauri events          |
+---------------------------------+---------------------------------+
                                  | Tauri IPC (invoke / emit)
+---------------------------------v---------------------------------+
|  Backend Native Rust (Tokio Async Runtime)                        |
|  - src-tauri/src/commands/   : Search, Manuscript, AI, File Ops   |
|  - src-tauri/src/scraper/    : SINTA, Garuda, Dedup, OJS Deep     |
|  - src-tauri/src/parser/     : APC, Month, Contact, Focus, Doc    |
|  - src-tauri/src/ai/         : Ollama Local LLM + Circuit Breaker |
|  - src-tauri/src/cache.rs    : SQLite WAL Cache (14-day TTL)      |
|  - src-tauri/src/exporter/   : XLSX, Markdown, CSV, Share TXT     |
+-------------------------------------------------------------------+
```

Operasi I/O sinkron (SQLite, ekstraksi ZIP/OOXML `.docx`, parsing `.pdf`, dan penulisan `.xlsx`) diisolasi di `tokio::task::spawn_blocking` agar thread runtime asinkron dan antarmuka WebView2 tetap responsif tanpa *freeze*.

---

## 2. Alur Pipeline Pencarian (4 Fase)

### Fase 1 — Dual-Source Portal Discovery (`scraper/sinta.rs`, `scraper/garuda.rs`)
- Mengambil daftar jurnal secara konkuren dari portal **SINTA** dan **Garuda Kemdikbud** menggunakan `reqwest` (`rustls-tls`, `gzip`, `hickory-dns`).
- Menerapkan *pre-deep deduplication* (`scraper/dedup.rs`) berdasarkan ISSN dan domain/URL kanonik sehingga jurnal yang muncul di kedua portal tidak di-crawl dua kali.
- **Filter-Before-Deep**: Kandidat disaring berdasarkan peringkat SINTA target (`S1–S6`) dan diurutkan berdasarkan relevansi metadata awal sebelum memasuki tahap *deep crawl*.

### Fase 2 — Deep OJS Crawler (`scraper/ojs_deep.rs`)
- Menelusuri hingga 5 titik halaman OJS per jurnal dengan batas konkurensi `Semaphore(32)` dan *hard timeout* `5.5` detik per jurnal.
- Dilengkapi `DashMap`-backed `DomainBreaker` untuk mencegah pengulangan permintaan ke host yang bermasalah serta batas baca stream HTML `1.5 MB`.
- Mengekstrak **APC** (`parser/apc.rs`), **Bulan Terbit** (`parser/month.rs`), **Kontak Editor & Email** termasuk dekode Cloudflare XOR (`parser/contact.rs`), dan **Focus & Scope** (`parser/focus.rs`).

### Fase 3 — Multi-Provider Smart Matcher (`parser/doc_matcher.rs`, `ai/cloud.rs`, `ai/ollama.rs`)
- **Built-in Smart Matcher (Default Zero-Setup)**: Ekstraksi `.docx`/`.pdf` murni Rust dikombinasikan dengan **Bilingual BM25 + Field-Weighted Scoring + Academic Synonym Dictionary (ID–EN)** untuk menghitung skor kesesuaian (*Match Score*) tanpa dependensi atau instalasi tambahan.
- **Cloud AI API Opsional (`ai/cloud.rs`)**: Pengayaan selektif via **Groq**, **Google Gemini**, atau **OpenAI-Compatible endpoint** menggunakan API Key opsional dari pengguna (dibatasi maksimal 8 kandidat teratas, anggaran fase `45` detik, dan *circuit breaker*).
- **Ollama Lokal (`ai/ollama.rs`)**: Opsi *power-user* menggunakan `http://127.0.0.1:11434` dengan batas keamanan yang sama.

### Fase 4 — Local Cache & Multi-Format Export (`cache.rs`, `exporter/`)
- Hasil lengkap disimpan di **SQLite WAL mode** (`.cache/jsfs_cache.db`, TTL 14 hari) untuk mempercepat pencarian ulang.
- Laporan otomatis diekspor ke folder `output/` dalam 4 format: **Excel 3-sheet (`.xlsx`)**, **Markdown (`.md`)**, **CSV UTF-8 BOM (`.csv`)**, dan **Daftar Share/Draft Editor (`.txt`)**.
