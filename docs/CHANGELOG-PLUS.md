# Changelog JSFS+ Workstation

Seluruh perubahan dan fitur khusus pada edisi perangkat spesifikasi tinggi **JSFS+ (Workstation Edition)** dicatat di berkas ini secara terpisah dari edisi Standard (`CHANGELOG.md`).

Format mengacu pada standar Keep a Changelog dengan Semantic Versioning pada kanal rilis `plus-v*`.

## [plus-v5.0.0] - 2026-11-26

### Added
- **Arsitektur Dual-Track Feature Flag `plus` (`Cargo.toml`, `src/config.rs`, `src/engine/mod.rs`)**:
  - Ruang kerja terisolasi `02-active-dev-plus/` dengan feature `plus` aktif default dan binary `jsfs-plus.exe` (`APP_EDITION = "JSFS+ Workstation"`, `APP_VERSION = "5.0.0-plus"`).
  - Skalabilitas workstation: `MAX_TARGET_COUNT = 5000`, `MAX_HTML_BYTES = 12 MB`, `MAX_STREAM_CONTENT_LENGTH_BYTES = 15 MB`, `SQLITE_CACHE_SIZE_KIB = 131072` (~128 MB), runtime Tokio `4 hingga 16` worker thread, dan hingga `64` worker paralel.
- **Headless Browser Sidecar Anti-Cloudflare & SPA (`src/plus/browser_sidecar.rs`)**:
  - Deteksi otomatis `msedge.exe` / `chrome.exe` / `chromium` terpasang untuk fallback DOM dump saat portal OJS mengembalikan blokir bot `403`/`503` atau halaman SPA kosong berbasis JavaScript, dibatasi maksimal 2 proses paralel.
- **Jembatan OCR PDF Hasil Scan (`src/plus/ocr.rs`)**:
  - Pemulihan otomatis naskah `.pdf` dan PDF kebijakan biaya OJS yang teksnya kurang dari 80 karakter melalui `tesseract` CLI (`PATH`) atau Windows Native WinRT OCR (`Windows.Data.Pdf` + `Windows.Media.Ocr`) dengan degradasi halus tanpa error fatal.
- **Hybrid Semantic Reranker & Citation Velocity (`src/plus/semantic.rs`, `src/models.rs`, `src/ui/detail_page.rs`)**:
  - Proyeksi vektor semantik 256-D Pure Rust (unigram, bigram kata, 3-gram karakter dengan normalisasi L2 + cosine similarity), integrasi opsional Ollama `/api/embeddings` (`nomic-embed-text`, `bge-m3`), serta metrik dampak `cited_by_count` / `works_count` dari OpenAlex.
- **Identitas UI Workstation, Kanal Updater `plus-v*`, Installer & CI/CD Terpisah (`src/ui/`, `src/engine/updater.rs`, `installer/jsfs-plus.iss`, `.github/workflows/release.yml`)**:
  - Badge `Workstation` di sidebar, kartu status fitur Plus pada halaman Pengaturan, pemisahan kanal update `plus-v*` vs `v*`, installer Windows terpisah `JSFS-Plus-windows-x64-setup.exe`, dan workflow GitHub Actions jalur ganda.
