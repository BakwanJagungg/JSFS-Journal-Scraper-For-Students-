<div align="center">

<img src="assets/icon.png" alt="JSFS Logo" width="72" height="72" />

# JSFS - Journal Scraper For Students

Aplikasi desktop Windows untuk mencari jurnal SINTA, Garuda, dan internasional Q1-Q4, mengecek biaya publikasi (APC), mencocokkannya dengan draf naskah Anda, hingga menyiapkan berkas pengajuan ke editor.

[![Release](https://img.shields.io/badge/Release-v4.12.1-2563eb?style=flat-square)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.12.1)
[![Platform](https://img.shields.io/badge/Windows-10%2F11%2064--bit-0078D6?style=flat-square&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.12.1)
[![License: MIT](https://img.shields.io/badge/License-MIT-32d74b?style=flat-square)](LICENSE)

[![Download Installer](https://img.shields.io/badge/Download-Installer_Windows_(Setup.exe)-2563eb?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.12.1/JSFS-windows-x64-setup.exe)
[![Download Portable ZIP](https://img.shields.io/badge/Download-Portable_ZIP_(Tanpa_Instal)-059669?style=for-the-badge&logo=github&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.12.1/JSFS-windows-x64-portable.zip)

[Download jsfs.exe (Single File)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.12.1/jsfs.exe) •
[SHA256SUMS.txt](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.12.1/SHA256SUMS.txt) •
[Semua Rilis](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.12.1)

</div>

## Pilihan Unduhan & Cara Mulai

| Pilihan Unduhan | Cocok Untuk | Lokasi Hasil Excel |
| :--- | :--- | :--- |
| **[Installer Windows (`JSFS-windows-x64-setup.exe`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.12.1/JSFS-windows-x64-setup.exe)** | **Direkomendasikan**: pasang rapi dengan ikon Desktop & Start Menu tanpa hak Admin | `Documents\JSFS Output` |
| **[Portabel ZIP (`JSFS-windows-x64-portable.zip`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.12.1/JSFS-windows-x64-portable.zip)** | Langsung ekstrak dan jalankan dari folder mana saja tanpa instalasi | Folder `output\` di samping `jsfs.exe` |

**Perintah Cepat PowerShell (Unduh & Jalankan Installer v4.12.1):**

```powershell
$p="$env:TEMP\JSFS-windows-x64-setup.exe"; Invoke-WebRequest "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.12.1/JSFS-windows-x64-setup.exe" -OutFile $p; Start-Process $p
```

1. **Unduh** `JSFS-windows-x64-setup.exe` (lalu pasang seperti aplikasi biasa) atau ekstrak `JSFS-windows-x64-portable.zip`.
2. **Buka JSFS** (tidak perlu Visual C++ Redistributable maupun WebView2).
3. **Ketik topik skripsi/penelitian** atau **Unggah Naskah** (`.docx` / `.pdf`), lalu klik **Mulai Cari Jurnal**.

**Muncul peringatan Windows SmartScreen?** Ini wajar karena JSFS adalah proyek open-source gratis tanpa sertifikat code signing berbayar. Seluruh kode sumber terbuka dan file rilis dibangun otomatis oleh GitHub Actions. Klik **More info** lalu **Run anyway**, atau klik kanan file > **Properties** > centang **Unblock** > **OK**. Untuk memeriksa keaslian file, cocokkan hasil `Get-FileHash <namafile> -Algorithm SHA256` dengan [SHA256SUMS.txt](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.12.1/SHA256SUMS.txt).

## Apa yang Bisa Dilakukan JSFS?

- **Cari jurnal nasional & internasional sekaligus**: SINTA 1-6 dan Garuda, ditambah jurnal internasional Q1-Q4 melalui API resmi **OpenAlex**, **DOAJ**, dan **Crossref**.
- **Deteksi biaya publikasi (APC) & mata uang seragam**: membedakan biaya publikasi asli (termasuk total pajak pada halaman `editorialPolicies`) dari biaya submit Rp 0, klausa denda penarikan naskah (*withdrawal fee*), serta klausa "free for readers", lengkap dengan filter **Gratis Rp 0 / Diamond OA**, batas anggaran, dan konversi **IDR / USD**.
- **Proteksi anti-blokir & validasi integritas akademik**: *rate-limiter* per domain dengan jeda acak, auto-update redirect OJS 2 -> OJS 3, normalisasi tautan web ke beranda root OJS, validasi checksum ISSN ISO 3297, deteksi domain kloning/mencurigakan, dan pemeriksaan indeks tambahan (DOAJ, Scopus, Copernicus, Crossref, Google Scholar).
- **Asisten naskah & topik cerdas (offline)**: unggah `.docx` / `.pdf` untuk mengekstrak chip topik klik-pakai, menghitung *Match Score* BM25 dwibahasa, mengecek kesesuaian batas kata & gaya sitasi (*Pre-Flight Fit Check*), serta mencocokkan *Cite-to-Target* dari daftar pustaka Anda.
- **Paket Pengajuan `.docx` & `.zip` 1-Klik**: simpan profil penulis (Nama, Afiliasi, Email, ORCID, Program Studi), unduh dokumen Word siap edit (`.docx` Cover Letter & Surat Keaslian bermaterai), atau ekspor bundel **`.zip`** beserta pengingat kalender **`.ics`**.
- **Ekspor & Kolaborasi Dosen Pembimbing**: tab **Shortlist & Dospen** untuk membandingkan kandidat favorit, melacak progres pengiriman, memicu draf pesan WhatsApp dan email redaksi, serta ekspor ke Excel (`.xlsx`), Markdown (`.md`), CSV (`.csv`), dan Zotero/Mendeley (`.ris` & `.bib`).

## Baru di Rilis v4.12.1

- **Persistensi Permanen Shortlist Favorit (`★`)**: Seluruh jurnal favorit yang ditandai bintang `★` tersimpan permanen ke `.cache/saved_shortlist.json` dan langsung muncul kembali saat aplikasi dibuka tanpa perlu mengulang pencarian.
- **Onboarding Cepat Pemula & Produktivitas Naskah**: 3 kartu aksi cepat beranda kosong (*Target Skripsi S1*, *Jurnal Gratis Rp 0*, *Unggah Naskah Word/PDF*), tombol 1-klik cari topik naskah teratas, dan penanda riwayat `[Dilihat]`.
- **Pintasan Keyboard Global & Tampilan Adaptif**: Pintasan cepat `Ctrl+K` (fokus cari), `Esc` (bersihkan/tutup), `Ctrl+E` (buka file Excel terakhir), toggle kepadatan tabel **Padat / Lega**, serta jaring pengaman horizontal scrollbar untuk laptop 1366×768 (scaling 125%).
- **Etika Publikasi & Generator Dokumen Resmi (.docx)**: Peringatan pencegahan submisi ganda (*Double-Submission Guard*), generator otomatis **Matriks Tanggapan Reviewer (.docx)** untuk status *Revision*, serta **Lembar Persetujuan Dosbing (.docx)**.
- **Pemeriksa Update GitHub & Cadangan Data Lokal**: Notifikasi pembaruan otomatis yang aman (*panic-safe*), alur kerja bernomor di sidebar (1. Cari Jurnal, 2. Detail & Naskah, 3. Shortlist & Dospen), serta pencadangan data lokal (`.json`).

Antarmuka tersedia dalam **Bahasa Indonesia** dan **English** dengan tema gelap maupun terang (menu **Pengaturan**).

## Kebutuhan Sistem

- Windows 10 atau Windows 11 (64-bit)
- RAM minimal 2 GB
- Ruang penyimpanan sekitar 15 MB
- Koneksi internet untuk pencarian jurnal

JSFS dirancang ringan untuk laptop spesifikasi rendah: tidak memakai CPU saat diam, jumlah koneksi paralel otomatis dikurangi pada prosesor 4 core atau kurang, dan cache SQLite dibatasi sekitar 4 MB.

## Kompilasi dari Kode Sumber

Membutuhkan [Rust](https://rustup.rs/) dan MSVC Build Tools (Desktop development with C++).

```bash
git clone https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-.git && cd JSFS-Journal-Scraper-For-Students-
cargo build --release --locked
```

Hasil build berada di `target/release/jsfs.exe`. Jalankan `cargo test --locked` untuk menjalankan seluruh unit test.

Dokumentasi teknis: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) • Riwayat versi: [docs/CHANGELOG.md](docs/CHANGELOG.md)

## Lisensi

Dirilis di bawah lisensi [MIT](LICENSE) oleh BakwanJagungg.
