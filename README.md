<div align="center">

<img src="assets/icon.png" alt="JSFS Logo" width="72" height="72" />

# JSFS - Journal Scraper For Students

Aplikasi desktop Windows untuk mencari jurnal SINTA, Garuda, dan internasional Q1-Q4, mengecek biaya publikasi (APC), lalu mencocokkannya dengan draf naskah Anda.

[![Release](https://img.shields.io/badge/Release-v4.3.0-2563eb?style=flat-square)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.3.0)
[![Platform](https://img.shields.io/badge/Windows-10%2F11%2064--bit-0078D6?style=flat-square&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.3.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-32d74b?style=flat-square)](LICENSE)

[![Download Installer](https://img.shields.io/badge/Download-Installer_Windows_(Setup.exe)-2563eb?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.3.0/JSFS-windows-x64-setup.exe)
[![Download Portable ZIP](https://img.shields.io/badge/Download-Portable_ZIP_(Tanpa_Instal)-059669?style=for-the-badge&logo=github&logoColor=white)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.3.0/JSFS-windows-x64-portable.zip)

[Download jsfs.exe (Single File)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.3.0/jsfs.exe) •
[SHA256SUMS.txt](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.3.0/SHA256SUMS.txt) •
[Semua Rilis](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.3.0)

</div>

## Pilihan Unduhan & Cara Mulai

| Pilihan Unduhan | Cocok Untuk | Lokasi Hasil Excel |
| :--- | :--- | :--- |
| **[Installer Windows (`JSFS-windows-x64-setup.exe`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.3.0/JSFS-windows-x64-setup.exe)** | **Direkomendasikan**: pasang rapi dengan ikon Desktop & Start Menu tanpa hak Admin | `Documents\JSFS Output` |
| **[Portabel ZIP (`JSFS-windows-x64-portable.zip`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.3.0/JSFS-windows-x64-portable.zip)** | Langsung ekstrak dan jalankan dari folder mana saja tanpa instalasi | Folder `output\` di samping `jsfs.exe` |

1. **Unduh** `JSFS-windows-x64-setup.exe` (lalu pasang seperti aplikasi biasa) atau ekstrak `JSFS-windows-x64-portable.zip`.
2. **Buka JSFS** (tidak perlu Visual C++ Redistributable maupun WebView2).
3. **Ketik topik skripsi/penelitian** Anda, lalu klik **Mulai Cari Jurnal**.

**Muncul peringatan Windows SmartScreen?** Ini wajar karena JSFS adalah proyek open-source gratis tanpa sertifikat code signing berbayar. Seluruh kode sumber terbuka dan file rilis dibangun otomatis oleh GitHub Actions. Klik **More info** lalu **Run anyway**, atau klik kanan file > **Properties** > centang **Unblock** > **OK**. Untuk memeriksa keaslian file, cocokkan hasil `Get-FileHash <namafile> -Algorithm SHA256` dengan [SHA256SUMS.txt](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.3.0/SHA256SUMS.txt).

## Apa yang Bisa Dilakukan JSFS?

- **Cari jurnal nasional & internasional sekaligus**: SINTA 1-6 dan Garuda, ditambah jurnal internasional Q1-Q4 melalui API resmi OpenAlex dan DOAJ.
- **Deteksi biaya publikasi (APC) yang akurat**: membedakan biaya publikasi asli dari klausa "free for readers" dan biaya historis, lengkap dengan filter **Gratis Rp 0 / Diamond OA** dan batas anggaran.
- **Audit keaktifan jurnal**: memeriksa tahun edisi terbaru dan otomatis menyaring jurnal dorman yang tidak terbit lebih dari 2 tahun.
- **Cocokkan draf naskah (`.docx` / `.pdf`)**: menghitung persentase kesesuaian topik (*Match Score*) secara offline, tanpa internet atau AI tambahan. Groq, Gemini, dan Ollama tersedia sebagai opsi.
- **Ekspor 1 klik**: Excel (`.xlsx`), Markdown (`.md`), CSV (`.csv`), serta template pesan siap kirim untuk WhatsApp dan email redaksi.

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
