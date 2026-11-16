# Changelog

All notable changes to **JSFS (Journal Scraper For Students)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [4.9.1] - 2026-11-16

### Added
- **Multi-Manuscript Switcher (`src/config.rs`, `src/ui/detail_page.rs`, `src/ui/mod.rs`)**:
  - Persisted metadata for up to 5 recently loaded manuscripts in `.cache/saved_manuscripts.json` and added the **Koleksi Naskah Saya** card in `Detail & Persiapan` for one-click manuscript switching.

### Improved
- **Clean Toolbar, Left-Aligned Table Column 1 & Streamlined Export Card (`src/ui/search_page.rs`, `src/ui/detail_page.rs`)**:
  - Moved the local SQLite **Offline** toggle into the `Preset` dropdown and removed the standalone `Offline` pill button from row 2.
  - Adjusted `TableBuilder` width calculation (`available_width() - 16.0`) and switched column 1 (`Jurnal & Penerbit`) to left-aligned `ui.vertical` truncation so titles never clip on the left edge.
  - Consolidated the 6 template copy buttons in Card 3 into one template selector `ComboBox` + **Salin Teks** button, and showed muted `Belum terdata` / `Not recorded` fallbacks for empty editor email or phone fields.

---

## [4.9.0] - 2026-11-16

### Added
- **Dedicated Sidebar Workspaces (`src/ui/detail_page.rs`, `src/ui/dospen_page.rs`, `src/ui/sidebar.rs`, `src/ui/i18n.rs`)**:
  - Replaced modal popups with two spacious sidebar tabs below Search: **Detail & Persiapan** (with **Quick 1-URL OJS Analyzer** and Recent Inspected list) and **Shortlist & Dospen** (side-by-side comparison, Advisor Brief generator, and Submission Tracker).
- **Native OOXML `.docx` & Calendar `.ics` Generators (`src/exporter/docx_writer.rs`, `src/exporter/calendar.rs`, `src/exporter/bundle.rs`)**:
  - Generated editable Word `.docx` files for Cover Letters and Originality Statements (bundled inside the 1-click `.zip` package alongside rules summary), plus `.ics` submission calendar reminders.
- **Editor-in-Chief Extraction, Live USD/IDR Rate & Citation Density Check (`src/parser/heuristics.rs`, `src/parser/doc_matcher.rs`, `src/engine/file_ops.rs`, `src/ui/settings_page.rs`)**:
  - Extracted OJS `editor_in_chief` names for personalized Cover Letters/emails, flagged long manuscript paragraphs (`> 70` words) missing citations, and added on-demand live USD/IDR exchange rate fetching in Settings.

---

## [4.8.1] - 2026-11-15

### Improved
- **Compact Detail Modal & Left-Aligned Table Column 1 (`src/ui/detail_modal.rs`, `src/ui/search_page.rs`)**:
  - Shortened the header match badge to `"Match XX%"` so header badges remain neatly aligned on a single line.
  - Moved template letter/email buttons (`Cover Letter (ID/EN)`, `Surat Keaslian`, `Email Editor (ID/EN)`) into a dedicated card inside the modal `ScrollArea` and restored a clean 4-button footer (`Salin Draf WA`, `Paket Pengajuan (.zip)`, `Cari Jurnal Mirip`, `Buka Web OJS`).
  - Anchored column 1 (`Jurnal & Penerbit`) in `search_page.rs` strictly left-to-right (`allocate_ui_with_layout(..., Align::Min)` + explicit `inner_w`) to eliminate left-edge text clipping.

---

## [4.8.0] - 2026-11-15

### Added
- **Crossref Journals API Integration (`src/scraper/international.rs`, `src/engine/search.rs`)**:
  - Added concurrent **Crossref Journals API** search alongside OpenAlex and DOAJ using polite User-Agent `JSFS-Academic-Research-Tool/4.8.0 (mailto:research@jsfs.local)` and ISSN/title deduplication.

### Improved
- **Indonesian/International APC Accuracy & OJS Home Normalization (`src/parser/apc.rs`, `src/scraper/dedup.rs`, `src/scraper/ojs_deep.rs`)**:
  - Separated `Article Submission Charge: 0.00 (IDR)` from `Article Publication Charge`, parsed post-`=` tax totals (`IDR 2,000,000 (+tax 11%) = IDR 2,220,000,-`), and ignored withdrawal/retraction penalty fines (`US$1000`, `penarikan naskah`).
  - Enforced Paid OJS evidence priority over Free metadata (e.g., DOAJ `Rp 0`) during cross-source merge, normalized saved `website_url` values to the OJS home root via `normalize_journal_home_url`, and added automatic HTTP `404` root fallback.
- **Filter Cards Spacing (`src/ui/filters_page.rs`, `src/ui/detail_modal.rs`)**:
  - Increased filter card padding/spacing (`14.0 px`) with concise muted card descriptions and unified vertical spacing across sections.

---

## [4.7.0] - 2026-11-15

### Added
- **Anti-Block Shield & OJS 2->3 Redirect Detector (`src/scraper/mod.rs`, `src/scraper/ojs_deep.rs`, `src/parser/heuristics.rs`)**:
  - Added per-host rate limiting with random jitter, browser User-Agent rotation (*Mode Bot Akademik Sopan*), automatic OJS 2->3 redirect detection, Cloudflare `403`/`503` protection flagging, and `2 MB` streaming abort guards.
- **Academic Integrity & Extra Indexing (`src/parser/heuristics.rs`, `src/models.rs`)**:
  - Added pure ISO 3297 ISSN Modulo-11 checksum validation, cloned/suspicious domain checks, `Readiness` scoring (`0–100%`), and extra index detection (DOAJ, Scopus, Copernicus, Google Scholar, Crossref).
- **Smart Cache Hit, Author Profile & 1-Click Submission Bundle (`src/engine/search.rs`, `src/exporter/bundle.rs`, `src/ui/settings_page.rs`)**:
  - Reused fresh incomplete/cached metadata during Phase 2 deep crawl, added persistent **Profil Penulis** (`name`, `affiliation`, `email`, `orcid`, `study_program`), originality statement generator, and 1-click `.zip` submission bundle export.

---

## [4.6.0] - 2026-11-15

### Added
- **Multi-Topic Manuscript Chips (`src/models.rs`, `src/engine/manuscript.rs`, `src/parser/doc_matcher.rs`, `src/ui/search_page.rs`)**:
  - Added `recommended_topics: Vec<String>` extracted via `extract_recommended_topics` (and helper `push_topic_chip`) into clickable short 1–2 word topic chips alongside a **Gunakan Semua Topik / Use All Topics** action.
- **Submission Status (CFP), Follow-Up Editor & Graduation Target Lead-Time (`src/parser/heuristics.rs`, `src/scraper/ojs_deep.rs`, `src/exporter/templates.rs`, `src/ui/detail_modal.rs`)**:
  - Added `submission_open: Option<bool>` detected via `extract_submission_open` inside `ojs_deep::absorb_page`, surfaced as `Open Submit / CFP` or `Registrasi Tutup` badges and rows in `detail_modal.rs`.
  - Added `generate_followup_email` in `templates.rs` with one-click **Follow-Up (ID)** and **Follow-Up (EN)** copy actions in the Submission Tracker, plus **Saran Target Wisuda** lead-time guidance.

### Improved
- **Anti-Overflow Home Header & Dynamic Table Width (`src/ui/search_page.rs`)**:
  - Constrained KPI strip (`render_kpi_strip`) and card widths to `ui.available_width()`, switched Baris 1 to flexible `right_to_left` layout, moved **Preset** and saved **Profil** controls into compact dropdowns (`ComboBox`), and sized the primary table column dynamically (`col0_w = (table_w - fixed_w).max(220.0)`) so all 6 table columns stay visible without right-edge clipping.

---

## [4.5.0] - 2026-11-15

### Added
- **Author Guidelines, Pre-Flight Fit Check & Cite-to-Target (`src/parser/heuristics.rs`, `src/parser/doc_matcher.rs`, `src/scraper/ojs_deep.rs`, `src/models.rs`)**:
  - Extracted journal citation style (`APA`, `IEEE`, `Chicago`, `Vancouver`, `Harvard`, `MLA`) and manuscript word-count limits (`word_limit`) directly from crawled OJS pages without extra HTTP requests.
  - Added `check_manuscript_word_fit` (`FitStatus`) to validate uploaded manuscript length against each journal's requirements, plus Cite-to-Target matching against bibliography references.
- **Auto-Fill Submission Templates (`src/exporter/templates.rs`, `src/ui/detail_modal.rs`)**:
  - Added one-click generators for formal **Cover Letters**, **Pre-Submission Inquiry Emails**, and supervisor briefs (**Dosbing Brief**) auto-filled from loaded manuscript metadata.
- **Side-by-Side Shortlist Compare Modal (`src/ui/compare_modal.rs`, `src/ui/search_page.rs`)**:
  - Added an interactive comparison modal (`2..=4` ★ shortlisted journals) comparing accreditation tier, APC, match score, turnaround speed, activity year, citation style, word limit, and submission fit side-by-side.

---

## [4.4.2] - 2026-11-15

### Added
- **Unified Single Currency Display IDR / USD (`src/ui/i18n.rs`, `src/ui/settings_page.rs`, `src/parser/apc.rs`, `src/ui/search_page.rs`)**:
  - Added `DisplayCurrency { Id/Idr, Usd }` in `AppSettings` (defaulting to IDR) with live switching in **Settings** and unified IDR/USD conversion across the results table, detail modal, and exports.

### Improved
- **Clean Toolbar & Fixed-Width Table Columns (`src/ui/search_page.rs`, `src/ui/mod.rs`)**:
  - Hid verbose technical log bar clutter from the main search workspace and stabilized table column sizing.

---

## [4.4.1] - 2026-11-15

### Improved
- **Table Row Height, APC Badge Labels & Toast Clarity (`src/ui/search_page.rs`, `src/ui/mod.rs`)**:
  - Raised `TableBuilder` row height and vertical cell spacing so two-line journal/schedule text never clips at the bottom, shortened the free Diamond OA label to `"Gratis (Diamond $0)"`, simplified the top preset strip, and refined toast notifications.
  - Enhanced smart manuscript keyword extraction for cleaner initial search topic suggestions.

---

## [4.4.0] - 2026-11-15

### Added
- **Research Productivity Suite (Zero-Bloat / On-Demand)**:
  - **Turnaround & Risk Heuristics (`src/parser/heuristics.rs`, `src/scraper/ojs_deep.rs`)**: Added review turnaround estimation (`turnaround_weeks` / `turnaround_label`), article language & template URL detection, and predatory/hijacked journal risk evaluation (`evaluate_journal_risk`).
  - **Zotero & Mendeley Citation Exporter (`src/exporter/citation.rs`, `src/ui/history_page.rs`)**: Added standard RIS (`.ris`) and BibTeX (`.bib`) export (`format_ris` and BibTeX formatter).
  - **Submission Tracker, Search Profiles & Cache Offline Browser (`src/cache.rs`, `src/ui/state.rs`, `src/ui/detail_modal.rs`, `src/ui/history_page.rs`)**: Added local submission status tracking (`Belum Dikirim` -> `Submitted` -> `Review` -> `Accepted`), reusable search + filter profiles, and instant offline browsing of cached journals (`search_cached_journals`).

---

## [4.3.0] - 2026-11-15

### Added
- **Official International Q1–Q4 Discovery via OpenAlex & DOAJ (`src/scraper/international.rs`, `src/models.rs`, `src/engine/search.rs`)**:
  - Added open REST integrations for **OpenAlex Sources API** (with polite-pool `User-Agent`) and **DOAJ Journals API** with zero Cloudflare blocking.
  - Added `quartile` (`Q1`–`Q4` mapped from `2yr_mean_citedness`, `h_index`, and DOAJ status), `is_international`, Diamond OA detection (`Gratis (Diamond OA • $0)`), configurable USD-to-IDR conversion (`$150 (~Rp 2,5 jt)`), and color-coded `Q1`–`Q4` badges in the results table, detail modal, and `.xlsx`/`.md`/`.csv` exports.
- **Settings Page & Bilingual ID/EN UI (`src/ui/i18n.rs`, `src/ui/settings_page.rs`, `src/ui/sidebar.rs`, `src/ui/filters_page.rs`)**:
  - Added `AppLanguage { Id, En }` live UI localization across sidebar navigation, page headings, search toolbar, filters, history/export controls, and notifications.
  - Added **Pengaturan / Settings** sidebar view with instant language & theme toggles, default portal toggles (Garuda, SINTA, OpenAlex, DOAJ), custom USD-to-IDR exchange rate (default `16.500`), and automatic persistence in `.cache/app_settings.json`.
  - Added `Q1`–`Q4` quartile filter pills and the `Portal Internasional (OpenAlex & DOAJ)` toggle in `Filter Lanjutan`.

### Improved
- **Low-End Hardware & Portable Distribution**:
  - Deep-crawl concurrency now adapts to the CPU (`14` workers on <= 4 logical CPUs, `32` otherwise) and the SQLite page cache is capped at ~4 MB (`PRAGMA cache_size = -4000`).
  - Release binary is linked with static CRT (`+crt-static`, no `VCRUNTIME140.dll`), embeds an `asInvoker` manifest with Windows 10/11 compatibility, falls back to `%LOCALAPPDATA%\JSFS` when the executable folder is read-only, and ships with `SHA256SUMS.txt`.
  - Rewrote `README.md` as a concise quick-start guide for first-time users.
- **History & Export Layout Redesign (`src/ui/history_page.rs`)**:
  - Split targeted export controls into 3 clearly labeled vertical sub-sections (`1. Pilih Data...`, `2. Simpan & Buka File Laporan`, `3. Salin Cepat ke Clipboard`), reduced card spacing/padding (`10px` gap / `12px` padding), compacted recent Excel path display with hover tooltip, made the Activity Log card collapsible, and added bottom scroll clearance so no controls clip at window edges.

---

## [4.2.0] - 2026-11-15

### Added
- **Search & Filter Session History (`src/ui/state.rs`, `src/ui/history_page.rs`, `src/ui/mod.rs`)**:
  - Added `SearchHistoryEntry` auto-persisted to `.cache/search_history.json` (up to 15 most recent sessions) capturing keyword, timestamp, filter summary, `FilterSettings` snapshot, total/free counts, and result records.
  - Added **"Riwayat Pencarian & Filter"** card in `Riwayat & Ekspor` with **`Muat Hasil`** (restores results directly without re-scraping) and **`Terapkan Filter`** (restores historical filter settings).
- **Targeted Export & Bulk Editorial Email Copy (`src/ui/history_page.rs`, `src/engine/file_ops.rs`, `src/exporter/mod.rs`, `src/models.rs`)**:
  - Added `ExportScope` selector (`Semua Hasil`, `Hanya ★ Pilihan`, `Hanya Gratis (Rp 0)`, `Sesuai Filter Aktif di Tabel`) with direct **`Ekspor Excel (.xlsx)`**, **`Ekspor Markdown (.md)`**, **`Ekspor CSV (.csv)`**, and **`Salin Email Redaksi`** actions.

### Improved
- **Spacious Results Table & Fixed `Aksi` Column (`src/ui/search_page.rs`)**:
  - Adjusted `TableBuilder` columns (`Column::remainder().at_least(280.0)`, `SINTA 84`, `Match 68`, `Biaya APC 132`, `Keaktifan & Terbit 168`, `Aksi 154`) and compact button padding so `☆`, `Detail`, and `Web` render unclipped.
  - Added auto-compact 1-line progress summary once searches finish and raised table scroll bounds (`min_scrolled_height(460.0)`, `max_scroll_height` up to `780.0`) so more journal rows fit on screen.

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
  - Removed all leftover legacy prototype theme/brand references and standardized theme names to **`JSFS Dark`** and **`JSFS Daylight`**.
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
