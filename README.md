# JSFS — Journal Scraper For Students

[![Release](https://img.shields.io/github/v/release/BakwanJagungg/JSFS-Journal-Scraper-For-Students-?style=flat-square&color=2563eb)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)](LICENSE)
[![Built with Tauri v2](https://img.shields.io/badge/Tauri-v2-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6?style=flat-square&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases)

**JSFS (Journal Scraper For Students)** adalah aplikasi desktop native berkecepatan tinggi berbasis **Rust + Tauri v2** untuk membantu mahasiswa, dosen, dan peneliti mencari, memfilter, dan mengevaluasi jurnal ilmiah terakreditasi **SINTA (S1–S6)** serta **Garuda Kemdikbud** secara cepat, akurat, dan bebas *freeze*.

---

## Fitur Utama

### 1. Mesin Scraping Asinkron Berkecepatan Tinggi
- **Dual-Source Portal Discovery**: Mengumpulkan daftar jurnal dari **SINTA** dan **Garuda** secara konkuren dengan *rate-limiting* dan *circuit breaker* per domain.
- **Filter-Before-Deep & Pre-Deep Deduplication**: Menyaring duplikasi ISSN/URL dan memvalidasi peringkat SINTA sebelum melakukan *deep crawl* ke situs OJS penerbit.
- **Deep OJS Crawler**: Mengekstrak informasi **Biaya Publikasi (APC)**, **Bulan & Jadwal Terbit**, **Kontak Editor (Email / WhatsApp)**, serta **Focus & Scope** langsung dari halaman OJS jurnal.

### 2. AI Semantic Match Score (Opsional via Ollama Lokal)
- **Analisis Kesesuaian Naskah (`.docx` / `.pdf`)**: Unggah draf artikel untuk menghitung skor kecocokan terhadap *Focus & Scope* jurnal menggunakan pencocokan leksikal + **Ollama Local LLM**.
- **Privasi Terjamin**: Seluruh pemrosesan AI berjalan 100% lokal di perangkat tanpa mengirim naskah ke layanan cloud pihak ketiga.
- **Graceful Degradation**: Jika Ollama belum terpasang atau tidak aktif, aplikasi tetap berjalan normal dengan skor berbasis pencocokan teks.

### 3. Ekspor Multi-Format Siap Pakai
Hasil pencarian otomatis tersimpan di folder `output/` dalam 4 format:
- **Excel (`.xlsx`)**: Workbook 3 sheet terformat rapi (*freeze pane*, *auto-filter*, pembungkus teks, dan tautan aktif).
- **Markdown (`.md`)**: Ringkasan laporan yang mudah dibaca dan disalin ke catatan riset.
- **CSV (`.csv`)**: Format mentah (UTF-8 BOM) untuk analisis data lanjutan.
- **Daftar Share / Draft Email Editor (`.txt`)**: Siap salin langsung dari antarmuka aplikasi.

### 4. Antarmuka Desktop Native Ringan
- Dibangun dengan **Tauri v2 + WebView2** (~9.2 MB binary, konsumsi memori ~37 MB saat idle).
- Streaming progres *real-time* berbasis *event-driven IPC*, paginasi tabel responsif hingga 1.000 jurnal, dan cache lokal **SQLite (WAL mode)**.

---

## Unduh & Jalankan Langsung (Tanpa Build)

Bagi pengguna yang ingin langsung menggunakan aplikasi tanpa menginstal Rust atau Node.js:

1. Buka halaman **[Releases](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases)**.
2. Pada rilis terbaru (**v3.0.0**), unduh salah satu paket berikut:
   - **Setup Installer Windows** (`*_x64-setup.exe` atau `*.msi`) — disarankan untuk instalasi standar.
   - **Portable Executable** (`jsfs.exe`) — dapat langsung dijalankan tanpa instalasi.
3. *(Opsional)* Instal [Ollama](https://ollama.com/) jika ingin mengaktifkan fitur **AI Semantic Match Score** secara lokal.

---

## Arsitektur Teknologi

| Komponen | Teknologi |
| :--- | :--- |
| **Desktop Shell** | Tauri v2 (`tauri`, `WebView2`) |
| **Backend & Concurrency** | Rust (Edition 2021), `tokio`, `futures-util`, `dashmap` |
| **HTTP & HTML Parsing** | `reqwest` (`rustls-tls`, `gzip`, `hickory-dns`), `scraper`, `regex` |
| **Local Cache** | SQLite WAL Mode (`rusqlite` bundled) |
| **Document Parsing** | `zip` + `quick-xml` (`.docx`), `pdf-extract` (`.pdf`) |
| **Export Engine** | `rust_xlsxwriter` (`.xlsx`), native writers (`.md`, `.csv`, `.txt`) |
| **Frontend UI** | Vanilla HTML5, CSS3, ES6+ (`ui/`) |

---

## Struktur Proyek

```text
.
├── .github/workflows/          # GitHub Actions CI/CD rilis Windows otomatis
├── docs/
│   ├── ARCHITECTURE.md         # Penjelasan arsitektur teknis Rust + Tauri v2
│   └── CHANGELOG.md            # Catatan perubahan versi rilis
├── src-tauri/
│   ├── capabilities/           # Konfigurasi izin Tauri v2
│   ├── icons/                  # Ikon aplikasi
│   ├── src/
│   │   ├── ai/                 # Integrasi Ollama lokal & circuit breaker AI
│   │   ├── commands/           # Tauri IPC command handlers
│   │   ├── exporter/           # Generator ekspor .xlsx, .md, .csv, .txt
│   │   ├── parser/             # Ekstraktor APC, kontak, bulan terbit, fokus, doc matcher
│   │   ├── scraper/            # Scraper SINTA, Garuda, OJS Deep, & deduplikasi
│   │   ├── cache.rs            # SQLite WAL cache engine
│   │   ├── config.rs           # Konfigurasi & direktori runtime
│   │   ├── lib.rs              # Registrasi Tauri builder & state
│   │   └── main.rs             # Entry point aplikasi desktop
│   ├── Cargo.toml              # Dependensi & profil rilis Rust
│   └── tauri.conf.json         # Konfigurasi jendela & bundling installer
├── ui/                         # Antarmuka frontend (HTML, CSS, JS)
├── package.json                # Skrip npm untuk dev & build Tauri
├── Jalankan_JSFS_v3.bat        # Peluncur cepat Windows
├── .gitignore                  # Pengecualian file build, cache, dan log
└── LICENSE                     # Lisensi MIT
```

Untuk detail teknis alur data dan modul internal, lihat **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** dan riwayat rilis di **[docs/CHANGELOG.md](docs/CHANGELOG.md)**.

---

## Build dari Source Code

### Prasyarat
- **Rust Toolchain** (`rustc` & `cargo` terbaru) — [rustup.rs](https://rustup.rs/)
- **Node.js** (v18+) — [nodejs.org](https://nodejs.org/)
- **Microsoft Edge WebView2 Runtime** (bawaan Windows 10/11)

### Langkah Instalasi & Build
```bash
# 1. Clone repositori
git clone https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-.git
cd JSFS-Journal-Scraper-For-Students-

# 2. Instal dependensi CLI
npm install

# 3. Jalankan mode pengembangan (live hot-reload)
npm run tauri dev

# 4. Build installer & executable produksi
npm run tauri build
# atau hanya standalone executable tanpa installer:
npx tauri build --no-bundle
```

### Pengujian & Linting
```bash
cd src-tauri
cargo test
cargo clippy --all-targets -- -D warnings
```

---

## Kontribusi & Lisensi

Kontribusi, laporan bug, dan permintaan fitur sangat terbuka melalui *Issues* maupun *Pull Requests*.  
Proyek ini dirilis di bawah lisensi **MIT** — lihat berkas [LICENSE](LICENSE) untuk detail selengkapnya.
