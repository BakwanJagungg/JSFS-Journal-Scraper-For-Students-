# JSFS v3.0.0 — Journal Scraper For Students

**JSFS (Journal Scraper For Students)** adalah aplikasi desktop native berkecepatan tinggi berbasis **Rust + Tauri v2** yang dirancang untuk membantu mahasiswa, dosen, dan peneliti menemukan jurnal ilmiah terakreditasi **SINTA (S1–S6)** dan **Garuda Kemdikbud** secara cepat, akurat, dan bebas *freeze*.

---

## Fitur Utama

### 1. Mesin Scraping Asinkron Berkecepatan Tinggi (Rust + Tokio)
- **Dual-Source Portal Discovery**: Mengumpulkan data jurnal dari **SINTA** dan **Garuda** secara konkuren dengan *rate-limiting* dan *circuit breaker* per domain (`DomainBreaker`).
- **Pre-Deep Deduplication & Filter-Before-Deep**: Menyaring duplikasi ISSN/URL dan memvalidasi kriteria peringkat SINTA sebelum melakukan *deep crawl* ke situs OJS penerbit.
- **Deep OJS Crawler (`ojs_deep.rs`)**: Mengekstrak informasi **Biaya Publikasi (APC)**, **Bulan Terbit**, **Kontak Editor / Email / WhatsApp**, **Focus & Scope**, serta **Jadwal Terbit** langsung dari halaman OJS jurnal.

### 2. AI Semantic Match Score (Opsional via Ollama Lokal)
- **Analisis Kesesuaian Naskah (`.docx` / `.pdf`)**: Unggah draf artikel Anda dan JSFS akan menghitung skor kecocokan terhadap *Focus & Scope* jurnal menggunakan kombinasi pencocokan leksikal + **Ollama Local LLM**.
- **Privasi Terjamin**: Seluruh proses AI berjalan 100% lokal di perangkat Anda melalui Ollama tanpa mengirim naskah ke cloud pihak ketiga.
- **Graceful Degradation**: Jika Ollama belum terpasang atau tidak aktif, aplikasi tetap berjalan normal dengan skor berbasis pencocokan teks dan menyediakan tombol instalasi/peluncuran otomatis di UI.

### 3. Ekspor Multi-Format Siap Pakai
Setiap hasil pencarian otomatis diekspor ke folder `output/` dalam 4 format:
- **Excel (`.xlsx`)**: Tabel terformat rapi dengan *freeze pane*, *auto-filter*, pembungkus teks, dan tautan URL aktif (`rust_xlsxwriter`).
- **Markdown (`.md`)**: Ringkasan laporan yang mudah dibaca dan disalin ke catatan riset.
- **CSV (`.csv`)**: Format mentah untuk analisis data lanjutan.
- **Daftar Share / Draft Email Editor (`.txt`)**: Siap salin langsung dari antarmuka aplikasi.

### 4. Antarmuka Desktop Native Ringan (< 40 MB RAM)
- Dibangun dengan **Tauri v2** + **WebView2** (tanpa membundel Chromium penuh seperti Electron).
- Streaming progres *real-time* via Tauri Events, paginasi tabel yang responsif hingga 1.000 jurnal, dan notifikasi suara lembut saat proses selesai.

---

## Arsitektur Teknologi

| Komponen | Teknologi |
| :--- | :--- |
| **Desktop Shell** | Tauri v2 (`tauri`, `WebView2`) |
| **Backend & Concurrency** | Rust (Edition 2021), `tokio`, `futures-util`, `dashmap` |
| **HTTP & HTML Parsing** | `reqwest` (rustls-tls, gzip, cookies, hickory-dns), `scraper`, `regex` |
| **Local Cache** | SQLite Mode WAL (`rusqlite` bundled) |
| **Document Parsing** | `zip` + `quick-xml` (`.docx`), `pdf-extract` (`.pdf`) |
| **Export Engine** | `rust_xlsxwriter` (`.xlsx`), native writers (`.md`, `.csv`, `.txt`) |
| **Frontend UI** | Vanilla HTML5, CSS3, ES6+ (`ui/`) |

---

## Struktur Direktori

```text
.
├── ui/                         # Antarmuka frontend (HTML, CSS, JS, favicon)
├── src-tauri/
│   ├── src/
│   │   ├── ai/                 # Integrasi Ollama lokal & circuit breaker AI
│   │   ├── commands/           # Tauri IPC commands (search, manuscript, ai_ops, file_ops)
│   │   ├── exporter/           # Generator ekspor .xlsx, .md, .csv, .txt
│   │   ├── parser/             # Ekstraktor APC, kontak, bulan terbit, fokus, doc matcher
│   │   ├── scraper/            # Scraper SINTA, Garuda, OJS Deep, & deduplikasi
│   │   ├── cache.rs            # SQLite WAL cache engine
│   │   ├── config.rs           # Konfigurasi & direktori runtime
│   │   ├── lib.rs              # Registrasi Tauri builder & state
│   │   └── main.rs             # Entry point aplikasi desktop
│   ├── capabilities/           # Konfigurasi izin Tauri v2
│   ├── Cargo.toml              # Dependensi & profil rilis Rust
│   └── tauri.conf.json         # Konfigurasi jendela & bundling Tauri
├── package.json                # Skrip npm untuk dev & build Tauri
├── Jalankan_JSFS_v3.bat        # Peluncur cepat Windows
└── .gitignore                  # Pengecualian file build, cache, dan log
```

---

## Cara Menjalankan & Build

### Prasyarat
1. **Rust Toolchain** (`rustc` & `cargo` versi terbaru) — [rustup.rs](https://rustup.rs/)
2. **Node.js** (v18 atau lebih baru) — [nodejs.org](https://nodejs.org/)
3. **Microsoft Edge WebView2 Runtime** (sudah terpasang secara bawaan di Windows 10/11).
4. *(Opsional)* **Ollama** — [ollama.com](https://ollama.com/) untuk fitur *AI Semantic Match Score*.

### 1. Clone & Instalasi Dependensi
```bash
git clone https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-.git
cd JSFS-Journal-Scraper-For-Students-
npm install
```

### 2. Mode Pengembangan (Live Hot-Reload)
```bash
npm run tauri dev
```

### 3. Build Rilis Produksi (`.exe`)
Untuk menghasilkan file biner standalone `src-tauri/target/release/jsfs.exe` (atau installer NSIS dengan `npm run tauri build`):
```bash
npm run tauri build
# atau hanya standalone executable tanpa installer:
npx tauri build --no-bundle
```

Setelah selesai di-build, Anda dapat menjalankan aplikasi langsung dengan mengklik ganda **`Jalankan_JSFS_v3.bat`** atau menjalankan `src-tauri/target/release/jsfs.exe`.

---

## Pengujian & Linting (Rust)
```bash
cd src-tauri
cargo test
cargo clippy --all-targets -- -D warnings
```

---

## Lisensi
Proyek ini dilisensikan di bawah **MIT License** — lihat berkas [LICENSE](LICENSE) untuk detail selengkapnya.
Copyright © 2026 **Dava Algafiri Sidiq**.
