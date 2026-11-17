<div align="center">

<img src="assets/icon.png" alt="JSFS Logo" width="72" height="72" />

# JSFS - Journal Scraper For Students

Aplikasi desktop Windows untuk mencari jurnal SINTA, Garuda, dan internasional Q1-Q4, mengecek biaya publikasi (APC), mencocokkannya dengan draf naskah Anda, hingga menyiapkan berkas pengajuan ke editor.

[![Release](https://img.shields.io/badge/Release-v4.13.0-2563eb?style=flat-square)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.13.0)
[![Platform](https://img.shields.io/badge/Windows-64--bit%20%26%2032--bit-0078D6?style=flat-square&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.13.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-32d74b?style=flat-square)](LICENSE)

[![Download Installer](https://img.shields.io/badge/Download-Installer_Windows_x64_(Setup.exe)-2563eb?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/JSFS-windows-x64-setup.exe)
[![Download Portable ZIP](https://img.shields.io/badge/Download-Portable_ZIP_x64-059669?style=for-the-badge&logo=github&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/JSFS-windows-x64-portable.zip)
[![Download Portable x86](https://img.shields.io/badge/Download-Portable_ZIP_x86_(32--bit)-0284c7?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/JSFS-windows-x86-portable.zip)

[Download jsfs.exe (Single Binary x64)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/jsfs.exe) •
[SHA256SUMS.txt](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/SHA256SUMS.txt) •
[Semua Rilis](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases) •
[Catatan Riwayat Rilis (CHANGELOG.md)](docs/CHANGELOG.md)

</div>

## Pilihan Unduhan & Cara Mulai

| Pilihan Unduhan | Arsitektur | Keterangan & Lokasi Hasil |
| :--- | :---: | :--- |
| **[Installer Windows (`JSFS-windows-x64-setup.exe`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/JSFS-windows-x64-setup.exe)** | 64-bit | **Direkomendasikan**: pasang dengan ikon Desktop dan Start Menu tanpa hak Admin. Output: `Documents\JSFS Output` |
| **[Portabel ZIP 64-bit (`JSFS-windows-x64-portable.zip`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/JSFS-windows-x64-portable.zip)** | 64-bit | Ekstrak dan jalankan langsung tanpa instalasi. Output: folder `output\` di samping `jsfs.exe` |
| **[Portabel ZIP 32-bit (`JSFS-windows-x86-portable.zip`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/JSFS-windows-x86-portable.zip)** | 32-bit | Khusus perangkat laptop atau Windows 10/11 32-bit. Output: folder `output\` di samping `jsfs.exe` |
| **[Single Binary EXE (`jsfs.exe`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/jsfs.exe)** | 64-bit | Satu file biner mandiri tanpa kompresi zip |
| **[Checksum SHA-256 (`SHA256SUMS.txt`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/SHA256SUMS.txt)** | Semua | Verifikasi integritas hash seluruh paket unduhan |

**Perintah Cepat PowerShell (Unduh & Jalankan Installer v4.13.0):**

```powershell
$p="$env:TEMP\JSFS-windows-x64-setup.exe"; Invoke-WebRequest "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/JSFS-windows-x64-setup.exe" -OutFile $p; Start-Process $p
```

1. **Unduh** berkas installer setup atau portabel zip sesuai arsitektur Windows Anda.
2. **Buka JSFS** (tidak membutuhkan runtime eksternal seperti Visual C++ Redistributable maupun WebView2).
3. **Ketik topik penelitian** atau **Unggah Naskah** (`.docx` / `.pdf`), lalu klik **Mulai Cari Jurnal**.

**Muncul peringatan Windows SmartScreen?** Ini wajar karena JSFS adalah proyek open source gratis tanpa sertifikat code signing berbayar. Seluruh kode sumber terbuka dan file rilis dibangun otomatis oleh GitHub Actions. Klik **More info** lalu **Run anyway**, atau klik kanan file > **Properties** > centang **Unblock** > **OK**. Untuk memeriksa keaslian file, cocokkan hasil `Get-FileHash <namafile> -Algorithm SHA256` dengan [SHA256SUMS.txt](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.13.0/SHA256SUMS.txt).

## Apa yang Bisa Dilakukan JSFS?

- **Cari jurnal nasional & internasional sekaligus**: SINTA 1-6 dan Garuda, ditambah jurnal internasional Q1-Q4 melalui API resmi **OpenAlex**, **DOAJ**, dan **Crossref**.
- **Deteksi biaya publikasi (APC) & mata uang seragam**: membedakan biaya publikasi asli (termasuk total pajak pada halaman `editorialPolicies`) dari biaya submit Rp 0, klausa denda penarikan naskah (*withdrawal fee*), serta klausa "free for readers", lengkap dengan filter **Gratis Rp 0 / Diamond OA**, batas anggaran, dan konversi **IDR / USD**.
- **Proteksi anti-blokir & validasi integritas akademik**: *rate-limiter* per domain dengan jeda acak, auto-update redirect OJS 2 -> OJS 3, normalisasi tautan web ke beranda root OJS, validasi checksum ISSN ISO 3297, deteksi domain kloning/mencurigakan, dan pemeriksaan indeks tambahan (DOAJ, Scopus, Copernicus, Crossref, Google Scholar).
- **Asisten naskah & topik cerdas (offline)**: unggah `.docx` / `.pdf` untuk mengekstrak chip topik klik-pakai, menghitung *Match Score* BM25 dwibahasa, mengecek kesesuaian batas kata & gaya sitasi (*Pre-Flight Fit Check*), serta mencocokkan *Cite-to-Target* dari daftar pustaka Anda.
- **Paket Pengajuan `.docx` & `.zip` 1-Klik**: simpan profil penulis (Nama, Afiliasi, Email, ORCID, Program Studi), unduh dokumen Word siap edit (`.docx` Cover Letter & Surat Keaslian bermaterai), atau ekspor bundel **`.zip`** beserta pengingat kalender **`.ics`**.
- **Ekspor & Kolaborasi Dosen Pembimbing**: tab **Shortlist & Dospen** untuk membandingkan kandidat favorit, melacak progres pengiriman, memicu draf pesan WhatsApp dan email redaksi, serta ekspor ke Excel (`.xlsx`), Markdown (`.md`), CSV (`.csv`), dan Zotero/Mendeley (`.ris` & `.bib`).

## Baru di Rilis v4.13.0

- **Dukungan Penuh Windows 32-bit (x86)**: Dukungan kompilasi resmi untuk arsitektur target i686-pc-windows-msvc berdampingan dengan x86_64-pc-windows-msvc.
- **Paket Portabel Khusus 32-bit**: Rilis portabel zip JSFS-windows-x86-portable.zip yang ringan untuk laptop spesifikasi rendah dan perangkat Windows 32-bit.
- **Konsumsi Memori Sangat Hemat**: Penggunaan RAM hanya sekitar 107 MB pada Windows 32-bit dan 117 MB pada Windows 64-bit dengan beban CPU idle nol persen.
- **Persistensi Permanen Shortlist Favorit**: Seluruh jurnal berbintang tersimpan otomatis ke penyimpanan lokal dan langsung dimuat kembali saat aplikasi dibuka.
- **Onboarding Cepat dan Dokumen Revisi Word**: Kartu aksi cepat beranda kosong, generator Word docx untuk matriks tanggapan reviewer, dan lembar persetujuan dosen pembimbing.

> Riwayat perubahan lengkap untuk seluruh versi (v3.0.0 sampai v4.13.0) dapat dilihat di dokumen **[docs/CHANGELOG.md](docs/CHANGELOG.md)**.

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
