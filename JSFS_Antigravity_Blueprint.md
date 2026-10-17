Saya baca dulu semua modul yang ada sebelum menulis blueprint — biar instruksi ke Antigravity akurat dan tidak konflik dengan kode yang sudah berjalan.Semua modul sudah terbaca. Saya catat poin-poin kritis untuk Antigravity sebelum menulis:

- `scrape_garuda()` dan `scrape_sinta()` menggunakan `rich.console.Console()` untuk log — ini harus di-redirect ke Queue GUI
- `run()` di `jsfs.py` adalah orchestrator utama yang bisa dipanggil langsung dari thread
- `exporter.py` juga pakai `rich.Console` — perlu wrapper yang sama
- `jsfs.py` (CLI) tidak disentuh, hanya ditambah `app_ui.py` sebagai entry point baru

Sekarang tulis blueprint-nya.Semua konteks sudah terkumpul. Menulis blueprint sekarang.I'm a planning agent - I can read and analyze code but not modify it. I can help you plan the implementation instead.

Karena saya tidak bisa menulis file langsung, saya berikan seluruh konten `JSFS_Antigravity_Blueprint.md` di bawah ini. Antigravity tinggal copy-paste ke file tersebut.

---

```markdown
# JSFS_Antigravity_Blueprint.md
# Dokumen Arsitektur & Instruksi Implementasi — JSFS Fase 1.5 (GUI Desktop)

**Disiapkan oleh:** Kiro (Lead Architect)
**Ditujukan kepada:** Antigravity (Builder / Eksekutor)
**Versi:** 1.0
**Tanggal:** 2026

---

## DAFTAR ISI

1. [Executive Summary](#1-executive-summary)
2. [Tech Stack & Strategy](#2-tech-stack--strategy)
3. [UI/UX Layout Plan](#3-uiux-layout-plan)
4. [Struktur File yang Diharapkan](#4-struktur-file-yang-diharapkan)
5. [Instruksi Eksekusi untuk Antigravity](#5-instruksi-eksekusi-untuk-antigravity)

---

## 1. EXECUTIVE SUMMARY

### Konteks Proyek

JSFS (Journal Scraper For Students) adalah alat bantu mahasiswa untuk mencari jurnal terakreditasi SINTA secara otomatis dari Portal Garuda dan direktori SINTA. MVP v1.0 sudah berjalan sebagai skrip CLI (`jsfs.py`) dan terbukti mampu melakukan scraping serta mengekspor hasil ke Markdown, CSV, dan Excel.

### Tujuan Fase 1.5

Fase ini adalah **transisi dari CLI ke Desktop Application**. Tiga tujuan utama:

| # | Tujuan | Keterangan |
|---|--------|------------|
| 1 | **Hilangkan CLI** | Pengguna tidak perlu buka terminal sama sekali |
| 2 | **UI ramah mahasiswa** | Cukup isi keyword, centang checkbox, klik tombol |
| 3 | **Distribusi sebagai `.exe`** | File tunggal yang bisa langsung dijalankan di Windows tanpa instalasi Python |

### Yang Tidak Berubah

Antigravity **TIDAK perlu menyentuh** file-file berikut:
- `config.py`
- `scraper_garuda.py`
- `scraper_sinta.py`
- `exporter.py`
- `jsfs.py` (tetap ada sebagai fallback CLI)

Seluruh logika scraping yang sudah berjalan **dipakai ulang apa adanya** — GUI hanya membungkusnya.

---

## 2. TECH STACK & STRATEGY

### 2.1 GUI Framework — CustomTkinter ✅ WAJIB

**Gunakan `customtkinter` versi `5.2.x`.**

| Kriteria | CustomTkinter | Tkinter Standar | PyQt6 |
|----------|:---:|:---:|:---:|
| Tampilan modern / dark mode | ✅ | ❌ | ✅ |
| Ringan (tidak butuh Qt runtime) | ✅ | ✅ | ❌ |
| Ukuran `.exe` yang dihasilkan | ~30MB | ~25MB | ~80MB+ |
| Cocok untuk satu file `app_ui.py` | ✅ | ❌ | ❌ |
| Kurva belajar untuk builder | Rendah | Rendah | Tinggi |

CustomTkinter adalah pilihan tepat karena ringan, menghasilkan tampilan gelap yang modern, dan semua widget-nya adalah drop-in replacement dari Tkinter standar — tidak perlu belajar framework baru.

**Instalasi:**
```bash
pip install customtkinter==5.2.2
```

**Dark mode default:**
```python
import customtkinter as ctk
ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("blue")
```

---

### 2.2 Concurrency — `threading` ⚠️ WAJIB, TIDAK BOLEH DIABAIKAN

> **⚠️ PERINGATAN KERAS UNTUK ANTIGRAVITY:**
>
> Fungsi scraping (`scrape_garuda`, `scrape_sinta`, `scrape_sinta_selenium`) adalah operasi **blocking** — masing-masing bisa memakan waktu 10–60 detik karena melakukan HTTP request dan menunggu respons server.
>
> Jika fungsi ini dipanggil **langsung dari main thread GUI**, seluruh jendela aplikasi akan **freeze / "Not Responding"** selama proses berjalan. Pengguna tidak bisa mengklik apapun, dan Windows akan menampilkan notifikasi "Program Not Responding".
>
> **Ini adalah bug fatal yang tidak bisa ditoleransi.**

**Solusi wajib: jalankan semua fungsi scraping di thread terpisah menggunakan `threading.Thread`.**

Pola yang harus diimplementasikan:

```python
import threading

def _start_scraping_thread(self):
    """Dipanggil oleh tombol Scrape. Meluncurkan thread scraping."""
    # Nonaktifkan tombol agar tidak bisa diklik dua kali
    self.btn_scrape.configure(state="disabled", text="⏳ Scraping...")
    
    # Buat dan jalankan thread — JANGAN panggil scraper langsung di sini
    thread = threading.Thread(target=self._run_scraping_task, daemon=True)
    thread.start()

def _run_scraping_task(self):
    """Berjalan di background thread. Berisi logika scraping sesungguhnya."""
    try:
        # ... panggil scraper di sini ...
        pass
    finally:
        # Kembalikan state tombol ke UI thread via after()
        self.after(0, self._on_scraping_done)
```

**Aturan threading yang harus dipatuhi:**

1. **JANGAN** update widget Tkinter/CTk langsung dari background thread — ini menyebabkan crash atau race condition
2. **GUNAKAN** `self.after(0, callback)` untuk setiap update UI yang dipicu dari background thread
3. **GUNAKAN** `queue.Queue` untuk mengirim pesan log dari scraper ke UI (detail di Langkah 2)
4. Set `daemon=True` pada thread agar otomatis mati ketika window ditutup

---

### 2.3 Build Tool — PyInstaller

**Gunakan PyInstaller versi `6.x`.**

```bash
pip install pyinstaller==6.11.0
```

**Perintah build final (akan dijelaskan detail di Langkah 3):**

```bash
pyinstaller \
  --onefile \
  --noconsole \
  --name "JSFS" \
  --icon "assets/icon.ico" \
  app_ui.py
```

Flag kritis yang **WAJIB** disertakan:

| Flag | Fungsi | Kenapa Wajib |
|------|--------|--------------|
| `--onefile` | Kemas semua jadi satu `.exe` | Distribusi mudah — cukup kirim satu file |
| `--noconsole` | Sembunyikan jendela terminal hitam | Tanpa ini, akan ada jendela CMD kosong yang mengganggu |
| `--name "JSFS"` | Nama file output | Agar hasilnya `JSFS.exe`, bukan `app_ui.exe` |
| `--icon` | Ikon aplikasi | Opsional tapi direkomendasikan untuk kesan profesional |

**Catatan penting untuk Antigravity:**

- PyInstaller **tidak kompatibel** dengan Selenium WebDriver Manager secara out-of-the-box. Jika mode Selenium diaktifkan, ChromeDriver perlu di-bundle terpisah menggunakan `--add-binary`. Alternatif: nonaktifkan opsi Selenium di build `.exe` dan tampilkan pesan "Selenium tidak tersedia di mode exe, gunakan jsfs.py".
- Pastikan virtual environment **aktif** saat menjalankan PyInstaller agar semua dependency tertangkap dengan benar.

---

## 3. UI/UX LAYOUT PLAN

### 3.1 Spesifikasi Window

```
Ukuran    : 800 x 640 pixel (tidak resizable)
Mode      : Dark (default)
Font      : "Segoe UI" atau "Roboto" — CTkFont(size=13)
Theme     : Blue (default CustomTkinter)
```

### 3.2 Sketsa Layout (ASCII Wireframe)

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│   📚  JSFS — Journal Scraper For Students          [v1.5 GUI]      │
│   ─────────────────────────────────────────────────────────────    │
│                                                                     │
│   Keyword Pencarian                                                 │
│   ┌──────────────────────────────────────────────┐  ┌──────────┐  │
│   │  bisnis, e-commerce, consumer behavior...    │  │  🔍 CARI │  │
│   └──────────────────────────────────────────────┘  └──────────┘  │
│                                                                     │
│   ─── Sumber Data ──────────────────────────────────────────────   │
│   [✓] Garuda      [✓] SINTA                                        │
│                                                                     │
│   ─── Filter Peringkat SINTA ───────────────────────────────────   │
│   [✓] Sinta 3    [✓] Sinta 4    [✓] Sinta 5    [✓] Sinta 6       │
│   [ ] Sinta 1    [ ] Sinta 2                                        │
│                                                                     │
│   ─── Format Output ────────────────────────────────────────────   │
│   [✓] Markdown (.md)    [✓] Excel (.xlsx)    [ ] CSV (.csv)        │
│                                                                     │
│   ─── Opsi Lanjutan ────────────────────────────────────────────   │
│   [ ] Gunakan Selenium (bypass JS/Cloudflare — butuh Chrome)       │
│                                                                     │
│   ┌─────────────────────────── Log / Progress ──────────────────┐  │
│   │ [13:42:01] 🔍 Memulai scraping Portal Garuda...             │  │
│   │ [13:42:02] 📄 Halaman 1 → garuda.kemdiktisaintek.go.id      │  │
│   │ [13:42:04]    ✓ Ditemukan 12 jurnal di halaman 1            │  │
│   │ [13:42:06] 📄 Halaman 2 → ...                               │  │
│   │ [13:42:09] ✅ Selesai! 24 jurnal ditemukan.                  │  │
│   │ [13:42:09] 💾 Tersimpan → output\hasil_pencarian_jsfs.md     │  │
│   │ [13:42:09] 💾 Tersimpan → output\hasil_pencarian_jsfs.xlsx   │  │
│   │                                                              │  │
│   └──────────────────────────────────────────────────────────── ┘  │
│                                                                     │
│   ┌─────────────────────────────────────────────────────────────┐  │
│   │          🚀  MULAI SCRAPING                                  │  │
│   └─────────────────────────────────────────────────────────────┘  │
│                                                                     │
│   ──────────────────────────────────────────────────────────────   │
│   Output disimpan di folder: output/        [📂 Buka Folder]       │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.3 Spesifikasi Tiap Elemen

| Elemen | Widget CTk | ID / Nama Variabel | Keterangan |
|--------|-----------|-------------------|------------|
| Judul header | `CTkLabel` | `lbl_title` | Font size 18, bold, warna accent |
| Input keyword | `CTkEntry` | `entry_keyword` | Placeholder "bisnis, e-commerce..." |
| Checkbox Garuda | `CTkCheckBox` | `chk_garuda` | Default: ON |
| Checkbox SINTA | `CTkCheckBox` | `chk_sinta` | Default: ON |
| Checkbox Sinta 3–6 | `CTkCheckBox` x4 | `chk_s3` – `chk_s6` | Default: semua ON |
| Checkbox Sinta 1–2 | `CTkCheckBox` x2 | `chk_s1`, `chk_s2` | Default: OFF |
| Checkbox MD | `CTkCheckBox` | `chk_fmt_md` | Default: ON |
| Checkbox Excel | `CTkCheckBox` | `chk_fmt_excel` | Default: ON |
| Checkbox CSV | `CTkCheckBox` | `chk_fmt_csv` | Default: OFF |
| Toggle Selenium | `CTkCheckBox` | `chk_selenium` | Default: OFF |
| Log box | `CTkTextbox` | `txt_log` | Read-only, font monospace, auto-scroll |
| Tombol Scrape | `CTkButton` | `btn_scrape` | Lebar penuh, warna hijau/biru saat idle |
| Label output path | `CTkLabel` | `lbl_output_path` | Tampilkan path folder output |
| Tombol buka folder | `CTkButton` | `btn_open_folder` | Kecil, di samping label path |

### 3.4 State Management Tombol

```
Idle       → teks "🚀 MULAI SCRAPING", warna hijau, state="normal"
Running    → teks "⏳ Scraping... (klik untuk batalkan)", state="normal"
            (klik saat running = trigger cancel via threading.Event)
Done/Error → kembali ke Idle otomatis via self.after(0, ...)
```

---

## 4. STRUKTUR FILE YANG DIHARAPKAN

### 4.1 Pemetaan File Lama → Baru

```
JSFS/
│
│  ── FILE LAMA (JANGAN DIMODIFIKASI) ──────────────────────────────
├── config.py              ← Tidak berubah. Diimport oleh scraper.
├── scraper_garuda.py      ← Tidak berubah. JournalEntry dataclass ada di sini.
├── scraper_sinta.py       ← Tidak berubah.
├── exporter.py            ← Tidak berubah.
├── jsfs.py                ← Tidak berubah. Tetap bisa dipakai via CLI.
├── requirements.txt       ← DITAMBAH: customtkinter, pyinstaller
│
│  ── FILE BARU (TUGAS ANTIGRAVITY) ────────────────────────────────
├── app_ui.py              ← ⭐ Entry point GUI. File utama yang dibuild ke .exe
├── log_redirect.py        ← Helper: redirect rich.Console output ke Queue
│
│  ── ASET & BUILD ────────────────────────────────────────────────
├── assets/
│   └── icon.ico           ← Ikon aplikasi (buat/cari ikon buku, opsional)
├── JSFS.spec              ← Auto-generated oleh PyInstaller (jangan diedit manual)
│
│  ── OUTPUT (auto-created saat runtime) ──────────────────────────
└── output/
    ├── hasil_pencarian_jsfs.md
    ├── hasil_pencarian_jsfs.csv
    └── hasil_pencarian_jsfs.xlsx
```

### 4.2 Tanggung Jawab Tiap File Baru

**`app_ui.py`** — Main GUI Application
- Mendefinisikan class `JSFSApp(ctk.CTk)`
- Membangun semua widget (layout dari Section 3)
- Mengambil nilai dari semua checkbox/entry saat tombol diklik
- Meluncurkan `threading.Thread` ke fungsi scraping
- Membaca dari `queue.Queue` setiap 100ms via `self.after(100, _poll_log_queue)`
- Memanggil `export_markdown`, `export_csv`, `export_excel` dari `exporter.py`

**`log_redirect.py`** — Log Interceptor
- Membuat custom `io.StringIO`-like sink yang menangkap output `rich.Console`
- Memasukkan setiap baris teks ke `queue.Queue` yang dibagikan ke `app_ui.py`
- Ini diperlukan karena `scraper_garuda.py` dan `scraper_sinta.py` menggunakan `rich.Console` untuk logging — output itu harus diarahkan ke GUI bukan ke terminal

---

## 5. INSTRUKSI EKSEKUSI UNTUK ANTIGRAVITY

### Langkah 0 — Persiapan Environment

```bash
# Buat virtual environment baru (direkomendasikan)
python -m venv venv
venv\Scripts\activate   # Windows

# Install semua dependency termasuk yang baru
pip install customtkinter==5.2.2 pyinstaller==6.11.0

# Atau update requirements.txt lalu:
pip install -r requirements.txt
```

Tambahkan dua baris berikut ke `requirements.txt`:
```
customtkinter==5.2.2
pyinstaller==6.11.0
```

---

### Langkah 1 — Buat `log_redirect.py` (Log Interceptor)

File ini adalah jembatan antara output teks dari `rich.Console` (yang dipakai scraper) dengan widget `CTkTextbox` di GUI.

**Konsep:** Buat subclass `io.StringIO` yang intercept method `write()`. Setiap kali scraper menulis log (mis: `"📄 Halaman 1 → ..."`) teks itu dimasukkan ke `queue.Queue`. GUI polling queue tersebut setiap 100ms.

Implementasikan `log_redirect.py` dengan struktur ini:

```python
# log_redirect.py
import queue
import io
import re
from rich.console import Console


class QueueWriter(io.TextIOBase):
    """
    Custom IO writer yang meneruskan setiap baris teks ke queue.
    Digunakan sebagai file= argument pada rich.Console.
    """
    def __init__(self, log_queue: queue.Queue):
        self.queue = log_queue

    def write(self, text: str) -> int:
        # Bersihkan escape code ANSI dari rich sebelum masuk queue
        clean = re.sub(r'\x1b\[[0-9;]*m', '', text)
        clean = clean.strip()
        if clean:
            self.queue.put(clean)
        return len(text)

    def flush(self):
        pass


def make_queue_console(log_queue: queue.Queue) -> Console:
    """
    Buat instance rich.Console yang output-nya diarahkan ke queue,
    bukan ke terminal.
    
    Gunakan console ini sebagai pengganti console = Console() di scraper
    dengan cara meneruskannya sebagai argumen ke fungsi scraping.
    """
    writer = QueueWriter(log_queue)
    return Console(file=writer, highlight=False, markup=True)
```

> **Catatan implementasi:** `scraper_garuda.py` dan `scraper_sinta.py` sudah menginstansiasi `console = Console()` di level modul. Cara paling bersih untuk redirect tanpa memodifikasi scraper adalah dengan **monkey-patch** variabel `console` di masing-masing modul **sebelum** fungsi scraping dipanggil:
>
> ```python
> # Di dalam app_ui.py, sebelum memanggil scraper:
> import scraper_garuda
> import scraper_sinta
> import exporter
>
> queue_console = make_queue_console(self.log_queue)
> scraper_garuda.console = queue_console
> scraper_sinta.console = queue_console
> exporter.console = queue_console
> ```
>
> Ini aman karena assignment ke module-level variable di Python bersifat thread-safe untuk tipe immutable reference.

---

### Langkah 2 — Buat `app_ui.py` (Main GUI)

Bangun `app_ui.py` mengikuti urutan ini:

#### 2a. Skeleton Class

```python
# app_ui.py
import customtkinter as ctk
import threading
import queue
import os
import subprocess
from datetime import datetime

import scraper_garuda
import scraper_sinta
import exporter
from log_redirect import make_queue_console
from config import DEFAULT_SINTA_RANKS

ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("blue")


class JSFSApp(ctk.CTk):
    def __init__(self):
        super().__init__()
        self.title("JSFS — Journal Scraper For Students")
        self.geometry("800x640")
        self.resizable(False, False)

        self.log_queue: queue.Queue = queue.Queue()
        self._cancel_event = threading.Event()

        self._build_ui()
        self._poll_log_queue()   # mulai polling loop

    def _build_ui(self):
        # ... implementasi semua widget dari Section 3.3 ...
        pass

    def _poll_log_queue(self):
        """
        Polling queue setiap 100ms di main thread.
        Ambil semua pesan yang ada dan tambahkan ke txt_log.
        WAJIB menggunakan self.after() — jangan pakai loop blocking.
        """
        try:
            while True:
                msg = self.log_queue.get_nowait()
                timestamp = datetime.now().strftime("%H:%M:%S")
                self.txt_log.configure(state="normal")
                self.txt_log.insert("end", f"[{timestamp}] {msg}\n")
                self.txt_log.see("end")         # auto-scroll ke bawah
                self.txt_log.configure(state="disabled")
        except queue.Empty:
            pass
        finally:
            # Jadwalkan polling berikutnya — ini menciptakan "loop" tanpa blocking
            self.after(100, self._poll_log_queue)

    def _on_scrape_clicked(self):
        """Handler tombol Scrape. Validasi input, lalu launch thread."""
        keyword = self.entry_keyword.get().strip()
        if not keyword:
            self._log_ui("⚠️  Keyword tidak boleh kosong.")
            return
        if not self.chk_garuda.get() and not self.chk_sinta.get():
            self._log_ui("⚠️  Pilih minimal satu sumber (Garuda atau SINTA).")
            return

        # Reset cancel event dan nonaktifkan tombol
        self._cancel_event.clear()
        self.btn_scrape.configure(state="disabled", text="⏳ Scraping...")

        # Redirect console scraper ke queue GUI
        queue_console = make_queue_console(self.log_queue)
        scraper_garuda.console = queue_console
        scraper_sinta.console = queue_console
        exporter.console = queue_console

        # Kumpulkan parameter dari UI
        params = {
            "keyword": keyword,
            "use_garuda": bool(self.chk_garuda.get()),
            "use_sinta": bool(self.chk_sinta.get()),
            "sinta_ranks": self._get_selected_ranks(),
            "use_selenium": bool(self.chk_selenium.get()),
            "formats": self._get_selected_formats(),
        }

        thread = threading.Thread(
            target=self._scraping_task,
            args=(params,),
            daemon=True,
        )
        thread.start()

    def _scraping_task(self, params: dict):
        """
        Berjalan di background thread.
        JANGAN update widget Tkinter di sini secara langsung.
        Gunakan self.after(0, ...) untuk semua callback ke UI.
        """
        from scraper_garuda import scrape_garuda, JournalEntry
        from scraper_sinta import scrape_sinta, scrape_sinta_selenium
        from exporter import export_markdown, export_csv, export_excel

        all_results: list[JournalEntry] = []

        try:
            if params["use_garuda"]:
                results = scrape_garuda(params["keyword"], params["sinta_ranks"])
                all_results.extend(results)

            if params["use_sinta"]:
                if params["use_selenium"]:
                    results = scrape_sinta_selenium(params["keyword"], params["sinta_ranks"])
                else:
                    results = scrape_sinta(params["keyword"], params["sinta_ranks"])
                all_results.extend(results)

            # Deduplikasi
            seen = set()
            unique = []
            for j in all_results:
                key = j.nama_jurnal.lower().strip()
                if key and key not in seen:
                    seen.add(key)
                    unique.append(j)

            self.log_queue.put(f"📦 Total unik: {len(unique)} jurnal")

            # Export
            exported = []
            if "md" in params["formats"]:
                p = export_markdown(unique, params["keyword"], params["sinta_ranks"])
                exported.append(p)
            if "csv" in params["formats"]:
                p = export_csv(unique)
                exported.append(p)
            if "excel" in params["formats"]:
                p = export_excel(unique, params["keyword"])
                exported.append(p)

            self.log_queue.put(f"✅ Selesai! {len(unique)} jurnal ditemukan.")
            for p in exported:
                self.log_queue.put(f"💾 Tersimpan → {p}")

        except Exception as e:
            self.log_queue.put(f"❌ Error: {e}")

        finally:
            # Kembalikan state tombol ke main thread
            self.after(0, self._on_scraping_done)

    def _on_scraping_done(self):
        """Dipanggil dari main thread setelah scraping selesai/error."""
        self.btn_scrape.configure(state="normal", text="🚀 MULAI SCRAPING")

    def _get_selected_ranks(self) -> list[int]:
        ranks = []
        rank_map = {1: self.chk_s1, 2: self.chk_s2, 3: self.chk_s3,
                    4: self.chk_s4, 5: self.chk_s5, 6: self.chk_s6}
        for rank, chk in rank_map.items():
            if chk.get():
                ranks.append(rank)
        return ranks if ranks else DEFAULT_SINTA_RANKS

    def _get_selected_formats(self) -> list[str]:
        fmts = []
        if self.chk_fmt_md.get():    fmts.append("md")
        if self.chk_fmt_csv.get():   fmts.append("csv")
        if self.chk_fmt_excel.get(): fmts.append("excel")
        return fmts if fmts else ["md"]

    def _log_ui(self, msg: str):
        """Tambahkan pesan langsung ke log box (dari main thread)."""
        timestamp = datetime.now().strftime("%H:%M:%S")
        self.txt_log.configure(state="normal")
        self.txt_log.insert("end", f"[{timestamp}] {msg}\n")
        self.txt_log.see("end")
        self.txt_log.configure(state="disabled")

    def _open_output_folder(self):
        """Buka folder output di File Explorer."""
        output_path = os.path.abspath("output")
        os.makedirs(output_path, exist_ok=True)
        subprocess.Popen(f'explorer "{output_path}"')


if __name__ == "__main__":
    app = JSFSApp()
    app.mainloop()
```

#### 2b. Implementasi `_build_ui()`

Bangun widget sesuai wireframe di Section 3.2. Gunakan `CTkFrame` sebagai container section. Urutan layout dari atas ke bawah:

```
CTkFrame (header)
  └─ CTkLabel (judul)

CTkFrame (input section)
  ├─ CTkLabel "Keyword Pencarian"
  └─ CTkEntry (entry_keyword) — lebar penuh

CTkFrame (source section)
  ├─ CTkLabel "Sumber Data"
  ├─ CTkCheckBox (chk_garuda)
  └─ CTkCheckBox (chk_sinta)

CTkFrame (rank section)
  ├─ CTkLabel "Filter Peringkat SINTA"
  └─ [CTkCheckBox x6] chk_s1 ... chk_s6  — layout grid 3 kolom

CTkFrame (format section)
  ├─ CTkLabel "Format Output"
  └─ [CTkCheckBox x3] chk_fmt_md, chk_fmt_excel, chk_fmt_csv

CTkFrame (advanced section)
  └─ CTkCheckBox (chk_selenium) "Gunakan Selenium (bypass JS/Cloudflare)"

CTkTextbox (txt_log) — lebar penuh, tinggi ~200px, state="disabled"

CTkButton (btn_scrape) — lebar penuh, tinggi 50px

CTkFrame (footer)
  ├─ CTkLabel (lbl_output_path)
  └─ CTkButton (btn_open_folder) "📂 Buka Folder"
```

---

### Langkah 3 — Build ke `.exe` dengan PyInstaller

#### 3a. Pre-build Checklist

Sebelum build, pastikan:
- [ ] `app_ui.py` berjalan sempurna via `python app_ui.py` (tidak ada error)
- [ ] Virtual environment aktif (`venv\Scripts\activate`)
- [ ] PyInstaller terinstall (`pip show pyinstaller`)
- [ ] Folder `assets/` ada (buat kosong jika tidak punya ikon)

#### 3b. Perintah Build

```bash
# Pastikan berada di root folder JSFS
cd "d:\FileDava\JSFS (Journal Scraper For Students)"

# Aktifkan venv
venv\Scripts\activate

# Build — perintah satu baris
pyinstaller --onefile --noconsole --name "JSFS" app_ui.py
```

Jika punya ikon `.ico`:
```bash
pyinstaller --onefile --noconsole --name "JSFS" --icon "assets/icon.ico" app_ui.py
```

#### 3c. Hasil Build

```
dist/
└── JSFS.exe    ← File yang siap didistribusikan (~30–40MB)

build/          ← Folder sementara PyInstaller, bisa dihapus setelah build
JSFS.spec       ← File spec yang dihasilkan, simpan untuk rebuild
```

#### 3d. Jalankan `.exe`

```bash
dist\JSFS.exe
```

Atau double-click `JSFS.exe` dari File Explorer. **Tidak perlu Python terinstall di komputer penerima.**

#### 3e. Masalah Umum & Solusinya

| Masalah | Penyebab | Solusi |
|---------|----------|--------|
| `ModuleNotFoundError` saat `.exe` dijalankan | PyInstaller tidak otomatis detect hidden import | Tambahkan `--hidden-import=nama_modul` ke perintah build |
| `lxml` tidak ditemukan | lxml perlu flag khusus | Tambahkan `--hidden-import=lxml.etree` |
| `customtkinter` tidak menemukan assets | CTk punya folder `assets` internal | Tambahkan `--collect-data customtkinter` |
| Jendela CMD muncul sebentar lalu hilang | Lupa `--noconsole` | Pastikan flag ada di perintah build |
| `.exe` diblokir Windows Defender | False positive umum untuk PyInstaller | Klik "More info" → "Run anyway", atau sign dengan code signing cert |

**Perintah build lengkap dengan semua fix:**

```bash
pyinstaller \
  --onefile \
  --noconsole \
  --name "JSFS" \
  --collect-data customtkinter \
  --hidden-import=lxml.etree \
  --hidden-import=lxml._elementpath \
  --hidden-import=bs4 \
  --hidden-import=pandas \
  --hidden-import=openpyxl \
  app_ui.py
```

---

## RINGKASAN UNTUK ANTIGRAVITY

| Prioritas | File yang Dibuat | Estimasi Kompleksitas |
|-----------|-----------------|----------------------|
| 1 (Wajib) | `log_redirect.py` | Rendah — ~30 baris |
| 2 (Wajib) | `app_ui.py` | Sedang — ~200–250 baris |
| 3 (Opsional) | `assets/icon.ico` | Rendah — cari/buat ikon |
| 4 (Final) | Build `JSFS.exe` | Rendah — satu perintah |

**Urutan kerja yang disarankan:**
1. Buat `log_redirect.py` dan test di terminal
2. Buat skeleton `app_ui.py` dengan layout kosong — pastikan window tampil
3. Tambahkan widget satu section per commit
4. Integrasikan `log_redirect.py` dan test scraping dari GUI
5. Test end-to-end: keyword → scrape → export → buka file output
6. Jalankan PyInstaller build
7. Test `dist/JSFS.exe` di komputer yang **tidak punya Python**

---

*Blueprint ini disiapkan berdasarkan analisis penuh terhadap kode JSFS MVP v1.0.*
*Semua nama fungsi, variabel, dan path mengacu pada kode aktual yang sudah ada.*
*— Kiro*
```

---

Itu adalah isi lengkap `JSFS_Antigravity_Blueprint.md`. Tinggal buat file baru di folder proyek, paste konten di atas.

Beberapa keputusan desain yang perlu diperhatikan:

**Monkey-patching `console`** — ini sengaja dipilih daripada memodifikasi signature scraper. Cara ini memungkinkan `scraper_garuda.py` dan `scraper_sinta.py` tetap tidak tersentuh, sementara output rich-nya tetap bisa ditangkap GUI.

**`_poll_log_queue` via `self.after()`** — ini adalah pola standar Tkinter untuk komunikasi cross-thread yang aman. Jangan ganti dengan `tkinter.StringVar.trace` atau update langsung dari thread — keduanya bisa menyebabkan crash intermittent.

**`--collect-data customtkinter`** di PyInstaller — ini wajib karena CustomTkinter memuat tema dan aset dari folder internal yang tidak otomatis terdeteksi oleh PyInstaller.