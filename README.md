<div align="center">

<img src="assets/icon.png" alt="JSFS Logo" width="80" height="80" />

# JSFS & JSFS+ Workstation

**Cari jurnal ilmiah yang tepat untuk skripsi atau tesis Anda, cek biaya publikasi asli, dan ekspor daftar siap kirim ke dosen pembimbing — semuanya dalam satu aplikasi desktop gratis.**

[![Standard Release](https://img.shields.io/badge/JSFS_Standard-v4.16.3-2563eb?style=flat-square)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.16.3)
[![Workstation Release](https://img.shields.io/badge/JSFS%2B_Workstation-plus_v5.0.1-7c3aed?style=flat-square)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/plus-v5.0.1)
[![Platforms](https://img.shields.io/badge/Platforms-Windows_x64%2Fx86_%7C_Linux_%7C_macOS-0284c7?style=flat-square)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases)
[![Pure Rust](https://img.shields.io/badge/Built_with-Pure_Rust_(egui)-f97316?style=flat-square&logo=rust&logoColor=white)](docs/ARCHITECTURE.md)
[![License: MIT](https://img.shields.io/badge/License-MIT-16a34a?style=flat-square)](LICENSE)

<br />

<a href="https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/v4.16.3">
  <img src="https://img.shields.io/badge/Unduh_JSFS_Standard-v4.16.3-2563eb?style=for-the-badge&logo=windows&logoColor=white" alt="Download JSFS Standard v4.16.3" />
</a>
<a href="https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/plus-v5.0.1">
  <img src="https://img.shields.io/badge/Unduh_JSFS%2B_Workstation-plus_v5.0.1-7c3aed?style=for-the-badge&logo=rust&logoColor=white" alt="Download JSFS+ Workstation plus-v5.0.1" />
</a>

<p>
  <a href="https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases">Semua Rilis</a> •
  <a href="docs/CHANGELOG.md">Changelog Standard</a> •
  <a href="docs/CHANGELOG-PLUS.md">Changelog JSFS+</a> •
  <a href="docs/ARCHITECTURE.md">Arsitektur Teknis</a>
</p>

</div>

## Tampilan Antarmuka Nyata

<table>
  <tr>
    <td align="center" width="50%">
      <strong>1. Cari Jurnal &amp; Filter Biaya APC Real-Time</strong><br /><br />
      <img src="assets/preview-search.png" alt="Halaman 1. Cari Jurnal JSFS" width="100%" />
    </td>
    <td align="center" width="50%">
      <strong>2. Detail Jurnal, Estimasi Terbit &amp; Kesiapan Naskah</strong><br /><br />
      <img src="assets/preview-detail.png" alt="Halaman 2. Detail dan Naskah JSFS" width="100%" />
    </td>
  </tr>
</table>

## Pilih Edisi yang Sesuai Laptop Anda

<table>
  <tr>
    <th align="left" width="50%">JSFS Standard (v4.16.3) — Ringan untuk Semua Laptop</th>
    <th align="left" width="50%">JSFS+ Workstation (plus-v5.0.1) — Khusus Laptop Spek Tinggi</th>
  </tr>
  <tr>
    <td valign="top">
      Cocok untuk laptop perkuliahan apa pun — Windows 64/32-bit, Linux, maupun macOS Apple Silicon atau Intel. Ringan di memori dan tetap nyaman dipakai seharian.
      <ul>
        <li>Hemat memori (RAM di bawah 500 MB), tidak bikin laptop lemot</li>
        <li>Mencari hingga <strong>1.000 jurnal per sesi</strong> dari SINTA, Garuda, dan internasional Q1 hingga Q4</li>
        <li>Tetap bisa mengakses website jurnal kampus meskipun sertifikat HTTPS-nya sudah kedaluwarsa, dan bisa membaca biaya publikasi langsung dari dokumen PDF</li>
        <li>Mengenali format daftar pustaka <strong>IEEE, APA, Vancouver, dan Chicago/Turabian</strong> secara otomatis</li>
        <li>Pembaruan otomatis tanpa perlu unduh ulang secara manual</li>
      </ul>
    </td>
    <td valign="top">
      Dirancang untuk laptop atau PC spek tinggi (RAM 16 hingga 32 GB) dan dapat dipasang berdampingan dengan JSFS Standard tanpa konflik.
      <ul>
        <li>Kapasitas hingga <strong>5.000 jurnal per sesi</strong> dengan kecepatan tinggi berkat banyak proses paralel</li>
        <li>Menembus halaman jurnal yang diproteksi anti-bot (misalnya Cloudflare) menggunakan browser bawaan aplikasi</li>
        <li>Membaca teks dari PDF hasil scan gambar (OCR) tanpa perlu pasang software tambahan</li>
        <li>Pencocokan makna abstrak lebih mendalam menggunakan AI lokal opsional</li>
        <li>Menampilkan tren sitasi jurnal berdasarkan data internasional terkini</li>
      </ul>
    </td>
  </tr>
</table>

## Unduhan Resmi

### JSFS Standard (v4.16.3)

| Platform &amp; Paket | Arsitektur | Keterangan |
| :--- | :---: | :--- |
| **[Windows Installer (`JSFS-windows-x64-setup.exe`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.16.3/JSFS-windows-x64-setup.exe)** | Windows x64 | **Direkomendasikan**: pasang dengan ikon Desktop &amp; Start Menu, tidak perlu hak Admin. Hasil simpan di `Documents\JSFS Output` |
| **[Windows Portable (`JSFS-windows-x64-portable.zip`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.16.3/JSFS-windows-x64-portable.zip)** | Windows x64 | Ekstrak lalu langsung jalankan, tanpa proses instalasi. Hasil simpan di folder `output\` di samping `jsfs.exe` |
| **[Windows 32-bit Portable (`JSFS-windows-x86-portable.zip`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.16.3/JSFS-windows-x86-portable.zip)** | Windows x86 | Khusus laptop atau PC Windows 10/11 32-bit. Hasil simpan di folder `output\` di samping `jsfs.exe` |
| **[Windows Single Binary (`jsfs.exe`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.16.3/jsfs.exe)** | Windows x64 | Satu file siap pakai tanpa perlu ekstrak |

<details><summary>Lihat Unduhan Linux, macOS &amp; Verifikasi SHA256SUMS</summary>

| Platform &amp; Paket | Arsitektur | Keterangan |
| :--- | :---: | :--- |
| **[Linux Desktop (`JSFS-linux-x64.tar.gz`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.16.3/JSFS-linux-x64.tar.gz)** | Linux x86_64 | Ekstrak arsip lalu jalankan `./jsfs` (mendukung X11 dan Wayland) |
| **[macOS Apple Silicon (`JSFS-macos-arm64-preview.tar.gz`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.16.3/JSFS-macos-arm64-preview.tar.gz)** | macOS arm64 | Build pratinjau native Apple Silicon (ekstrak lalu jalankan `./jsfs`) |
| **[macOS Intel (`JSFS-macos-x64-preview.tar.gz`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.16.3/JSFS-macos-x64-preview.tar.gz)** | macOS x86_64 | Build pratinjau native Mac Intel (ekstrak lalu jalankan `./jsfs`) |
| **[Verifikasi Integritas (`SHA256SUMS.txt`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v4.16.3/SHA256SUMS.txt)** | Semua Platform | Daftar hash SHA-256 resmi untuk memverifikasi keaslian file unduhan |

</details>

### JSFS+ Workstation (plus-v5.0.1)

| Platform &amp; Paket | Arsitektur | Keterangan |
| :--- | :---: | :--- |
| **[Windows Plus Installer (`JSFS-Plus-windows-x64-setup.exe`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/plus-v5.0.1/JSFS-Plus-windows-x64-setup.exe)** | Windows x64 | Terpasang terpisah di folder `JSFS-Plus`, aman berdampingan dengan JSFS Standard |
| **[Windows Plus Portable (`JSFS-Plus-windows-x64-portable.zip`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/plus-v5.0.1/JSFS-Plus-windows-x64-portable.zip)** | Windows x64 | Ekstrak lalu langsung jalankan `jsfs-plus.exe` |
| **[Windows Plus Single Binary (`jsfs-plus.exe`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/plus-v5.0.1/jsfs-plus.exe)** | Windows x64 | Satu file biner mandiri edisi Workstation |

<details><summary>Lihat Unduhan Linux, macOS &amp; Verifikasi SHA256SUMS</summary>

| Platform &amp; Paket | Arsitektur | Keterangan |
| :--- | :---: | :--- |
| **[Linux Plus Desktop (`JSFS-Plus-linux-x64.tar.gz`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/plus-v5.0.1/JSFS-Plus-linux-x64.tar.gz)** | Linux x86_64 | Ekstrak arsip lalu jalankan `./jsfs-plus` |
| **[macOS Plus Apple Silicon (`JSFS-Plus-macos-arm64-preview.tar.gz`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/plus-v5.0.1/JSFS-Plus-macos-arm64-preview.tar.gz)** | macOS arm64 | Build pratinjau Apple Silicon edisi Workstation |
| **[macOS Plus Intel (`JSFS-Plus-macos-x64-preview.tar.gz`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/plus-v5.0.1/JSFS-Plus-macos-x64-preview.tar.gz)** | macOS x86_64 | Build pratinjau Mac Intel edisi Workstation |
| **[Verifikasi Integritas Plus (`SHA256SUMS.txt`)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/plus-v5.0.1/SHA256SUMS.txt)** | Semua Platform | Daftar hash SHA-256 resmi untuk memverifikasi keaslian file unduhan |

</details>

## Apa Saja yang Bisa Dilakukan JSFS?

- **Cari sekaligus dari banyak sumber**: Satu klik mencari di SINTA (peringkat 1 hingga 6), Garuda, OpenAlex, DOAJ, dan Crossref secara bersamaan — tidak perlu buka banyak tab browser.
- **Pisahkan biaya nyata dari yang palsu**: Tampilkan biaya publikasi (APC) yang sesungguhnya, abaikan label "gratis submit" yang sebenarnya berbayar saat diterima, dan baca biaya langsung dari dokumen PDF jurnal.
- **Tahu berapa lama naskah bakal terbit**: Estimasi waktu tunggu dari submit sampai terbit ditampilkan langsung, sehingga Anda bisa merencanakan jadwal sidang lebih akurat.
- **Periksa kesiapan naskah Word atau PDF Anda**: Audit struktur IMRAD, periksa batas jumlah kata, dan pastikan format daftar pustaka sesuai gaya jurnal target (IEEE, APA, Vancouver, atau Chicago).
- **Salin 5 referensi terbaru jurnal pilihan**: Satu tombol langsung menyalin daftar referensi terkini dari jurnal tersebut ke clipboard Anda.
- **Ekspor paket siap kirim ke dosen pembimbing**: Buat daftar kurasi jurnal, ringkasan HTML, Cover Letter, Surat Keaslian, hingga arsip `.zip` lengkap — semua dalam format yang rapi dan siap dilampirkan ke email.
- **Pembaruan otomatis per kanal**: JSFS Standard memperbarui dirinya sendiri via kanal `v*`, sedangkan JSFS+ Workstation via kanal `plus-v*`, keduanya terverifikasi keasliannya secara otomatis.

## Panduan Singkat Keamanan OS (SmartScreen &amp; Gatekeeper)

> **Windows SmartScreen**: Karena JSFS adalah proyek open-source gratis tanpa sertifikat tanda tangan digital berbayar, Windows Defender SmartScreen mungkin menampilkan peringatan saat pertama kali dijalankan. Klik **More info** (Info lebih lanjut) lalu pilih **Run anyway** (Tetap jalankan).

> **macOS Gatekeeper (Build Pratinjau)**: Untuk build pratinjau macOS yang belum melalui proses notarisasi Apple, jalankan perintah berikut di Terminal pada folder hasil ekstrak sebelum membuka aplikasi:
> ```bash
> xattr -d com.apple.quarantine ./jsfs && chmod +x ./jsfs && ./jsfs
> ```

<details><summary>Untuk Pengembang: Kompilasi dari Kode Sumber (Rust)</summary>

Membutuhkan [Rust toolchain](https://rustup.rs/) versi stabil terbaru. Pada Linux Debian/Ubuntu, pasang paket desktop minimal terlebih dahulu:

```bash
sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev pkg-config
```

Bangun edisi yang diinginkan dari sumber:

```bash
git clone https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-.git
cd JSFS-Journal-Scraper-For-Students-

# Membangun JSFS Standard:
cargo build --release --locked --no-default-features --bin jsfs

# Membangun JSFS+ Workstation:
cargo build --release --locked --bin jsfs-plus --features plus
```

</details>

## Lisensi

Dirilis di bawah lisensi [MIT](LICENSE) oleh BakwanJagungg.
