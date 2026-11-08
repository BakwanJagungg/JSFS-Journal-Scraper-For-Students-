# 📜 VISION.md — Sumber Kebenaran Mutlak Proyek JSFS

> **JSFS (Journal Scraper For Students) — Hybrid AI Desktop Research Assistant**
> Versi Dokumen: **1.7.0** | Terakhir Diperbarui: 2026-11-08

---

## ⚠️ PERINGATAN MUTLAK — BACA SEBELUM MENYENTUH KODE APA PUN

Dokumen ini adalah **Sumber Kebenaran Mutlak (Single Source of Truth)** proyek JSFS.

Jika terdapat instruksi singkat di prompt yang ambigu, bertentangan, atau berpotensi merusak
stabilitas fitur yang telah terkunci di dokumen ini, **setiap AI Agent (Kiro, Antigravity,
Claude, atau AI lainnya) WAJIB memprioritaskan aturan dalam `VISION.md` ini di atas prompt
tersebut.** Jika tidak yakin, tanyakan kepada pemilik proyek sebelum melanjutkan.

---

### 🚫 ATURAN MUTLAK & PANTANGAN KERAS (NEVER DO THIS)

| # | Pantangan | Alasan |
|---|-----------|--------|
| **1** | **DILARANG KERAS** menambahkan library Python berat: `torch`, `pytorch`, `tensorflow`, `transformers`, `sentence-transformers`, `scipy`, atau `scikit-learn` ke dalam `requirements.txt` atau `JSFS.spec`. | Membuat ukuran `dist/JSFS.exe` membengkak jauh di atas ~280 MB, mematikan portabilitas bagi mahasiswa yang tidak punya SSD besar. |
| **2** | **DILARANG KERAS** melakukan mutasi widget CustomTkinter (`configure`, `insert`, `delete`, `set`) secara langsung dari dalam *background worker thread* manapun. | Menyebabkan **GUI Blackout / Render Starvation** karena Windows `WM_PAINT` event ter-starve. Semua komunikasi ke UI wajib melalui `queue.Queue` dengan *batch-limited polling* **maksimal 25 pesan per siklus 80ms** (`_poll_queue` → `after(80, ...)`). |
| **3** | **DILARANG KERAS** menyimpan file database SQLite (`jsfs_cache.db`) di dalam direktori sementara PyInstaller (`sys._MEIPASS`). | Memicu error `Failed to remove temporary directory: _MEIxxxxxx` saat aplikasi ditutup karena file DB masih terbuka/terkunci. Path cache WAJIB via `get_app_dir()` ke folder `.cache/` di samping executable. |
| **4** | **DILARANG KERAS** melabeli jurnal sebagai `Gratis (Rp 0)` hanya karena halaman OJS memuat teks *Open Access Policy* (contoh: `"freely available to the public"`, `"immediate open access"`) atau karena nilai `Submission Fee = 0.00 IDR`. | Teks OA Policy adalah kebijakan akses **pembaca**, bukan bebas biaya terbit untuk **penulis**. Ini adalah sumber false-positive terbesar yang merusak kepercayaan pengguna. |

---

---

## Bagian 1: Deskripsi, Latar Belakang & Tujuan Utama Proyek

### 1.1. Identitas Proyek

| Atribut | Nilai |
|---------|-------|
| **Nama Resmi** | JSFS — Journal Scraper For Students |
| **Subtitle** | Hybrid AI Desktop Research Assistant |
| **Versi Terkini** | 1.6.2 (Forensic Data Purity & Smart Deduplication) |
| **Target Versi Berikutnya** | 1.7.0 (Deep APC Hunter Engine + Modern Bright GUI) |
| **Platform** | Windows 10/11 64-bit (portable `.exe`, tidak perlu Python) |
| **Bahasa UI** | Bahasa Indonesia (teks, label, log, laporan output) |
| **Entry Point** | `app_ui.py` (GUI) · `jsfs.py` (CLI) |

### 1.2. Latar Belakang Masalah

Mahasiswa dan akademisi Indonesia menghabiskan waktu **berjam-jam** membuka ratusan situs OJS
(Open Journal Systems) dari portal **SINTA** (<https://sinta.kemdiktisaintek.go.id>) dan
**Portal Garuda** (<https://garuda.kemdiktisaintek.go.id>) satu per satu hanya untuk menjawab
pertanyaan sederhana:

- 🗓️ Kapan jurnal ini menerbitkan edisi berikutnya?
- 💰 Berapa biaya publikasinya (APC)? Apakah gratis?
- 📚 Apakah fokus keilmuannya sesuai dengan topik skripsi/tesis saya?
- 📞 Ke mana harus menghubungi editor/redaksi?

Kondisi diperparah oleh: proteksi Cloudflare pada email redaksi, informasi biaya yang tersembunyi
di halaman tersier OJS, perbedaan struktur OJS 2 vs OJS 3, serta server kampus `.ac.id` yang
sering tidak stabil.

### 1.3. Tujuan Utama — Empat Pilar

```
┌─────────────────────────────────────────────────────────────────────┐
│  PILAR 1: Human-Parity Accuracy                                     │
│  Hasil ekstraksi APC, Bulan Terbit, Focus & Scope, dan Kontak       │
│  100% akurat — setara kurasi manual oleh mata manusia.              │
├─────────────────────────────────────────────────────────────────────┤
│  PILAR 2: High-Yield & Fast                                         │
│  Puluhan–ratusan jurnal dalam detik/menit via 4-Fase Pipeline       │
│  paralel + 3× Candidate Pool Over-Fetching.                         │
├─────────────────────────────────────────────────────────────────────┤
│  PILAR 3: Zero-Crash & Zero-Freeze                                  │
│  Tidak pernah crash menghadapi server mati, SSL expired,            │
│  redirect loop, PDF nyasar, atau Ollama tidak aktif.                │
├─────────────────────────────────────────────────────────────────────┤
│  PILAR 4: Modern Bright UX & 1-Click Share                          │
│  Antarmuka cerah (Daylight Theme), Mode Simpel + Quick Presets,     │
│  output siap share ke WhatsApp/Telegram dalam 1 klik.               │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Bagian 2: Tech Stack Resmi & Arsitektur Modul

### 2.1. Tech Stack Resmi

> ⚠️ **Jangan menambah library baru** tanpa diskusi dengan pemilik proyek. Setiap library baru
> wajib dievaluasi dampaknya terhadap ukuran `dist/JSFS.exe` (batas keras ~280 MB).

| Kategori | Library / Tool | Versi Terkunci | Catatan |
|----------|---------------|----------------|---------|
| **Runtime** | Python | 3.10+ | Dibundle via PyInstaller |
| **Build** | PyInstaller | 6.11.0 | Spec: `JSFS.spec` |
| **HTTP Scraping** | requests | 2.32.3 | `HTTPAdapter` connection pooling |
| **HTML Parsing** | beautifulsoup4 | 4.12.3 | Parser: `lxml` (utama) / `html.parser` (fallback) |
| **HTML Parser Engine** | lxml | 5.3.0 | — |
| **Selenium Fallback** | selenium | 4.27.1 | Bypass Cloudflare SINTA (opsional) |
| **Chrome Driver** | undetected-chromedriver | 3.5.5 | Auto-manage via selenium |
| **AI Engine** | Ollama REST API | — | `localhost:11434`, model `qwen2.5:3b` |
| **Local Cache** | sqlite3 | bawaan Python | WAL mode, path via `get_app_dir()` |
| **GUI Framework** | customtkinter | 5.2.2 | Soft Slate / Daylight Theme |
| **Data Export** | pandas | 2.2.3 | DataFrame → Excel/CSV |
| **Excel Writer** | openpyxl | 3.1.5 | Multi-sheet `.xlsx` |
| **PDF Reader** | pypdf | 6.21.0 | Ekstrak teks naskah |
| **DOCX Reader** | python-docx | 1.2.0 | Ekstrak teks naskah |
| **CLI Display** | rich | 13.9.4 | Tabel & progress CLI |

### 2.2. Arsitektur 4-Fase Pipeline

```
Pengguna Input (Kata Kunci + Filter)
         │
         ▼
┌──────────────────────────────────────────────────────────────┐
│ FASE 1: Fast Broad Scraping                                  │
│  Garuda (scraper_garuda.py) + SINTA (scraper_sinta.py)       │
│  Pool kandidat: max(target × 3, 60) jurnal                   │
│  ThreadPoolExecutor: 64 workers (Turbo) / 32 workers (Normal)│
│  safe_fetch_html() → Content-Type guard + 1.5 MB size limit  │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────┐
│ FASE 2: Pre-Filter                                           │
│  Filter: SINTA rank, bulan terbit, APC range                 │
│  Smart APC Preservation: "Cek Web" wajib lolos default       │
│  Deduplikasi: canonical_journal_key() + ISSN/E-ISSN          │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────┐
│ FASE 3: AI/Regex Deep Enrichment                             │
│  extract_ojs_deep_info() — Deep scrape 5 titik OJS           │
│  ai_engine.py — Ollama qwen2.5:3b (JSON mode)                │
│  Circuit Breaker → Regex fallback jika Ollama mati           │
│  cache_engine.py — SQLite WAL cache (TTL 14 hari)            │
│  Incomplete Cache Bypass: sumber_apc == "Kosong" → re-scrape │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────┐
│ FASE 4: Multi-Format Export                                  │
│  exporter.py → .xlsx (multi-sheet) + .md + .csv + _LIST.txt  │
│  AdvisorScore → Top 3 Rekomendasi AI                         │
│  generate_copypaste_list() → Format Share WA                 │
└──────────────────────────────────────────────────────────────┘
```

### 2.3. Peta Tanggung Jawab Setiap File

| File | Tanggung Jawab Utama | Fungsi/Kelas Kunci |
|------|---------------------|-------------------|
| `config.py` | Konfigurasi global terpusat: URL, timeout, headers, thread pool, APC sanity range, path output | `get_app_dir()`, `generate_output_filename()`, `APP_VERSION`, `TURBO_SCRAPE_WORKERS`, `APC_SANITY_MIN/MAX_IDR` |
| `version_info.txt` | Metadata versi untuk PyInstaller Windows executable (VERSIONINFO resource) | `filevers`, `FileDescription`, `ProductVersion` |
| `cache_engine.py` | SQLite WAL cache management, normalisasi URL root OJS, auto-purge pada version bump, `close_all_connections()` saat `atexit` | `JSFSCacheEngine`, `normalize_ojs_root_url()`, `normalize_ojs_url()`, `close_all_connections()` |
| `month_parser.py` | **Single Source of Truth** untuk: ekstraksi bulan terbit (1–12), sinkronisasi frekuensi, pembacaan arsip tahun terbaru, Next Issue Countdown | `parse_bulan_terbit()`, `resolve_schedule_and_frequency()`, `extract_months_from_archive_titles()`, `get_next_issue_estimate()`, `lolos_filter_bulan()` |
| `doc_reader.py` | Pembaca naskah `.docx`/`.pdf`, kamus sinonim akademik dwibahasa (8 klaster), Always-On Keyword & Manuscript Match Score | `extract_manuscript_metadata()`, `calculate_keyword_match_score()`, `calculate_match_score()`, `expand_academic_synonyms()` |
| `ai_engine.py` | Klien Ollama REST API, Smart Truncation, Circuit Breaker (5 fail → 60s cooldown), validator anti-halusinasi APC/Scope, pembersih boilerplate DOM | `check_ollama_active()`, `smart_truncate()`, `_validate_apc()`, `_validate_focus()`, `enrich_records()`, `is_boilerplate_noise()` |
| `scraper_garuda.py` | Mesin scraping Garuda utama: 3× multiplier kandidat, Dynamic Menu Anchor Discovery, Cloudflare email decoder, phone validator, deduplikasi, synthesize_focus_scope | `JournalEntry`, `decode_cfemail()`, `is_valid_indonesian_phone()`, `canonical_journal_key()`, `extract_ojs_deep_info()`, `scrape_garuda()`, `synthesize_focus_scope()` |
| `scraper_sinta.py` | Pengumpul kandidat dari portal SINTA, cross-referencing dengan Garuda, deduplikasi lintas sumber | `scrape_sinta()`, `merge_cross_source_entries()` |
| `exporter.py` | Generator 4 format output serentak, AdvisorScore (Top 3), copy-paste list, WhatsApp Click-to-Chat | `generate_copypaste_list()`, `select_top_recommendations()`, `export_excel()`, `export_markdown()`, `export_csv()`, `write_exports()` |
| `app_ui.py` | GUI desktop utama, antarmuka Mode Simpel/Lengkap, 4 Quick Presets, JSFSDataPreviewer interaktif, queue polling thread-safe | `JSFSApp`, `JSFSDataPreviewer`, `FitCheckBox`, `FitSwitch`, `DualProgress`, `MonthFilter`, `_poll_queue()`, `_insert_log_batch()` |
| `ai_installer.py` | Wizard 1-klik instalasi Ollama + download model `qwen2.5:3b` | `AISetupDialog`, `_on_click_download_ollama()`, `_on_click_pull_model()` |
| `jsfs.py` | Entry point CLI, argparse, orchestrator pipeline non-GUI | `parse_args()`, `run()`, `main()` |
| `log_redirect.py` | Redirect `stdout`/`stderr` ke log file saat berjalan sebagai `.exe` frozen | — |
| `JSFS.spec` | Konfigurasi build PyInstaller: `hiddenimports`, `collect_all`, `assets/` bundle | — |

---

## Bagian 3: Aturan Bisnis & Algoritma Inti yang Terkunci

> ⚠️ **LOCKED CORE LOGIC** — Bagian ini tidak boleh diubah tanpa persetujuan eksplisit pemilik
> proyek. Setiap AI yang akan memodifikasi logika di sini wajib menampilkan *diff* rencana
> perubahan dan mendapat konfirmasi terlebih dahulu.

### 3.1. Standar Akurasi Biaya Publikasi (Deep APC Hunter Engine)

#### 3.1.1. Penelusuran 5 Titik OJS Wajib

`extract_ojs_deep_info()` di `scraper_garuda.py` WAJIB memindai kelima titik ini secara berurutan:

| Titik | URL / Selector | Keterangan |
|-------|---------------|------------|
| **(a)** | `/about/submissions` | OJS 3 — halaman utama biaya APC modern |
| **(b)** | `/about/editorialPolicies` | OJS 2 — institusi lama (UII, PNB, dll.) |
| **(c)** | `/about` | Halaman about generik |
| **(d)** | `div.pkp_block`, `[id*="fee" i]`, `[id*="apc" i]`, `[class*="customblock" i]` | Blok sidebar/custom block di beranda |
| **(e)** | Seluruh `<a href>` navigasi yang mengandung kata kunci: `fee`, `apc`, `biaya`, `charge`, `payment`, `tarif`, `guidelines`, `panduan` | Dynamic Menu Anchor Discovery |

#### 3.1.2. Eliminasi Jebakan Open Access (Anti-OA-Trap)

Frasa-frasa berikut adalah kebijakan akses **pembaca**, BUKAN pernyataan bebas biaya untuk penulis.
Wajib dibuang/di-skip **sebelum** pencocokan kata kunci `gratis`/`free`:

```
"making research freely available to the public"
"immediate open access"
"free to read and download"
"open access to all users"
"publicly available"
"freely available online"
```

Implementasi: lihat `_extract_apc_contextual()` dan `parse_apc_from_context()` di `scraper_garuda.py`.

#### 3.1.3. Hierarki 3-Status Biaya & Paid Priority Rule

```
Prioritas Pencarian (dari tertinggi ke terendah):
  1. Cari nominal BERBAYAR (Rp 50.000 – Rp 15.000.000 atau USD) terlebih dahulu
  2. Jika ditemukan nominal, tetapkan sebagai BERBAYAR (meski Submission Fee = 0)
  3. Jika ada pernyataan eksplisit gratis tanpa ada nominal berbayar → GRATIS
  4. Jika tidak ada konfirmasi keduanya → CEK WEB

Contoh edge case wajib benar:
  Input:  "Submission: 0.00 IDR  |  Publication: 500,000.00 IDR"
  Output: Rp 500.000  ← BUKAN Gratis!

  Input:  "Biaya Submit: Rp 600.000  |  Biaya Publikasi: (kosong)"
  Output: Rp 600.000  ← Smart Fallback Submit → Publikasi (threshold > Rp 50.000)
```

Status yang valid **hanya ada 3**:

| Status | Kapan Digunakan |
|--------|----------------|
| `Gratis (Rp 0)` | Ada pernyataan **eksplisit** bebas biaya terbit (bukan hanya OA policy) |
| `Rp X.XXX.XXX` | Ditemukan nominal biaya (IDR atau konversi USD × 17.500) |
| `Cek Web / Belum Terdata` | Tidak ada konfirmasi keduanya setelah 5-titik scrape |

#### 3.1.4. Aturan Incomplete Cache Bypass

Cache jurnal dianggap **BELUM LENGKAP** dan WAJIB di-bypass (lakukan deep-scrape ulang) jika:

```python
# Kondisi wajib bypass cache — implementasi di cache_engine.py get_journal()
biaya_publikasi in ("", "—", "-", "Cek Web / Belum Terdata", None)
OR sumber_apc in ("Kosong", "", None)
```

---

### 3.2. Standar Kebersihan Kontak Redaksi (Zero-Noise Contact Rules)

#### 3.2.1. Email Redaksi — Hard Cap Maksimal 3 Email

**Wajib dilakukan** sebelum menyimpan email:

1. **Dekode Cloudflare**: `decode_cfemail()` untuk tag `<a class="__cf_email__" data-cfemail="...">`.
2. **Deobfuskasi teks**: `deobfuscate_email_text()` untuk format `nama [at] kampus.ac.id`.
3. **Isolasi sumber**: Hapus seluruh blok artikel (`.obj_article_summary`, `.cmp_article_list`,
   `meta[name="citation_author_email"]`) sebelum ekstraksi — email penulis naskah TIDAK boleh bocor.

**Tolak 100%** email yang memenuhi kriteria berikut:

```
Domain mahasiswa : @student.  @students.  @mhs.  @mail.ugm.  @alumni.  @pasca.  @pps.
NIM di username  : >= 6 digit angka berurutan di bagian username
Sumber artikel   : email yang diekstrak dari .obj_article_summary atau citation_author_email
```

**Prioritaskan** (via `rank_and_filter_emails()`):
- Email yang domainnya sama dengan domain host OJS kampus (`.ac.id`)
- Email dengan prefix kata kunci redaksi: `jurnal`, `journal`, `redaksi`, `editor`, `ejournal`,
  `publikasi`, `ojs`, `admin`

#### 3.2.2. Telepon & WhatsApp — Hard Cap Maksimal 2 Nomor

**Urutan prioritas** (utamakan WhatsApp/seluler):

```
1. Nomor seluler/WA Indonesia:
   Format : +628... atau 08...
   Prefix  : 811–819, 821–823, 831–838, 851–859, 877–878, 881–889, 895–899
   Panjang : 10–13 digit (setelah angka 0)

2. Nomor kabel (landline) — HANYA jika SEMUA syarat terpenuhi:
   ✓ Kode area ada dalam VALID_INDONESIAN_AREA_CODES (whitelist resmi Telkom)
   ✓ Panjang total 9–12 digit (TOLAK 14–16 digit = rekening bank / NIP!)
   ✓ Digit pertama setelah kode area BUKAN 0 atau 1 (TOLAK +622100..., +621...)
   ✓ Ada kata kunci konteks dalam 35 karakter di kiri:
     telp / telepon / phone / ph: / tel: / fax / wa / whatsapp / contact / hubungi
     ATAU memiliki format kurung kode area: (0271) xxx
```

Implementasi: `is_valid_indonesian_phone()` di `scraper_garuda.py`.

---

### 3.3. Standar Jadwal Terbit, Deduplikasi & Fokus Keilmuan

#### 3.3.1. Unifikasi Bulan Terbit & Frekuensi

Diimplementasikan di `month_parser.py` — ini adalah **satu-satunya tempat** yang boleh mengubah
logika bulan terbit.

```
INVARIANT yang harus selalu benar:
  len(bulan_terbit_list) == frekuensi_angka

Contoh:
  ✓ BENAR  : Bulan Terbit: ["Jul", "Des"]  →  Frekuensi: "2 kali setahun (Juli, Desember)"
  ✗ SALAH  : Bulan Terbit: ["Jul", "Des"]  →  Frekuensi: "3 kali setahun"
```

**Aturan pembacaan arsip `/issue/archive`**:
- Kelompokkan judul edisi berdasarkan tahun (`2026`, `2025`, dst.)
- Ambil bulan **HANYA dari 1 tahun penuh terbaru** (atau 4–6 edisi teratas)
- Abaikan widget kalender bulanan yang menghasilkan >= 11 bulan (itu noise kalender, bukan frekuensi)

Implementasi: `extract_months_from_archive_titles()` di `month_parser.py`.

#### 3.3.2. Deduplikasi Cerdas & Koreksi Nama Jurnal

```python
# canonical_journal_key() — scraper_garuda.py
# Normalisasi: lowercase, hilangkan tanda baca, spasi berlebih, titik dua
key = canonical_journal_key(title, publisher, issn)

# Penggabungan jurnal migran (http ↔ https versi sama):
# Menggunakan key + ISSN/E-ISSN + path URL OJS
# Contoh: "Balance UM Palembang" (http) + "Balance UM Palembang" (https) → 1 entri
```

**Koreksi judul artikel → nama jurnal**:
- Jika judul dari Garuda > 55 karakter DAN tidak mengandung kata `Jurnal`/`Journal`/`Berkala`
- → Koreksi otomatis menggunakan `<meta name="citation_journal_title">` dari situs OJS

#### 3.3.3. Focus & Scope — 100% Terisi, Bahasa Indonesia Bersih

**Pipeline pembersihan** (berurutan):

```
1. strip_ojs_profile_table_noise()  → Hapus tabel biodata OJS (Journal Title|ISSN|DOI Prefix|...)
2. is_focus_scope_noise()           → Tolak teks yang mengandung metadata indeksasi, lisensi CC,
                                      tanggal rilis, atau abstrak artikel
3. normalize_focus_and_scope()      → Normalisasi teks menjadi 2–3 kalimat, ≤300 karakter
4. synthesize_focus_scope()         → Jika masih kosong, sintesis dari rumpun SINTA/Garuda
                                      dalam Bahasa Indonesia TANPA: "Other", duplikasi,
                                      potongan menggantung (,...)
```

**Token yang wajib dibuang** dari `ai_engine.py` `BOILERPLATE_TOKENS`:

```
"editor-in-chief", "editorial board", "managing editor", "dewan redaksi",
"reviewer", "scopus id", "google scholar", "indexed by", "creative commons",
"copyright ©", "this issue has been available", "this study aims to"
```

---

## Bagian 4: Standar Desain Antarmuka (GUI) & Pengalaman Pengguna

### 4.1. Tema Visual — Modern Academic Daylight / Soft Slate

```
Mode Default  : ☀️ Mode Terang (Daylight) — wajib sebagai default saat pertama buka
Mode Malam    : 🌙 Mode Malam — sakelar di header kanan atas
DILARANG      : Kembali ke tampilan hitam legam gelap sebagai default!
```

**Palette warna referensi Soft Slate Daylight:**

| Elemen | Warna |
|--------|-------|
| Background utama | `#f8fafc` (slate-50) |
| Card/Panel | `#ffffff` dengan border `#e2e8f0` |
| Header bar | `#1e3a5f` (navy academic) |
| Tombol Primer (Mulai) | `#0284c7` (sky-600) |
| Tombol Sukses (Preview) | `#059669` (emerald-600) |
| Tombol Aksen (Copy/Share) | `#6366f1` (indigo-500) |
| Teks utama | `#1e293b` (slate-800) |
| Teks sekunder | `#64748b` (slate-500) |

### 4.2. Aturan Zero-Clipping Mutlak

```
Ukuran jendela : 900×700 (minsize: 880×660)
Font engine    : fit_w(font, text) untuk mengukur lebar dinamis setiap checkbox/switch/tombol
Komponen       : FitCheckBox, FitSwitch — WAJIB digunakan, bukan CTkCheckBox mentah
Target layar   : 1366×768 (laptop mahasiswa) hingga 1920×1080 — ZERO clipping di keduanya
```

### 4.3. Progressive Disclosure — Mode Simpel vs Mode Lengkap

```
DEFAULT : Mode Simpel (3 Kartu Langkah)
  ┌──────────────────┐  ┌───────────────────┐  ┌────────────────────┐
  │ 1. Topik &       │  │ 2. Quick Presets  │  │ 3. Live KPI Pills  │
  │    Naskah        │  │    + Filter Aktif │  │    & Log           │
  └──────────────────┘  └───────────────────┘  └────────────────────┘

ADVANCED: Mode Pengaturan Lengkap (semua kontrol tampak)
  Filter SINTA rank, range APC, filter bulan, thread mode, selenium toggle
```

**4 Quick Presets Wajib Ada:**

| Preset | SINTA | APC | Target | Catatan |
|--------|-------|-----|--------|---------|
| 🎓 Target Skripsi S1 | 3–5 | ≤ Rp 1.500.000 | 25 | Semua bulan |
| 💰 Jurnal Gratis (Rp 0) | 2–6 | Hanya Gratis | 50 | + Cek Web pelengkap kuota |
| ⚡ Terbit Terdekat | 3–6 | — | 25 | Filter bulan 1–3 bulan ke depan |
| 🏆 Reputasi Tinggi | 1–2 | — | 20 | Untuk tesis/disertasi/hibah |

### 4.4. Aksi Cepat 1-Klik (Wajib Selalu Mudah Diakses)

| Tombol | Lokasi | Fungsi |
|--------|--------|--------|
| `📋 Salin Daftar (Format Share WA)` | Header `JSFSDataPreviewer` | `_copy_share_list()` — salin seluruh daftar terfilter ke clipboard |
| `💬 Salin Draf Chat Editor` | Panel detail `JSFSDataPreviewer` | `_copy_editor_chat_draft()` — draf pesan sopan ke redaksi |
| `❓ Panduan Singkat` | Header `JSFSApp` | `_open_quick_guide()` — modal dialog interaktif |
| `● AI Aktif` / `○ Mode Regex` | Header kanan `JSFSApp` | `_open_ai_setup_manager()` — wizard Ollama |

---

## Bagian 5: Format Standar Output "Copy-Paste List"

Setiap modifikasi pada `exporter.py` wajib menjaga format output `_LIST.txt` dan seksi
copy-paste di `.md` persis seperti struktur berikut. Lihat `generate_copypaste_list()` di
`exporter.py`.

### 5.1. Format Blok Per Jurnal

```
{nomor}. {Nama Jurnal} ({Peringkat SINTA} - {Nama Institusi Penerbit})
Email: {email1@kampus.ac.id}, {email2@kampus.ac.id}
Telepon/WA: {+628xxxxxxxxxx}
Biaya Publikasi: {Gratis (Rp 0) / Rp 500.000 / Cek Web} | Terbit: {Juni, Desember}
Fokus: {Ringkasan fokus keilmuan bersih dalam Bahasa Indonesia, maks 150 karakter}
Link OJS: [{https://jurnal.kampus.ac.id/...}]({https://jurnal.kampus.ac.id/...})
```

**Contoh nyata:**

```
3. Jurnal Ekonomi & Manajemen Indonesia (Sinta 3 - Universitas Brawijaya)
Email: jemi@ub.ac.id, redaksi.jemi@ub.ac.id
Telepon/WA: +6281234567890
Biaya Publikasi: Rp 500.000 | Terbit: Maret, September
Fokus: Riset ilmiah di bidang Ekonomi, Manajemen Bisnis, dan Keuangan Perusahaan.
Link OJS: [https://jemi.ub.ac.id/index.php/jemi](https://jemi.ub.ac.id/index.php/jemi)
```

### 5.2. Aturan Baris Kondisional

| Kondisi | Aturan |
|---------|--------|
| `email` kosong atau `"—"` | **Sembunyikan** seluruh baris `Email:` |
| `telepon` kosong atau `"—"` | **Sembunyikan** seluruh baris `Telepon/WA:` |
| `fokus_keilmuan` kosong | Gunakan output `synthesize_focus_scope()` — jangan biarkan kosong |
| `bulan_terbit` tidak diketahui | Tampilkan `Terbit: (Belum terdata)` |

### 5.3. Nama File Output

```python
# generate_output_filename() di config.py
JSFS_{keyword_clean}_{YYYYMMDD_HHMMSS}.xlsx   # Excel multi-sheet
JSFS_{keyword_clean}_{YYYYMMDD_HHMMSS}.md     # Markdown lengkap + seksi copy-paste
JSFS_{keyword_clean}_{YYYYMMDD_HHMMSS}.csv    # CSV flat
JSFS_{keyword_clean}_{YYYYMMDD_HHMMSS}_LIST.txt  # Copy-paste list murni
```

---

## Bagian 6: Pre-Flight Checklist Sebelum Coding

AI Agent **WAJIB** menyelesaikan checklist ini sebelum memodifikasi kode apapun:

### Checklist Verifikasi Lingkungan

- [ ] **Baca `config.py`** — Catat `APP_VERSION` saat ini. Semua perubahan versi dimulai dari sini.
- [ ] **Baca `version_info.txt`** — Pastikan `FileVersion` dan `ProductVersion` sinkron dengan `APP_VERSION`.
- [ ] **Periksa `requirements.txt`** — Semua library standar harus tersedia. Jangan tambahkan library berat.
- [ ] **Pahami alur 4-Fase Pipeline** (lihat Bagian 2.2) sebelum menyentuh `scraper_garuda.py`, `scraper_sinta.py`, `ai_engine.py`, atau `jsfs.py`.
- [ ] **Baca modul yang akan diubah** secara lengkap — jangan modifikasi kode yang belum dibaca.

### Checklist Aturan Coding

- [ ] **Tidak ada mutasi widget CTk di background thread** — Semua update UI melalui `queue.Queue`.
- [ ] **Tidak ada import library baru** tanpa evaluasi dampak ukuran `.exe`.
- [ ] **Setiap penambahan field cache** harus disertai `cache_version` bump di `cache_engine.py` agar cache lama auto-invalidate.
- [ ] **Setiap perubahan logika bulan terbit** hanya boleh dilakukan di `month_parser.py`.
- [ ] **Setiap perubahan logika APC** harus menjaga invariant 3-status dan Paid Priority Rule (Bagian 3.1.3).
- [ ] **Setiap perubahan format output** harus menjaga format Copy-Paste List (Bagian 5).

### Checklist Sebelum Commit

- [ ] `python -m py_compile *.py` — 0 syntax error di seluruh modul.
- [ ] Jalankan unit test verifikasi fungsi kritis (lihat Bagian 9).
- [ ] Hapus semua file `.md` blueprint/rencana sementara (kecuali `VISION.md`, `README.md`, `CHANGELOG.md`).
- [ ] Update `CHANGELOG.md` dengan deskripsi perubahan.
- [ ] Update `README.md` dengan fitur baru jika relevan.
- [ ] Build ulang `dist/JSFS.exe` via: `python -m PyInstaller --noconfirm --clean JSFS.spec`.

---

## Bagian 7: Troubleshooting Playbook

Panduan diagnosis langkah-demi-langkah untuk isu spesifik pada proyek ini.

### 🔴 Masalah: Hasil Scraping Sedikit (< 10 Jurnal)

```
Diagnosis:
1. Periksa Candidate Pool Multiplier di scrape_garuda() / scrape_sinta():
   → pool_size = max(target_results * 3, 60)  ← wajib >= 3× target

2. Periksa apakah filter APC membuang jurnal "Cek Web / Belum Terdata":
   → Jika include_na=True atau max_rp > 0, jurnal Cek Web WAJIB lolos
   → Lihat _apply_apc_filter() — pastikan kondisi include_na diperiksa

3. Periksa apakah filter bulan membuang jurnal tanpa data bulan:
   → Lihat lolos_filter_bulan() di month_parser.py
   → Jika semua 12 bulan dipilih (default), bypass langsung tanpa drop recall
```

### 🔴 Masalah: APC Banyak "Cek Web / Belum Terdata"

```
Diagnosis:
1. Periksa apakah 5 titik OJS sudah dikunjungi (Bagian 3.1.1):
   → /about/submissions ✓
   → /about/editorialPolicies ✓ (OJS 2!)
   → /about ✓
   → Sidebar pkp_block / custom block ✓
   → Dynamic Menu Anchor Discovery ✓

2. Periksa Incomplete Cache Bypass:
   → cache_engine.py get_journal() — apakah jurnal dengan sumber_apc="Kosong"
     masih dikembalikan dari cache? Wajib return None untuk force re-scrape.

3. Periksa apakah Anti-OA-Trap sudah aktif:
   → parse_apc_from_context() — apakah frasa OA policy sudah di-strip sebelum
     pencocokan kata "free"/"gratis"?
```

### 🔴 Masalah: GUI Freeze atau Layar Hitam (Black Screen)

```
Diagnosis:
1. Cari mutasi CTk di luar main thread:
   grep "\.configure\|\.insert\|\.set\|\.delete" app_ui.py
   → Semua hit harus ada di dalam _poll_queue() atau fungsi yang dipanggil dari main thread

2. Periksa _poll_queue() di app_ui.py:
   → max_batch_size = 25  ← wajib ada batasan ini
   → interval = 80ms via after(80, self._poll_queue)

3. Periksa apakah log buffer dibatasi:
   → Maksimal 500 baris terbaru (_insert_log_batch)
   → Lebih dari itu akan menyebabkan CTkTextbox render starvation
```

### 🔴 Masalah: Error `_MEIxxxxxx` saat Keluar dari .exe

```
Diagnosis:
1. Periksa close_all_connections() di cache_engine.py:
   → Wajib execute: PRAGMA wal_checkpoint(TRUNCATE)
   → Wajib close() semua sqlite3.Connection yang terbuka

2. Periksa _on_close() di app_ui.py:
   → Urutan wajib:
     a. Set stop_event (batalkan thread scraping aktif)
     b. Panggil close_all_connections() dari cache_engine
     c. Tutup semua requests.Session (session.close())
     d. Panggil self.destroy()

3. Periksa path database:
   → get_app_dir() harus return path di samping .exe, BUKAN sys._MEIPASS
```

### 🔴 Masalah: Email Mahasiswa Bocor ke Kolom Redaksi

```
Diagnosis:
1. Periksa apakah blok artikel sudah dihapus sebelum ekstraksi:
   → clean_soup_for_scope() harus menghapus .obj_article_summary
   → Hapus juga meta[name="citation_author_email"]

2. Periksa rank_and_filter_emails() di scraper_garuda.py:
   → Pattern NIM: r'\d{6,}' di username ← wajib ada
   → Domain mahasiswa: @student., @mhs., @mail.ugm. ← wajib ada

3. Hard cap 3 email diterapkan SETELAH filtering/ranking, bukan sebelum
```

### 🔴 Masalah: Nomor Telepon Palsu (DOI/NIP Masuk sebagai Nomor)

```
Diagnosis:
1. Periksa is_valid_indonesian_phone() di scraper_garuda.py:
   → VALID_INDONESIAN_AREA_CODES harus memuat area code real Telkom
   → Tolak jika digit pertama setelah kode area adalah 0 atau 1
   → Tolak jika total digit > 12 (kemungkinan rekening / NIP)

2. Periksa proximity context (35 karakter kiri):
   → Landline tanpa format (0271) xxx wajib ada kata kunci kontak di kirinya
```

---

## Bagian 8: Perintah Khusus (Custom Trigger Commands)

AI Agent yang bekerja di workspace ini wajib mengenali dan mengeksekusi perintah berikut:

### `/jelaskan`
**Deskripsi**: Jelaskan rencana arsitektur dalam 3–5 poin singkat **sebelum** menyentuh kode.

**Format output wajib:**
```
Rencana:
1. [File yang akan diubah] — [Apa yang akan diubah]
2. ...
Risiko: [Potensi dampak terhadap fitur yang sudah stabil]
Estimasi perubahan: [Jumlah baris / fungsi yang terpengaruh]
```

---

### `/audit-hasil [nama_file.md/.txt]`
**Deskripsi**: Audit forensik baris-per-baris terhadap file hasil scraping.

**Checklist audit:**
- [ ] Email mahasiswa bocor (`@student.`, `@mhs.`, `@alumni.`, >= 6 digit NIM)?
- [ ] Nomor telepon palsu (DOI, NIP, rekening bank masuk sebagai kontak)?
- [ ] APC false-positive Gratis (dari OA policy atau Submission Fee = 0)?
- [ ] Anomali bulan terbit (> 12 bulan terdaftar, atau 7–8 bulan dari gabungan multi-tahun)?
- [ ] Focus & Scope noise (nama editor, indeksasi Scopus, lisensi CC, abstrak artikel)?
- [ ] Format Copy-Paste List rusak (baris Email/Telepon tidak disembunyikan saat kosong)?

---

### `/tes-apc [url_ojs]`
**Deskripsi**: Uji terisolasi fungsi ekstraksi biaya APC pada URL OJS tertentu.

**Output yang diharapkan:**
```
URL: {url_ojs}
Teks mentah ditemukan di titik [a/b/c/d/e]:
  "{teks relevan biaya yang ditemukan}"
Keputusan APC:
  Biaya Submit    : {nilai}
  Biaya Publikasi : {nilai}
  Status          : {Gratis / Rp X / Cek Web}
  Sumber APC      : {AI / Regex / Kosong}
Catatan: {alasan keputusan}
```

---

### `/rilis [versi]`
**Deskripsi**: Prosedur lengkap rilis versi baru. Contoh: `/rilis 1.7.0`

**Langkah eksekusi (urutan wajib):**
```
1. Naikkan APP_VERSION di config.py           → APP_VERSION = "{versi}"
2. Update version_info.txt                    → filevers, prodvers, semua string versi
3. Update CHANGELOG.md                        → Tambah seksi [X.Y.Z] dengan tanggal hari ini
4. Update README.md                           → Versi di judul H1 dan badge
5. Syntax check: python -m py_compile *.py    → Wajib 0 error
6. Unit test: python -c "..."                 → Wajib semua PASS
7. Hapus file .md sementara                   → Kecuali VISION.md, README.md, CHANGELOG.md
8. Build: python -m PyInstaller --noconfirm --clean JSFS.spec
9. Verifikasi dist/JSFS.exe berjalan          → Buka GUI, cek versi di title bar
10. Git: add spesifik + commit + tag vX.Y.Z  → Merge ke main
```

---

## Bagian 9: Standarisasi Git Workflow, Branch & Build

### 9.1. Enam Kategori Penamaan Branch Resmi

| Kategori | Format | Contoh | Gunakan untuk |
|----------|--------|--------|---------------|
| Fitur baru | `fitur/<deskripsi>` | `fitur/deep-apc-5-titik` | Penambahan kapabilitas baru |
| Perbaikan bug | `perbaikan/<deskripsi>` | `perbaikan/email-mahasiswa-bocor` | Bug akurasi scraper, filter, crash |
| Revisi aturan bisnis | `revisi/<deskripsi>` | `revisi/format-copypaste-list` | Penyesuaian format laporan / aturan |
| UI/UX | `ui/<deskripsi>` | `ui/daylight-theme-default` | Tampilan CTk, tema, previewer |
| Refaktor | `refaktor/<deskripsi>` | `refaktor/cache-incomplete-bypass` | Optimasi kode, cache, arsitektur |
| Rilis | `rilis/vX.Y.Z` | `rilis/v1.7.0` | Cabang rilis versi baru |

### 9.2. Prosedur Wajib Sebelum Commit & Merge ke `main`

```bash
# 1. Syntax check semua modul
python -m py_compile config.py cache_engine.py month_parser.py doc_reader.py \
  ai_engine.py scraper_garuda.py scraper_sinta.py exporter.py app_ui.py \
  ai_installer.py jsfs.py log_redirect.py

# 2. Unit test fungsi kritis
python -c "
from month_parser import resolve_schedule_and_frequency, extract_months_from_archive_titles
from scraper_garuda import decode_cfemail, is_valid_indonesian_phone, canonical_journal_key
from exporter import generate_copypaste_list

# Test: sinkronisasi bulan & frekuensi
r = resolve_schedule_and_frequency('Terbit setiap Maret dan September')
assert len(r['months']) == int(r['freq_num']), f'FAIL: month-freq sync: {r}'

# Test: decode Cloudflare email
decoded = decode_cfemail('f0e1e2e3f1')
assert '@' in decoded or decoded == '', f'FAIL: cfemail decoder: {decoded}'

# Test: validasi telepon Indonesia
ok, _ = is_valid_indonesian_phone('+6281234567890', has_contact_context=True)
assert ok, 'FAIL: valid WA number rejected'
bad, _ = is_valid_indonesian_phone('+6221005162', has_contact_context=True)
assert not bad, 'FAIL: fake landline accepted'

# Test: format copy-paste list
from scraper_garuda import JournalEntry
j = JournalEntry()
j['nama'] = 'Test Jurnal'; j['peringkat'] = 'Sinta 3'; j['penerbit'] = 'Universitas Test'
j['email'] = 'test@univ.ac.id'; j['biaya_publikasi'] = 500000
j['bulan_terbit'] = [6, 12]; j['fokus_keilmuan'] = 'Teknologi Informasi'
j['link'] = 'https://jurnal.univ.ac.id'; j['link_ojs'] = 'https://jurnal.univ.ac.id'
result = generate_copypaste_list([j])
assert 'Test Jurnal' in result, 'FAIL: journal name missing from list'
assert 'test@univ.ac.id' in result, 'FAIL: email missing from list'
assert 'Rp 500.000' in result or '500' in result, 'FAIL: APC missing from list'

print('✅ Semua unit test PASS')
"

# 3. Bersihkan file .md sementara
# (Hanya VISION.md, README.md, CHANGELOG.md yang boleh tersisa)

# 4. Build executable
python -m PyInstaller --noconfirm --clean JSFS.spec

# 5. Verifikasi ukuran exe (batas keras 280 MB)
# dist/JSFS.exe size < 280 MB

# 6. Commit & tag
git add config.py version_info.txt CHANGELOG.md README.md VISION.md \
  [file-yang-diubah]
git commit -m "rilis: v1.7.0 - Deep APC Hunter Engine + Modern Bright GUI"
git tag v1.7.0
git push origin main --tags
```

### 9.3. Aturan Commit Message

Format: `<kategori>: <deskripsi singkat dalam Bahasa Indonesia>`

| Kategori | Kapan |
|----------|-------|
| `fitur:` | Menambah kapabilitas baru |
| `perbaikan:` | Memperbaiki bug |
| `revisi:` | Mengubah aturan bisnis / format |
| `ui:` | Perubahan tampilan |
| `refaktor:` | Perubahan internal tanpa ubah perilaku |
| `rilis:` | Commit rilis versi baru |
| `dokumen:` | Update README, CHANGELOG, VISION.md |

---

## Bagian 10: Roadmap v1.7.0 (Target Pengembangan Berikutnya)

Ini adalah scope resmi v1.7.0. AI tidak boleh menambah scope di luar ini tanpa persetujuan.

### 10.1. Fitur Wajib v1.7.0

#### F1: Deep APC Hunter Engine (Peningkatan Akurasi Biaya)
- [ ] Pastikan 5 titik OJS semua dikunjungi konsisten (Bagian 3.1.1)
- [ ] Perkuat Anti-OA-Trap: extend daftar frasa pembaca yang harus di-strip
- [ ] Validasi Paid Priority Rule pada edge case Submission=0 + Publication>0
- [ ] Incomplete Cache Bypass: `sumber_apc == "Kosong"` → force re-scrape

#### F2: Modern Bright GUI (Perombakan Tema Daylight)
- [ ] Ubah `appearance_mode` default dari `"dark"` ke `"light"` di `JSFSApp.__init__()`
- [ ] Implementasi Soft Slate color palette (Bagian 4.1) di seluruh komponen
- [ ] Tambahkan sakelar `☀️/🌙` di header untuk toggle mode terang/malam
- [ ] Pastikan `FitCheckBox` dan `FitSwitch` adaptif ke kedua mode

#### F3: Live KPI Counter Pills di Mode Simpel
- [ ] Pills counter real-time: `{N} Jurnal Ditemukan`, `{N} Gratis`, `{N} Sinta 2+`
- [ ] Update via `queue.Queue` — tidak boleh dari background thread langsung

#### F4: Indikator Filter Aktif pada Quick Presets
- [ ] Badge/label kecil menampilkan filter yang sedang aktif di bawah setiap preset button

### 10.2. Peningkatan Stabilitas v1.7.0

- [ ] `close_all_connections()` — verifikasi urutan cleanup yang benar di `_on_close()`
- [ ] Tambahkan guard `if not self.winfo_exists(): return` pada semua `after()` callback
- [ ] Pastikan `DualProgress.reset()` dipanggil di awal setiap run baru

### 10.3. Update Dokumentasi v1.7.0

- [ ] `CHANGELOG.md` — seksi `[1.7.0]` dengan tanggal rilis
- [ ] `README.md` — versi, fitur baru di bagian fitur utama
- [ ] `VISION.md` — update versi dokumen ke `1.7.0`

---

## Lampiran A: Daftar Konstanta Kritis (Jangan Ubah Sembarangan)

```python
# config.py
APC_SANITY_MIN_IDR = 50_000       # Threshold minimum nominal APC dianggap valid
APC_SANITY_MAX_IDR = 15_000_000   # Threshold maksimum (toleransi jurnal mahal)
USD_TO_IDR_RATE    = 17_500       # Kurs konversi tetap (update jika perlu)
CACHE_TTL_DAYS     = 7            # TTL cache (default config.py, override di cache_engine)
TURBO_SCRAPE_WORKERS = min(64, cpu*4)   # Max thread pool scraping
DEEP_SCRAPE_TIMEOUT  = (2.0, 3.5)       # (connect, read) timeout deep OJS scrape
FAST_FAIL_TIMEOUT    = (2.0, 2.5)       # Circuit breaker domain check

# app_ui.py
MAX_BATCH_SIZE = 25     # Pesan per siklus queue polling (JANGAN naikkan)
POLL_INTERVAL  = 80     # Milidetik per siklus after() (JANGAN turunkan)
MAX_LOG_LINES  = 500    # Batas buffer teks log CTkTextbox

# scraper_garuda.py
PHONE_HARD_CAP  = 2     # Maksimal nomor telepon/WA per jurnal
EMAIL_HARD_CAP  = 3     # Maksimal email redaksi per jurnal
NIM_MIN_DIGITS  = 6     # Min digit angka di username = indikasi NIM mahasiswa
TITLE_MAX_CHARS = 55    # Threshold judul terlalu panjang (indikasi judul artikel)
PHONE_CONTEXT_RADIUS = 35  # Karakter kiri untuk proximity context nomor kabel

# exporter.py AdvisorScore weights
WEIGHT_RELEVANCE = 0.40   # Relevansi kata kunci / naskah
WEIGHT_COST      = 0.25   # Biaya publikasi (gratis lebih tinggi)
WEIGHT_SPEED     = 0.20   # Kecepatan terbit (edisi terdekat)
WEIGHT_SINTA     = 0.15   # Peringkat akreditasi SINTA
```

---

## Lampiran B: Referensi Cepat Struktur `JournalEntry`

`JournalEntry` adalah dataclass di `scraper_garuda.py` yang merupakan unit data tunggal jurnal.
Semua field ini wajib ada dan maknanya tidak boleh diubah:

| Field | Tipe | Makna |
|-------|------|-------|
| `nama` / `judul` | `str` | Nama jurnal (alias saling sinkron) |
| `penerbit` | `str` | Nama institusi penerbit |
| `peringkat` | `str` | `"Sinta 1"` – `"Sinta 6"` |
| `link` / `link_ojs` | `str` | URL OJS jurnal |
| `link_sinta` | `str` | URL profil SINTA jurnal |
| `issn` / `e_issn` | `str` | ISSN cetak / elektronik |
| `email` | `str` | Email redaksi (maks 3, comma-separated) |
| `telepon` | `str` | Nomor WA/telepon (maks 2, comma-separated) |
| `whatsapp_link` | `str` | URL `https://wa.me/62...` Click-to-Chat |
| `biaya_submit` | `int\|None` | Biaya submission dalam IDR |
| `biaya_publikasi` | `int\|None` | Biaya APC/publikasi dalam IDR |
| `mata_uang` | `str` | `"IDR"` atau `"USD"` |
| `sumber_apc` | `str` | `"AI"` / `"Regex"` / `"Kosong"` |
| `fokus_keilmuan` | `str` | Focus & Scope bersih Bahasa Indonesia |
| `sumber_focus` | `str` | `"AI"` / `"Regex"` / `"SINTA/Garuda"` |
| `bulan_terbit` | `list[int]` | Daftar bulan (1–12) |
| `frekuensi` | `str` | `"2 kali setahun (Maret, September)"` |
| `edisi_terdekat` | `str` | `"November (1 bln lagi)"` / `"Sedang Terbit"` |
| `status_server` | `str` | `"Sangat Aktif"` / `"Aktif"` / `"SSL Warning"` / `"Non-Aktif"` |
| `transparansi_kontak` | `str` | `"Lengkap"` / `"Sedang"` / `"Terbatas"` / `"Tidak Transparan"` |
| `kesesuaian` | `str` | `"Sangat Sesuai"` / `"Sesuai"` / `"Cukup Sesuai"` |
| `skor` | `float` | Match score 0.0–1.0 |

---

*Dokumen ini dikelola oleh pemilik proyek JSFS. Untuk pertanyaan atau perubahan aturan,
buka diskusi sebelum mengubah kode yang terkait dengan bagian terkunci.*

*Versi dokumen ini: 1.7.0 — Berlaku mulai: 2026-11-08*
