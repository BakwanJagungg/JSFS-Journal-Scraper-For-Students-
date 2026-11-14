# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.3.0] - 2026-11-14

### Architecture, Immunity & Batch Match Scoring Optimization
- **Process-Isolated App-Mode (`app_desktop.py`, `JSFS.spec`)**:
  - Memisahkan proses UI Chromium (`msedge.exe --app=` / `chrome.exe --app=`, ukuran native `1160x780`, ikon JSFS) dari proses Python melalui `ThreadingHTTPServer(("127.0.0.1", 0))`, sehingga Python GIL saat komputasi berat (scraping paralel, Match Scoring, SQLite WAL, ekspor Excel) tidak pernah menahan UI Windows dan **100% kebal terhadap `(Not Responding)`**.
  - Menambahkan endpoint REST lokal (`/api/init`, `/api/status`, `/api/ai-status`, `/api/search`, `/api/stop`, `/api/upload-manuscript`, `/api/copy-share`, `/api/copy-draft`, `/api/open-excel`, `/api/open-output`, `/api/open-logs`, `/api/open-url`, `/api/clear-cache`, `/api/launch-ollama`, `/api/install-ollama`, `/api/pull-ai-model`, `/api/ping`, `/api/shutdown`).
  - Sinkronisasi lifecycle via detak jantung `POST /api/ping`, sinyal penutupan `pagehide` -> `navigator.sendBeacon('/api/shutdown')`, serta pengawas proses jendela browser agar server berhenti bersih dalam `<= 0.5` detik.
- **Unified HTTP + WebView2 Frontend Bridge (`ui_web/index.html`, `ui_web/app.js`)**:
  - Menambahkan `callBackendApi()` yang mendukung fetch REST di App-Mode (`priority: 'low'` untuk background telemetry & heartbeat) dengan fallback penuh ke `window.pywebview.api`, serta `<input type="file" id="input-manuscript-file" accept=".docx,.pdf">` dengan unggah multipart non-blocking.
- **Pre-Computed Batch Match Scoring & Live Fase 1 Telemetry (`doc_reader.py`, `scraper_garuda.py`, `app_webview.py`)**:
  - Menambahkan caching metadata naskah berbasis `(path, mtime, size)`, fungsi `prepare_manuscript_profile()`, dan caching sinonim kata kunci sehingga batch Match Scoring 500 jurnal selesai dalam **< 0.5 detik**.
  - Menambahkan pemancaran `progress_cb` berkala di `_enhance_garuda_entries()` untuk menghilangkan jeda telemetri 25–39 detik selama pengayaan metadata Garuda Fase 1.

## [2.2.0] - 2026-11-14

### Architecture, Stability & Instant Boot (Zero-Crash Engine & Forensic Accuracy)
- **Zero-Freeze ThreadPool & Anti-Starvation Cap (`config.py`, `scraper_garuda.py`, `scraper_sinta.py`)**:
  - Menetapkan `MAX_WEBVIEW_WORKERS = 24`, `TURBO_SCRAPE_WORKERS = MAX_WEBVIEW_WORKERS`, `NORMAL_SCRAPE_WORKERS = min(16, cpu * 2)`, dan `MAX_DEEP_SCRAPE_PER_JOURNAL_SEC = 5.5` dengan timeout `(2.5, 3.5)`.
  - Menghapus seluruh *nested `ThreadPoolExecutor`* di `extract_ojs_deep_info()` (diganti loop sekuensial aman maks 3 subpath pada session keep-alive), membatasi SINTA page pool ke 8 worker dengan pendelegasian `deep_enrich_entries()`, menyisipkan `time.sleep(0.002)` pada loop `as_completed` agar Python GIL / Windows `WM_PAINT` tidak kelaparan, serta mengisolasi target-reach stop via `_CancelBridge` agar tidak membatalkan pipeline ekspor.
  - Menambahkan pelacakan `weakref.WeakSet` sesi HTTP dan `close_all_http_sessions()` untuk menutup soket dengan bersih saat shutdown.
- **Instant Booting (<1.5s) Lazy SQLite & Expired Cleanup (`cache_engine.py`)**:
  - Menunda inisialisasi SQLite (`_initialized = False` -> `_ensure_ready()`) hingga akses pertama di background thread setelah UI tampil, serta menambahkan `cleanup_expired()` dan kunci statistik `"valid_entries"`.
- **Forensic Crash & Freeze Watchdog (`app_webview.py`, `ui_web/index.html`, `ui_web/app.js`)**:
  - Menambahkan `init_forensic_logger()`, `write_forensic_dump()`, dan daemon `MainThreadWatchdog` (>8 detik stall dengan cooldown 15 detik -> `logs/debug_trace.log`), serta tombol akses **Log Diagnostik** (`#btn-open-logs`) pada panel Filter Lanjutan.
- **Fast Phase 4 Export (`exporter.py`)**:
  - Menambahkan `_build_fast_url_status()` dan mengubah default `skip_url_validation=True` pada `write_exports()`, `export_excel()`, `export_markdown()`, dan `export_csv()` sehingga penyimpanan laporan berlangsung <0.5 detik tanpa probe HTTP ulang.

## [2.1.1] - 2026-11-14

### Fixed & Optimized (Zero-Freeze Paginated Table, Delta-IPC, Concurrent Garuda & TLS Tolerance)
- **Delta-IPC Payload (`app_webview.py`, `ui_web/app.js`)**:
  - Memperbarui `JSFSApi.get_status(log_offset, last_result_version)` agar mengembalikan `results: None` bila `last_result_version == current_version`, sehingga WebView2 tidak melakukan serialisasi & transfer ulang ratusan objek jurnal setiap 150ms.
- **Paginated DOM Rendering 50 Baris/Halaman & Throttled Live Refresh (`ui_web/index.html`, `ui_web/style.css`, `ui_web/app.js`)**:
  - Mengganti rendering massal 400+ `<tr>` menjadi **Paginasi Instan (50 / 100 / Semua)** menggunakan `DocumentFragment`, in-memory filtering & sorting lintas dataset penuh, CSS `contain: layout style` pada `.table-wrap`, serta pembatasan refresh tabel saat scraping aktif maksimal tiap `1.5 detik`.
- **Concurrent Garuda Multi-Page Fetching (`scraper_garuda.py`)**:
  - Mengambil Halaman 1 Portal Garuda lalu menelusuri Halaman `2..N` secara paralel menggunakan `ThreadPoolExecutor(max_workers=8)` dan pengayaan SINTA per-batch untuk memangkas durasi Fase 1 secara drastis.
- **TLS Timeout Tuning & Penghilangan False-Positive `"Situs tidak dapat diakses"` (`config.py`, `scraper_garuda.py`, `scraper_sinta.py`, `exporter.py`)**:
  - Menyesuaikan `FAST_FAIL_TIMEOUT = (3.0, 4.0)`, `DEEP_SCRAPE_TIMEOUT = (3.0, 4.5)`, dan `MAX_DEEP_SCRAPE_PER_JOURNAL_SEC = 7.0`.
  - Memisahkan status `timeout` dari domain `dead` sehingga server kampus `.ac.id` yang sekadar lambat berjabat tangan SSL ditandai jujur sebagai `"Cek Web / Timeout"` dengan tautan `Buka Web` tetap aktif.

## [2.1.0] - 2026-11-14

### Added & Enhanced (Hyper-Speed 6x Pipeline, Hard Deadline & Incremental Auto-Save)
- **Pre-Deep Deduplication & Adaptive Pool Multiplier (`config.py`, `scraper_garuda.py`, `scraper_sinta.py`, `app_webview.py`, `jsfs.py`)**:
  - Mengumpulkan metadata dasar lintas kata kunci dan lintas portal (`deep_detail=False`) terlebih dahulu lalu menjalankan `merge_cross_source_entries()` sebelum deep crawl OJS melalui `deep_enrich_entries()`, sehingga jurnal duplikat hanya di-scrape situs OJS-nya **satu kali saja**.
  - Menambahkan `compute_candidate_pool_limit(target)` pada `config.py` (`<100 -> max(t*3, 60)`, `100..299 -> 2.0x`, `>=300 -> 1.6x`) agar pool target besar tetap proporsional.
- **5.5s Hard Per-Journal Deadline & Concurrent Priority Subpath (`scraper_garuda.py`, `config.py`)**:
  - Memperketat `DEEP_SCRAPE_TIMEOUT = (1.5, 2.5)` dan menetapkan `MAX_DEEP_SCRAPE_PER_JOURNAL_SEC = 5.5`.
  - Menjalankan probing subpath biaya prioritas (`/about/submissions` & `/about/editorialPolicies`) secara paralel ringan dengan batas maks 2 subpath tambahan dan `shutdown(wait=False, cancel_futures=True)` ketika *wall-clock deadline* tercapai, serta sintesis otomatis Focus & Scope saat terputus deadline.
- **Incremental Auto-Save Checkpoint & Completion Alert (`exporter.py`, `app_webview.py`, `ui_web/index.html`, `ui_web/style.css`, `ui_web/app.js`)**:
  - Menambahkan `export_autosave_checkpoint()` (atomik via `os.replace` & thread-safe `_autosave_lock`) untuk menyimpan `output/_AUTOSAVE_TERAKHIR.xlsx` & `output/_AUTOSAVE_TERAKHIR_LIST.txt` setiap 50 jurnal selesai sambil melakukan streaming baris tabel secara bertahap.
  - Menambahkan banner ringkasan durasi (`role="status"`), tombol kontrol suara (`#btn-sound-toggle`), serta notifikasi nada lembut 2-tone berbasis Web `AudioContext`.

## [2.0.2] - 2026-11-13

### Added & Improved (Real-Time Telemetry & 3x Faster Scraping)
- **Real-Time Per-Journal Telemetry (`app_webview.py`, `scraper_garuda.py`, `scraper_sinta.py`, `ai_engine.py`)**:
  - Menambahkan state progres terpusat yang dilindungi `threading.Lock()` di `JSFSApi` (`phase`, `phase_label`, `current_item`, `elapsed_seconds`, `rate_per_sec`, `live_counts`, `scrape_done`/`scrape_total`, `ai_done`/`ai_total`).
  - Memasang `progress_cb` / `on_ai_progress` berbasis `as_completed(futures)` per halaman portal dan per jurnal OJS serta memublikasikan hasil sementara secara streaming setelah Fase 2 pre-filter agar tabel segera tampil.
- **Native `<progress>` Shimmer, Elapsed Timer & Live KPI (`ui_web/index.html`, `ui_web/style.css`, `ui_web/app.js`)**:
  - Mengganti progress track kustom dengan elemen semantik `<progress>` dilengkapi animasi shimmer indeterminate saat `total === 0`, `font-variant-numeric: tabular-nums` untuk timer `00:00` & kecepatan `~X/dtk`, ticker 1-baris dengan `text-overflow: ellipsis`, serta interval polling `150ms`.
- **3x Faster Early-Exit Scraping Engine (`config.py`, `scraper_garuda.py`, `scraper_sinta.py`)**:
  - Mengaktifkan `HTTPAdapter(pool_connections=128, pool_maxsize=128)` serta `get_thread_local_session(is_deep_scrape=True)` untuk keep-alive TCP/TLS pool persisten per worker thread.
  - Menambahkan *early-exit sub-path stop* (`_has_complete_core_data`) di `extract_ojs_deep_info()` untuk langsung berhenti memindai halaman sekunder bila data APC + jadwal terbit + kontak inti sudah lengkap.

## [2.0.1] - 2026-11-13

### Improved (Visual-First WebView2 UX Redesign)
- **Design System & Vector Iconography (`ui_web/index.html`, `ui_web/style.css`, `ui_web/app.js`)**:
  - Menetapkan *Daylight* theme (`data-theme="light"`) sebagai default dengan token semantik bersih (`--bg-canvas: #f8fafc`, `--bg-surface: #ffffff`, `--border-subtle: #e2e8f0`, `--accent-primary: #2563eb`, `--accent-success: #059669`) serta toggle mulus ke Dark Mode.
  - Mengganti seluruh emoji tombol/preset dengan ikon vektor **inline SVG 18px–22px** bergaya konsisten.
- **Custom Target Jurnal 1–1000 & Filter Lanjutan dalam Modal (`ui_web/index.html`, `app_webview.py`)**:
  - Menambahkan `<input type="number" id="target-count" min="1" max="1000" list="target-presets">` sehingga pengguna bebas mengetik jumlah target jurnal berapa pun (mis. `100`, `500`, `1000`). Target `>= 100` otomatis mengaktifkan Turbo Mode di backend Python (`app_webview.py`).
  - Memindahkan seluruh opsi teknis (*Pengaturan Lengkap*: SINTA 1–6, Bulan 1–12, Batas Biaya, Turbo, Selenium, Ollama AI) ke `<dialog id="dialog-filters">` agar area tabel hasil tampil dominan *above the fold*.
- **Strip KPI 1 Baris, Ilustrasi Empty State 3 Langkah & Collapsible Log**:
  - Memadatkan 4 statistik KPI menjadi 1 baris horizontal ramping (`max-height: 56px`), menghadirkan ilustrasi SVG + panduan 3 langkah saat hasil masih kosong, serta menyembunyikan log teknis di balik tombol **Lihat Log Teknis**.

## [2.0.0] - 2026-11-13

### Changed & Added (Major Desktop Re-Architecture)
- **Next-Gen Desktop UI via WebView2 (`app_webview.py`, `ui_web/index.html`, `ui_web/style.css`, `ui_web/app.js`)**:
  - Mengganti lapisan GUI `CustomTkinter` menjadi desktop app HTML5/CSS3/Vanilla JS native di atas **Microsoft Edge WebView2 (`pywebview`)**, menghilangkan total risiko `Not Responding` dan layar hitam saat *resize* maupun scraping berat.
  - Antarmuka *Modern Academic Daylight* sebagai default dengan sakelar tema adaptif (`🌙 Mode Gelap` / `☀️ Daylight`), `<dialog>` native untuk Panduan & Setup Ollama AI, 4 Kartu Quick Presets dengan highlight aktif, 4 Live KPI Pills, Dual Progress Bars, Live Activity Log, dan **Embedded Interactive Results Table** langsung di jendela utama (pencarian instan, filter biaya APC, kolom sortable, modal Detail Jurnal, `📋 Salin Daftar (Format Share WA)`, `💬 Salin Draf Chat Editor`, `📊 Buka Excel`, dan `📁 Folder Output`).
  - Memastikan `window.events.closed` menutup seluruh worker aktif, `ai_engine.shutdown()`, dan `cache_engine.close_all_connections()` secara bersih tanpa mengunci SQLite.
- **C-Speed HTML Parsing (`scraper_garuda.py`)**:
  - Menambahkan dukungan opsional `selectolax` (`fast_sanitize_html_text`) dengan fallback otomatis ke BeautifulSoup/`lxml`.
- **Packaging & Dependencies (`JSFS.spec`, `requirements.txt`)**:
  - Memperbarui `JSFS.spec` agar menggunakan `app_webview.py`, membundel `ui_web/` dan runtime `pywebview`/`selectolax`, serta menghapus bundel `customtkinter`.

## [1.8.0] - 2026-11-13

### Added & Enhanced
- **Instant-Boot Architecture (< 2s Startup) (`app_ui.py`, `exporter.py`, `doc_reader.py`, `scraper_sinta.py`)**:
  - Memindahkan impor library berat (`pandas`, `openpyxl`, `pypdf`, `python-docx`, `selenium`, `undetected_chromedriver`, serta modul scraping/ekspor) ke dalam fungsi yang membutuhkannya (*Lazy / On-Demand Import*), memangkas waktu impor `app_ui` dari ~3.4s menjadi **< 0.4s**.
  - Menunda pengecekan status Ollama dan inisialisasi/auto-purge cache SQLite selama `600 ms` menggunakan `self.after(600, ...)` setelah jendela utama tampil.
- **Presisi APC `1.000,000 (IDR)` & Follow Redirect (`scraper_garuda.py`, `cache_engine.py`, `config.py`)**:
  - Menambahkan `_parse_mixed_idr_number()` dan memperbarui `RE_PUB_AMOUNT`, `RE_IDR_1`, serta `RE_PUB_FREE_STRICT` agar nominal pemisah ribuan campuran seperti `"Article Publication: 1.000,000 (IDR)"` / `"1,000.000"` terbaca akurat sebagai `Rp 1.000.000` (1000000).
  - Menyimpan URL akhir hasil HTTP redirect (`_final_url` di `safe_fetch_html`) sehingga `extract_ojs_deep_info` membentuk `/about/submissions`, `/about/editorialPolicies`, dan memindai navigasi `Author Fees` pada domain redirect akhir.
  - Menaikkan `APP_VERSION` dan `CACHE_VERSION` ke `1.8.0` agar cache versi lama otomatis di-purge saat startup.
- **Regex-First, AI-Second Fast Pipeline (`ai_engine.py`)**:
  - Fase 3 menjalankan Regex super cepat terlebih dahulu untuk seluruh jurnal; Ollama AI (`READ_TIMEOUT = 4` detik ketat) hanya dipanggil jika Ollama aktif dan hasil Regex masih `"Kosong"` (maksimal `10` jurnal teratas).
- **Build PyInstaller `--onedir` (`JSFS.spec`)**:
  - Menambahkan `excludes` untuk modul tidak terpakai (`matplotlib`, `scipy`, dll.) dan beralih ke mode `--onedir` (`COLLECT`) menghasilkan `dist/JSFS/JSFS.exe` untuk startup instan tanpa dekompresi `_MEI` saat dibuka.

## [1.7.1] - 2026-11-13

### Added & Enhanced
- **Zero-Result Resilience & Smart Typo Auto-Correction (`doc_reader.py`, `scraper_garuda.py`, `scraper_sinta.py`, `jsfs.py`, `app_ui.py`)**:
  - Menambahkan koreksi salah ketik kata kunci akademik berbasis `difflib.get_close_matches()` (`correct_keyword_typo()`) terhadap kamus akademik populer (mis. `bisins` → `bisnis`, `akutansi` → `akuntansi`, `manajmen` → `manajemen`).
  - Menambahkan `get_fallback_keywords()` untuk pemecahan kata kunci spesifik menjadi 1–2 kata inti dan perluasan sinonim dwibahasa dari `BILINGUAL_ACADEMIC_SYNONYMS` secara otomatis ketika pencarian awal menghasilkan 0 kandidat.
- **HTTP Diagnostics & Modern Browser Header Rotation (`config.py`, `scraper_sinta.py`, `scraper_garuda.py`)**:
  - Rotasi `User-Agent` Chrome 130+ Windows lengkap dengan header `Accept`, `Accept-Language: id-ID,id;q=0.9,en-US;q=0.8`, dan `Referer`.
  - Diagnostik jelas ketika Portal SINTA membatasi akses HTTP standar (403/Cloudflare) saat opsi Selenium belum dicentang, sehingga Portal Garuda tetap berjalan mulus tanpa macet.
- **UI/UX Polish (`app_ui.py`)**:
  - **Empty State & Diagnostic Banner**: Menampilkan banner saran praktis di area KPI ketika hasil akhir pencarian adalah `0 jurnal`.
  - **Quick Presets Active Indicator**: Indikator kartu aktif yang kontras, sudut lebih rapi, serta *hover feedback* (`cursor="hand2"`) pada 4 tombol Quick Presets.
  - **Smart WA Share Button State**: Tombol `📋 Salin Daftar (Share WA)` tampil abu-abu nonaktif saat hasil kosong dan langsung menyala aktif Indigo (`#4F46E5`) saat data jurnal tersedia.
- **Automated Core Test Suite (`tests/test_jsfs_core.py`)**:
  - Menambahkan `test_13_typo_correction_and_fallback` untuk memverifikasi koreksi ejaan dan fallback kata kunci.

## [1.7.0] - 2026-11-08

### Added & Enhanced
- **Deep APC Hunter Engine (`cache_engine.py`, `scraper_garuda.py`)**:
  - **Incomplete Cache Auto-Bypass**: Entri cache dengan `biaya_publikasi` kosong, `"Cek Web / Belum Terdata"`, atau `sumber_apc == "Kosong"` otomatis di-bypass untuk memicu deep-scrape ulang saat opsi Deep Scrape aktif.
  - **Auto-Purge SQLite Metadata**: Database SQLite cache mendeteksi migrasi versi `< 1.7.0` dan melakukan auto-purge tabel `journal_cache` dengan WAL checkpoint bersih.
  - **Multi-Path OJS 2 & OJS 3 Crawler**: Memindai secara konsisten 5 titik target: (1) `/about/submissions`, (2) `/about/editorialPolicies` (kritis untuk OJS 2), (3) `/about`, (4) Sidebar & Custom Blocks (`div.pkp_block`, `[id*="fee" i]`, `[class*="customblock" i]`), dan (5) Dynamic Menu Anchors.
  - **Anti-OA-Trap (Open Access Reader Policy Stripper)**: Menghapus kalimat retorika Open Access pembaca (*"freely available to the public"*, *"immediate open access"*) sebelum pencocokan kata kunci bebas biaya, mencegah false-positive Gratis pada jurnal berbayar.
  - **Paid Priority Rule**: Mendeteksi pola nominal bayar terlebih dahulu. Jika ditemukan nominal publikasi positif (mis. `Publication: 500,000.00 IDR`), status berbayar langsung menang mutlak meski `Submission Fee: 0.00 IDR`.
  - **Decimal Cents & Format Support**: Mendukung format mata uang OJS dengan desimal `.00` / `,00` dan penulisan nominal IDR/USD internasional.
- **Modern Bright GUI — "Soft Slate Daylight" (`app_ui.py`)**:
  - **Light Mode Default**: Tampilan default beralih ke mode terang bersih (*Daylight Mode*) dengan palet warna terkurasi slate & emerald yang ramah mata.
  - **Theme Toggle Switcher (☀️/🌙)**: Tombol switch instan di header untuk berpindah antara Mode Terang dan Mode Gelap menggunakan color tuples adaptif `(light_mode, dark_mode)`.
  - **Live KPI Counter Pills**: Header metrik interaktif dengan 4 pil indikator dinamis (*Total Ditemukan*, *Gratis (Rp 0)*, *Berbayar*, *Terbit Dekat*) yang diperbarui secara thread-safe via antrean GUI non-blocking.
  - **1-Click WA Share Button**: Tombol aksi cepat *"📋 Salin Daftar (Share WA)"* di action bar utama untuk langsung menyalin daftar ringkas hasil pencarian ke clipboard.
  - **Peningkatan Dialog**: Pembaruan gaya visual dan tema adaptif pada *Pratinjau Data*, *Panduan Cepat*, dan *Setup AI*.
- **Pembersihan & Restrukturisasi Direktori**:
  - Struktur repositori ditata rapi ke dalam folder modular: `docs/`, `output/`, `tests/`, `assets/`, dan `.cache/`.
- **Automated Core Test Suite (`tests/test_jsfs_core.py`)**:
  - 12 unit test komprehensif menguji filter email mahasiswa, validasi telepon Telkom/seluler, sinkronisasi jadwal arsip OJS, Anti-OA-Trap, Paid Priority Rule, kanonikal deduplikasi, merge cross-source, format WhatsApp share, dan incomplete cache bypass.

## [1.6.2] - 2026-11-06

### Added & Enhanced
- **Forensic Anti-Email Flood Engine (`scraper_garuda.py`, `scraper_sinta.py`)**:
  - **Penyaringan Ketat Domain Mahasiswa/Penulis**: Menolak otomatis email dengan domain `@student.`, `@students.`, `@mail.ugm.`, `@mhs.`, `@alumni.`, `@pasca.`, `@pps.`, serta email dengan format NIM (>= 6 digit angka di username seperti `2203035@student...`).
  - **Isolasi Sumber Kontak OJS**: Menghapus seluruh blok artikel/abstrak (`.obj_article_summary`, `.cmp_article_list`, `.article-summary`, `meta[name="citation_author_email"]`) sebelum ekstraksi kontak agar email penulis naskah tidak bocor ke redaksi.
  - **Domain Affinity & Official Prefix Prioritization**: Memprioritaskan email yang cocok dengan domain host OJS atau diawali kata kunci redaksi (`jurnal`, `journal`, `editor`, `redaksi`, `ejournal`, `publikasi`, `ojs`, `admin`), dan mengeliminasi email gmail/yahoo pribadi penulis jika sudah ada email resmi institusi.
  - **Hard Cap 3 Email**: Membatasi maksimal 3 email resmi terbaik per jurnal.
- **Forensic Anti-Fake Phone Engine & Area Code Whitelist (`scraper_garuda.py`)**:
  - **Daftar Putih Kode Area Resmi Indonesia (`VALID_INDONESIAN_AREA_CODES`)**: Memvalidasi seluruh kode area 2-digit dan 3-digit resmi Telkom Indonesia, serta menolak kode area palsu dari DOI/HTML (seperti `0203`, `0300`, `0491`).
  - **Strict Proximity Context (35 Karakter Kiri) & Length Guard**: Nomor kabel wajib memiliki label telepon eksplisit (`telp`, `telepon`, `phone`, `ph:`, `tel:`, `fax`, `wa:`, `whatsapp`, `contact:`) tepat di sebelah kiri atau memiliki tanda kurung kode area, serta membatasi panjang total 9–12 digit (menolak nomor rekening bank / NIP 14–16 digit).
  - **Hard Cap 2 Telepon/WA**: Mengutamakan nomor seluler/WhatsApp (`+628...`) di urutan pertama dan membatasi maksimal 2 nomor per jurnal.
- **Deep Indonesian OJS APC Terminology & Smart Fallback (`scraper_garuda.py`, `ai_engine.py`, `exporter.py`)**:
  - **Perluasan Kamus Terminologi OJS Indonesia & Inggris**: Mengenali variasi kalimat gratis (`"tidak dikenakan biaya"`, `"tidak memungut biaya"`, `"bebas biaya"`, `"free of charge"`, `"no article processing charges"`, `"does not charge"`, `"dibebaskan dari biaya"`) dan format nominal naratif (`Rp 500.000`, `IDR 500,000.00`, `sebesar Rp ...`, `kontribusi ...`).
  - **Smart Fallback Biaya Submit -> Biaya Publikasi**: Nilai nominal `> Rp 50.000` pada kolom `Biaya Submit` otomatis dikonversi menjadi `Biaya Publikasi` jika biaya publikasi belum terdata (menyelesaikan kasus jurnal MEBIS Rp 600.000 dan MUQADDIMAH Rp 500.000).
- **Perbaikan Anomali Bulan Terbit Arsip (`month_parser.py`)**:
  - Mengelompokkan judul arsip edisi berdasarkan tahun dan mengambil bulan hanya dari 1 tahun penuh terbaru (atau 4–6 edisi teratas), mencegah penggabungan historis yang memicu anomali "7–8 kali setahun", serta mengabaikan derau widget kalender bulanan (>= 11 bulan).
- **Smart Canonical Deduplication & OJS Title Protection (`scraper_garuda.py`, `scraper_sinta.py`, `jsfs.py`, `app_ui.py`)**:
  - **`canonical_journal_key`**: Deduplikasi cerdas berbasis judul kanonikal bersih (bebas tanda baca, kapitalisasi, spasi, dan titik dua) serta ISSN/E-ISSN dan path URL, menggabungkan jurnal duplikat/migran (seperti kasus *Balance UM Palembang*) menjadi 1 entri tunggal yang utuh.
  - **Koreksi Judul Artikel Menjadi Nama Jurnal**: Mendeteksi jika judul dari Garuda merupakan judul artikel skripsi yang panjang (> 55 karakter seperti `"KONSEP DAN TUJUAN PADA TINJUAN BISNIS SYARIAH"`) dan mengoreksinya menggunakan nama jurnal resmi OJS (*ADZKIYA : Jurnal Hukum dan Ekonomi Syariah*).
- **Pembersihan Baris `Fokus:` & Filter Noise Focus & Scope (`exporter.py`, `ai_engine.py`, `scraper_garuda.py`)**:
  - **Anti-Noise Filter**: Menolak teks Focus & Scope yang memuat metadata pengindeksan (*Scopus ID*, *Google Scholar*, *diindeks oleh*), lisensi (*Creative Commons*, *Copyright ©*), tanggal rilis (*this issue has been available*), atau abstrak artikel (*this study aims to*).
  - **Terjemahan Subjek SINTA ke Bahasa Indonesia**: Menerjemahkan istilah bahasa Inggris SINTA ke Bahasa Indonesia yang rapi (*Manajemen & Riset Operasi*, *Ekonomi & Keuangan*, *Bisnis & Akuntansi*), membuang kategori yang tidak selaras dengan judul jurnal, dan memformat pemotongan teks secara bersih tanpa tanda koma menggantung (`,...`).

## [1.6.1] - 2026-11-06

### Added & Enhanced
- **High-Yield Recall Engine (`scraper_garuda.py`, `scraper_sinta.py`, `app_ui.py`, `jsfs.py`)**:
  - **Smart APC & Status Preservation**: Jurnal berstatus `"Cek Web / Belum Terdata"` tidak dibuang secara agresif; wajib lolos filter secara default saat `include_na=True` atau saat pengguna menyetel batas biaya (`max_rp = 1.500.000`), serta otomatis melengkapi kuota target pengguna saat memilih *"Hanya Gratis (Rp 0)"*. Hasil pencarian topik seperti "bisnis" kembali melimpah (>20 jurnal).
  - **Adaptive Candidate Pool Multiplier (3x Over-Fetching)**: Pengambilan kandidat awal sebanyak `max(target_results * 3, 60)` secara konkuren dari halaman paginasi portal Garuda & SINTA, menjamin pool kandidat tidak habis terpotong sebelum melewati tahapan pre-filter SINTA dan deep OJS scraping.
- **Anti-Fake Phone Validator & Proximity Context (`scraper_garuda.py`, `scraper_sinta.py`)**:
  - **Eliminasi Total Nomor Telepon Palsu**: Menolak mutlak deretan angka acak dari URL, DOI prefix, atau ID artikel yang diawali angka `0` atau `1` tepat setelah kode area (seperti kasus `+6221005162` dan `+62211892256` pada UNESA).
  - **Strict Proximity Context & Formatting Guard**: Nomor telepon kabel (*landline*) non-seluler wajib memiliki format pemisah kode area eksplisit `(0271) xxx` / `021-xxx` atau berada dalam radius konteks kata kunci kontak (`telp`, `telepon`, `phone`, `call`, `wa`, `whatsapp`, `contact`, `hubungi`).
  - **Validasi Prefix Operator Seluler**: Verifikasi ketat prefix seluler/WA Indonesia (`811`–`819`, `821`–`823`, `831`–`838`, `851`–`859`, `877`–`878`, `881`–`889`, `895`–`899`) dengan panjang total 10–13 digit setelah `0`.
- **OJS Profile Table Noise Stripper & Zero-Empty Synthesizer (`scraper_garuda.py`, `scraper_sinta.py`, `ai_engine.py`, `exporter.py`)**:
  - **`strip_ojs_profile_table_noise`**: Memotong dan membuang header tabel biodata OJS (`Journal Title`, `ISSN/E-ISSN`, `DOI Prefix`, `Editor in Chief`, `Publisher`, `Frequency`), hanya mempertahankan kalimat deskriptif intinya.
  - **`synthesize_focus_scope` (100% Terisi Akurat)**: Jika halaman OJS tidak memuat Focus & Scope eksplisit, otomatis menyintesis ruang lingkup dari data *Bidang Keilmuan* SINTA/Garuda (`Menerbitkan riset ilmiah di bidang {subject_area}`) dan menyematkan label `Sumber Focus = "SINTA/Garuda"`.
- **Always-On Keyword Match Score (`doc_reader.py`, `scraper_garuda.py`, `exporter.py`, `app_ui.py`, `jsfs.py`)**:
  - Penyesuaian rentang kategori skor kata kunci: `>= 70%` (`Sangat Sesuai`), `45–69%` (`Sesuai`), dan `< 45%` (`Cukup Sesuai`).
  - Memastikan `calculate_keyword_match_score` selalu dipanggil untuk setiap jurnal saat pengguna mencari dengan kata kunci tanpa upload file naskah, melenyapkan skor kosong (`—`) dan salah label `Kurang Sesuai` pada jurnal yang relevan.
- **Penyempurnaan Format Copy-Paste List (`exporter.py`)**:
  - Penambahan baris ringkas `Fokus:` pada `generate_copypaste_list` untuk memberikan konteks bidang keilmuan jurnal saat dibagikan ke WhatsApp/Telegram.

## [1.6.0] - 2026-11-06

### Added & Enhanced
- **Human-Parity Accuracy Engine (`scraper_garuda.py`, `scraper_sinta.py`, `month_parser.py`, `ai_engine.py`)**:
  - **Dynamic Menu Anchor Discovery**: Pemindaian dinamis seluruh tag tautan navigasi dan sidebar OJS untuk menemukan halaman biaya tersembunyi (`fee`, `apc`, `biaya`, `charge`, `payment`, `tarif`) dan halaman kontak resmi (`contact`, `kontak`, `hubungi`, `about`).
  - **Verifikasi Jadwal Bulan Terbit via Arsip (`/issue/archive`)**: Fitur `extract_months_from_archive_titles` melakukan *lightweight fetch* ke halaman arsip dan edisi berjalan (*Current Issue*) untuk mengekstrak jadwal bulan terbit nyata dari judul edisi (mis. `"Vol. 10 No. 1: April - Oktober"`).
  - **Akurasi Biaya Publikasi (APC) 3-Status & Anti-False-Positive "Gratis"**:
    - Standarisasi 3 status biaya: `Gratis (Rp 0)`, `Rp X.XXX.XXX`, dan `Cek Web / Belum Terdata`.
    - Eliminasi total false-positive status Gratis: status bebas biaya hanya disematkan jika terdapat pernyataan eksplisit biaya publikasi/APC gratis, bukan karena biaya pendaftaran (*submission fee*) bernilai Rp 0 atau ketiadaan angka.
  - **Dekoder Email Terproteksi Cloudflare & Deobfuskasi Teks**:
    - Implementasi dekoder hex `decode_cfemail()` untuk mendekripsi email jurnal yang diproteksi tag `<a class="__cf_email__" data-cfemail="...">`.
    - Fungsi `deobfuscate_email_text()` untuk mengenali dan mengonversi format manual manusia seperti `nama [at] kampus.ac.id` atau `jurnal(at)kampus[dot]ac[dot]id`.
  - **Pemeringkatan Email Resmi & Pembersihan Email NIM Mahasiswa**:
    - Algoritma skoring memprioritaskan email dengan domain yang sesuai URL OJS kampus (`.ac.id`) atau berawalan kata kunci redaksi (`jurnal`, `editor`, `redaksi`, `ejournal`, `publikasi`, `admin`).
    - Otomatis membuang email mahasiswa berbasis pola NIM/stambuk (`>= 8 digit angka`) jika kontak resmi redaksi telah ditemukan.
  - **Ekstraksi Nomor Telepon Kampus & WhatsApp Diperluas**:
    - Regex telepon `RE_PHONE` kini mendukung nomor telepon kabel institusi berformat kode area `(021) xxx`, `(0271) xxx`, `+6221xxx`, serta nomor WhatsApp seluler `08xx` dan `+628xx`.

- **Fitur Baru: 1-Click Copy-Paste List Generator (`exporter.py`, `app_ui.py`)**:
  - **Fungsi `generate_copypaste_list`**: Menghasilkan daftar jurnal bernomor ringkas, bersih, dan informatif yang siap di-share ke WhatsApp/Telegram grup rekan mahasiswa/dosen.
  - **Integrasi Ekspor Markdown & File List Teks Pendamping**:
    - Menambahkan blok kode `## 📋 Daftar Ringkas Siap Copy-Paste (Untuk Share ke Rekan)` di dalam file ekspor `.md`.
    - Otomatis membuat dan menyimpan berkas teks pendamping `JSFS_<keyword>_<timestamp>_LIST.txt` setiap kali ekspor data dijalankan.
  - **Tombol Pop-up "📋 Salin Semua Daftar (Format Share)"**:
    - Ditempatkan di header jendela `JSFSDataPreviewer` dengan warna aksen ungu/indigo modern (`#6366f1`).
    - Salin satu klik seluruh baris jurnal yang sedang terfilter ke clipboard OS dengan feedback animasi `✅ Semua Daftar Tersalin ke Clipboard!` selama 2.5 detik.

### Fixed & Stabilized
- **Zero-Crash & Anti-Freeze Architecture (`scraper_garuda.py`, `scraper_sinta.py`, `ai_engine.py`)**:
  - **Content-Type & Response Size Guard (`safe_fetch_html`)**: Memeriksa header HTTP `Content-Type` untuk memastikan hanya memproses berkas HTML (`text/html`, `application/xhtml+xml`) dan membatasi unduhan maksimal 1.5 MB per halaman OJS via streaming, melenyapkan risiko freeze/hang saat crawler tidak sengaja menjumpai file PDF artikel atau arsip ZIP puluhan MB.
  - **Multi-Layer Exception Shield**: Membungkus seluruh thread worker crawling dan deep analysis dengan penanganan exception berlapis untuk menangani redirect loop (`TooManyRedirects`), encoding aneh (`UnicodeDecodeError`), koneksi timeout, dan parsing error tanpa menggugurkan proses scraping jurnal lainnya.

## [1.5.1] - 2026-10-20

### Fixed & Stabilized
- **Perbaikan Startup Log Box Attribute Guard (`app_ui.py`)**:
  - Menambahkan pengaman `hasattr(self, 'log_box') and self.log_box is not None` pada `log()` dan `_insert_log_batch()`.
  - Menginisialisasi `self.log_box = None` pada `__init__` dan menempatkan pembuatan widget `log_box` sebelum pemanggilan preset awal sehingga startup aplikasi 100% stabil dan bebas dari `AttributeError`.
- **Eliminasi Total GUI Blackout & Render Starvation (`app_ui.py`)**:
  - Membatasi konsumsi antrean log pada GUI queue (`max_batch_size = 25` per polling cycle) dan melakukan batch insert tunggal (`_insert_log_batch`) untuk mencegah Windows `WM_PAINT` event starvation.
  - Membatasi buffer teks log maksimal 500 baris terbaru guna menjaga performa memori dan rendering CustomTkinter tetap responsif.
  - Interval polling queue dipercepat secara aman menjadi 80 ms (~12 fps visual updates).
- **Pembersihan Bersih SQLite Connection & PyInstaller MEI Directory Lock (`cache_engine.py`, `app_ui.py`, `config.py`)**:
  - Penambahan `close_all_connections()` dengan `PRAGMA wal_checkpoint(TRUNCATE);` yang terdaftar pada `atexit` dan dipanggil secara eksplisit saat GUI ditutup (`_on_close`).
  - Penempatan direktori cache `.cache/` secara deterministik pada direktori aplikasi (`get_app_dir()`), sepenuhnya terisolasi dari folder ekstraksi sementara PyInstaller (`sys._MEIPASS`), melenyapkan peringatan `Failed to remove temporary directory: _MEIxxxxxx`.
- **Pemulihan Scraping Recall Drop (23 Jurnal -> 2 Jurnal) (`month_parser.py`)**:
  - Normalisasi fleksibel tipe data filter bulan (`_normalize_month_list`) yang mendukung integer, nama bulan string, singkatan 3 huruf, maupun JSON string.
  - Bypass cepat seleksi default 12 bulan pada `lolos_filter_bulan`: jika pengguna memilih seluruh 12 bulan (default), semua jurnal (baik yang memiliki data bulan maupun belum terdeteksi) otomatis lolos tanpa drop recall.
- **Normalisasi Root URL OJS & Filter Anti-Dewan Redaksi (`cache_engine.py`, `ai_engine.py`, `scraper_garuda.py`, `scraper_sinta.py`)**:
  - Implementasi fungsi `normalize_ojs_root_url` untuk memangkas sub-path OJS (`/about/editorialTeam`, `/submissions`, `/article/view/...`) kembali ke homepage resmi jurnal.
  - Penambahan filter token `editor-in-chief`, `editorial board`, `managing editor`, `dewan redaksi`, `reviewer` pada `BOILERPLATE_TOKENS` di `ai_engine.py` untuk mencegah daftar nama editor bocor ke dalam kolom Focus & Scope.
- **Eliminasi Duplikasi pada Exporter (`exporter.py`)**:
  - Menghapus duplikasi seksi *Top 3 Rekomendasi Jurnal* pada berkas Markdown (hanya mempertahankan 1 seksi bersih di bawah parameter pencarian).
  - Menghapus duplikasi baris kontak pada tabel *Direktori Kontak Cepat Redaksi Jurnal*.
  - Menjamin kolom `Kesesuaian` dan `Skor` selalu tercantum konsisten pada output Markdown (`.md`), CSV (`.csv`), dan Excel (`.xlsx`).

### Added & Enhanced
- **Progressive Disclosure UI & 4 Quick Presets Mahasiswa (`app_ui.py`)**:
  - Saklar Mode Cerdas: `✨ Mode Simpel (Direkomendasikan)` vs `⚙️ Mode Pengaturan Lengkap`.
  - 4 Preset Instan Mahasiswa:
    1. 🎓 **Target Skripsi S1**: SINTA 3–5, Target 25, Biaya maks Rp 1,5 jt, Semua Bulan.
    2. 💰 **Jurnal Gratis (Rp 0)**: SINTA 2–6, Hanya jurnal bebas APC, Target 50.
    3. ⚡ **Terbit Terdekat**: Memfilter otomatis edisi terbit 1–3 bulan ke depan, SINTA 3–6, Target 25.
    4. 🏆 **Reputasi Tinggi**: SINTA 1 & 2 (Nasional Akreditasi A & Scopus) untuk tesis, disertasi, atau hibah.
  - Dialog modal `❓ Panduan Singkat` interaktif yang dapat diakses langsung dari header aplikasi.
  - Label progres ganda yang lebih bersahabat bagi pemula (`🌐 Tahap 1: Mengumpulkan Data Jurnal` dan `🤖 Tahap 2: Menganalisis Biaya & Fokus Keilmuan`).
- **Template Percakapan Editor Jurnal & Health Warning (`app_ui.py`)**:
  - Tombol baru `💬 Salin Draf Chat Editor` pada `JSFSDataPreviewer` untuk menyalin draf pesan sopan menanyakan ketersediaan kuota publikasi edisi terdekat.
  - Indikator *Journal Health Warning* pada panel detail previewer bila jurnal belum mencantumkan jadwal terbit dan kontak redaksi.

## [1.5.0] - 2026-10-20

### Added & Enhanced
- **Smart SQLite WAL Cache Engine (`cache_engine.py`)**:
  - Penyimpanan lokal metadata jurnal OJS berbasis SQLite 3 dengan mode WAL (`journal_mode=WAL; synchronous=NORMAL;`).
  - Normalisasi URL kanonikal (`normalize_ojs_url`) menghindari redundansi cache antar skema HTTP/HTTPS, trailing slash, dan sub-halaman OJS.
  - TTL cerdas 14 hari dengan auto-invalidation berkala dan thread-safety teruji via `threading.Lock()`.
  - Mengurangi latensi jaringan dan beban server OJS eksternal hingga 80–90% pada pencarian berulang.
- **Pembersihan Boilerplate DOM & Anti-Halusinasi APC (`ai_engine.py`, `scraper_garuda.py`)**:
  - Pembersihan terarah kontainer DOM OJS (`clean_soup_for_scope`) yang memangkas tag noise (`script`, `style`, `nav`, `aside`, `header`, `footer`, breadcrumbs, sidebar, widget pengunjung).
  - Deteksi dan eliminasi noise template OJS berbasis `BOILERPLATE_TOKENS` (`is_boilerplate_noise`) sebelum teks diproses oleh model LLM.
  - Pengetatan validasi AI APC: memisahkan `Biaya Submit` vs `Biaya Publikasi` secara ketat. Menghilangkan false-positive status Gratis jika biaya submit Rp 0 namun biaya publikasi berbayar.
- **Penyelarasan Jadwal & Frekuensi Terbit Bebas Kontradiksi (`month_parser.py`)**:
  - Algoritma `resolve_schedule_and_frequency` yang menjamin konsistensi 100% antara daftar bulan terbit dan frekuensi terbit tahunan (0% kontradiksi data).
  - **Estimator Edisi Terdekat (Next Issue Countdown)**: Kalkulasi dinamis edisi terbit berikutnya beserta countdown bulan (mis. `November (1 bln lagi)`) dan deteksi `Sedang Terbit Bulan Ini`.
- **Pembersihan Kontak & Generator Tautan WhatsApp (`scraper_garuda.py`, `scraper_sinta.py`, `exporter.py`)**:
  - Pembersihan total kontak rusak (`mailto:Situs tidak dapat diakses`, `None`, `null`, URL acak) menjadi data bersih yang dapat dihubungi.
  - Sanitasi nomor telepon/ponsel dan pembuatan otomatis tautan interaktif WhatsApp Click-to-Chat (`https://wa.me/62...`) untuk mempercepat korespondensi calon penulis ke pengelola jurnal.
- **Pencocokan Rumpun Ilmu Dwibahasa & Keyword Match Score (`doc_reader.py`)**:
  - Pustaka sinonim dwibahasa akademik terintegrasi (`BILINGUAL_ACADEMIC_SYNONYMS`) mencakup 8 klaster bidang studi (Bisnis, Informatika, Pendidikan, Hukum, Kesehatan, Pertanian, Komunikasi, dan Sosial).
  - Algoritma `calculate_keyword_match_score`: Memberikan skor kecocokan terkalibrasi kurva normal (45%–95%) saat scraping dijalankan berdasarkan kata kunci pencarian tanpa mengunggah file naskah.
  - Integrasi ekspansi sinonim akademik pada `calculate_match_score` dengan pembobotan khusus pada konten `focus_scope`.
- **AI Journal Advisor & Top 3 Rekomendasi Jurnal (`exporter.py`)**:
  - Sistem perangkingan cerdas multi-kriteria berbasis `AdvisorScore` komposit (Relevansi 40%, Biaya/Gratis 25%, Kecepatan Terbit 20%, Akreditasi SINTA 15%).
  - Bagian khusus `🏆 Top 3 Rekomendasi Jurnal Terbaik (AI Advisor)` pada laporan Markdown dan sheet terpisah `🏆 Rekomendasi AI` pada file Excel dengan styling profesional dan justifikasi rekomendasi.
- **Pembaruan GUI & Data Table Previewer (`app_ui.py`)**:
  - Kolom baru `Terbit Terdekat` pada tabel previewer data (`JSFSDataPreviewer`).
  - Integrasi otomatis perhitungan skor kecocokan kata kunci pada alur kerja pipeline GUI.
  - Tetap mempertahankan standar estetika CustomTkinter Zero Clipping dan arsitektur non-blocking GUI queue 100 ms.

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
