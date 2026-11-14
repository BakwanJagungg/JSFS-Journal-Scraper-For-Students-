# Changelog

All notable changes to **JSFS (Journal Scraper For Students)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [3.0.0] - 2026-11-14

### Added
- **Native Rust + Tauri v2 Architecture**: Full migration to an asynchronous Rust backend (`tokio`, `reqwest`, `scraper`, `rusqlite`, `rust_xlsxwriter`) paired with a lightweight WebView2 desktop shell.
- **Dual-Source Concurrent Scraper**: Asynchronous discovery across **SINTA (S1–S6)** and **Garuda Kemdikbud** with `Semaphore(32)` concurrency, per-host `DomainBreaker`, and `1.5 MB` HTML stream caps.
- **Filter-Before-Deep & Pre-Deep Deduplication**: Filters target SINTA ranks and deduplicates cross-source ISSN/URL entries before executing the 5-point OJS deep crawler (`5.5s` hard per-journal timeout).
- **Domain Parsers**:
  - Publication month and schedule extraction (`month.rs`).
  - APC / publication fee detection in IDR and USD (`apc.rs`).
  - Editor contact, WhatsApp, and Cloudflare XOR email decoding (`contact.rs`).
  - Focus & Scope extraction (`focus.rs`).
  - Pure-Rust `.docx` and `.pdf` manuscript parsing with bilingual lexical match scoring (`doc_matcher.rs`).
- **Local SQLite WAL Cache**: 14-day TTL cache (`cache.rs`) with automatic version-bump purging and clean shutdown checkpointing.
- **Multi-Provider Smart Matcher (Zero-Setup Default)**: Built-in **Bilingual BM25 + Academic Synonym Matcher (ID–EN)** (`doc_matcher.rs`) works out-of-the-box without external software, alongside optional free **Cloud AI API** (**Groq**, **Google Gemini**, **OpenAI-Compatible** via `ai/cloud.rs`) and local **Ollama** (`ai/ollama.rs`), configurable via the interactive header settings modal and persisted in `.cache/matcher_config.json`.
- **Multi-Format Export Engine**: Pure-Rust generators for 3-sheet styled Excel (`.xlsx`), Markdown (`.md`), UTF-8 BOM CSV (`.csv`), and shareable editor drafts (`.txt`).
- **Event-Driven Frontend & Windows Installers**: Real-time Tauri event streaming (`scrape-progress`, `result-batch`, `autosave-done`, `scrape-complete`), NSIS/MSI setup installers, and automated GitHub Actions Windows release workflow.

### Changed
- Reduced binary footprint to ~9.2 MB and idle memory usage to ~37 MB while eliminating UI thread freezes during large-scale scraping (up to 1,000 journals).

---

## [2.x] - Legacy Python Era (Archived)
- **2.4.0**: Introduced Filter-Before-Deep pipeline, 8-item AI cap with 45s hard budget, and live ETA telemetry.
- **2.3.0**: Process-isolated local HTTP + Chromium App-Mode architecture eliminating UI freezes during batch scoring and exports.
- **2.2.0**: Zero-freeze thread-pool limits, lazy SQLite initialization, and forensic freeze watchdog.
- **2.1.0 – 2.1.1**: Pre-deep deduplication, delta-IPC payloads, paginated DOM rendering, and concurrent Garuda page fetching.
- **2.0.0 – 2.0.2**: Desktop WebView2 UI redesign with custom 1–1,000 journal targets and real-time progress telemetry.
