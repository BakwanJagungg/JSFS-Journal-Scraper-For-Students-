# =============================================================================
# JSFS — Journal Scraper For Students
# app_ui.py — Antarmuka Grafis (Desktop GUI) Utama berbasis CustomTkinter v1.3.8
# =============================================================================

import os
import sys
import queue
import re
import subprocess
import threading
import webbrowser
from datetime import datetime
from typing import Optional
from tkinter import ttk
import customtkinter as ctk

# Import modul scraper dan konfigurasi
import scraper_garuda
import scraper_sinta
import exporter
import jsfs
from scraper_garuda import JournalEntry, sanitize_text, scrape_garuda, merge_cross_source_entries
from scraper_sinta import scrape_sinta, scrape_sinta_selenium
from exporter import export_markdown, export_csv, export_excel
from config import (
    DEFAULT_SINTA_RANKS,
    OUTPUT_DIR,
    MAX_RESULTS,
    TARGET_OPTIONS,
    INFO_NOT_FOUND,
)
from log_redirect import make_queue_console

# Konfigurasi Tampilan CustomTkinter
ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("blue")


class JSFSApp(ctk.CTk):
    """
    Jendela Utama Aplikasi Desktop JSFS v1.3.8.
    Mendukung tata letak bebas terpotong (Zero Clipping), Focus & Scope extraction,
    Indikator Kesehatan Server OJS & Transparansi Kontak, Data Table Previewer GUI interaktif,
    speed acceleration engine (fast-fail circuit breaker), analisis naskah artikel (.docx/.pdf),
    dan ekspor presisi tinggi (.xlsx, .md, .csv).
    """

    def __init__(self):
        super().__init__()

        # Konfigurasi Window Kompak & Zero Clipping (No Vertical Clipping pada 1366x768 & 1920x1080)
        self.title("JSFS v1.3.8 — Journal Scraper For Students (Pixel-Perfect Alignment & Data Previewer)")
        self.geometry("900x700")
        self.minsize(880, 660)
        self.resizable(False, False)

        # State Management
        self.log_queue: queue.Queue = queue.Queue()
        self._is_running: bool = False
        self._cancel_event = threading.Event()
        self._manuscript_meta: Optional[dict] = None
        self._latest_results: list[JournalEntry] = []
        self._latest_excel_path: Optional[str] = None
        self._previewer_window: Optional[ctk.CTkToplevel] = None

        # Set Icon jika tersedia
        self._set_app_icon()

        # Bangun Komponen UI
        self._build_ui()

        # Mulai polling queue log secara asinkronus (anti-freeze)
        self._poll_log_queue()

    def _set_app_icon(self):
        """Memuat ikon aplikasi dari folder assets jika ada."""
        base_dir = getattr(sys, "_MEIPASS", os.path.dirname(os.path.abspath(__file__)))
        icon_path = os.path.join(base_dir, "assets", "icon.ico")
        if os.path.exists(icon_path):
            try:
                self.iconbitmap(icon_path)
            except Exception:
                pass

    def _build_ui(self):
        """Membangun tata letak widget antarmuka pengguna v1.3.8 yang padat, simetris & proporsional."""
        main_container = ctk.CTkFrame(self, fg_color="transparent")
        main_container.pack(fill="both", expand=True, padx=16, pady=8)

        # ── 1. HEADER SECTION ────────────────────────────────────────────────
        header_frame = ctk.CTkFrame(main_container, fg_color="transparent")
        header_frame.pack(fill="x", pady=(0, 4))

        title_label = ctk.CTkLabel(
            header_frame,
            text="📚 JSFS v1.3.8 — Journal Scraper For Students",
            font=ctk.CTkFont(family="Segoe UI", size=16, weight="bold"),
            text_color="#38bdf8",
        )
        title_label.pack(anchor="w")

        subtitle_label = ctk.CTkLabel(
            header_frame,
            text="Multi-Tier Focus & Scope Mining • Pixel-Perfect Alignment • Server Health • GUI Data Previewer",
            font=ctk.CTkFont(family="Segoe UI", size=11),
            text_color="#94a3b8",
        )
        subtitle_label.pack(anchor="w")

        # ── 2. INPUT KEYWORD SECTION ─────────────────────────────────────────
        keyword_card = ctk.CTkFrame(main_container, corner_radius=8)
        keyword_card.pack(fill="x", pady=(0, 4), padx=2)

        input_row = ctk.CTkFrame(keyword_card, fg_color="transparent")
        input_row.pack(fill="x", padx=12, pady=5)

        lbl_keyword = ctk.CTkLabel(
            input_row,
            text="🔍 Kata Kunci:",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
        )
        lbl_keyword.pack(side="left", padx=(0, 8))

        self.entry_keyword = ctk.CTkEntry(
            input_row,
            placeholder_text="Masukkan topik riset (misal: bisnis, akuntansi, machine learning, consumer behavior...)",
            font=ctk.CTkFont(family="Segoe UI", size=12),
            height=32,
        )
        self.entry_keyword.pack(side="left", fill="x", expand=True, padx=(0, 8))
        self.entry_keyword.bind("<Return>", lambda event: self._on_scrape_clicked())

        btn_quick_search = ctk.CTkButton(
            input_row,
            text="Cari",
            width=70,
            height=32,
            font=ctk.CTkFont(family="Segoe UI", size=12, weight="bold"),
            command=self._on_scrape_clicked,
        )
        btn_quick_search.pack(side="right")

        # ── 2B. MANUSCRIPT ANALYSIS SECTION (1 BARIS HORIZONTAL KOMPAK) ──────
        ms_card = ctk.CTkFrame(main_container, corner_radius=8)
        ms_card.pack(fill="x", pady=(0, 4), padx=2)

        self.ms_action_row = ctk.CTkFrame(ms_card, fg_color="transparent")
        self.ms_action_row.pack(fill="x", padx=12, pady=4)

        lbl_ms = ctk.CTkLabel(
            self.ms_action_row,
            text="📄 Naskah Ilmiah:",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
            text_color="#38bdf8",
        )
        lbl_ms.pack(side="left", padx=(0, 6))

        self.btn_upload_ms = ctk.CTkButton(
            self.ms_action_row,
            text="📁 Unggah (.docx/.pdf)",
            width=130,
            height=25,
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
            command=self._on_upload_manuscript,
        )
        self.btn_upload_ms.pack(side="left", padx=(0, 8))

        self.chk_match_scoring = ctk.CTkCheckBox(
            self.ms_action_row,
            text="Match Score (0–100%)",
            font=ctk.CTkFont(size=11, weight="bold"),
            text_color="#4ade80",
        )
        self.chk_match_scoring.pack(side="left", padx=(0, 8))

        self.lbl_ms_status = ctk.CTkLabel(
            self.ms_action_row,
            text="Belum ada naskah (Opsional — untuk rekomendasi kesesuaian)",
            font=ctk.CTkFont(family="Segoe UI", size=11),
            text_color="#94a3b8",
            anchor="w",
        )
        self.lbl_ms_status.pack(side="left", fill="x", expand=True)

        self.btn_clear_ms = ctk.CTkButton(
            self.ms_action_row,
            text="❌ Hapus",
            width=55,
            height=24,
            fg_color="#475569",
            hover_color="#64748b",
            font=ctk.CTkFont(size=10),
            command=self._on_clear_manuscript,
        )

        # ── 3. FILTER & OPTIONS SECTION (Dua Kolom Grid Kompak) ──────────────
        options_container = ctk.CTkFrame(main_container, fg_color="transparent")
        options_container.pack(fill="x", pady=(0, 4))

        # --- Kolom Kiri: Sumber Data, Format Output & Kontrol Target ---
        left_card = ctk.CTkFrame(options_container, corner_radius=8)
        left_card.pack(side="left", fill="both", expand=True, padx=(0, 4))

        # Sumber Data & Format Output Sejajar
        top_opt_row = ctk.CTkFrame(left_card, fg_color="transparent")
        top_opt_row.pack(fill="x", padx=12, pady=(4, 2))

        lbl_source = ctk.CTkLabel(
            top_opt_row,
            text="📡 Sumber:",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
        )
        lbl_source.pack(side="left", padx=(0, 4))

        self.chk_garuda = ctk.CTkCheckBox(top_opt_row, text="Garuda", font=ctk.CTkFont(size=11))
        self.chk_garuda.select()
        self.chk_garuda.pack(side="left", padx=(0, 8))

        self.chk_sinta = ctk.CTkCheckBox(top_opt_row, text="SINTA", font=ctk.CTkFont(size=11))
        self.chk_sinta.select()
        self.chk_sinta.pack(side="left", padx=(0, 12))

        lbl_format = ctk.CTkLabel(
            top_opt_row,
            text="💾 Format:",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
        )
        lbl_format.pack(side="left", padx=(0, 4))

        self.chk_fmt_md = ctk.CTkCheckBox(top_opt_row, text=".md", font=ctk.CTkFont(size=11))
        self.chk_fmt_md.select()
        self.chk_fmt_md.pack(side="left", padx=(0, 6))

        self.chk_fmt_excel = ctk.CTkCheckBox(top_opt_row, text=".xlsx", font=ctk.CTkFont(size=11))
        self.chk_fmt_excel.select()
        self.chk_fmt_excel.pack(side="left", padx=(0, 6))

        self.chk_fmt_csv = ctk.CTkCheckBox(top_opt_row, text=".csv", font=ctk.CTkFont(size=11))
        self.chk_fmt_csv.pack(side="left")

        # Kontrol Beban: Target Jumlah Jurnal (Hingga 1000 atau Custom)
        target_row = ctk.CTkFrame(left_card, fg_color="transparent")
        target_row.pack(fill="x", padx=12, pady=(2, 2))

        lbl_target = ctk.CTkLabel(
            target_row,
            text="🎯 Target Jurnal:",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
        )
        lbl_target.pack(side="left", padx=(0, 6))

        self.combo_target = ctk.CTkComboBox(
            target_row,
            values=["10", "50", "100", "250", "500", "1000"],
            width=80,
            height=24,
            font=ctk.CTkFont(size=11),
        )
        self.combo_target.set("50")
        self.combo_target.pack(side="left")

        lbl_target_hint = ctk.CTkLabel(
            target_row,
            text="(Maks. 1000 / custom)",
            font=ctk.CTkFont(size=10),
            text_color="#94a3b8",
        )
        lbl_target_hint.pack(side="left", padx=(6, 0))

        # Filter Rentang Biaya Publikasi (APC)
        apc_card_row = ctk.CTkFrame(left_card, fg_color="transparent")
        apc_card_row.pack(fill="x", padx=12, pady=(2, 2))

        self.chk_apc_free_only = ctk.CTkCheckBox(
            apc_card_row,
            text="💰 Hanya Gratis (Rp 0)",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
            text_color="#4ade80",
            command=self._on_free_only_toggled,
        )
        self.chk_apc_free_only.pack(side="left", padx=(0, 12))

        self.chk_include_unknown_apc = ctk.CTkCheckBox(
            apc_card_row,
            text="Sertakan N/A",
            font=ctk.CTkFont(family="Segoe UI", size=10),
            text_color="#38bdf8",
        )
        self.chk_include_unknown_apc.select()
        self.chk_include_unknown_apc.pack(side="left")

        price_range_row = ctk.CTkFrame(left_card, fg_color="transparent")
        price_range_row.pack(fill="x", padx=12, pady=(2, 4))

        lbl_min_apc = ctk.CTkLabel(price_range_row, text="Min (Rp):", font=ctk.CTkFont(family="Segoe UI", size=10, weight="bold"))
        lbl_min_apc.pack(side="left", padx=(0, 4))
        self.entry_min_apc = ctk.CTkEntry(price_range_row, width=80, height=24, font=ctk.CTkFont(size=10))
        self.entry_min_apc.insert(0, "0")
        self.entry_min_apc.pack(side="left", padx=(0, 10))

        lbl_max_apc = ctk.CTkLabel(price_range_row, text="Max (Rp):", font=ctk.CTkFont(family="Segoe UI", size=10, weight="bold"))
        lbl_max_apc.pack(side="left", padx=(0, 4))
        self.entry_max_apc = ctk.CTkEntry(price_range_row, width=95, height=24, font=ctk.CTkFont(size=10))
        self.entry_max_apc.insert(0, "15.000.000")
        self.entry_max_apc.pack(side="left")

        # --- Kolom Kanan: Filter Peringkat SINTA & Mode Detail OJS ---
        right_card = ctk.CTkFrame(options_container, corner_radius=8)
        right_card.pack(side="right", fill="both", expand=True, padx=(4, 0))

        rank_header_row = ctk.CTkFrame(right_card, fg_color="transparent")
        rank_header_row.pack(fill="x", padx=12, pady=(4, 1))

        lbl_sinta = ctk.CTkLabel(
            rank_header_row,
            text="📊 Peringkat SINTA:",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
        )
        lbl_sinta.pack(side="left")

        btn_preset_default = ctk.CTkButton(
            rank_header_row,
            text="S3–S6",
            width=44,
            height=18,
            font=ctk.CTkFont(size=10),
            command=self._preset_s3_s6,
        )
        btn_preset_default.pack(side="right", padx=(4, 0))

        btn_preset_all = ctk.CTkButton(
            rank_header_row,
            text="Semua",
            width=44,
            height=18,
            font=ctk.CTkFont(size=10),
            command=self._preset_all_ranks,
        )
        btn_preset_all.pack(side="right")

        # Grid Checkbox SINTA 1-6 (Symmetric 2-Column Pixel-Perfect Layout)
        ranks_grid = ctk.CTkFrame(right_card, fg_color="transparent")
        ranks_grid.pack(fill="x", padx=10, pady=(2, 3))
        ranks_grid.grid_columnconfigure((0, 1), weight=1)

        # Kolom 1: Sinta 1, Sinta 2, Sinta 3
        self.chk_s1 = ctk.CTkCheckBox(ranks_grid, text="Sinta 1", font=ctk.CTkFont(family="Segoe UI", size=11))
        self.chk_s1.grid(row=0, column=0, sticky="w", padx=(10, 10), pady=(3, 3))

        self.chk_s2 = ctk.CTkCheckBox(ranks_grid, text="Sinta 2", font=ctk.CTkFont(family="Segoe UI", size=11))
        self.chk_s2.grid(row=1, column=0, sticky="w", padx=(10, 10), pady=(3, 3))

        self.chk_s3 = ctk.CTkCheckBox(ranks_grid, text="Sinta 3", font=ctk.CTkFont(family="Segoe UI", size=11))
        self.chk_s3.select()
        self.chk_s3.grid(row=2, column=0, sticky="w", padx=(10, 10), pady=(3, 3))

        # Kolom 2: Sinta 4, Sinta 5, Sinta 6
        self.chk_s4 = ctk.CTkCheckBox(ranks_grid, text="Sinta 4", font=ctk.CTkFont(family="Segoe UI", size=11))
        self.chk_s4.select()
        self.chk_s4.grid(row=0, column=1, sticky="w", padx=(10, 10), pady=(3, 3))

        self.chk_s5 = ctk.CTkCheckBox(ranks_grid, text="Sinta 5", font=ctk.CTkFont(family="Segoe UI", size=11))
        self.chk_s5.select()
        self.chk_s5.grid(row=1, column=1, sticky="w", padx=(10, 10), pady=(3, 3))

        self.chk_s6 = ctk.CTkCheckBox(ranks_grid, text="Sinta 6", font=ctk.CTkFont(family="Segoe UI", size=11))
        self.chk_s6.select()
        self.chk_s6.grid(row=2, column=1, sticky="w", padx=(10, 10), pady=(3, 3))

        # Sakelar Hardware Turbo Mode (Kecepatan Maksimal) & OJS Mode
        modes_row = ctk.CTkFrame(right_card, fg_color="transparent")
        modes_row.pack(fill="x", padx=10, pady=(2, 2))

        self.chk_turbo_mode = ctk.CTkCheckBox(
            modes_row,
            text="⚡ Turbo Mode",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
            text_color="#f59e0b",
        )
        self.chk_turbo_mode.select()
        self.chk_turbo_mode.pack(side="left", padx=(10, 8))

        self.chk_deep_detail = ctk.CTkCheckBox(
            modes_row,
            text="🔍 Mode Detail OJS",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
            text_color="#38bdf8",
        )
        self.chk_deep_detail.select()
        self.chk_deep_detail.pack(side="left", padx=(0, 10))

        self.chk_selenium = ctk.CTkCheckBox(
            right_card,
            text="Gunakan Selenium (fallback Cloudflare)",
            font=ctk.CTkFont(family="Segoe UI", size=10),
        )
        self.chk_selenium.pack(anchor="w", padx=(20, 10), pady=(1, 3))

        # ── 4. LOG / PROGRESS MONITOR SECTION ────────────────────────────────
        log_header_frame = ctk.CTkFrame(main_container, fg_color="transparent")
        log_header_frame.pack(fill="x", pady=(1, 2))

        lbl_log = ctk.CTkLabel(
            log_header_frame,
            text="📜 Pemantauan Proses & Riwayat Log:",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
        )
        lbl_log.pack(side="left")

        btn_clear_log = ctk.CTkButton(
            log_header_frame,
            text="🧹 Bersihkan Log",
            width=85,
            height=18,
            font=ctk.CTkFont(size=10),
            fg_color="#334155",
            hover_color="#475569",
            command=self._clear_log,
        )
        btn_clear_log.pack(side="right")

        self.txt_log = ctk.CTkTextbox(
            main_container,
            height=160,
            font=ctk.CTkFont(family="Consolas", size=11),
            wrap="word",
        )
        self.txt_log.pack(fill="both", expand=True, pady=(0, 4))
        self.txt_log.configure(state="disabled")

        # Pesan awal
        self._log_ui("Selamat datang di JSFS Desktop v1.3.8 (Multi-Tier Focus & Scope Mining, Pixel-Perfect Layout, Data Previewer).")
        self._log_ui("Siap mencari jurnal terakreditasi SINTA & Garuda dengan direct concurrent HTTP dan presisi data tingkat tinggi.")

        # ── 5. MAIN ACTION BUTTONS ROW ───────────────────────────────────────
        action_btn_row = ctk.CTkFrame(main_container, fg_color="transparent")
        action_btn_row.pack(fill="x", pady=(2, 4))

        self.btn_scrape = ctk.CTkButton(
            action_btn_row,
            text="🚀  MULAI SCRAPING",
            height=40,
            font=ctk.CTkFont(family="Segoe UI", size=13, weight="bold"),
            fg_color="#0284c7",
            hover_color="#0369a1",
            command=self._on_scrape_clicked,
        )
        self.btn_scrape.pack(side="left", fill="x", expand=True, padx=(0, 8))

        self.btn_preview = ctk.CTkButton(
            action_btn_row,
            text="👁️  PREVIEW HASIL DATA",
            height=40,
            width=210,
            font=ctk.CTkFont(family="Segoe UI", size=12, weight="bold"),
            fg_color="#059669",
            hover_color="#047857",
            command=self._open_data_previewer,
            state="disabled",
        )
        self.btn_preview.pack(side="right")

        # ── 6. FOOTER SECTION ────────────────────────────────────────────────
        footer_frame = ctk.CTkFrame(main_container, fg_color="transparent")
        footer_frame.pack(fill="x")

        self.lbl_output_path = ctk.CTkLabel(
            footer_frame,
            text=f"Output disimpan di folder: {OUTPUT_DIR}/ (Format: JSFS_<keyword>_<timestamp>)",
            font=ctk.CTkFont(family="Segoe UI", size=11),
            text_color="#94a3b8",
        )
        self.lbl_output_path.pack(side="left")

        self.btn_open_folder = ctk.CTkButton(
            footer_frame,
            text="📂 Buka Folder Output",
            width=135,
            height=26,
            font=ctk.CTkFont(family="Segoe UI", size=11),
            command=self._open_output_folder,
        )
        self.btn_open_folder.pack(side="right")

    # ── LOGIKA ANALISIS NASKAH ARTIKEL ───────────────────────────────────────
    def _on_upload_manuscript(self):
        """Membuka dialog pemilihan naskah .docx / .pdf dan mengekstrak metadata."""
        from tkinter import filedialog
        import doc_reader

        filetypes = [
            ("Dokumen Ilmiah (*.docx, *.pdf)", "*.docx;*.pdf"),
            ("Word Document (*.docx)", "*.docx"),
            ("PDF Document (*.pdf)", "*.pdf"),
            ("Semua Berkas (*.*)", "*.*"),
        ]
        file_path = filedialog.askopenfilename(
            title="Pilih Berkas Naskah Artikel Ilmiah",
            filetypes=filetypes,
        )
        if not file_path:
            return

        try:
            meta = doc_reader.extract_manuscript_metadata(file_path)
            self._manuscript_meta = meta
            self.chk_match_scoring.select()
            self.btn_clear_ms.pack(side="right")

            fname = os.path.basename(file_path)
            title_snip = (meta.get("title") or "Naskah")[:45]
            kw_snip = ", ".join(meta.get("keywords", [])[:3])
            status_text = f"✅ {fname} • Judul: {title_snip}... | Keywords: {kw_snip}" if kw_snip else f"✅ {fname} • Judul: {title_snip}..."
            self.lbl_ms_status.configure(text=status_text, text_color="#4ade80")

            # Isi otomatis keyword jika kotak keyword kosong
            if not self.entry_keyword.get().strip() and meta.get("keywords"):
                self.entry_keyword.insert(0, ", ".join(meta["keywords"][:3]))

            self.log_queue.put(
                f"[bold green]📄 Naskah Berhasil Dianalisis:[/bold green] {fname}\n"
                f"   [dim]Judul   : {meta.get('title')}[/dim]\n"
                f"   [dim]Keywords: {', '.join(meta.get('keywords', []))}[/dim]\n"
                f"   [dim]Istilah : {', '.join(meta.get('dominant_terms', [])[:6])}[/dim]"
            )
        except Exception as e:
            self.lbl_ms_status.configure(text=f"❌ Gagal membaca naskah: {e}", text_color="#f87171")
            self._manuscript_meta = None

    def _on_clear_manuscript(self):
        """Mereset berkas naskah yang diunggah."""
        self._manuscript_meta = None
        self.lbl_ms_status.configure(
            text="Belum ada naskah diunggah (Opsional — untuk rekomendasi kesesuaian otomatis)",
            text_color="#94a3b8",
        )
        self.chk_match_scoring.deselect()
        self.btn_clear_ms.pack_forget()
        self.log_queue.put("[dim]🗑️ Berkas naskah telah dihapus dari antarmuka.[/dim]")

    # ── LOGIKA TOGGLE & PRESET ───────────────────────────────────────────────
    def _on_free_only_toggled(self):
        """Atur status input min/max ketika 'Hanya Gratis' dicentang."""
        if self.chk_apc_free_only.get():
            self.entry_min_apc.configure(state="disabled")
            self.entry_max_apc.configure(state="disabled")
        else:
            self.entry_min_apc.configure(state="normal")
            self.entry_max_apc.configure(state="normal")

    def _preset_all_ranks(self):
        """Memilih seluruh peringkat Sinta 1 sampai 6."""
        for chk in [self.chk_s1, self.chk_s2, self.chk_s3, self.chk_s4, self.chk_s5, self.chk_s6]:
            chk.select()

    def _preset_s3_s6(self):
        """Mengatur ke preset mahasiswa: Sinta 3 hingga 6 terpilih, Sinta 1 & 2 tidak."""
        self.chk_s1.deselect()
        self.chk_s2.deselect()
        self.chk_s3.select()
        self.chk_s4.select()
        self.chk_s5.select()
        self.chk_s6.select()

    # ── POLLING DAN UPDATE LOG ───────────────────────────────────────────────
    def _poll_log_queue(self):
        """
        Polling antrean log (log_queue) setiap 100ms di Main UI Thread.
        Menyisipkan pesan ke widget Textbox secara aman tanpa freeze.
        """
        try:
            while True:
                msg = self.log_queue.get_nowait()
                timestamp = datetime.now().strftime("%H:%M:%S")
                self.txt_log.configure(state="normal")
                self.txt_log.insert("end", f"[{timestamp}] {msg}\n")
                self.txt_log.see("end")
                self.txt_log.configure(state="disabled")
        except queue.Empty:
            pass
        finally:
            self.after(100, self._poll_log_queue)

    def _log_ui(self, msg: str):
        """Menambahkan pesan log langsung dari thread antarmuka pengguna."""
        timestamp = datetime.now().strftime("%H:%M:%S")
        self.txt_log.configure(state="normal")
        self.txt_log.insert("end", f"[{timestamp}] {msg}\n")
        self.txt_log.see("end")
        self.txt_log.configure(state="disabled")

    def _clear_log(self):
        """Membersihkan seluruh teks di jendela log."""
        self.txt_log.configure(state="normal")
        self.txt_log.delete("1.0", "end")
        self.txt_log.configure(state="disabled")

    # ── HELPER PENGAMBILAN FILTER DARI UI ────────────────────────────────────
    def _get_selected_ranks(self) -> list[int]:
        """Mengembalikan list peringkat Sinta yang dicentang pengguna."""
        ranks = []
        rank_map = {
            1: self.chk_s1,
            2: self.chk_s2,
            3: self.chk_s3,
            4: self.chk_s4,
            5: self.chk_s5,
            6: self.chk_s6,
        }
        for rank, chk in rank_map.items():
            if chk.get():
                ranks.append(rank)
        return ranks if ranks else DEFAULT_SINTA_RANKS

    def _get_selected_formats(self) -> list[str]:
        """Mengembalikan list format berkas output yang dicentang."""
        fmts = []
        if self.chk_fmt_md.get():
            fmts.append("md")
        if self.chk_fmt_excel.get():
            fmts.append("excel")
        if self.chk_fmt_csv.get():
            fmts.append("csv")
        return fmts if fmts else ["md"]

    # ── EKSEKUSI SCRAPING & THREADING ────────────────────────────────────────
    def _on_scrape_clicked(self):
        """Handler saat tombol Mulai Scraping diklik."""
        if self._is_running:
            return

        keyword = self.entry_keyword.get().strip()
        if not keyword:
            self._log_ui("⚠️  Kata kunci (keyword) pencarian tidak boleh kosong.")
            return

        use_garuda = bool(self.chk_garuda.get())
        use_sinta = bool(self.chk_sinta.get())
        if not use_garuda and not use_sinta:
            self._log_ui("⚠️  Pilih minimal satu sumber pencarian (Garuda atau SINTA).")
            return

        # Target hasil dari UI (Dinamis hingga 1000)
        try:
            raw_target = self.combo_target.get().strip()
            target_limit = int(re.sub(r"\D", "", raw_target))
            target_limit = max(1, min(1000, target_limit))
        except (ValueError, Exception):
            target_limit = 50

        # Ambil parameter filter APC
        free_only = bool(self.chk_apc_free_only.get())
        include_unknown = bool(self.chk_include_unknown_apc.get())
        try:
            min_apc = int(re.sub(r"\D", "", self.entry_min_apc.get() or "0"))
        except ValueError:
            min_apc = 0

        try:
            max_apc = int(re.sub(r"\D", "", self.entry_max_apc.get() or "15000000"))
        except ValueError:
            max_apc = 15000000

        deep_detail = bool(self.chk_deep_detail.get())
        turbo_mode = bool(self.chk_turbo_mode.get())

        # Kunci tombol dan tandai status aplikasi berjalan
        self._is_running = True
        self._cancel_event.clear()
        self.btn_scrape.configure(
            state="disabled",
            text="⏳ Scraping sedang berjalan... Mohon tunggu...",
            fg_color="#475569",
        )

        # Terapkan Monkey-Patching rich.Console di modul scraper agar log diarahkan ke queue
        queue_console = make_queue_console(self.log_queue)
        scraper_garuda.console = queue_console
        scraper_sinta.console = queue_console
        exporter.console = queue_console
        jsfs.console = queue_console

        params = {
            "keyword": keyword,
            "use_garuda": use_garuda,
            "use_sinta": use_sinta,
            "sinta_ranks": self._get_selected_ranks(),
            "max_results": target_limit,
            "deep_detail": deep_detail,
            "turbo_mode": turbo_mode,
            "use_selenium": bool(self.chk_selenium.get()),
            "formats": self._get_selected_formats(),
            "free_only": free_only,
            "include_unknown_apc": include_unknown,
            "min_apc": min_apc,
            "max_apc": max_apc,
        }

        # Jalankan di Background Worker Thread terpisah (MUTLAK ANTI-FREEZE)
        thread = threading.Thread(
            target=self._scraping_task,
            args=(params,),
            daemon=True,
        )
        thread.start()

    def _scraping_task(self, params: dict):
        """
        Logika scraping yang berjalan di latar belakang (background thread).
        """
        all_results: list[JournalEntry] = []
        keyword = params["keyword"]
        ranks = params["sinta_ranks"]
        target_limit = params["max_results"]
        deep_mode = params["deep_detail"]
        turbo_mode = params.get("turbo_mode", True)

        try:
            self.log_queue.put(f"🚀 Memulai scraping untuk keyword: '{keyword}'...")
            if turbo_mode:
                self.log_queue.put("⚡ HARDWARE TURBO MODE AKTIF: Max throughput paralelisme & high-capacity connection pool!")
            self.log_queue.put(f"🎯 Target: {target_limit} jurnal | Mode Detail OJS: {'Aktif (Quad-Pass)' if deep_mode else 'Mati (Mode Cepat)'}")
            self.log_queue.put(f"📊 Target peringkat SINTA: {ranks}")
            if params["free_only"]:
                unk_note = " (termasuk belum teridentifikasi)" if params["include_unknown_apc"] else ""
                self.log_queue.put(f"💰 Filter APC: Hanya Jurnal Gratis (Rp 0){unk_note}")
            else:
                self.log_queue.put(f"💰 Filter Rentang APC: Rp {params['min_apc']:,} s.d. Rp {params['max_apc']:,}".replace(",", "."))

            # 1. Scraping Portal Garuda
            if params["use_garuda"]:
                self.log_queue.put("📡 Menghubungi Portal Garuda...")
                garuda_results = scrape_garuda(
                    keyword,
                    ranks,
                    deep_detail=deep_mode,
                    max_results=target_limit,
                    turbo_mode=turbo_mode,
                    free_only=params["free_only"],
                    include_unknown_apc=params["include_unknown_apc"],
                )
                all_results.extend(garuda_results)

            # 2. Scraping Direktori SINTA
            if params["use_sinta"]:
                self.log_queue.put("📡 Menghubungi Direktori SINTA...")
                if params["use_selenium"]:
                    sinta_results = scrape_sinta_selenium(
                        keyword,
                        ranks,
                        deep_detail=deep_mode,
                        max_results=target_limit,
                        turbo_mode=turbo_mode,
                        free_only=params["free_only"],
                        include_unknown_apc=params["include_unknown_apc"],
                    )
                else:
                    sinta_results = scrape_sinta(
                        keyword,
                        ranks,
                        deep_detail=deep_mode,
                        max_results=target_limit,
                        turbo_mode=turbo_mode,
                        free_only=params["free_only"],
                        include_unknown_apc=params["include_unknown_apc"],
                    )
                all_results.extend(sinta_results)

            # 3. Deduplikasi & Penggabungan Data Lintas Sumber (Cross-Source Fallback Merging)
            unique_results = merge_cross_source_entries(all_results)
            self.log_queue.put(f"📦 Total jurnal unik terhimpun sebelum filter APC: {len(unique_results)}")

            # 4. Filter Biaya APC (Hanya Gratis, Rentang Harga, dan Opsi Sertakan N/A)
            filtered_results: list[JournalEntry] = []
            free_only = params["free_only"]
            include_unknown = params["include_unknown_apc"]
            min_apc = params["min_apc"]
            max_apc = params["max_apc"]

            for j in unique_results:
                apc_low = j.apc_status.lower()
                is_free = (j.apc_amount == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("free" in apc_low)
                is_unknown = (j.apc_amount is None) or (j.apc_status == INFO_NOT_FOUND)

                if free_only:
                    if is_free:
                        filtered_results.append(j)
                    elif is_unknown and include_unknown:
                        filtered_results.append(j)
                else:
                    # Filter rentang harga
                    if j.apc_amount is not None:
                        if min_apc <= j.apc_amount <= max_apc:
                            filtered_results.append(j)
                    elif is_free:
                        if min_apc == 0:
                            filtered_results.append(j)
                    else:
                        # Jurnal tanpa info tarif tertera di situs: sertakan jika opsi include_unknown aktif atau min_apc adalah 0
                        if include_unknown or min_apc == 0:
                            filtered_results.append(j)

            # Batasi sesuai target kuota jika melebihi
            if len(filtered_results) > target_limit:
                filtered_results = filtered_results[:target_limit]

            self.log_queue.put(f"🎯 Total jurnal lolos kriteria akhir: {len(filtered_results)}")

            # 4B. Evaluasi Kesesuaian Naskah Ilmiah (Match Score 0–100%) jika naskah dimuat
            if self._manuscript_meta and self.chk_match_scoring.get():
                import doc_reader
                self.log_queue.put(f"\n[bold cyan]🎯 Menghitung Match Score Kesesuaian Naskah terhadap {len(filtered_results)} Jurnal...[/bold cyan]")
                for j in filtered_results:
                    res_m = doc_reader.calculate_match_score(self._manuscript_meta, j)
                    j.match_score = res_m["score"]
                    j.kesesuaian_naskah = res_m["level"]

                # Urutkan berdasarkan skor kesesuaian tertinggi
                filtered_results.sort(key=lambda x: getattr(x, "match_score", 0), reverse=True)

                c_sangat = len([j for j in filtered_results if j.kesesuaian_naskah == "Sangat Sesuai"])
                c_sesuai = len([j for j in filtered_results if j.kesesuaian_naskah == "Sesuai"])
                c_kurang = len([j for j in filtered_results if j.kesesuaian_naskah == "Kurang Sesuai"])
                self.log_queue.put(
                    f"[bold green]✓ Kompatibilitas Naskah:[/bold green] "
                    f"[bold green]{c_sangat} Sangat Sesuai[/bold green], "
                    f"[bold blue]{c_sesuai} Sesuai[/bold blue], "
                    f"[dim]{c_kurang} Kurang Sesuai[/dim]"
                )

            # 5. Simpan ke berkas output unik (dengan timestamp)
            exported_files = []
            if "md" in params["formats"]:
                path_md = export_markdown(filtered_results, keyword, ranks)
                exported_files.append(path_md)

            if "csv" in params["formats"]:
                path_csv = export_csv(filtered_results, keyword)
                exported_files.append(path_csv)

            path_excel = None
            if "excel" in params["formats"]:
                path_excel = export_excel(filtered_results, keyword)
                exported_files.append(path_excel)

            # Simpan hasil scraping terakhir untuk Data Table Previewer
            self._latest_results = filtered_results
            self._latest_excel_path = path_excel

            # 6. Notifikasi akhir
            if filtered_results:
                self.log_queue.put(f"✅ Sukses! Ditemukan {len(filtered_results)} jurnal relevan.")
                self.log_queue.put("💡 Klik tombol '👁️ PREVIEW HASIL DATA' untuk melihat & memfilter tabel secara interaktif di GUI!")
            else:
                self.log_queue.put("ℹ️  Pencarian selesai, namun tidak ada jurnal yang cocok dengan kriteria.")

            for fpath in exported_files:
                self.log_queue.put(f"💾 Berkas tersimpan → {fpath}")

        except Exception as e:
            self.log_queue.put(f"❌ Terjadi kesalahan saat scraping: {str(e)}")
        finally:
            self.after(0, self._on_scraping_done)

    def _on_scraping_done(self):
        """Mengembalikan kondisi tombol dan status aplikasi ke kondisi idle."""
        self._is_running = False
        self.btn_scrape.configure(
            state="normal",
            text="🚀  MULAI SCRAPING",
            fg_color="#0284c7",
        )
        if self._latest_results:
            self.btn_preview.configure(
                state="normal",
                text=f"👁️  PREVIEW HASIL ({len(self._latest_results)})",
                fg_color="#059669",
            )
        else:
            self.btn_preview.configure(
                state="disabled",
                text="👁️  PREVIEW HASIL DATA",
                fg_color="#475569",
            )

    def _open_data_previewer(self):
        """Membuka jendela interaktif Data Table Previewer untuk melihat hasil scraping."""
        if not self._latest_results:
            self._log_ui("⚠️ Belum ada hasil scraping untuk ditampilkan. Silakan jalankan scraping terlebih dahulu.")
            return

        if self._previewer_window is not None and self._previewer_window.winfo_exists():
            self._previewer_window.update_data(self._latest_results, self._latest_excel_path)
            self._previewer_window.lift()
            self._previewer_window.focus()
            return

        self._previewer_window = JSFSDataPreviewer(
            master=self,
            journals=self._latest_results,
            excel_path=self._latest_excel_path,
        )

    def _open_output_folder(self):
        """Membuka folder output secara langsung di Windows File Explorer."""
        out_path = os.path.abspath(OUTPUT_DIR)
        os.makedirs(out_path, exist_ok=True)
        try:
            os.startfile(out_path)
        except Exception:
            subprocess.Popen(f'explorer "{out_path}"')


class JSFSDataPreviewer(ctk.CTkToplevel):
    """
    Jendela Pop-up Interaktif Data Table Previewer (v1.3.8).
    Memungkinkan pengguna melihat, mencari, memfilter, membaca Focus & Scope,
    dan membuka URL OJS langsung dari aplikasi tanpa perlu Microsoft Excel.
    """

    def __init__(self, master, journals: list[JournalEntry], excel_path: Optional[str] = None):
        super().__init__(master)

        self.title("👁️ JSFS Data Table Previewer v1.3.8 — Hasil Scraping Jurnal")
        self.geometry("1180x720")
        self.minsize(980, 600)

        self._all_journals: list[JournalEntry] = list(journals)
        self._filtered_journals: list[JournalEntry] = list(journals)
        self._excel_path: Optional[str] = excel_path
        self._selected_journal: Optional[JournalEntry] = None

        self._build_previewer_ui()
        self._apply_filters()

        self.after(100, self.lift)

    def _build_previewer_ui(self):
        container = ctk.CTkFrame(self, fg_color="transparent")
        container.pack(fill="both", expand=True, padx=14, pady=12)

        # 1. Header & Actions Bar
        header_bar = ctk.CTkFrame(container, fg_color="transparent")
        header_bar.pack(fill="x", pady=(0, 8))

        title_lbl = ctk.CTkLabel(
            header_bar,
            text="📊 Data Table Previewer",
            font=ctk.CTkFont(family="Segoe UI", size=18, weight="bold"),
            text_color="#38bdf8",
        )
        title_lbl.pack(side="left")

        self.lbl_stats = ctk.CTkLabel(
            header_bar,
            text=f"Total: {len(self._all_journals)} jurnal | Dobel-klik baris untuk membuka OJS di Web Browser",
            font=ctk.CTkFont(family="Segoe UI", size=11),
            text_color="#94a3b8",
        )
        self.lbl_stats.pack(side="left", padx=(14, 0))

        btn_close = ctk.CTkButton(
            header_bar,
            text="✕ Tutup",
            width=75,
            height=30,
            fg_color="#334155",
            hover_color="#475569",
            command=self.destroy,
        )
        btn_close.pack(side="right", padx=(8, 0))

        if self._excel_path and os.path.exists(self._excel_path):
            btn_open_excel = ctk.CTkButton(
                header_bar,
                text="📊 Buka di Excel",
                width=110,
                height=30,
                fg_color="#047857",
                hover_color="#065f46",
                command=self._open_excel_file,
            )
            btn_open_excel.pack(side="right")

        # 2. Filter & Search Controls Card
        filter_card = ctk.CTkFrame(container, corner_radius=8)
        filter_card.pack(fill="x", pady=(0, 8), padx=2)

        filter_row = ctk.CTkFrame(filter_card, fg_color="transparent")
        filter_row.pack(fill="x", padx=10, pady=6)

        lbl_s = ctk.CTkLabel(
            filter_row,
            text="🔍 Cari:",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
        )
        lbl_s.pack(side="left", padx=(0, 6))

        self.entry_filter = ctk.CTkEntry(
            filter_row,
            placeholder_text="Ketik nama jurnal, topik, instansi, focus & scope, atau kontak...",
            font=ctk.CTkFont(family="Segoe UI", size=11),
            height=28,
        )
        self.entry_filter.pack(side="left", fill="x", expand=True, padx=(0, 8))
        self.entry_filter.bind("<KeyRelease>", lambda e: self._apply_filters())

        # Dropdown SINTA
        self.combo_sinta = ctk.CTkComboBox(
            filter_row,
            values=["Semua SINTA", "S1", "S2", "S3", "S4", "S5", "S6"],
            width=115,
            height=28,
            font=ctk.CTkFont(size=11),
            command=lambda val: self._apply_filters(),
        )
        self.combo_sinta.set("Semua SINTA")
        self.combo_sinta.pack(side="left", padx=(0, 6))

        # Dropdown APC
        self.combo_apc = ctk.CTkComboBox(
            filter_row,
            values=["Semua Biaya", "Hanya Gratis (Rp 0)", "Berbayar"],
            width=140,
            height=28,
            font=ctk.CTkFont(size=11),
            command=lambda val: self._apply_filters(),
        )
        self.combo_apc.set("Semua Biaya")
        self.combo_apc.pack(side="left", padx=(0, 6))

        # Dropdown Status Server
        self.combo_server = ctk.CTkComboBox(
            filter_row,
            values=["Semua Server", "Sangat Aktif", "Aktif", "SSL Warning", "Non-Aktif"],
            width=120,
            height=28,
            font=ctk.CTkFont(size=11),
            command=lambda val: self._apply_filters(),
        )
        self.combo_server.set("Semua Server")
        self.combo_server.pack(side="left", padx=(0, 6))

        btn_reset = ctk.CTkButton(
            filter_row,
            text="🔄 Reset",
            width=70,
            height=28,
            fg_color="#334155",
            hover_color="#475569",
            command=self._reset_filters,
        )
        btn_reset.pack(side="left", padx=(0, 6))

        self.lbl_count_match = ctk.CTkLabel(
            filter_row,
            text="0 Jurnal",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
            text_color="#38bdf8",
        )
        self.lbl_count_match.pack(side="right")

        # 3. Treeview Table Container with Dark Styling
        tree_container = ctk.CTkFrame(container, corner_radius=8)
        tree_container.pack(fill="both", expand=True, pady=(0, 8))

        style = ttk.Style()
        style.theme_use("clam")
        style.configure(
            "JSFS.Treeview",
            background="#0f172a",
            foreground="#f8fafc",
            rowheight=26,
            fieldbackground="#0f172a",
            font=("Segoe UI", 10),
            borderwidth=0,
        )
        style.configure(
            "JSFS.Treeview.Heading",
            background="#1e293b",
            foreground="#38bdf8",
            relief="flat",
            font=("Segoe UI", 10, "bold"),
            padding=4,
        )
        style.map(
            "JSFS.Treeview",
            background=[("selected", "#0284c7")],
            foreground=[("selected", "#ffffff")],
        )
        style.map(
            "JSFS.Treeview.Heading",
            background=[("active", "#334155")],
        )

        cols = ("no", "title", "sinta", "apc", "server", "transparency", "freq", "contact", "scope")
        self.tree = ttk.Treeview(
            tree_container,
            columns=cols,
            show="headings",
            style="JSFS.Treeview",
            selectmode="browse",
        )

        self.tree.heading("no", text="#")
        self.tree.heading("title", text="Nama Jurnal")
        self.tree.heading("sinta", text="SINTA")
        self.tree.heading("apc", text="Biaya APC")
        self.tree.heading("server", text="Status Server")
        self.tree.heading("transparency", text="Transparansi")
        self.tree.heading("freq", text="Frekuensi")
        self.tree.heading("contact", text="Kontak (Email / WA)")
        self.tree.heading("scope", text="Focus & Scope (Ringkasan)")

        self.tree.column("no", width=36, anchor="center")
        self.tree.column("title", width=260, anchor="w")
        self.tree.column("sinta", width=65, anchor="center")
        self.tree.column("apc", width=115, anchor="center")
        self.tree.column("server", width=130, anchor="center")
        self.tree.column("transparency", width=125, anchor="center")
        self.tree.column("freq", width=80, anchor="center")
        self.tree.column("contact", width=160, anchor="w")
        self.tree.column("scope", width=250, anchor="w")

        v_scroll = ttk.Scrollbar(tree_container, orient="vertical", command=self.tree.yview)
        h_scroll = ttk.Scrollbar(tree_container, orient="horizontal", command=self.tree.xview)
        self.tree.configure(yscrollcommand=v_scroll.set, xscrollcommand=h_scroll.set)

        v_scroll.pack(side="right", fill="y")
        h_scroll.pack(side="bottom", fill="x")
        self.tree.pack(side="left", fill="both", expand=True)

        self.tree.bind("<<TreeviewSelect>>", self._on_tree_select)
        self.tree.bind("<Double-1>", self._on_tree_double_click)

        # 4. Bottom Detail & Full Focus & Scope Panel
        detail_card = ctk.CTkFrame(container, corner_radius=8)
        detail_card.pack(fill="x", padx=2)

        detail_top = ctk.CTkFrame(detail_card, fg_color="transparent")
        detail_top.pack(fill="x", padx=12, pady=(8, 2))

        self.lbl_detail_title = ctk.CTkLabel(
            detail_top,
            text="Pilih jurnal dari tabel di atas untuk melihat detail lengkap & Focus & Scope.",
            font=ctk.CTkFont(family="Segoe UI", size=13, weight="bold"),
            text_color="#f8fafc",
            anchor="w",
        )
        self.lbl_detail_title.pack(side="left", fill="x", expand=True)

        self.btn_open_url = ctk.CTkButton(
            detail_top,
            text="🌐 Buka OJS di Browser",
            width=150,
            height=28,
            font=ctk.CTkFont(size=11, weight="bold"),
            fg_color="#0284c7",
            hover_color="#0369a1",
            command=self._open_selected_url,
            state="disabled",
        )
        self.btn_open_url.pack(side="right")

        self.detail_badges_row = ctk.CTkFrame(detail_card, fg_color="transparent")
        self.detail_badges_row.pack(fill="x", padx=12, pady=(0, 4))

        self.lbl_detail_meta = ctk.CTkLabel(
            self.detail_badges_row,
            text="Penerbit: - | Frekuensi: - | Kontak: -",
            font=ctk.CTkFont(family="Segoe UI", size=10),
            text_color="#94a3b8",
            anchor="w",
        )
        self.lbl_detail_meta.pack(anchor="w")

        scope_box_frame = ctk.CTkFrame(detail_card, fg_color="transparent")
        scope_box_frame.pack(fill="x", padx=12, pady=(0, 8))

        lbl_scope_heading = ctk.CTkLabel(
            scope_box_frame,
            text="🎯 Focus & Scope (Ruang Lingkup Penelitian):",
            font=ctk.CTkFont(family="Segoe UI", size=10, weight="bold"),
            text_color="#38bdf8",
        )
        lbl_scope_heading.pack(anchor="w", pady=(0, 2))

        self.txt_detail_scope = ctk.CTkTextbox(
            scope_box_frame,
            height=60,
            font=ctk.CTkFont(family="Segoe UI", size=11),
            wrap="word",
        )
        self.txt_detail_scope.pack(fill="x")
        self.txt_detail_scope.insert("1.0", "-")
        self.txt_detail_scope.configure(state="disabled")

    def _reset_filters(self):
        self.entry_filter.delete(0, "end")
        self.combo_sinta.set("Semua SINTA")
        self.combo_apc.set("Semua Biaya")
        self.combo_server.set("Semua Server")
        self._apply_filters()

    def _apply_filters(self):
        query = self.entry_filter.get().strip().lower()
        filter_sinta = self.combo_sinta.get().strip()
        filter_apc = self.combo_apc.get().strip()
        filter_server = self.combo_server.get().strip()

        matched = []
        for j in self._all_journals:
            if filter_sinta != "Semua SINTA":
                if filter_sinta.lower() not in j.peringkat.lower().replace(" ", ""):
                    continue

            if filter_apc == "Hanya Gratis (Rp 0)":
                apc_low = j.apc_status.lower()
                is_free = (j.apc_amount == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("free" in apc_low)
                if not is_free:
                    continue
            elif filter_apc == "Berbayar":
                apc_low = j.apc_status.lower()
                is_free = (j.apc_amount == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("free" in apc_low)
                if is_free or (j.apc_amount is None and j.apc_status == INFO_NOT_FOUND):
                    continue

            if filter_server != "Semua Server":
                if filter_server.lower() not in j.status_server.lower():
                    continue

            if query:
                haystack = f"{j.nama} {j.penerbit} {j.focus_scope} {j.email} {j.telepon} {j.peringkat} {j.link}".lower()
                if query not in haystack:
                    continue

            matched.append(j)

        self._filtered_journals = matched
        self._populate_tree()

    def _populate_tree(self):
        for item in self.tree.get_children():
            self.tree.delete(item)

        for idx, j in enumerate(self._filtered_journals, start=1):
            contact_str = j.email if j.email != INFO_NOT_FOUND else j.telepon
            short_scope = j.focus_scope.replace("\n", " ") if j.focus_scope != INFO_NOT_FOUND else "-"
            if len(short_scope) > 75:
                short_scope = short_scope[:72] + "..."

            self.tree.insert(
                "",
                "end",
                iid=str(idx - 1),
                values=(
                    idx,
                    j.nama,
                    j.peringkat,
                    j.apc_status,
                    j.status_server,
                    j.transparansi_kontak,
                    j.frekuensi,
                    contact_str,
                    short_scope,
                ),
            )

        self.lbl_count_match.configure(text=f"{len(self._filtered_journals)} / {len(self._all_journals)} Jurnal")

    def _on_tree_select(self, event):
        selected = self.tree.selection()
        if not selected:
            return
        idx = int(selected[0])
        if 0 <= idx < len(self._filtered_journals):
            j = self._filtered_journals[idx]
            self._selected_journal = j
            self.lbl_detail_title.configure(text=f"📚 {j.nama} ({j.peringkat})")
            meta_txt = (
                f"🏛️ Penerbit: {j.penerbit} | 🔄 Frekuensi: {j.frekuensi} | "
                f"💰 Biaya: {j.apc_status} | 🌐 Server: {j.status_server} | "
                f"🛡️ Transparansi: {j.transparansi_kontak}\n"
                f"📧 Email: {j.email} | 📞 WA/Telp: {j.telepon}"
            )
            self.lbl_detail_meta.configure(text=meta_txt)

            self.txt_detail_scope.configure(state="normal")
            self.txt_detail_scope.delete("1.0", "end")
            self.txt_detail_scope.insert("1.0", j.focus_scope if j.focus_scope else INFO_NOT_FOUND)
            self.txt_detail_scope.configure(state="disabled")

            self.btn_open_url.configure(state="normal")

    def _on_tree_double_click(self, event):
        self._open_selected_url()

    def _open_selected_url(self):
        if self._selected_journal and self._selected_journal.link:
            webbrowser.open(self._selected_journal.link)

    def _open_excel_file(self):
        if self._excel_path and os.path.exists(self._excel_path):
            try:
                os.startfile(self._excel_path)
            except Exception:
                subprocess.Popen(f'explorer "{os.path.abspath(self._excel_path)}"')

    def update_data(self, journals: list[JournalEntry], excel_path: Optional[str] = None):
        self._all_journals = list(journals)
        self._filtered_journals = list(journals)
        self._excel_path = excel_path
        self.lbl_stats.configure(
            text=f"Total: {len(self._all_journals)} jurnal | Dobel-klik baris untuk membuka OJS di Web Browser"
        )
        self._apply_filters()


def main():
    """Fungsi entry point untuk menjalankan antarmuka grafis JSFS v1.3.8."""
    app = JSFSApp()
    app.mainloop()


if __name__ == "__main__":
    main()
