# Arsitektur Sistem: JSFS v4.16.2 & JSFS+ Workstation v5.0.0-plus (Dual-Track Pure Rust)

Dokumen ini menjelaskan arsitektur teknis **JSFS (Journal Scraper For Students)** dalam dua edisi terintegrasi di atas satu basis kode **100% Pure Rust** lintas platform (**Windows 64/32-bit, Linux, dan macOS**) dengan antarmuka GPU native (`eframe` / `egui` + `glow` OpenGL), bebas dari Electron maupun Node.js.

## 1. Arsitektur Dual-Track Cargo Feature (`standard` vs `plus`)

Pemilihan edisi dikendalikan pada waktu kompilasi melalui feature flag `plus` di `Cargo.toml`:
- **JSFS Standard (`v4.16.2`, `--no-default-features --bin jsfs`)**: Profil hemat memori (< 500 MB RAM target, 4 MB batas HTML, 16 MB SQLite cache, maksimal 1.000 jurnal per sesi, worker Tokio `2 hingga 4` dan worker OJS `14 hingga 32`), kompatibel luas termasuk Windows 32-bit (`i686`).
- **JSFS+ Workstation (`v5.0.0-plus`, `--features plus --bin jsfs-plus`)**: Profil perangkat spesifikasi tinggi (12 MB batas HTML, 15 MB stream, ~128 MB SQLite cache, maksimal 5.000 jurnal per sesi, worker Tokio `4 hingga 16` dan worker OJS `24 hingga 64`), mengaktifkan modul terisolasi `src/plus/`.

```text
+-----------------------------------------------------------------------+
|  Pure Native GPU Frontend (src/ui/ : eframe 0.37 + egui + glow GL)    |
|  - sidebar.rs       : Navigasi adaptif (248 px), badge Workstation    |
|  - theme.rs / i18n  : Palet Dark / Daylight, kamus dwibahasa ID/EN    |
|  - search_page.rs   : Pencarian live, Auto-Query Expansion            |
|  - detail_page.rs   : Gaya sitasi APA/IEEE/Vancouver/Chicago,         |
|                       Turnaround Estimator, Citation Velocity         |
|  - dospen_page.rs   : Kurasi .jsfs-list, HTML dosbing, .docx/.zip     |
|  - settings_page.rs : Pembaruan terpisah kanal v* vs plus-v* & status  |
+--------------------------+-----------------------^--------------------+
                           | Dispatch Async Cmd    | Non-blocking Stream
                           v (engine/*_async)      | (EventSink::send)
+--------------------------+-----------------------+--------------------+
|  Decoupled Rust Engine (src/engine/ + Tokio Multi-Thread Runtime)     |
|  - search.rs / manuscript.rs / ai_ops.rs / file_ops.rs / updater.rs  |
+----------------------------------+------------------------------------+
                                   | #[cfg(feature = "plus")]
+----------------------------------v------------------------------------+
|  JSFS+ Workstation Extensions (src/plus/)                             |
|  - browser_sidecar.rs : Fallback Chromium/Edge --headless=new (<= 2)  |
|  - ocr.rs             : Jembatan OCR PDF scan (Tesseract / WinRT OCR) |
|  - semantic.rs        : 256-D Vector Projection + Ollama Embeddings   |
|                         + OpenAlex Citation Velocity Impact Boost     |
+-----------------------------------------------------------------------+
|  Core Domain Modules (scraper/, parser/, ai/, cache.rs, exporter/)    |
+-----------------------------------------------------------------------+
```

## 2. Alur Pipeline Pencarian & Ekstraksi

### Fase 1: Discovery & Deduplikasi Lintas Sumber (`scraper/`)
- Menggabungkan hasil secara konkuren dari **SINTA**, **Garuda**, **OpenAlex** (beserta metrik `cited_by_count` & `works_count`), **DOAJ**, dan **Crossref**.
- `scraper/dedup.rs` menggabungkan entri berdasar E-ISSN, P-ISSN, URL OJS kanonik, judul kanonik, serta kemiripan token judul fuzzy (`>= 85%`) tanpa konflik ISSN, memastikan bukti APC berbayar tetap memprioritaskan akurasi di atas status gratis historis.

### Fase 2: Deep OJS Crawler, TLS Adaptif, Sidecar & PDF APC (`scraper/mod.rs`, `ojs_deep.rs`)
- **Strict-First TLS + Fallback Aman**: Koneksi awal menegakkan verifikasi sertifikat TLS ketat; jika domain OJS kampus gagal karena sertifikat SSL kedaluwarsa/self-signed, crawler mengulang satu kali lewat klien fallback dan menampilkan catatan pada `data_source_note`.
- **Paralelisasi Sub-Halaman & PDF OJS**: Sub-halaman kunci (`/about/submissions`, `/about/editorialPolicies`, kontak, fokus) diunduh serentak via `join_all`, disertai ekstraksi teks dokumen `.pdf` biaya publikasi (< 3 MB).
- **JSFS+ Headless Sidecar (`src/plus/browser_sidecar.rs`)**: Saat mode `plus` mendeteksi blokir Cloudflare `403`/`503` atau shell JavaScript SPA kosong, peramban Chromium/Edge lokal dijalankan terkendali (maks 2 proses paralel lewat `Semaphore(2)`).

### Fase 3: Analisis Naskah, OCR & Hybrid Semantic Reranking (`parser/`, `plus/`)
- **OCR PDF Hasil Scan (`src/plus/ocr.rs`)**: Ketika `pdf-extract` menghasilkan `< 80` karakter pada unggahan naskah atau PDF biaya OJS, modul mencoba pemulihan lewat `tesseract` CLI + `pdftoppm` atau Windows Native WinRT OCR (`Windows.Data.Pdf` & `Windows.Media.Ocr`) secara aman tanpa crash.
- **Gaya Sitasi & Sinonim Dwibahasa (`doc_matcher.rs`)**: Mendeteksi format **IEEE (Numeric)**, **Vancouver (Numeric)**, **APA (Author-Year)**, dan **Chicago / Turabian**, didukung kamus sinonim akademik lintas bidang serta normalisasi kata dasar tak beraturan.
- **Hybrid Semantic Reranker (`src/plus/semantic.rs`)**: Pada edisi Plus, skor relevansi BM25 dipadukan dengan cosine similarity vektor padat 256-D Pure Rust, embedding opsional Ollama lokal (`/api/embeddings`), dan dorongan dampak sitasi OpenAlex.

## 3. Updater Terpisah & Distribusi Lintas Platform (`updater.rs`, `release.yml`)

- **Kanal Pembaruan Terpisah**: `is_newer_version` dan `tag_matches_edition_channel` memastikan JSFS Standard hanya memeriksa tag berawalan `v` (seperti `v4.16.2`), sedangkan JSFS+ Workstation hanya memeriksa tag berawalan `plus-v` (seperti `plus-v5.0.0`), lengkap dengan verifikasi `SHA256SUMS.txt` di Windows, Linux, maupun macOS (`arm64` dan `x64`).
- **Instalasi Berdampingan di Windows**: `installer/jsfs.iss` dan `installer/jsfs-plus.iss` memakai `AppId` dan folder instalasi berbeda (`JSFS` vs `JSFS-Plus`) sehingga keduanya dapat terpasang bersamaan tanpa konflik.
