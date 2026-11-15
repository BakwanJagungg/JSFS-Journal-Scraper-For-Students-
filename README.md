<div align="center">

<img src="assets/icon.png" alt="JSFS Logo" width="96" height="96" />

# JSFS — Journal Scraper For Students

**Pure Rust Native Desktop App untuk Pencarian, Audit Akurasi APC, dan Pencocokan Naskah ke Jurnal SINTA & Garuda**

[![Release](https://img.shields.io/github/v/release/BakwanJagungg/JSFS-Journal-Scraper-For-Students-?style=flat-square&color=0a84ff)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-32d74b?style=flat-square)](LICENSE)
[![Pure Rust](https://img.shields.io/badge/Rust-2021-orange?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![GUI: eframe + egui](https://img.shields.io/badge/GUI-Pure%20Native%20GPU%20(egui)-0a84ff?style=flat-square)](https://github.com/emilk/egui)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11%20x64-0078D6?style=flat-square&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases)

[Fitur Utama](#fitur-utama) •
[Evolusi v3 vs v4](#evolusi-arsitektur-v310-vs-v400) •
[Unduh & Jalankan](#unduh--jalankan-langsung) •
[Arsitektur](#arsitektur-4-fase) •
[Build dari Source](#build-dari-source-code)

</div>

---

## Ringkasan

**JSFS (Journal Scraper For Students) v4.0.0** adalah aplikasi desktop **100% Pure Native Rust** yang dirender langsung di GPU menggunakan **`eframe` / `egui` (`glow` OpenGL)**. Tanpa *browser wrapper*, tanpa WebView2, dan tanpa dependensi Node.js saat runtime, JSFS memindai direktori **SINTA (S1–S6)** dan **Garuda Kemdikbud**, mengaudit biaya publikasi (APC) secara langsung dari halaman OJS jurnal, mendeteksi keaktifan edisi terbit terbaru, serta mencocokkan naskah `.docx`/`.pdf` secara cerdas.

---

## Evolusi Arsitektur: v3.1.0 vs v4.0.0

| Aspek | v3.1.0 (Tauri + WebView2) | v4.0.0 (Pure Rust Native GPU) |
| :--- | :--- | :--- |
| **Rendering Antarmuka** | Edge WebView2 (`HTML` / `CSS` / `JS`) | 100% Pure Rust (`eframe` + `egui` + `glow` OpenGL) |
| **Model Proses** | Multi-proses IPC (WebView2 + Rust Backend) | **Single Native Binary** tanpa lapisan IPC browser |
| **Dependensi Runtime** | Memerlukan WebView2 Runtime | **Zero Runtime Dependencies** (cukup Windows 10/11 OpenGL) |
| **Sistem Desain** | Halaman web tunggal & modal DOM | **TwinDesk-Inspired Sidebar Workspace** + Tema Dark/Light Instan |
| **Tata Letak Navigasi** | Toolbar + modal terpisah | 5 tampilan terstruktur (`Pencarian`, `Filter Lanjutan`, `Match Engine & AI`, `Riwayat & Ekspor`, `Panduan`) |
| **Ukuran Binary Standalone** | ~9.2 MB (+ WebView2 runtime host) | **~12.1 MB** lengkap ter-embed UI, ikon & *resource manifest* DPI |

---

## Fitur Utama

### 1. Pencarian & Audit Jurnal Real-Time (`Pencarian Jurnal`)
- **Dual-Source Portal Discovery**: Mengambil daftar kandidat dari **SINTA** dan **Garuda** secara konkuren dengan *circuit breaker* per domain (`DomainBreaker`) dan *rate limiting* asinkron.
- **Zero-Trap APC Scanner**: Membedakan biaya publikasi reguler (*Article Publication Charge*) dari klausa historis (mis. bebas biaya 2008–2018), klausa gratis pembaca Open Access, dan paket *Fast-Track*. Setiap harga dilengkapi cuplikan bukti kalimat (`apc_evidence`) di modal detail.
- **Journal Activity & Recency Audit**: Memeriksa tahun terbit edisi terbaru (`last_issue_year`) dan pengumuman pada situs OJS jurnal untuk menandai jurnal aktif (`Aktif`), kurang aktif, hingga dorman (`Dorman / Tidak Terurus`).
- **Privacy-Statement Cleaner**: Membersihkan kontaminasi teks *Privacy Statement* bawaan OJS dari kolom **Focus & Scope** serta menormalkan nama penerbit secara konsisten.

### 2. Filter Lanjutan & Gate Akurasi (`Filter Lanjutan`)
- **Preset Satu Klik**: Preset cepat *Gratis Murni*, *Skripsi Ramah Budget (<= Rp 500 rb)*, dan *Semua SINTA Terakreditasi*.
- **Strict SINTA Gate & Budget Cap**: Penyaringan ulang deterministik **setelah** fase *deep crawl* dan pengayaan selesai sehingga tidak ada jurnal berbayar yang lolos pada mode gratis atau melebihi batas anggaran (`Rp min..maks`).
- **Filter Bulan Terbit & Dormancy Gate**: Saring target bulan terbit terdekat serta kecualikan jurnal tidak terurus (> 2 tahun tidak terbit) dan Garuda non-SINTA secara default.

### 3. Multi-Provider Smart Matcher (`Match Engine & AI`)
- **Built-in Smart Matcher (Zero-Setup Default)**: Mesin **Bilingual BM25 + Academic Synonym Dictionary (ID–EN)** murni Rust langsung siap pakai secara offline untuk mencocokkan naskah `.docx`/`.pdf` terhadap *Focus & Scope* jurnal.
- **Cloud AI API Opsional**: Mendukung **Groq (`llama-3.1-8b-instant`)**, **Google Gemini (`gemini-2.5-flash`)**, atau **OpenAI-Compatible URL** dengan API key pengguna.
- **Ollama Lokal (Power-User)**: Deteksi otomatis instance Ollama lokal (`http://127.0.0.1:11434`) dilengkapi pembatasan kandidat teratas, *timeout* keras, dan tombol tes koneksi langsung.

### 4. Riwayat Ekspor & Modal Detail (`Riwayat & Ekspor`)
- **4 Format Output Otomatis** di folder `output/`:
  - **Excel (`.xlsx`)**: Workbook 3 sheet berformat rapi (*freeze pane*, *auto-filter*, hyperlink aktif).
  - **Markdown (`.md`)** & **CSV UTF-8 BOM (`.csv`)** untuk catatan riset dan analisis lanjut.
  - **Draft & Daftar Share Editor (`.txt`)**: Siap salin langsung bersama template pesan WhatsApp/Email sopan di modal detail jurnal.
- **SQLite WAL Cache (14 Hari)**: Menyimpan hasil audit per kueri agar pencarian ulang berjalan instan tanpa membebani server OJS berulang kali.

---

## Unduh & Jalankan Langsung

Pengguna Windows 10/11 (64-bit) dapat langsung menjalankan JSFS tanpa menginstal Rust atau alat pengembang apa pun:

1. Buka halaman **[GitHub Releases](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases)**.
2. Pada rilis **v4.0.0**, unduh berkas **`jsfs.exe`** atau arsip **`JSFS-v4.0.0-windows-x64-portable.zip`**.
3. Letakkan `jsfs.exe` pada folder pilihan Anda (folder `.cache/`, `logs/`, dan `output/` akan dibuat otomatis di direktori kerja yang sama) lalu jalankan.

> **Catatan Windows SmartScreen**: Karena executable sumber terbuka ini belum menandatangani sertifikat komersial (*EV Code Signing*), Windows Defender SmartScreen dapat menampilkan dialog *"Windows protected your PC"*. Klik **More info** -> **Run anyway** untuk melanjutkan, atau bangun langsung dari source code di bawah untuk verifikasi mandiri.

---

## Arsitektur 4 Fase

1. **Fase 1 — Dual-Source Discovery & Pre-Deep Dedup** (`scraper/sinta.rs`, `scraper/garuda.rs`, `scraper/dedup.rs`): Mengumpulkan kandidat, menghilangkan duplikasi ISSN/URL kanonik lintas portal, dan menyaring peringkat SINTA target sebelum *deep crawl*.
2. **Fase 2 — Deep OJS Crawler** (`scraper/ojs_deep.rs`, `parser/`): Menelusuri halaman beranda, editorial policy, submission, issue archive, dan contact OJS (konkurensi `Semaphore(32)`, timeout keras `5.5s` per jurnal) untuk mengekstrak APC, kontak terdekode (termasuk Cloudflare XOR email), Focus & Scope, dan tahun edisi terbaru.
3. **Fase 3 — Multi-Provider Smart Matcher** (`parser/doc_matcher.rs`, `ai/cloud.rs`, `ai/ollama.rs`): Menghitung `Match Score` berbasis naskah dengan BM25 + sinonim akademik ID–EN, lalu opsional diperkaya dengan Cloud AI / Ollama lokal.
4. **Fase 4 — Strict Post-Filter, SQLite WAL Cache & Export** (`engine/search.rs`, `cache.rs`, `exporter/`): Menegakkan seluruh filter anggaran/SINTA/bulan/keaktifan secara final, menyimpan cache 14 hari, dan mengekspor 4 format laporan sekaligus memperbarui antarmuka native GPU via `EventSink`.

Dokumentasi teknis lengkap tersedia di **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** dan riwayat perubahan versi di **[docs/CHANGELOG.md](docs/CHANGELOG.md)**.

---

## Struktur Proyek

```text
.
├── .github/workflows/
│   └── release.yml          # CI/CD Windows otomatis (test, clippy, release portable)
├── assets/
│   ├── icon.ico             # Ikon multi-resolusi aplikasi Windows
│   └── icon.png             # Aset ikon resolusi tinggi untuk README & jendela
├── docs/
│   ├── ARCHITECTURE.md      # Penjelasan arsitektur Pure Rust v4.0.0
│   └── CHANGELOG.md         # Riwayat rilis Keep a Changelog
├── src/
│   ├── ai/                  # Cloud AI (Groq/Gemini/OpenAI-compatible) & Ollama lokal
│   ├── engine/              # Orkerstrasi pencarian, naskah, AI ops, cache & file (EventSink)
│   ├── exporter/            # Generator ekspor .xlsx, .md, .csv, .txt
│   ├── parser/              # Parser APC, bulan terbit, kontak, Focus & Scope, BM25 matcher
│   ├── scraper/             # Scraper SINTA, Garuda, OJS deep crawler & deduplikasi
│   ├── ui/                  # Antarmuka Pure Native GPU (theme, sidebar, 5 view & modal)
│   ├── cache.rs             # SQLite WAL mode cache (TTL 14 hari)
│   ├── config.rs            # Konstanta versi & direktori runtime
│   ├── error.rs             # Tipe error terstruktur (thiserror)
│   ├── models.rs            # Model data & parameter filter
│   ├── lib.rs               # Modul pustaka & unit test core
│   └── main.rs              # Entry point Windows Subsystem + ikon & DPI PerMonitorV2
├── build.rs                 # Integrasi ikon & VersionInfo Win32 via winres
├── Cargo.toml               # Konfigurasi crate Pure Rust & profil rilis LTO
├── Cargo.lock               # Lockfile dependensi teruji
├── .gitignore               # Pengecualian target/, cache, log & hasil ekspor
└── LICENSE                  # Lisensi MIT
```

---

## Build dari Source Code

### Prasyarat
- **Rust Toolchain** terbaru (`rustc` >= 1.75 / Edition 2021, termasuk `cargo` dan `clippy`) via [rustup.rs](https://rustup.rs/)
- Pada Windows: **MSVC Build Tools** (*Desktop development with C++* / Windows SDK untuk linking resource `winres`)

### Langkah Build & Verifikasi
```bash
# 1. Clone repositori
git clone https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-.git
cd JSFS-Journal-Scraper-For-Students-

# 2. Jalankan seluruh unit test (30 test backend & UI state)
cargo test --locked

# 3. Verifikasi bebas warning dengan Clippy
cargo clippy --all-targets --locked -- -D warnings

# 4. Build executable rilis teroptimasi (output: target/release/jsfs.exe)
cargo build --release --locked
```

---

## Lisensi

Proyek ini dirilis di bawah lisensi **MIT** — lihat berkas [LICENSE](LICENSE) untuk informasi selengkapnya.
