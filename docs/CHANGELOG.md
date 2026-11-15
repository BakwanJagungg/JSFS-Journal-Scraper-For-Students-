# Changelog

All notable changes to **JSFS (Journal Scraper For Students)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [4.1.0] - 2026-11-15

### Added
- **Shortlist Bookmark (`★` / `☆`) & Filter Pilihan (`src/ui/state.rs`, `src/ui/search_page.rs`, `src/ui/mod.rs`)**:
  - Added `AppState` (`bookmarked_ids: HashSet<String>`, `only_bookmarked: bool`) with deterministic canonical key matching (`AppState::bookmark_key`) and `FilterSettings::summary_text()`.
  - Added compact star toggle buttons (`★` / `☆`) in the table **Aksi** column alongside a toolbar filter button **`★ Pilihan (N)`** to isolate shortlisted journals in one click.
  - Added **`Segoe UI Symbol`** (`seguisym.ttf`) font fallback in `src/ui/theme.rs` so Unicode star icons render sharply on Windows.
- **Quick Export Toolbar (`Salin WA` & `Excel`) (`src/ui/search_page.rs`, `src/ui/mod.rs`)**:
  - Added **`Salin WA`** (copies visible or shortlisted table rows to clipboard in WhatsApp list format via `format_wa_share_from_ui`) and **`Excel`** (opens the latest exported `.xlsx` workbook via `open_excel_async`) directly in the `Hasil Pencarian` toolbar.

### Improved
- **Compact Preset Bar & Live Active-Filter Summary (`src/ui/search_page.rs`, `src/ui/state.rs`)**:
  - Grouped the 4 quick presets with `8.0 px` spacing behind a `Preset:` label, enhanced `accent_soft` active tinting, and added a right-aligned live summary (e.g. `Filter: SINTA 2-6 • Gratis Rp 0 • Semua Bulan`).
- **Top-Right Toast Placement (`src/ui/mod.rs`)**:
  - Anchored floating toast notifications to `Align2::RIGHT_TOP` (`(-20.0, 16.0)`) so table `Detail` and `Web` action buttons in lower rows remain unobstructed.

---

## [4.0.2] - 2026-11-15

### Improved
- **Scrollable Search Workspace & Compact Hero/KPI Layout (`src/ui/search_page.rs`)**:
  - Wrapped `render_search_page()` in a full-page vertical `ScrollArea` (`search_page_scroll`) and increased the results `TableBuilder` minimum scrolled height to `360.0 px` so both long workspaces and result rows remain smoothly accessible without bottom clipping.
  - Replaced stacked preset boxes with compact 1-line 32 px preset chips (`preset_chip`, full criteria in hover tooltips), merged the 4 separate KPI cards into a single 44 px horizontal KPI strip (`render_kpi_strip`), and removed the idle manuscript helper caption and subtitle clutter.
- **High-Contrast JSFS Daylight Theme (`src/ui/theme.rs`)**:
  - Refined `ThemeMode::Light` with cool slate surfaces (`#eef2f6` canvas, `#f8fafc` sidebar, `#ffffff` cards, `#f1f5f9` inputs, `#cbd5e1` borders), crisp slate foregrounds (`#0f172a` / `#475569`), and high-contrast dark-on-pastel status badge pairs.
- **UI Copy & Version Synchronization (`src/config.rs`, `Cargo.toml`, `build.rs`, `src/main.rs`, `src/ui/guide_page.rs`, `src/ui/mod.rs`)**:
  - Replaced decorative em-dash separators in window/header/toast labels with concise single hyphens and bumped `APP_VERSION`, `CACHE_VERSION`, and Win32 `VersionInfo` to `4.0.2`.

---

## [4.0.1] - 2026-11-15

### Improved
- **Rebranding & Native Windows Segoe UI Typography (`src/ui/theme.rs`, `src/ui/sidebar.rs`, `src/exporter/markdown.rs`)**:
  - Removed all leftover `"TwinDesk"` theme/brand references and standardized theme names to **`JSFS Dark`** and **`JSFS Daylight`**.
  - Loaded Windows native **`Segoe UI`** regular system font (`C:\Windows\Fonts\segoeui.ttf`) with proportional typography scale (`Heading: 21px`, `Subheading: 15px`, `Body/Button: 13.5px`, `Small: 11.5px`) and generous `item_spacing`.
  - Replaced hardcoded `"JSFS v3.1.0"` in Markdown export headers with `crate::config::APP_VERSION`.
- **Search Toolbar & Results Table Alignment (`src/ui/search_page.rs`, `src/ui/filters_page.rs`, `src/ui/match_engine_page.rs`, `src/ui/state.rs`)**:
  - Unified height (`36.0 px`) and vertical alignment across the hero search row (`Keyword`, `Target`, `Unggah Naskah`, `Mulai Cari Jurnal`, `Filter`).
  - Added consistent `8.0 px` horizontal cell/header padding, `52.0 px` row height, and bottom viewport padding on `egui_extras::TableBuilder` so bottom rows never clip.
  - Added `4.0 px` vertical field spacing on budget inputs in `filters_page.rs`, updated the default **`Target Skripsi S1`** preset (`target_count: 40`, `Rp 0 – Rp 1.500.000`, `include_na = true`, `only_free = false`) for broader varied results, and synchronized the live mode badge beside `Simpan Pengaturan Match Engine`.

### Fixed
- **Strict Landline Phone Area-Code Validation (`src/parser/contact.rs`)**:
  - Exposed `is_valid_phone()` and tightened landline rules to require standard office separators (`(024) 8449610`, `021-7863456`) or explicit immediate phone cues while rejecting unspaced 8+ digit internal extension/ISSN patterns such as `+6221117123` and `+6224196207` (verified via `test_reject_internal_extension_like_21117123`).

---

## [4.0.0] - 2026-11-15

### Added
- **Pure Rust Native GPU-Rendered GUI (`src/ui/`, `eframe` + `egui` 0.37 + `glow`)**:
  - Replaced the WebView2 / HTML / CSS / JavaScript frontend with a 100% pure Rust OpenGL desktop interface featuring a modern solid sidebar (`src/ui/sidebar.rs`), instant **Dark / Daylight** theme toggle (`src/ui/theme.rs`), and 5 dedicated views: `Pencarian Jurnal`, `Filter Lanjutan`, `Match Engine & AI`, `Riwayat & Ekspor`, and `Panduan & Tentang`.
  - Built native virtualized results table (`egui_extras::TableBuilder`), interactive detail modal (`src/ui/detail_modal.rs`) with `apc_evidence`, `last_issue_year`, full Focus & Scope, Cloudflare-decoded editor email/WhatsApp links, and one-click polite WhatsApp draft copy.
  - Added native OS file pickers (`rfd`) for manuscript uploads (`.docx` / `.pdf`) and custom folder/file opening.
- **Decoupled Async Engine (`src/engine/`)**:
  - Replaced Tauri IPC (`commands/*.rs`, `AppHandle`) with pure Rust `Arc<EngineState>` and lock-free `EventSink` channel (`src/engine/{mod,search,manuscript,ai_ops,file_ops,cache_ops}.rs`), keeping UI frames smooth while preserving 100% of v3.1.0 scraping, Zero-Trap APC scanning, strict filter gates, SQLite WAL caching, and 4-format exporters.
- **Windows High-DPI & Resource Integration (`build.rs`, `src/main.rs`)**:
  - Embedded multi-resolution `assets/icon.ico`, `PerMonitorV2` DPI awareness manifest, Common Controls v6, and Win32 `VersionInfo` (`4.0.0.0`) directly via `winres` without requiring external WebView2 installers.

### Changed
- **Zero-Web / Single-Binary Repository Layout**:
  - Removed `ui/`, `src-tauri/`, `package.json`, and `package-lock.json`; promoted pure Rust `Cargo.toml`, `Cargo.lock`, `build.rs`, `src/`, and `assets/` directly to the repository root.
  - Updated `.github/workflows/release.yml` to test (`cargo test --locked`), lint (`cargo clippy --all-targets --locked -- -D warnings`), and publish standalone `jsfs.exe` + portable Windows `.zip` releases cleanly.
  - Bumped `APP_VERSION` and `CACHE_VERSION` in `src/config.rs` to `"4.0.0"`.

---

## [3.1.0] - 2026-11-15

### Added
- **Journal Activity & Recency Audit (`models.rs`, `parser/month.rs`, `scraper/ojs_deep.rs`)**:
  - Scans homepage Current Issue blocks, announcement dates, metadata dates, and `/issue/archive` headings to extract `last_issue_year`, compute `activity_status` (`Aktif (2026)`, `Aktif (2025)`, `Kurang Aktif (Terakhir 2024)`, `Dorman / Tidak Terurus (<2024)`), and flag dormant/abandoned OJS sites (`is_dormant`, including broken `navigation.archives` templates without recent issues).
  - Automatically exposes APC verification snippets (`apc_evidence`, max 140 chars) in the Journal Detail modal.
- **Strict SINTA Gate & Dormancy Exclusion (`commands/search.rs`, `models.rs`)**:
  - Added `include_non_sinta_garuda` (default `false`) and `exclude_dormant` (default `true`) filter controls so SINTA-filtered searches no longer leak unranked Garuda-only journals or dormant (>2-year inactive) journals unless explicitly enabled.
- **UI & Filter Modal Polish (`ui/index.html`, `ui/style.css`, `ui/app.js`)**:
  - Widened `.target-wrap` / `.target-input` (`min-width: 132px`, `width: 68px`, `tabular-nums`, hidden spin overlay) so 4-digit targets such as `1000` render without clipping.
  - Added color-coded activity status pills (`Aktif`, `Terakhir 2024`, `Dorman`) in the results table, sticky modal footer (`Reset Default` & `Terapkan Filter`), and new detail modal fields.

### Fixed
- **Privacy-Statement Contamination Cleaner (`parser/focus.rs`)**:
  - Rejected OJS standard *Privacy Statement* sentences (`"The names and email addresses entered in this journal site..."` / Indonesian variants) with immediate boilerplate guards and automatic fallback synthesis (`synthesize_focus_scope`).
- **Publisher Prefix Hygiene (`scraper/mod.rs`, `scraper/garuda.rs`, `scraper/sinta.rs`, `scraper/ojs_deep.rs`)**:
  - Unified publisher sanitization across all portals to strip `"Published by"`, `"Diterbitkan oleh"`, and link-bar noise.
- **Cache Version Invalidation (`config.rs`)**:
  - Bumped `CACHE_VERSION` to `"3.1.0"` to refresh cached records with recency metadata cleanly.

---

## [3.0.1] - 2026-11-15

### Fixed
- **Zero-Trap Global APC Scanner (`parser/apc.rs`, `scraper/ojs_deep.rs`)**:
  - Added sanitizers to strip historical fee ranges (e.g., `from 2008 to 2018 was free of charge`) and Open-Access reader phrases (`readers can read and download any full-text articles for free of charge`) before checking free publication cues.
  - Enforced **Paid Always Beats Free** globally across all crawled OJS pages and DOM fee blocks (`#custom-0`, `#custom-1`, `/about/editorialPolicies`, `/about/submissions`), preventing early `"IDR 0"` / `"No charge"` clauses from masking downstream publication charges.
  - Normalized mixed Indonesian/US currency formats (`3.000.000,00 (IDR)`, `Rp. 400.000,-`, `1.000,000`) and separated **Regular Fees** from optional **Fast-Track Fees** so regular fees take precedence.
- **Post-Enrichment Strict Budget & Filter Gate (`commands/search.rs`)**:
  - Added `apply_strict_filters` and re-ran strict `only_free`, `min_rp..=max_rp`, `include_na`, publish-month, and SINTA filters *after* Phase 3 AI/BM25 enrichment so no over-budget or paid journal escapes when `only_free` or budget caps are active.
- **SINTA Publisher Hygiene (`scraper/sinta.rs`)**:
  - Filtered SINTA navigation link-bar text (`Google Scholar`, `Website`, `Editor URL`, `Published by`) via `sanitize_sinta_publisher` so publisher columns remain clean or fall back to OJS `citation_publisher` metadata.
- **AI APC Prompt Sync (`ai/mod.rs`)**:
  - Updated `PROMPT_APC_TPL` (shared by Cloud AI and Ollama) to ignore historical years and prioritize downstream Regular fees over earlier `"IDR 0"` or Fast-Track numbers.
- **Cache Version Invalidation (`config.rs`)**:
  - Bumped `CACHE_VERSION` to `"3.0.1"` to purge stale `"3.0.0"` cached APC entries automatically.

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
