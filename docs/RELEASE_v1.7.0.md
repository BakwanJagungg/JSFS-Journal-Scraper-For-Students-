# 🎓 JSFS v1.7.0 — Deep APC Hunter Engine & Modern Bright GUI

Selamat datang di rilis **JSFS v1.7.0**! Pembaruan ini membawa lompatan terbesar dalam akurasi data setingkat kurasi manusia (*Human-Parity Accuracy*), antarmuka baru **Soft Slate Daylight**, metrik langsung *Live KPI Counter Pills*, tombol 1-klik *Salin Daftar WhatsApp*, dan eliminasi total kebocoran email/nomor telepon palsu.

---

### 📦 Berkas Biner Portabel (Siap Pakai, Tanpa Python)

| Berkas | Sistem Operasi | Ukuran | SHA-256 Checksum |
|---|---|---|---|
| **[JSFS.exe](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download/v1.7.0/JSFS.exe)** | Windows 10 / 11 (64-bit) | ~275 MB | `D0E78ABEE52878C495007C756DA3D99E2BEFBD41E89936EC3FF82352EF8760E6` |

> [!TIP]
> Cukup unduh `JSFS.exe` dan klik dua kali untuk menjalankan. Tidak memerlukan instalasi Python, Git, atau dependensi eksternal apapun!

---

### 🌟 Fitur Baru & Peningkatan Utama

#### 1. 🎯 Deep APC Hunter Engine
- **Multi-Path OJS 2 & OJS 3 Crawler**: Memindai 5 titik target secara konsisten: `/about/submissions`, `/about/editorialPolicies` (kritis untuk OJS 2 institusi lama), `/about`, Sidebar/Custom Blocks, dan *Dynamic Menu Anchor Discovery*.
- **Anti-OA-Trap**: Menghapus kalimat retorika Open Access pembaca (*"freely available to the public"*, *"immediate open access"*) sebelum pendeteksian biaya gratis untuk mencegah 100% *false-positive*.
- **Paid Priority Rule**: Nominal biaya publikasi berbayar selalu menang mutlak meskipun pendaftaran berbiaya Rp 0.
- **Incomplete Cache Auto-Bypass**: Entri cache dengan status APC belum terdata otomatis di-bypass untuk deep scrape ulang saat opsi Deep Scrape aktif.

#### 2. ☀️ Modern Bright GUI ("Soft Slate Daylight")
- **Light Mode Default**: Tampilan visual ramah mata dengan palet slate daylight (`#f8fafc` / `#1e3a5f`).
- **Theme Switcher (☀️/🌙)**: Sakelar instan antara Mode Terang dan Mode Gelap di header.
- **Live KPI Counter Pills**: 4 pil indikator metrik dinamis (*Total Ditemukan*, *Gratis (Rp 0)*, *Berbayar*, *Terbit Dekat*) yang diperbarui secara thread-safe.
- **1-Click WA Share Button**: Tombol aksi cepat *"📋 Salin Daftar (Share WA)"* di action bar utama.

#### 3. 🛡️ Forensic Data Purity & Anti-Noise
- **Anti-Email Flood (Maks 3 Email)**: Mengeliminasi total email mahasiswa (`@student.*`, `@mhs.*`, dll.) dan NIM (>= 6 digit).
- **Anti-Fake Phone (Maks 2 Telepon/WA)**: Memvalidasi nomor telepon kabel dengan 200+ whitelist kode area Telkom Indonesia dan menolak deretan angka acak dari DOI.
- **Focus & Scope Bersih**: Menghapus derau tabel profil OJS dan menyintesis bidang keilmuan secara elegan dari rumpun SINTA/Garuda.
- **Smart Canonical Deduplication**: Menggabungkan jurnal migran/duplikat menjadi 1 entri utuh.

---

### 📋 Contoh Output Format Share WhatsApp (`_LIST.txt`)

```text
1. Jurnal Manajemen dan Bisnis (Sinta 3 - Universitas Muhammadiyah Surakarta)
Email: benefit@ums.ac.id, redaksi.benefit@ums.ac.id
Telepon/WA: +6281234567890
Biaya Publikasi: Rp 500.000 | Terbit: Juni, Desember
Fokus: Riset ilmiah di bidang Manajemen Strategis, Bisnis Syariah, Pemasaran, dan Kewirausahaan.
Link OJS: https://journals.ums.ac.id/index.php/benefit
```

---

### 💬 Masukan & Pelaporan Kendala
Jika Anda menemukan website OJS jurnal yang gagal discrape atau ingin mengusulkan fitur baru, silakan buka tiket di **[GitHub Issues](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/issues)**.

