"""
app_ui.py — JSFS v1.7.0 (Deep APC Hunter Engine & Modern Bright GUI)
Antarmuka desktop dengan responsivitas tinggi (Pixel-Perfect, Zero Clipping),
Soft Slate Daylight Theme, Live KPI Pills Counter, dan 1-Click Share List.
"""
from __future__ import annotations

import os
import queue
import re
import sys
import threading
import time
import webbrowser
from tkinter import filedialog, ttk
from typing import Optional, List, Dict, Any

import customtkinter as ctk

import ai_engine
import ai_installer
import cache_engine
from month_parser import lolos_filter_bulan, format_bulan_terbit
import exporter
# Modul pembaca berkas naskah dengan fallback aman
try:
    from doc_reader import (
        extract_manuscript_metadata as parse_manuscript,
        calculate_match_score as calculate_manuscript_match,
        calculate_keyword_match_score,
    )
except ImportError:
    try:
        from doc_reader import parse_manuscript, calculate_manuscript_match, calculate_keyword_match_score
    except ImportError:
        def parse_manuscript(p: str) -> dict:
            return {"status": "empty", "keywords": [], "dominant_terms": [], "title": ""}
        def calculate_manuscript_match(meta: dict, j: any) -> dict:
            return {"score": 0, "level": "Kurang Sesuai", "matched_terms": [], "explanation": ""}
        def calculate_keyword_match_score(k: str, j: any) -> dict:
            return {"score": 0, "level": "Kurang Sesuai", "matched_terms": [], "explanation": ""}

from scraper_garuda import (
    JournalEntry,
    scrape_garuda,
    merge_cross_source_entries,
    get_high_capacity_session,
    INFO_NOT_FOUND,
    DEAD_LINK_TEXT,
)
from scraper_sinta import scrape_sinta, scrape_sinta_selenium

# ───────────────────────── Konstanta Desain Soft Slate Daylight (v1.7.0) ─────────────────────────
PAD, GAP = 12, 6                      # padding kartu, jarak antar elemen
BG_MAIN = ("#f8fafc", "#18181b")       # Soft Slate Daylight / Night
BG_CARD = ("#ffffff", "#242427")       # Card surface
BORDER_CARD = ("#e2e8f0", "#3f3f46")   # Subtle border
HEADER_NAVY = "#1e3a5f"                # Academic Navy
BLUE = ("#0284c7", "#1f6aa5")          # Sky-600 / Dark blue
BLUE_H = ("#0369a1", "#144870")
GREEN = ("#059669", "#13a06f")        # Emerald-600 / Dark green
GREEN_H = ("#047857", "#0e7a55")
INDIGO = ("#6366f1", "#4f46e5")        # Indigo-500
INDIGO_H = ("#4f46e5", "#4338ca")
RED = ("#dc2626", "#c0392b")           # Red-600
RED_H = ("#b91c1c", "#962d22")
TEXT_PRIMARY = ("#1e293b", "#f1f5f9")  # Slate-800 / Slate-100
MUTED = ("#64748b", "#94a3b8")         # Slate-500 / Slate-400
OK_GREEN = ("#16a34a", "#22a45d")

ctk.set_appearance_mode("light")       # Daylight mode default (VISION.md Bagian 4.1)!
ctk.set_default_color_theme("blue")


# ───────────────────────── Helper Anti-Clipping ─────────────────────────
def fit_w(font: ctk.CTkFont, text: str, base: int = 0, margin: int = 16) -> int:
    """Lebar minimum agar `text` tidak terpotong (Zero Clipping)."""
    return base + font.measure(text) + margin


class FitCheckBox(ctk.CTkCheckBox):
    def __init__(self, master, text: str, font: ctk.CTkFont, **kw):
        kw.setdefault("checkbox_width", 20)
        kw.setdefault("checkbox_height", 20)
        kw.setdefault("width", fit_w(font, text, kw["checkbox_width"] + 10))
        super().__init__(master, text=text, font=font, **kw)


class FitSwitch(ctk.CTkSwitch):
    def __init__(self, master, text: str, font: ctk.CTkFont, **kw):
        kw.setdefault("width", fit_w(font, text, 36 + 10))
        super().__init__(master, text=text, font=font, **kw)


def make_card(parent, title: str, font_h: ctk.CTkFont):
    """Kartu seragam: judul + body. Kembalikan (card, body, header)."""
    card = ctk.CTkFrame(parent, corner_radius=10, fg_color=BG_CARD, border_width=1, border_color=BORDER_CARD)
    card.grid_columnconfigure(0, weight=1)
    card.grid_rowconfigure(1, weight=1)
    header = ctk.CTkFrame(card, fg_color="transparent")
    header.grid(row=0, column=0, sticky="ew", padx=PAD, pady=(PAD, GAP))
    header.grid_columnconfigure(0, weight=1)
    ctk.CTkLabel(header, text=title, font=font_h, anchor="w", text_color=TEXT_PRIMARY).grid(row=0, column=0, sticky="w")
    body = ctk.CTkFrame(card, fg_color="transparent")
    body.grid(row=1, column=0, sticky="nsew", padx=PAD, pady=(0, PAD))
    return card, body, header


def fmt_ranges(nums) -> str:
    nums = sorted(nums)
    if not nums:
        return "—"
    out, start, prev = [], nums[0], nums[0]
    for n in nums[1:] + [None]:
        if n is not None and n == prev + 1:
            prev = n
            continue
        out.append(f"S{start}" if start == prev else f"S{start}–S{prev}")
        start = prev = n
    return ", ".join(out)


# ───────────────────────── Komponen: Filter Bulan Terbit ─────────────────────────
class MonthFilter(ctk.CTkFrame):
    NAMES = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"]
    COLS = 4                                         # 4 kolom x 3 baris

    def __init__(self, master, font: ctk.CTkFont, **kw):
        super().__init__(master, fg_color="transparent", **kw)
        self.vars = [ctk.BooleanVar(value=True) for _ in self.NAMES]
        self.var_unknown = ctk.BooleanVar(value=True)
        for c in range(self.COLS):
            self.grid_columnconfigure(c, weight=1, uniform="mon")

        w = max(fit_w(font, n, 18 + 8) for n in self.NAMES)
        for i, name in enumerate(self.NAMES):
            FitCheckBox(self, name, font, variable=self.vars[i], width=w,
                        checkbox_width=18, checkbox_height=18).grid(
                row=i // self.COLS, column=i % self.COLS, sticky="w", padx=4, pady=3)

        tools = ctk.CTkFrame(self, fg_color="transparent")
        tools.grid(row=3, column=0, columnspan=self.COLS, sticky="ew", pady=(GAP, 0))
        tools.grid_columnconfigure(2, weight=1)
        small = ctk.CTkFont(size=11)
        ctk.CTkButton(tools, text="Semua", width=64, height=24, font=small,
                      command=lambda: self._set_all(True)).grid(row=0, column=0, padx=(0, 4))
        ctk.CTkButton(tools, text="Kosongkan", width=80, height=24, font=small,
                      fg_color="#3a3a3a", hover_color="#4a4a4a",
                      command=lambda: self._set_all(False)).grid(row=0, column=1, padx=(0, 8))
        self.lbl_count = ctk.CTkLabel(tools, text="12/12 bulan", font=small, text_color=MUTED, anchor="e")
        self.lbl_count.grid(row=0, column=2, sticky="e")

        FitCheckBox(self, "Sertakan jurnal yang bulannya tidak terdeteksi", small,
                    variable=self.var_unknown, checkbox_width=18, checkbox_height=18).grid(
            row=4, column=0, columnspan=self.COLS, sticky="w", padx=4, pady=(GAP, 0))

        for v in self.vars:
            v.trace_add("write", lambda *_: self._update_count())

    def _set_all(self, flag: bool):
        for v in self.vars:
            v.set(flag)

    def _update_count(self):
        self.lbl_count.configure(text=f"{sum(v.get() for v in self.vars)}/12 bulan")

    def selected_months(self) -> set[int]:
        return {i + 1 for i, v in enumerate(self.vars) if v.get()}

    def include_unknown(self) -> bool:
        return bool(self.var_unknown.get())


# ───────────────────────── Komponen: Progress Ganda ─────────────────────────
class DualProgress(ctk.CTkFrame):
    ROWS = (
        ("🌐 Tahap 1: Mengumpulkan Data Jurnal (SINTA & Garuda)", "scrape", BLUE),
        ("🤖 Tahap 2: Menganalisis Biaya & Fokus Keilmuan (AI / Smart Parser)", "ai", "#e08a1e"),
    )

    def __init__(self, master, font: ctk.CTkFont, **kw):
        super().__init__(master, fg_color=BG_CARD, corner_radius=10, border_width=1, border_color=BORDER_CARD, **kw)
        self.grid_columnconfigure(1, weight=1)
        lw = max(font.measure(t) for t, _, _ in self.ROWS) + 20
        cw = font.measure("9999/9999") + 20
        self.bars, self.counts = {}, {}
        for r, (text, key, color) in enumerate(self.ROWS):
            pady = (PAD, GAP) if r == 0 else (0, PAD)
            ctk.CTkLabel(self, text=text, font=font, anchor="w", width=lw).grid(
                row=r, column=0, sticky="w", padx=(PAD, GAP), pady=pady)
            bar = ctk.CTkProgressBar(self, height=10, progress_color=color)
            bar.set(0)
            bar.grid(row=r, column=1, sticky="ew", padx=GAP, pady=pady)
            cnt = ctk.CTkLabel(self, text="0/0", font=font, anchor="e", width=cw, text_color=MUTED)
            cnt.grid(row=r, column=2, sticky="e", padx=(GAP, PAD), pady=pady)
            self.bars[key], self.counts[key] = bar, cnt

    def update_bar(self, key: str, done: int, total: int):
        if key in self.bars:
            self.bars[key].set(0 if total <= 0 else min(1.0, done / total))
            self.counts[key].configure(text=f"{done}/{total}")

    def reset(self):
        for k in self.bars:
            self.update_bar(k, 0, 0)


# ───────────────────────── Pipeline Backend ─────────────────────────
def run_scraper(params: dict, on_progress, cancel: threading.Event, on_log=None) -> list[JournalEntry]:
    """
    Fase 1: Fast Broad Scraping (128 workers HTTP/Selenium).
    Mengumpulkan data mentah (raw_about, raw_submissions, bulan_terbit) tanpa menyentuh LLM.
    v1.6.1: High-Yield Candidate Pool Multiplier (3x over-fetching / minimal 60 kandidat).
    v1.7.1: Smart Typo Correction & Synonym Auto-Fallback saat hasil = 0.
    """
    from doc_reader import get_fallback_keywords

    keyword = params["keyword"]
    target = int(params["target"])
    candidate_target = max(target * 3, 60)
    sinta_ranks = sorted(list(params["sinta"]))
    use_garuda = params.get("garuda", True)
    use_sinta = params.get("sinta_src", True)
    turbo = params.get("turbo", True)
    use_selenium = params.get("selenium", False)

    def _scrape_pass(kw_query: str) -> list[JournalEntry]:
        pass_records: list[JournalEntry] = []
        if use_garuda and not cancel.is_set():
            g_results = scrape_garuda(
                keyword=kw_query,
                sinta_ranks=sinta_ranks,
                deep_detail=True,
                max_results=candidate_target,
                turbo_mode=turbo,
                free_only=False,
                include_unknown_apc=True,
            )
            pass_records.extend(g_results)
            on_progress(len(pass_records), target)

        if use_sinta and not cancel.is_set():
            if use_selenium:
                s_results = scrape_sinta_selenium(
                    keyword=kw_query,
                    sinta_ranks=sinta_ranks,
                    deep_detail=True,
                    max_results=candidate_target,
                    turbo_mode=turbo,
                    free_only=False,
                    include_unknown_apc=True,
                )
            else:
                s_results = scrape_sinta(
                    keyword=kw_query,
                    sinta_ranks=sinta_ranks,
                    deep_detail=True,
                    max_results=candidate_target,
                    turbo_mode=turbo,
                    free_only=False,
                    include_unknown_apc=True,
                    allow_selenium_fallback=False,
                    log_cb=on_log,
                )
            pass_records.extend(s_results)
            on_progress(len(pass_records), target)
        return pass_records

    records: list[JournalEntry] = _scrape_pass(keyword)

    if not records and not cancel.is_set():
        fallback_kws = get_fallback_keywords(keyword)
        if fallback_kws:
            warn_msg = f"⚠️ Kata kunci '{keyword}' tidak menemukan hasil. Mencoba perluasan kata kunci / koreksi ejaan..."
            if on_log:
                on_log(warn_msg)
            for fb_kw in fallback_kws:
                if cancel.is_set():
                    break
                if on_log:
                    on_log(f"↻ Mencoba kata kunci alternatif: '{fb_kw}'")
                fb_records = _scrape_pass(fb_kw)
                if fb_records:
                    records.extend(fb_records)
                    break

    if records:
        records = merge_cross_source_entries(records)

    on_progress(len(records), target)
    return records


# ───────────────────────── Data Previewer Interaktif ─────────────────────────
class JSFSDataPreviewer(ctk.CTkToplevel):
    """
    Jendela Pop-up Interaktif Data Table Previewer v1.7.0.
    Menampilkan data terstruktur dengan kolom Submit, Publikasi, Bulan Terbit, Sumber, dan Copy-Paste List Generator.
    """

    def __init__(self, master, journals: list[JournalEntry], excel_path: Optional[str] = None):
        super().__init__(master)

        self.title("👁️ JSFS Data Table Previewer v1.7.0 — Hasil Scraping Jurnal")
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
            text="📊 Data Table Previewer v1.7.0",
            font=ctk.CTkFont(family="Segoe UI", size=18, weight="bold"),
            text_color=("#0284c7", "#38bdf8"),
        )
        title_lbl.pack(side="left")

        self.lbl_stats = ctk.CTkLabel(
            header_bar,
            text=f"Total: {len(self._all_journals)} jurnal | Dobel-klik baris untuk membuka situs OJS",
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

        self.btn_copy_share = ctk.CTkButton(
            header_bar,
            text="📋 Salin Semua Daftar (Format Share)",
            width=230,
            height=30,
            fg_color="#6366f1",
            hover_color="#4f46e5",
            font=ctk.CTkFont(family="Segoe UI", size=11, weight="bold"),
            command=self._copy_share_list,
        )
        self.btn_copy_share.pack(side="right", padx=(8, 0))

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
        filter_card = ctk.CTkFrame(container, corner_radius=8, fg_color=BG_CARD)
        filter_card.pack(fill="x", pady=(0, 8), padx=2)

        filter_row = ctk.CTkFrame(filter_card, fg_color="transparent")
        filter_row.pack(fill="x", padx=10, pady=6)

        ctk.CTkLabel(filter_row, text="🔍 Cari:", font=ctk.CTkFont(size=11, weight="bold")).pack(side="left", padx=(0, 6))

        self.entry_filter = ctk.CTkEntry(
            filter_row,
            placeholder_text="Ketik nama jurnal, topik, penerbit, focus & scope...",
            font=ctk.CTkFont(size=11),
            height=28,
        )
        self.entry_filter.pack(side="left", fill="x", expand=True, padx=(0, 8))
        self.entry_filter.bind("<KeyRelease>", lambda e: self._apply_filters())

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

        self.combo_src = ctk.CTkComboBox(
            filter_row,
            values=["Semua Sumber", "Hanya AI", "Hanya Regex"],
            width=120,
            height=28,
            font=ctk.CTkFont(size=11),
            command=lambda val: self._apply_filters(),
        )
        self.combo_src.set("Semua Sumber")
        self.combo_src.pack(side="left", padx=(0, 6))

        btn_reset = ctk.CTkButton(
            filter_row,
            text="🔄 Reset",
            width=70,
            height=28,
            fg_color="#3a3a3a",
            hover_color="#4a4a4a",
            command=self._reset_filters,
        )
        btn_reset.pack(side="left", padx=(0, 6))

        self.lbl_count_match = ctk.CTkLabel(
            filter_row,
            text="0 Jurnal",
            font=ctk.CTkFont(size=11, weight="bold"),
            text_color="#38bdf8",
        )
        self.lbl_count_match.pack(side="right")

        # 3. Treeview Table Container with Dynamic Daylight/Night Styling
        tree_container = ctk.CTkFrame(container, corner_radius=8, fg_color=BG_CARD, border_width=1, border_color=BORDER_CARD)
        tree_container.pack(fill="both", expand=True, pady=(0, 8))

        style = ttk.Style()
        style.theme_use("clam")
        is_light = ctk.get_appearance_mode().lower() == "light"
        tv_bg = "#ffffff" if is_light else "#1e1e1e"
        tv_fg = "#1e293b" if is_light else "#f8fafc"
        th_bg = "#f1f5f9" if is_light else "#2d3748"
        th_fg = "#1e3a5f" if is_light else "#38bdf8"
        style.configure(
            "JSFS.Treeview",
            background=tv_bg,
            foreground=tv_fg,
            rowheight=26,
            fieldbackground=tv_bg,
            font=("Segoe UI", 10),
            borderwidth=0,
        )
        style.configure(
            "JSFS.Treeview.Heading",
            background=th_bg,
            foreground=th_fg,
            relief="flat",
            font=("Segoe UI", 10, "bold"),
            padding=4,
        )
        style.map("JSFS.Treeview", background=[("selected", "#0284c7")], foreground=[("selected", "#ffffff")])

        cols = ("no", "title", "sinta", "submit", "pub", "cur", "month", "next_issue", "src_focus", "src_apc", "scope")
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
        self.tree.heading("submit", text="Biaya Submit")
        self.tree.heading("pub", text="Biaya Publikasi")
        self.tree.heading("cur", text="Mata Uang")
        self.tree.heading("month", text="Bulan Terbit")
        self.tree.heading("next_issue", text="Terbit Terdekat")
        self.tree.heading("src_focus", text="Sumber Focus")
        self.tree.heading("src_apc", text="Sumber APC")
        self.tree.heading("scope", text="Focus & Scope (Ringkasan)")

        self.tree.column("no", width=36, anchor="center")
        self.tree.column("title", width=240, anchor="w")
        self.tree.column("sinta", width=65, anchor="center")
        self.tree.column("submit", width=105, anchor="center")
        self.tree.column("pub", width=115, anchor="center")
        self.tree.column("cur", width=75, anchor="center")
        self.tree.column("month", width=95, anchor="center")
        self.tree.column("next_issue", width=120, anchor="center")
        self.tree.column("src_focus", width=95, anchor="center")
        self.tree.column("src_apc", width=95, anchor="center")
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
        detail_card = ctk.CTkFrame(container, corner_radius=8, fg_color=BG_CARD)
        detail_card.pack(fill="x", padx=2)

        detail_top = ctk.CTkFrame(detail_card, fg_color="transparent")
        detail_top.pack(fill="x", padx=12, pady=(8, 2))

        self.lbl_detail_title = ctk.CTkLabel(
            detail_top,
            text="Pilih jurnal dari tabel di atas untuk melihat detail lengkap & Focus & Scope.",
            font=ctk.CTkFont(size=12, weight="bold"),
            text_color="#f8fafc",
            anchor="w",
        )
        self.lbl_detail_title.pack(side="left", fill="x", expand=True)

        self.btn_open_ojs = ctk.CTkButton(
            detail_top,
            text="🌐 Buka Situs OJS",
            width=130,
            height=28,
            font=ctk.CTkFont(size=11, weight="bold"),
            fg_color=BLUE,
            hover_color=BLUE_H,
            state="disabled",
            command=self._open_selected_ojs,
        )
        self.btn_open_ojs.pack(side="right")

        self.btn_copy_chat = ctk.CTkButton(
            detail_top,
            text="💬 Salin Draf Chat Editor",
            width=175,
            height=28,
            font=ctk.CTkFont(size=11, weight="bold"),
            fg_color="#0d9488",
            hover_color="#0f766e",
            state="disabled",
            command=self._copy_editor_chat_draft,
        )
        self.btn_copy_chat.pack(side="right", padx=(0, 8))

        self.detail_textbox = ctk.CTkTextbox(
            detail_card,
            height=90,
            font=ctk.CTkFont(family="Segoe UI", size=11),
            wrap="word",
        )
        self.detail_textbox.pack(fill="x", padx=12, pady=(4, 10))
        self.detail_textbox.insert("1.0", "Informasi rinci Focus & Scope serta Author Fees akan tampil di sini.")
        self.detail_textbox.configure(state="disabled")

    def _open_excel_file(self):
        if self._excel_path and os.path.exists(self._excel_path):
            try:
                os.startfile(os.path.abspath(self._excel_path))
            except Exception:
                pass

    def _copy_share_list(self):
        """Menyalin seluruh daftar jurnal yang sedang terfilter ke clipboard dalam format siap share grup (v1.6.0)."""
        targets = self._filtered_journals if self._filtered_journals else self._all_journals
        if not targets:
            return
        share_text = exporter.generate_copypaste_list(targets)
        try:
            self.clipboard_clear()
            self.clipboard_append(share_text)
            self.update()
            self.btn_copy_share.configure(
                text="✅ Semua Daftar Tersalin ke Clipboard!",
                fg_color="#10b981",
            )
            self.after(2500, self._reset_copy_share_btn)
        except Exception:
            pass

    def _reset_copy_share_btn(self):
        try:
            if self.winfo_exists():
                self.btn_copy_share.configure(
                    text="📋 Salin Semua Daftar (Format Share)",
                    fg_color="#6366f1",
                )
        except Exception:
            pass

    def _reset_filters(self):
        self.entry_filter.delete(0, "end")
        self.combo_sinta.set("Semua SINTA")
        self.combo_src.set("Semua Sumber")
        self._apply_filters()

    def _apply_filters(self):
        query = self.entry_filter.get().strip().lower()
        sinta_filter = self.combo_sinta.get()
        src_filter = self.combo_src.get()

        filtered = []
        for j in self._all_journals:
            # SINTA
            if sinta_filter != "Semua SINTA":
                if sinta_filter not in j.peringkat_sinta.replace("Sinta ", "S"):
                    continue
            # Sumber
            if src_filter == "Hanya AI":
                if getattr(j, "sumber_focus", "") != "AI" and getattr(j, "sumber_apc", "") != "AI":
                    continue
            elif src_filter == "Hanya Regex":
                if getattr(j, "sumber_focus", "") == "AI" or getattr(j, "sumber_apc", "") == "AI":
                    continue
            # Query
            if query:
                combined_txt = f"{j.nama_jurnal} {j.penerbit} {j.bidang} {j.focus_scope} {j.email} {j.telepon}".lower()
                if query not in combined_txt:
                    continue
            filtered.append(j)

        self._filtered_journals = filtered
        self.lbl_count_match.configure(text=f"{len(filtered)} Jurnal")
        self._populate_tree()

    def _populate_tree(self):
        for item in self.tree.get_children():
            self.tree.delete(item)

        for i, j in enumerate(self._filtered_journals, 1):
            cur = getattr(j, "mata_uang", None) or "—"
            b_sub = exporter._format_amount(getattr(j, "biaya_submit", None), getattr(j, "mata_uang", None))
            b_pub_num = getattr(j, "biaya_publikasi", None)
            if b_pub_num is None and getattr(j, "apc_amount", None) is not None:
                b_pub_num = getattr(j, "apc_amount", None)
            apc_raw = getattr(j, "apc_status", "Cek Web / Belum Terdata")
            if apc_raw == "Gratis (Rp 0)":
                b_pub = "Gratis (Rp 0)"
            elif b_pub_num is not None:
                b_pub = exporter._format_amount(b_pub_num, getattr(j, "mata_uang", None))
            elif apc_raw and any(c.isdigit() for c in apc_raw) and "tidak dapat diakses" not in apc_raw.lower():
                b_pub = apc_raw
            else:
                b_pub = "Cek Web / Belum Terdata"

            bulan = format_bulan_terbit(getattr(j, "bulan_terbit", []))
            terbit_est = exporter.get_next_issue_estimate(getattr(j, "bulan_terbit", []))
            s_focus = getattr(j, "sumber_focus", "Regex") or "Regex"
            s_apc = getattr(j, "sumber_apc", "Regex") or "Regex"
            scope_snip = (getattr(j, "focus_scope", "") or "—")[:120]

            self.tree.insert(
                "",
                "end",
                iid=str(i - 1),
                values=(
                    str(i),
                    j.nama_jurnal,
                    j.peringkat_sinta,
                    b_sub,
                    b_pub,
                    cur,
                    bulan,
                    terbit_est,
                    s_focus,
                    s_apc,
                    scope_snip,
                ),
            )

    def _on_tree_select(self, event):
        sel = self.tree.selection()
        if not sel:
            return
        idx = int(sel[0])
        if idx < len(self._filtered_journals):
            j = self._filtered_journals[idx]
            self._selected_journal = j
            self.lbl_detail_title.configure(text=f"📑 {j.nama_jurnal} ({j.peringkat_sinta}) • {j.penerbit}")
            self.btn_open_ojs.configure(state="normal" if (j.link_ojs and j.link_ojs.startswith("http")) else "disabled")
            self.btn_copy_chat.configure(state="normal")

            b_sub = exporter._format_amount(getattr(j, "biaya_submit", None), getattr(j, "mata_uang", None))
            b_pub_num = getattr(j, "biaya_publikasi", None)
            if b_pub_num is None and getattr(j, "apc_amount", None) is not None:
                b_pub_num = getattr(j, "apc_amount", None)
            b_pub = exporter._format_amount(b_pub_num, getattr(j, "mata_uang", None))
            bulan = format_bulan_terbit(getattr(j, "bulan_terbit", []))
            terbit_est = exporter.get_next_issue_estimate(getattr(j, "bulan_terbit", []))
            s_focus = getattr(j, "sumber_focus", "Regex") or "Regex"
            s_apc = getattr(j, "sumber_apc", "Regex") or "Regex"

            # Indikator Journal Health Warning (Tahap 4.2)
            has_no_month = not getattr(j, "bulan_terbit", [])
            em = getattr(j, "email", "")
            tel = getattr(j, "telepon", "")
            has_no_contact = (not em or em == INFO_NOT_FOUND or "tidak dapat" in str(em).lower()) and \
                             (not tel or tel == INFO_NOT_FOUND or "tidak dapat" in str(tel).lower())

            warning_note = ""
            if has_no_month and has_no_contact:
                warning_note = "\n\n⚠️ PERINGATAN KELENGKAPAN JURNAL: Jurnal ini belum mencantumkan jadwal bulan terbit dan kontak redaksi. Pastikan Anda memeriksa aktivitas terbitan terakhir langsung di situs OJS sebelum mengirimkan naskah."

            txt = (
                f"• Focus & Scope [{s_focus}]: {getattr(j, 'focus_scope', INFO_NOT_FOUND)}\n"
                f"• Biaya Penulis [{s_apc}]: Submit: {b_sub} | Publikasi: {b_pub} | Mata Uang: {getattr(j, 'mata_uang', 'IDR') or 'IDR'}\n"
                f"• Jadwal Terbit: {bulan} (Terdekat: {terbit_est}) | Frekuensi: {getattr(j, 'frekuensi', INFO_NOT_FOUND)}\n"
                f"• Kontak: Email: {getattr(j, 'email', INFO_NOT_FOUND)} | Telepon/WA: {getattr(j, 'telepon', INFO_NOT_FOUND)} | Server: {getattr(j, 'status_server', 'Aktif')}\n"
                f"• Link Website: {j.link_ojs or INFO_NOT_FOUND}"
                f"{warning_note}"
            )
            self.detail_textbox.configure(state="normal")
            self.detail_textbox.delete("1.0", "end")
            self.detail_textbox.insert("1.0", txt)
            self.detail_textbox.configure(state="disabled")

    def _copy_editor_chat_draft(self):
        """Menyalin template pesan tanya kuota publikasi sopan ke clipboard (Tahap 4.1)."""
        if not self._selected_journal:
            return
        j = self._selected_journal
        j_name = j.nama_jurnal or "Jurnal"
        kw = getattr(self.master, "entry_kw", None)
        kw_text = kw.get().strip() if kw else ""
        if not kw_text:
            kw_text = "bidang terkait"

        terbit = exporter.get_next_issue_estimate(getattr(j, "bulan_terbit", []))
        if not terbit or terbit == "—":
            terbit = getattr(j, "frekuensi", "") or "edisi terdekat"

        draft = (
            f"Selamat pagi/siang Bapak/Ibu Tim Editor {j_name},\n\n"
            f"Perkenalkan saya mahasiswa yang sedang menyusun artikel ilmiah di bidang {kw_text}. "
            f"Saya ingin menanyakan apakah kuota publikasi untuk edisi {terbit} masih tersedia? "
            f"Terima kasih banyak atas perhatian dan kesediaan Bapak/Ibu."
        )
        self.clipboard_clear()
        self.clipboard_append(draft)

        self.btn_copy_chat.configure(text="✅ Draf Tersalin!", fg_color="#16a34a")
        self.after(2000, lambda: self.btn_copy_chat.configure(text="💬 Salin Draf Chat Editor", fg_color="#0d9488"))

    def _open_selected_ojs(self):
        sel = self.tree.selection()
        if not sel:
            return
        idx = int(sel[0])
        if idx < len(self._filtered_journals):
            j = self._filtered_journals[idx]
            if j.link_ojs and j.link_ojs.startswith("http"):
                webbrowser.open(j.link_ojs)

    def _on_tree_double_click(self, event):
        self._open_selected_ojs()

    def update_data(self, journals: list[JournalEntry], excel_path: Optional[str] = None):
        self._all_journals = list(journals)
        self._excel_path = excel_path
        self._apply_filters()


# ───────────────────────── 1-Click AI Setup Manager ─────────────────────────
class AISetupDialog(ctk.CTkToplevel):
    """
    Jendela Popup 1-Click AI Setup Manager.
    Mengunduh & memasang Ollama serta model AI lokal (qwen2.5:3b)
    secara otomatis dari dalam GUI.
    """
    def __init__(self, parent: "JSFSApp"):
        super().__init__(parent)
        self.parent_app = parent
        self.title("⚙️ 1-Click AI Setup Manager — JSFS v1.7.0")
        self.geometry("580x520")
        self.minsize(540, 480)
        self.resizable(False, False)
        self.transient(parent)
        self.grab_set()

        self.cancel_evt = threading.Event()
        self._is_busy = False
        self._poll_active = True

        self._build_ui()
        self.protocol("WM_DELETE_WINDOW", self._on_close)
        self.after(300, self._start_poll_loop)

    def _build_ui(self):
        self.grid_columnconfigure(0, weight=1)
        self.configure(fg_color=BG_MAIN)

        f_pad = 16
        # Header
        hdr = ctk.CTkFrame(self, fg_color="transparent")
        hdr.grid(row=0, column=0, sticky="ew", padx=f_pad, pady=(16, 8))
        hdr.grid_columnconfigure(0, weight=1)

        ctk.CTkLabel(
            hdr,
            text="🤖 1-Click AI Setup Manager",
            font=ctk.CTkFont(size=18, weight="bold"),
            text_color=("#0284c7", "#38bdf8"),
            anchor="w",
        ).grid(row=0, column=0, sticky="w")

        ctk.CTkLabel(
            hdr,
            text="JSFS membutuhkan Ollama & Model Qwen2.5 3B (Total ~2.2 GB)\nuntuk ekstraksi cerdas Focus & Scope dan Biaya Penulis (APC).",
            font=ctk.CTkFont(size=11),
            text_color=MUTED,
            justify="left",
            anchor="w",
        ).grid(row=1, column=0, sticky="w", pady=(4, 0))

        # Status Card
        sc = ctk.CTkFrame(self, fg_color=BG_CARD, corner_radius=10, border_width=1, border_color=BORDER_CARD)
        sc.grid(row=1, column=0, sticky="ew", padx=f_pad, pady=8)
        sc.grid_columnconfigure(1, weight=1)

        ctk.CTkLabel(sc, text="Status Ollama:", font=ctk.CTkFont(size=12, weight="bold"), text_color=TEXT_PRIMARY).grid(row=0, column=0, sticky="w", padx=12, pady=(10, 4))
        self.lbl_ollama_status = ctk.CTkLabel(sc, text="Memeriksa...", font=ctk.CTkFont(size=12), text_color="#eab308")
        self.lbl_ollama_status.grid(row=0, column=1, sticky="w", padx=12, pady=(10, 4))

        ctk.CTkLabel(sc, text="Status Model (qwen2.5:3b):", font=ctk.CTkFont(size=12, weight="bold"), text_color=TEXT_PRIMARY).grid(row=1, column=0, sticky="w", padx=12, pady=(4, 10))
        self.lbl_model_status = ctk.CTkLabel(sc, text="Memeriksa...", font=ctk.CTkFont(size=12), text_color="#eab308")
        self.lbl_model_status.grid(row=1, column=1, sticky="w", padx=12, pady=(4, 10))

        # Action Buttons Frame
        btn_frame = ctk.CTkFrame(self, fg_color=BG_CARD, corner_radius=10, border_width=1, border_color=BORDER_CARD)
        btn_frame.grid(row=2, column=0, sticky="ew", padx=f_pad, pady=8)
        btn_frame.grid_columnconfigure(0, weight=1)

        self.btn_download_ollama = ctk.CTkButton(
            btn_frame,
            text="1. Unduh & Pasang Ollama (~65 MB)",
            font=ctk.CTkFont(size=13, weight="bold"),
            height=38,
            corner_radius=8,
            fg_color=BLUE,
            hover_color=BLUE_H,
            command=self._on_click_download_ollama,
        )
        self.btn_download_ollama.grid(row=0, column=0, sticky="ew", padx=12, pady=(12, 6))

        self.btn_pull_model = ctk.CTkButton(
            btn_frame,
            text="2. Unduh Model AI (qwen2.5:3b) (~2.0 GB)",
            font=ctk.CTkFont(size=13, weight="bold"),
            height=38,
            corner_radius=8,
            fg_color=GREEN,
            hover_color=GREEN_H,
            state="disabled",
            command=self._on_click_pull_model,
        )
        self.btn_pull_model.grid(row=1, column=0, sticky="ew", padx=12, pady=(6, 12))

        # Progress Frame
        p_frame = ctk.CTkFrame(self, fg_color=BG_CARD, corner_radius=10, border_width=1, border_color=BORDER_CARD)
        p_frame.grid(row=3, column=0, sticky="ew", padx=f_pad, pady=8)
        p_frame.grid_columnconfigure(0, weight=1)

        self.lbl_progress = ctk.CTkLabel(
            p_frame,
            text="Status: Menunggu instruksi pengguna.",
            font=ctk.CTkFont(size=11),
            text_color="#38bdf8",
            anchor="w",
        )
        self.lbl_progress.grid(row=0, column=0, sticky="w", padx=12, pady=(10, 4))

        self.prog_bar = ctk.CTkProgressBar(p_frame, height=12, corner_radius=6, progress_color="#38bdf8")
        self.prog_bar.grid(row=1, column=0, sticky="ew", padx=12, pady=(4, 10))
        self.prog_bar.set(0.0)

        # Bottom Bar
        btm = ctk.CTkFrame(self, fg_color="transparent")
        btm.grid(row=4, column=0, sticky="ew", padx=f_pad, pady=(8, 16))
        btm.grid_columnconfigure(0, weight=1)

        self.btn_cancel = ctk.CTkButton(
            btm,
            text="Batal Unduhan",
            font=ctk.CTkFont(size=12),
            height=32,
            width=120,
            corner_radius=6,
            fg_color="#c0392b",
            hover_color="#962d22",
            state="disabled",
            command=self._on_cancel,
        )
        self.btn_cancel.grid(row=0, column=0, sticky="w")

        self.btn_close = ctk.CTkButton(
            btm,
            text="Tutup",
            font=ctk.CTkFont(size=12),
            height=32,
            width=100,
            corner_radius=6,
            fg_color="#4a4a4a",
            hover_color="#5a5a5a",
            command=self._on_close,
        )
        self.btn_close.grid(row=0, column=1, sticky="e")

    def _start_poll_loop(self):
        if not self._poll_active or not self.winfo_exists():
            return
        if not self._is_busy:
            threading.Thread(target=self._worker_poll_status, daemon=True).start()
        self.after(2000, self._start_poll_loop)

    def _worker_poll_status(self):
        try:
            running = ai_installer.is_ollama_running(timeout=1.5)
            model_ok = ai_installer.is_model_installed(ai_installer.DEFAULT_MODEL, timeout=1.5) if running else False
            exe_found = ai_installer.find_ollama_executable() is not None
            if self.winfo_exists():
                self.after(0, lambda: self._apply_poll_status(running, model_ok, exe_found))
        except Exception:
            pass

    def _apply_poll_status(self, running: bool, model_ok: bool, exe_found: bool):
        if not self.winfo_exists() or self._is_busy:
            return

        if running:
            self.lbl_ollama_status.configure(text="● Aktif (localhost:11434)", text_color="#10b981")
            self.btn_download_ollama.configure(text="✓ Ollama Sudah Aktif", state="disabled", fg_color="#374151")
        else:
            if exe_found:
                self.lbl_ollama_status.configure(text="○ Terpasang (Belum Berjalan)", text_color="#f59e0b")
                self.btn_download_ollama.configure(text="1. Jalankan Layanan Ollama", state="normal", fg_color="#1f6aa5")
            else:
                self.lbl_ollama_status.configure(text="○ Belum Terpasang", text_color="#ef4444")
                self.btn_download_ollama.configure(text="1. Unduh & Pasang Ollama (~65 MB)", state="normal", fg_color="#1f6aa5")

        if model_ok:
            self.lbl_model_status.configure(text="● Terpasang & Siap", text_color="#10b981")
            self.btn_pull_model.configure(text="✓ Model Qwen2.5:3b Siap", state="disabled", fg_color="#374151")
            if running:
                self.lbl_progress.configure(text="✅ AI Engine siap 100%! Anda dapat langsung menggunakan fitur AI.")
                self.prog_bar.set(1.0)
                self.parent_app._apply_badge(True)
        else:
            self.lbl_model_status.configure(text="○ Belum Terpasang", text_color="#ef4444" if running else "#a0a0a0")
            if running:
                self.btn_pull_model.configure(text="2. Unduh Model AI (qwen2.5:3b) (~2.0 GB)", state="normal", fg_color="#13a06f")
            else:
                self.btn_pull_model.configure(text="2. Unduh Model AI (Membutuhkan Ollama)", state="disabled", fg_color="#374151")

    def _on_click_download_ollama(self):
        if self._is_busy:
            return
        if not ai_installer.is_ollama_running() and ai_installer.find_ollama_executable():
            self.lbl_progress.configure(text="Meluncurkan layanan Ollama di latar belakang...")
            ok = ai_installer.launch_ollama_service()
            if ok:
                self.lbl_progress.configure(text="Layanan Ollama sedang diinisiasi. Mohon tunggu beberapa detik...")
                self.after(2000, lambda: threading.Thread(target=self._worker_poll_status, daemon=True).start())
                return

        self._is_busy = True
        self.cancel_evt.clear()
        self.btn_download_ollama.configure(state="disabled")
        self.btn_pull_model.configure(state="disabled")
        self.btn_cancel.configure(state="normal")
        self.prog_bar.set(0.0)
        self.lbl_progress.configure(text="Mempersiapkan unduhan installer Ollama...")

        threading.Thread(
            target=lambda: ai_installer.download_and_install_ollama(
                progress_cb=self._safe_progress,
                done_cb=self._on_download_ollama_done,
                cancel_evt=self.cancel_evt,
            ),
            daemon=True,
        ).start()

    def _on_download_ollama_done(self, success: bool, message: str):
        def _ui():
            self._is_busy = False
            self.btn_cancel.configure(state="disabled")
            self.lbl_progress.configure(text=message)
            if success:
                self.prog_bar.set(1.0)
            threading.Thread(target=self._worker_poll_status, daemon=True).start()
        if self.winfo_exists():
            self.after(0, _ui)

    def _on_click_pull_model(self):
        if self._is_busy:
            return
        self._is_busy = True
        self.cancel_evt.clear()
        self.btn_download_ollama.configure(state="disabled")
        self.btn_pull_model.configure(state="disabled")
        self.btn_cancel.configure(state="normal")
        self.prog_bar.set(0.0)
        self.lbl_progress.configure(text=f"Mempersiapkan pull model {ai_installer.DEFAULT_MODEL}...")

        threading.Thread(
            target=lambda: ai_installer.pull_model(
                model_name=ai_installer.DEFAULT_MODEL,
                progress_cb=self._safe_progress,
                done_cb=self._on_pull_model_done,
                cancel_evt=self.cancel_evt,
            ),
            daemon=True,
        ).start()

    def _on_pull_model_done(self, success: bool, message: str):
        def _ui():
            self._is_busy = False
            self.btn_cancel.configure(state="disabled")
            self.lbl_progress.configure(text=message)
            if success:
                self.prog_bar.set(1.0)
                self.parent_app._apply_badge(True)
            threading.Thread(target=self._worker_poll_status, daemon=True).start()
        if self.winfo_exists():
            self.after(0, _ui)

    def _safe_progress(self, current: int, total: int, pct: float, msg: str):
        if not self.winfo_exists():
            return
        fraction = max(0.0, min(1.0, pct / 100.0))
        self.after(0, lambda: self._apply_progress(fraction, msg))

    def _apply_progress(self, fraction: float, msg: str):
        if self.winfo_exists():
            self.prog_bar.set(fraction)
            self.lbl_progress.configure(text=msg)

    def _on_cancel(self):
        self.cancel_evt.set()
        self.lbl_progress.configure(text="Membatalkan proses unduhan...")
        self.btn_cancel.configure(state="disabled")

    def _on_close(self):
        self._poll_active = False
        self.cancel_evt.set()
        self.grab_release()
        self.destroy()


# ───────────────────────── Panduan Singkat Dialog (v1.7.0) ─────────────────────────
class QuickGuideDialog(ctk.CTkToplevel):
    """
    Dialog Modal Panduan Singkat Pemula (JSFS v1.7.0).
    Menjelaskan alur pencarian jurnal, peringkat SINTA, biaya APC, dan tips komunikasi editor.
    """
    def __init__(self, parent: "JSFSApp"):
        super().__init__(parent)
        self.title("❓ Panduan Singkat Penggunaan JSFS v1.7.0")
        self.geometry("640x560")
        self.minsize(580, 500)
        self.transient(parent)
        self.grab_set()

        self._build_ui()
        self.after(100, self.lift)

    def _build_ui(self):
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(1, weight=1)
        self.configure(fg_color=BG_MAIN)

        f_pad = 16
        # Header
        hdr = ctk.CTkFrame(self, fg_color="transparent")
        hdr.grid(row=0, column=0, sticky="ew", padx=f_pad, pady=(16, 8))
        hdr.grid_columnconfigure(0, weight=1)

        ctk.CTkLabel(
            hdr,
            text="📚 Panduan Singkat JSFS untuk Mahasiswa & Peneliti",
            font=ctk.CTkFont(size=17, weight="bold"),
            text_color=("#0284c7", "#38bdf8"),
            anchor="w",
        ).grid(row=0, column=0, sticky="w")

        ctk.CTkLabel(
            hdr,
            text="Tips praktis memilih jurnal ilmiah yang tepat, cepat, dan sesuai anggaran.",
            font=ctk.CTkFont(size=11),
            text_color=MUTED,
            anchor="w",
        ).grid(row=1, column=0, sticky="w", pady=(2, 0))

        # Scrollable content
        body = ctk.CTkScrollableFrame(self, fg_color=BG_CARD, corner_radius=10, border_width=1, border_color=BORDER_CARD)
        body.grid(row=1, column=0, sticky="nsew", padx=f_pad, pady=8)
        body.grid_columnconfigure(0, weight=1)

        sections = [
            ("1. Cara Mencari Jurnal Berdasarkan Topik Skripsi",
             "• Masukkan topik utama skripsi pada kolom 'Kata Kunci' (misal: 'akuntansi publik', 'machine learning', 'hukum perdata').\n"
             "• Jika Anda sudah memiliki naskah (.docx atau .pdf), klik tombol 'Unggah' dan centang 'Match Score' agar AI mencocokkan naskah dengan Focus & Scope jurnal."),

            ("2. Memahami Peringkat SINTA (S1 s.d. S6)",
             "• SINTA 1 & 2: Jurnal bereputasi tertinggi (terindeks Scopus/WoS), cocok untuk syarat kelulusan magister/doktoral atau hibah.\n"
             "• SINTA 3 & 4: Jurnal bereputasi menengah dengan proses telaah (review) stabil, sangat ideal untuk syarat skripsi S1.\n"
             "• SINTA 5 & 6: Jurnal baru/berkembang dengan seleksi lebih fleksibel dan peluang terbit lebih cepat."),

            ("3. Biaya Penulis (Author Fees / APC)",
             "• Biaya Submit: Biaya saat pertama kali mendaftarkan naskah (mayoritas Rp 0 / gratis).\n"
             "• Biaya Publikasi: Biaya yang dibayar HANYA setelah naskah diterima (Accepted).\n"
             "• Gunakan preset 'Jurnal Gratis' jika kampus tidak menyediakan subsidi publikasi."),

            ("4. Memilih Jadwal Terbit Terdekat",
             "• Periksa kolom 'Jadwal Terbit' dan 'Terbit Terdekat' pada hasil scraping.\n"
             "• Kirimkan naskah minimal 2-3 bulan sebelum bulan terbit target agar ada waktu telaah sejawat (peer review) dan revisi."),

            ("5. Salin Ringkas Siap Share & Hubungi Editor (Fitur v1.7.0)",
             "• Pada jendela utama atau pop-up Preview, klik tombol '📋 Salin Daftar (Share WA)' untuk menyalin daftar terformat ke WhatsApp/Telegram grup.\n"
             "• Anda juga dapat memilih jurnal lalu klik '💬 Salin Draf Chat Editor' untuk mengirim draf pesan menanyakan kuota terbitan.")
        ]

        for i, (stitle, sdesc) in enumerate(sections):
            card = ctk.CTkFrame(body, fg_color=("#f1f5f9", "#1e1e1e"), corner_radius=8, border_width=1, border_color=BORDER_CARD)
            card.pack(fill="x", padx=8, pady=6)
            card.grid_columnconfigure(0, weight=1)

            ctk.CTkLabel(
                card,
                text=stitle,
                font=ctk.CTkFont(size=12, weight="bold"),
                text_color=("#b45309", "#f59e0b"),
                anchor="w",
            ).grid(row=0, column=0, sticky="w", padx=12, pady=(10, 4))

            ctk.CTkLabel(
                card,
                text=sdesc,
                font=ctk.CTkFont(size=11),
                text_color=TEXT_PRIMARY,
                justify="left",
                anchor="w",
                wraplength=520,
            ).grid(row=1, column=0, sticky="w", padx=12, pady=(0, 10))

        # Bottom Close Button
        btn_close = ctk.CTkButton(
            self,
            text="Tutup Panduan",
            font=ctk.CTkFont(size=13, weight="bold"),
            height=36,
            fg_color="#334155",
            hover_color="#475569",
            command=self.destroy,
        )
        btn_close.grid(row=2, column=0, sticky="ew", padx=f_pad, pady=(6, 14))


# ───────────────────────── Aplikasi Utama ─────────────────────────
class JSFSApp(ctk.CTk):
    def __init__(self):
        super().__init__()
        self.title("JSFS v1.7.0 — Journal Scraper For Students (Deep APC Hunter Engine)")
        self.geometry("1040x870")
        self.minsize(980, 640)                                  # Zero Clipping aturan #3

        self.ui_q: queue.Queue = queue.Queue()
        self.cancel_evt = threading.Event()
        self._worker: threading.Thread | None = None
        self._ai_auto_off = False
        self.manuscript_path: str | None = None
        self._latest_results: list[JournalEntry] = []
        self._latest_excel_path: str | None = None
        self._previewer_window: Optional[JSFSDataPreviewer] = None
        self._ai_setup_dialog: Optional[AISetupDialog] = None
        self._guide_dialog: Optional[QuickGuideDialog] = None
        self.log_box: Optional[ctk.CTkTextbox] = None

        # Grid Window: baris 1 (body) fleksibel, baris 0 & 2 dipatok
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(1, weight=1)

        self._build_fonts()
        self._build_header()
        self._build_body()
        self._build_actionbar()

        self.protocol("WM_DELETE_WINDOW", self._on_close)
        self.after(80, self._poll_queue)             # pompa pesan thread -> GUI (80 ms)
        self.after(300, self._refresh_badge)         # cek status Ollama (loop 30 dtk)

    # ── fonts ──
    def _build_fonts(self):
        self.f_title = ctk.CTkFont(size=20, weight="bold")
        self.f_h = ctk.CTkFont(size=13, weight="bold")
        self.f_n = ctk.CTkFont(size=12)
        self.f_s = ctk.CTkFont(size=11)
        self.f_mono = ctk.CTkFont(family="Consolas", size=12)
        self.f_btn = ctk.CTkFont(size=14, weight="bold")

    # ── header + badge Ollama + Panduan + Sakelar Tema ──
    def _build_header(self):
        hdr = ctk.CTkFrame(self, fg_color="transparent")
        hdr.grid(row=0, column=0, sticky="ew", padx=PAD + 4, pady=(PAD, GAP))
        hdr.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(hdr, text="JSFS v1.7.0 — Journal Scraper For Students",
                     font=self.f_title, text_color=("#0284c7", "#38bdf8"), anchor="w").grid(row=0, column=0, sticky="w")
        ctk.CTkLabel(hdr, text="Deep APC Hunter • Soft Slate Daylight • Zero-Crash Shield • 1-Click Share List",
                     font=self.f_s, text_color=MUTED, anchor="w").grid(row=1, column=0, sticky="w", pady=(2, 0))

        btn_box = ctk.CTkFrame(hdr, fg_color="transparent")
        btn_box.grid(row=0, column=1, rowspan=2, sticky="e", padx=(GAP, 0))

        # Sakelar Tema ☀️/🌙 (VISION.md Bagian 4.1 & F2)
        self.btn_theme = ctk.CTkButton(
            btn_box,
            text="🌙 Mode Malam",
            font=self.f_s,
            height=28,
            corner_radius=14,
            fg_color=("#e2e8f0", "#334155"),
            hover_color=("#cbd5e1", "#475569"),
            text_color=("#1e293b", "#f1f5f9"),
            command=self._toggle_theme,
        )
        self.btn_theme.pack(side="left", padx=(0, 6))

        self.btn_help = ctk.CTkButton(
            btn_box,
            text="❓ Panduan Singkat",
            font=self.f_s,
            height=28,
            corner_radius=14,
            fg_color=("#e2e8f0", "#334155"),
            hover_color=("#cbd5e1", "#475569"),
            text_color=("#1e293b", "#f1f5f9"),
            command=self._open_quick_guide,
        )
        self.btn_help.pack(side="left", padx=(0, 6))

        texts = (f"●  AI Aktif · {ai_engine.MODEL}", "○  Mode Regex (Klik untuk Setup)")
        self.badge = ctk.CTkButton(
            btn_box,
            text=texts[1],
            font=self.f_s,
            height=28,
            corner_radius=14,
            fg_color="#4a4a4a",
            hover_color="#5a5a5a",
            text_color="#d0d0d0",
            cursor="hand2",
            width=max(fit_w(self.f_s, t, 0, 28) for t in texts) + 16,
            command=self._open_ai_setup_manager,
        )
        self.badge.pack(side="left")

    def _toggle_theme(self):
        """Mengganti tema antarmuka antara Daylight (Terang) dan Night (Gelap)."""
        curr = ctk.get_appearance_mode().lower()
        if curr == "light":
            ctk.set_appearance_mode("dark")
            self.btn_theme.configure(text="☀️ Mode Terang")
        else:
            ctk.set_appearance_mode("light")
            self.btn_theme.configure(text="🌙 Mode Malam")

    def _open_quick_guide(self):
        if self._guide_dialog is not None and self._guide_dialog.winfo_exists():
            self._guide_dialog.lift()
            self._guide_dialog.focus_force()
            return
        self._guide_dialog = QuickGuideDialog(self)

    # ── body (scrollable: Zero Clipping aturan #4) ──
    def _build_body(self):
        body = ctk.CTkScrollableFrame(self, fg_color="transparent")
        body.grid(row=1, column=0, sticky="nsew", padx=PAD, pady=0)
        body.grid_columnconfigure(0, weight=1)
        self.body = body

        # Baris 0: kata kunci
        kw = ctk.CTkFrame(body, fg_color=BG_CARD, corner_radius=10, border_width=1, border_color=BORDER_CARD)
        kw.grid(row=0, column=0, sticky="ew", pady=(0, GAP + 2))
        kw.grid_columnconfigure(1, weight=1)
        ctk.CTkLabel(kw, text="Kata Kunci:", font=self.f_h, text_color=TEXT_PRIMARY).grid(row=0, column=0, padx=(PAD, GAP), pady=PAD)
        self.entry_kw = ctk.CTkEntry(
            kw,
            height=34,
            font=self.f_n,
            placeholder_text="💡 Ketik topik skripsi / riset (contoh: kecerdasan buatan, akuntansi publik, manajemen sdm)",
        )
        self.entry_kw.grid(row=0, column=1, sticky="ew", pady=PAD)
        self.entry_kw.bind("<Return>", lambda _e: self._on_start())
        ctk.CTkButton(kw, text="Cari", width=76, height=34, font=self.f_h,
                      fg_color=BLUE, hover_color=BLUE_H,
                      command=self._on_start).grid(row=0, column=2, padx=(GAP, PAD), pady=PAD)

        # Baris 1: naskah
        ms = ctk.CTkFrame(body, fg_color=BG_CARD, corner_radius=10, border_width=1, border_color=BORDER_CARD)
        ms.grid(row=1, column=0, sticky="ew", pady=(0, GAP + 2))
        ms.grid_columnconfigure(3, weight=1)
        ctk.CTkLabel(ms, text="Naskah Ilmiah:", font=self.f_h, text_color=("#0284c7", "#38bdf8")).grid(
            row=0, column=0, padx=(PAD, GAP), pady=PAD)
        ctk.CTkButton(ms, text="Unggah (.docx/.pdf)", height=32, font=self.f_n,
                      fg_color=("#e2e8f0", "#334155"), hover_color=("#cbd5e1", "#475569"),
                      text_color=("#1e293b", "#f1f5f9"),
                      width=fit_w(self.f_n, "Unggah (.docx/.pdf)", 0, 28),
                      command=self._on_upload).grid(row=0, column=1, padx=GAP, pady=PAD)
        self.var_match = ctk.BooleanVar(value=False)
        FitCheckBox(ms, "Match Score (0–100%)", self.f_h, variable=self.var_match).grid(
            row=0, column=2, padx=GAP, pady=PAD)
        self.lbl_ms = ctk.CTkLabel(ms, text="Belum ada naskah (opsional)", font=self.f_s,
                                   text_color=MUTED, anchor="w")
        self.lbl_ms.grid(row=0, column=3, sticky="ew", padx=(GAP, PAD))

        # Baris 2: Mode Selector (Progressive Disclosure)
        mode_bar = ctk.CTkFrame(body, fg_color="transparent")
        mode_bar.grid(row=2, column=0, sticky="ew", pady=(0, GAP))
        mode_bar.grid_columnconfigure(0, weight=1)
        self.seg_mode = ctk.CTkSegmentedButton(
            mode_bar,
            values=["✨ Mode Simpel (Direkomendasikan)", "⚙️ Mode Pengaturan Lengkap"],
            font=self.f_h,
            command=self._on_mode_change,
        )
        self.seg_mode.set("✨ Mode Simpel (Direkomendasikan)")
        self.seg_mode.grid(row=0, column=0, sticky="ew")

        # Baris 3: Container Konfigurasi (Advanced vs Simple)
        # 1. Bangun frame advanced terlebih dahulu agar seluruh variabel & kontrol terinisialisasi
        self.frame_advanced_grid = ctk.CTkFrame(body, fg_color="transparent")
        self.frame_advanced_grid.grid_columnconfigure(0, weight=3, uniform="cards")
        self.frame_advanced_grid.grid_columnconfigure(1, weight=2, uniform="cards")
        for r in (0, 1):
            self.frame_advanced_grid.grid_rowconfigure(r, weight=1)
        self._card_source(self.frame_advanced_grid).grid(row=0, column=0, sticky="nsew", padx=(0, GAP), pady=(0, GAP))
        self._card_sinta(self.frame_advanced_grid).grid(row=0, column=1, sticky="nsew", padx=(GAP, 0), pady=(0, GAP))
        self._card_month(self.frame_advanced_grid).grid(row=1, column=0, sticky="nsew", padx=(0, GAP), pady=(GAP, 0))
        self._card_ai(self.frame_advanced_grid).grid(row=1, column=1, sticky="nsew", padx=(GAP, 0), pady=(GAP, 0))

        # 2. Bangun frame simple mode card
        self.frame_simple_mode = self._build_simple_mode_card(body)

        # Baris 4: progress ganda
        self.progress = DualProgress(body, self.f_n)
        self.progress.grid(row=4, column=0, sticky="ew", pady=(0, GAP + 4))

        # Baris 5: Live KPI Counter Pills (v1.7.0 F3) + Empty State Banner (v1.7.1)
        self.kpi_frame = self._build_kpi_pills(body)
        self.kpi_frame.grid(row=5, column=0, sticky="ew", pady=(0, GAP + 4))

        # Baris 6-7: log box (Dibuat terlebih dahulu sebelum memanggil preset log)
        lh = ctk.CTkFrame(body, fg_color="transparent")
        lh.grid(row=6, column=0, sticky="ew")
        lh.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(lh, text="Pemantauan Proses & Riwayat Log:", font=self.f_h, anchor="w", text_color=TEXT_PRIMARY).grid(
            row=0, column=0, sticky="w")
        ctk.CTkButton(lh, text="Bersihkan Log", width=fit_w(self.f_s, "Bersihkan Log", 0, 24), height=24,
                      font=self.f_s, fg_color=("#e2e8f0", "#3a3a3a"), hover_color=("#cbd5e1", "#4a4a4a"),
                      text_color=("#1e293b", "#f1f5f9"),
                      command=self._clear_log).grid(row=0, column=1, sticky="e")
        self.log_box = ctk.CTkTextbox(body, height=180, font=self.f_mono, wrap="word", state="disabled")
        self.log_box.grid(row=7, column=0, sticky="nsew", pady=(GAP, PAD))
        self.log("Selamat datang di JSFS v1.7.1 (Zero-Result Resilience & Soft Slate Daylight).")

        # 3. Tampilkan simple mode secara default & terapkan preset awal (aman karena log_box sudah aktif)
        self.frame_simple_mode.grid(row=3, column=0, sticky="nsew", pady=(0, GAP + 4))
        self._apply_preset_skripsi()

    def _build_kpi_pills(self, parent) -> ctk.CTkFrame:
        """Membangun 4 kartu Live KPI Counter Pills + Empty State Banner (v1.7.1)."""
        outer = ctk.CTkFrame(parent, fg_color="transparent")
        outer.grid_columnconfigure(0, weight=1)

        frame = ctk.CTkFrame(outer, fg_color="transparent")
        frame.grid(row=0, column=0, sticky="ew")
        frame.grid_columnconfigure((0, 1, 2, 3), weight=1, uniform="kpi")

        pills = [
            ("lbl_kpi_total", "📚 Total: 0", ("#0284c7", "#38bdf8")),
            ("lbl_kpi_free", "💚 Gratis: 0", ("#059669", "#34d399")),
            ("lbl_kpi_paid", "💳 Berbayar: 0", ("#d97706", "#fbbf24")),
            ("lbl_kpi_fast", "⚡ Terbit Dekat: 0", ("#7c3aed", "#a78bfa")),
        ]

        for col, (attr, text, text_col) in enumerate(pills):
            card = ctk.CTkFrame(frame, fg_color=BG_CARD, corner_radius=8, border_width=1, border_color=BORDER_CARD)
            card.grid(row=0, column=col, sticky="ew", padx=3, pady=2)
            lbl = ctk.CTkLabel(card, text=text, font=self.f_h, text_color=text_col)
            lbl.pack(padx=8, pady=6)
            setattr(self, attr, lbl)

        self.lbl_empty_banner = ctk.CTkLabel(
            outer,
            text="💡 Tips: Coba gunakan kata kunci 1–2 kata umum seperti 'manajemen' atau 'sistem informasi', atau aktifkan Mode Lengkap -> Gunakan Selenium.",
            font=self.f_s,
            fg_color=("#fef3c7", "#3b2f13"),
            text_color=("#92400e", "#fcd34d"),
            corner_radius=8,
            anchor="w",
            justify="left",
            wraplength=800,
            padx=12,
            pady=6,
        )
        self.lbl_empty_banner.grid(row=1, column=0, sticky="ew", padx=3, pady=(6, 0))
        self.lbl_empty_banner.grid_remove()

        return outer

    def _update_kpi(self, d: dict):
        """Memperbarui nilai counter KPI secara aman pada main GUI thread."""
        if not self.winfo_exists():
            return
        if hasattr(self, "lbl_kpi_total") and self.lbl_kpi_total:
            self.lbl_kpi_total.configure(text=f"📚 Total: {d.get('total', 0)}")
            self.lbl_kpi_free.configure(text=f"💚 Gratis: {d.get('free', 0)}")
            self.lbl_kpi_paid.configure(text=f"💳 Berbayar: {d.get('paid', 0)}")
            self.lbl_kpi_fast.configure(text=f"⚡ Terbit Dekat: {d.get('fast', 0)}")

    def _reset_kpi(self):
        """Mereset nilai counter KPI ke 0 saat memulai run baru."""
        self._update_kpi({"total": 0, "free": 0, "paid": 0, "fast": 0})
        if hasattr(self, "lbl_empty_banner") and self.lbl_empty_banner:
            self.lbl_empty_banner.grid_remove()

    # ── kartu-kartu ──
    def _row(self, parent, label: str, label_w: int):
        f = ctk.CTkFrame(parent, fg_color="transparent")
        f.pack(fill="x", pady=3)
        ctk.CTkLabel(f, text=label, font=self.f_h, anchor="w", width=label_w).pack(side="left")
        return f

    def _card_source(self, parent):
        card, body, _ = make_card(parent, "Sumber, Output & Biaya", self.f_h)
        lw = max(fit_w(self.f_h, t, 0, 12) for t in ("Sumber:", "Format:", "Target Jurnal:", "Biaya (Rp):"))
        self.var_garuda, self.var_sinta_src = ctk.BooleanVar(value=True), ctk.BooleanVar(value=True)
        r = self._row(body, "Sumber:", lw)
        FitCheckBox(r, "Garuda", self.f_n, variable=self.var_garuda).pack(side="left", padx=(0, 10))
        FitCheckBox(r, "SINTA", self.f_n, variable=self.var_sinta_src).pack(side="left")

        self.var_md, self.var_xlsx, self.var_csv = (ctk.BooleanVar(value=True), ctk.BooleanVar(value=True),
                                                    ctk.BooleanVar(value=False))
        r = self._row(body, "Format:", lw)
        for text, var in ((".md", self.var_md), (".xlsx", self.var_xlsx), (".csv", self.var_csv)):
            FitCheckBox(r, text, self.f_n, variable=var).pack(side="left", padx=(0, 10))

        r = self._row(body, "Target Jurnal:", lw)
        self.cmb_target = ctk.CTkComboBox(r, values=["10", "25", "50", "100", "250", "500", "1000"],
                                          width=92, height=30, font=self.f_n)
        self.cmb_target.set("50")
        self.cmb_target.pack(side="left")
        ctk.CTkLabel(r, text="(maks. 1000 / custom)", font=self.f_s, text_color=MUTED).pack(side="left", padx=GAP + 2)

        self.var_free, self.var_na = ctk.BooleanVar(value=False), ctk.BooleanVar(value=True)
        r = self._row(body, "Biaya (Rp):", lw)
        FitCheckBox(r, "Hanya Gratis (Rp 0)", self.f_n, variable=self.var_free).pack(side="left", padx=(0, 10))
        FitCheckBox(r, "Sertakan N/A", self.f_n, variable=self.var_na).pack(side="left")

        r = self._row(body, "", lw)
        ctk.CTkLabel(r, text="Min", font=self.f_s).pack(side="left")
        self.ent_min = ctk.CTkEntry(r, width=104, height=28, font=self.f_n)
        self.ent_min.insert(0, "0")
        self.ent_min.pack(side="left", padx=(4, 12))
        ctk.CTkLabel(r, text="Max", font=self.f_s).pack(side="left")
        self.ent_max = ctk.CTkEntry(r, width=124, height=28, font=self.f_n)
        self.ent_max.insert(0, "15.000.000")
        self.ent_max.pack(side="left", padx=(4, 0))
        return card

    def _card_sinta(self, parent):
        card, body, header = make_card(parent, "Peringkat SINTA", self.f_h)
        sample = "S1–S3, S5–S6"
        self.lbl_sinta = ctk.CTkLabel(header, text="S1–S6", font=self.f_s, height=22, corner_radius=11,
                                      fg_color=BLUE, width=fit_w(self.f_s, sample, 0, 24))
        self.lbl_sinta.grid(row=0, column=1, sticky="e")

        self.sinta_vars = {n: ctk.BooleanVar(value=True) for n in range(1, 7)}
        for c in range(3):
            body.grid_columnconfigure(c, weight=1, uniform="sinta")
        w = fit_w(self.f_n, "Sinta 6", 20 + 8)
        for i, n in enumerate(self.sinta_vars):
            FitCheckBox(body, f"Sinta {n}", self.f_n, variable=self.sinta_vars[n], width=w).grid(
                row=i // 3, column=i % 3, sticky="w", pady=3)
            self.sinta_vars[n].trace_add("write", lambda *_: self._update_sinta())

        sep = ctk.CTkFrame(body, height=1, fg_color="#3a3a3a")
        sep.grid(row=2, column=0, columnspan=3, sticky="ew", pady=(GAP + 2, GAP + 2))
        self.var_turbo, self.var_selenium = ctk.BooleanVar(value=True), ctk.BooleanVar(value=False)
        FitSwitch(body, "⚡ Turbo Mode", self.f_h, variable=self.var_turbo, text_color="#f59e0b").grid(
            row=3, column=0, columnspan=3, sticky="w", pady=3)
        FitSwitch(body, "Gunakan Selenium (fallback)", self.f_n, variable=self.var_selenium).grid(
            row=4, column=0, columnspan=3, sticky="w", pady=3)
        return card

    def _card_month(self, parent):
        card, body, _ = make_card(parent, "Filter Bulan Terbit", self.f_h)
        self.month_filter = MonthFilter(body, self.f_s)
        self.month_filter.pack(fill="x")
        return card

    def _card_ai(self, parent):
        card, body, _ = make_card(parent, "Mesin AI (Ollama)", self.f_h)
        self.var_ai = ctk.BooleanVar(value=True)
        self.sw_ai = FitSwitch(body, "Gunakan AI (Ollama)", self.f_n, variable=self.var_ai)
        self.sw_ai.pack(anchor="w", pady=3)
        r = ctk.CTkFrame(body, fg_color="transparent")
        r.pack(fill="x", pady=GAP)
        ctk.CTkLabel(r, text="AI workers:", font=self.f_n).pack(side="left", padx=(0, GAP))
        self.seg_workers = ctk.CTkSegmentedButton(r, values=["1", "2"], width=96, height=28, font=self.f_n)
        self.seg_workers.set("2")
        self.seg_workers.pack(side="left")
        ctk.CTkLabel(body, text="AI hanya memproses jurnal yang lolos pre-filter. "
                                "Bila Ollama mati, JSFS otomatis memakai Regex.",
                     font=self.f_s, text_color=MUTED, justify="left", anchor="w",
                     wraplength=300).pack(fill="x", pady=(GAP, 0))
        return card

    # ── kartu mode simpel & preset mahasiswa (v1.5.1 / v1.7.1) ──
    def _build_simple_mode_card(self, parent):
        card, body, _ = make_card(parent, "✨ Pilih Kategori / Preset Cepat Mahasiswa", self.f_h)
        body.grid_columnconfigure(0, weight=1)
        body.grid_columnconfigure(1, weight=1)

        presets = [
            ("skripsi", "🎓 Target Skripsi S1", "SINTA 3–5 • Biaya Murah/Gratis", "#1f6aa5", "#144870", self._apply_preset_skripsi),
            ("gratis", "💰 Jurnal Gratis (Rp 0)", "Bebas Biaya Publikasi • S2–S6", "#13a06f", "#0e7a55", self._apply_preset_gratis),
            ("cepat", "⚡ Terbit Terdekat", "Edisi 1–3 Bulan ke Depan", "#d97706", "#b45309", self._apply_preset_cepat),
            ("reputasi", "🏆 Reputasi Tinggi", "SINTA 1 & 2 (Scopus / Nasional)", "#7c3aed", "#6d28d9", self._apply_preset_reputasi),
        ]
        self._preset_cards: dict[str, tuple[ctk.CTkFrame, ctk.CTkButton, str]] = {}

        for idx, (key, title, sub, col, hover_col, cmd) in enumerate(presets):
            row_idx = idx // 2
            col_idx = idx % 2
            btn_f = ctk.CTkFrame(body, fg_color=("#f1f5f9", "#1c1c1c"), corner_radius=10, border_width=1, border_color=BORDER_CARD)
            btn_f.grid(row=row_idx, column=col_idx, sticky="ew", padx=(0, GAP) if col_idx == 0 else (GAP, 0), pady=5)
            btn_f.grid_columnconfigure(0, weight=1)

            b = ctk.CTkButton(
                btn_f,
                text=title,
                font=self.f_h,
                height=36,
                fg_color=col,
                hover_color=hover_col,
                cursor="hand2",
                command=cmd,
            )
            b.grid(row=0, column=0, sticky="ew", padx=8, pady=(8, 3))
            ctk.CTkLabel(btn_f, text=sub, font=self.f_s, text_color=MUTED).grid(row=1, column=0, sticky="ew", padx=8, pady=(0, 8))
            self._preset_cards[key] = (btn_f, b, col)

        ctrl_row = ctk.CTkFrame(body, fg_color="transparent")
        ctrl_row.grid(row=2, column=0, columnspan=2, sticky="ew", pady=(GAP + 2, 0))
        ctrl_row.grid_columnconfigure(1, weight=1)

        ctk.CTkLabel(ctrl_row, text="Target Jumlah Jurnal:", font=self.f_h, text_color=TEXT_PRIMARY).grid(row=0, column=0, sticky="w", padx=(4, GAP))
        self.cmb_target_simple = ctk.CTkComboBox(
            ctrl_row,
            values=["10", "25", "50", "100", "250"],
            width=90,
            height=28,
            font=self.f_n,
            command=self._on_simple_target_change,
        )
        self.cmb_target_simple.set("25")
        self.cmb_target_simple.grid(row=0, column=1, sticky="w")

        self.lbl_active_preset = ctk.CTkLabel(
            ctrl_row,
            text="Preset Aktif: 🎓 Target Skripsi S1 (S3–S5)",
            font=self.f_s,
            text_color=("#0284c7", "#38bdf8"),
            anchor="e",
        )
        self.lbl_active_preset.grid(row=0, column=2, sticky="e", padx=(0, 4))

        return card

    def _on_mode_change(self, val: str):
        if "Simpel" in val:
            self.frame_advanced_grid.grid_remove()
            self.frame_simple_mode.grid(row=3, column=0, sticky="nsew", pady=(0, GAP + 2))
            self.log("💡 Beralih ke Mode Simpel (Preset Mahasiswa).")
        else:
            self.frame_simple_mode.grid_remove()
            self.frame_advanced_grid.grid(row=3, column=0, sticky="nsew", pady=(0, GAP + 2))
            self.log("⚙️ Beralih ke Mode Pengaturan Lengkap.")

    def _on_simple_target_change(self, val: str):
        self.cmb_target.set(val)

    def _set_active_preset_ui(self, active_key: str):
        """Sorot kartu Quick Preset yang sedang aktif dengan indikator kontras (v1.7.1)."""
        if not hasattr(self, "_preset_cards"):
            return
        for key, (frame, _btn, accent) in self._preset_cards.items():
            if key == active_key:
                frame.configure(
                    border_width=2,
                    border_color=accent,
                    fg_color=("#e0f2fe", "#1e293b"),
                )
            else:
                frame.configure(
                    border_width=1,
                    border_color=BORDER_CARD,
                    fg_color=("#f1f5f9", "#1c1c1c"),
                )

    def _apply_preset_skripsi(self):
        for n in (1, 2, 6):
            self.sinta_vars[n].set(False)
        for n in (3, 4, 5):
            self.sinta_vars[n].set(True)
        self.cmb_target.set("25")
        if hasattr(self, "cmb_target_simple"):
            self.cmb_target_simple.set("25")
        self.var_free.set(False)
        self.var_na.set(True)
        self.ent_min.delete(0, "end")
        self.ent_min.insert(0, "0")
        self.ent_max.delete(0, "end")
        self.ent_max.insert(0, "1.500.000")
        self.month_filter._set_all(True)
        if hasattr(self, "lbl_active_preset"):
            self.lbl_active_preset.configure(text="Preset Aktif: 🎓 Target Skripsi S1 (S3–S5)")
        self._set_active_preset_ui("skripsi")
        self.log("📌 Preset Aktif: 🎓 Target Skripsi S1 (SINTA 3–5, Target 25, Biaya maks Rp 1,5 jt, Semua Bulan).")

    def _apply_preset_gratis(self):
        self.sinta_vars[1].set(False)
        for n in (2, 3, 4, 5, 6):
            self.sinta_vars[n].set(True)
        self.cmb_target.set("50")
        if hasattr(self, "cmb_target_simple"):
            self.cmb_target_simple.set("50")
        self.var_free.set(True)
        self.var_na.set(True)
        self.month_filter._set_all(True)
        if hasattr(self, "lbl_active_preset"):
            self.lbl_active_preset.configure(text="Preset Aktif: 💰 Jurnal Gratis (Rp 0)")
        self._set_active_preset_ui("gratis")
        self.log("📌 Preset Aktif: 💰 Jurnal Bebas Biaya (Hanya jurnal tanpa biaya publikasi / Bebas APC, S2–S6).")

    def _apply_preset_cepat(self):
        for n in (1, 2):
            self.sinta_vars[n].set(False)
        for n in (3, 4, 5, 6):
            self.sinta_vars[n].set(True)
        self.cmb_target.set("25")
        if hasattr(self, "cmb_target_simple"):
            self.cmb_target_simple.set("25")
        self.var_free.set(False)
        self.var_na.set(True)
        cur_m = time.localtime().tm_mon
        target_months = {
            cur_m,
            ((cur_m) % 12) + 1,
            ((cur_m + 1) % 12) + 1,
        }
        for i, v in enumerate(self.month_filter.vars):
            v.set((i + 1) in target_months)
        m_names = [self.month_filter.NAMES[m - 1] for m in sorted(target_months)]
        if hasattr(self, "lbl_active_preset"):
            self.lbl_active_preset.configure(text=f"Preset Aktif: ⚡ Terbit Terdekat ({', '.join(m_names)})")
        self._set_active_preset_ui("cepat")
        self.log(f"📌 Preset Aktif: ⚡ Terbit Terdekat (Edisi terbit {', '.join(m_names)}, S3–S6, Target 25).")

    def _apply_preset_reputasi(self):
        for n in (1, 2):
            self.sinta_vars[n].set(True)
        for n in (3, 4, 5, 6):
            self.sinta_vars[n].set(False)
        self.cmb_target.set("25")
        if hasattr(self, "cmb_target_simple"):
            self.cmb_target_simple.set("25")
        self.var_free.set(False)
        self.var_na.set(True)
        self.month_filter._set_all(True)
        if hasattr(self, "lbl_active_preset"):
            self.lbl_active_preset.configure(text="Preset Aktif: 🏆 Reputasi Tinggi (S1–S2)")
        self._set_active_preset_ui("reputasi")
        self.log("📌 Preset Aktif: 🏆 Reputasi Tinggi (SINTA 1 & 2 / Nasional Akreditasi A & Scopus untuk tesis/hibah).")

    # ── action bar (dipatok di bawah) ──
    def _build_actionbar(self):
        bar = ctk.CTkFrame(self, fg_color="transparent")
        bar.grid(row=2, column=0, sticky="ew", padx=PAD + 4, pady=(GAP, PAD))
        bar.grid_columnconfigure(0, weight=3, uniform="act")
        bar.grid_columnconfigure(1, weight=2, uniform="act")
        bar.grid_columnconfigure(2, weight=2, uniform="act")

        self.btn_start = ctk.CTkButton(bar, text="MULAI SCRAPING", height=46, font=self.f_btn,
                                       fg_color=BLUE, hover_color=BLUE_H, command=self._on_start)
        self.btn_start.grid(row=0, column=0, sticky="ew", padx=(0, GAP))
        self.btn_preview = ctk.CTkButton(bar, text="PREVIEW HASIL DATA", height=46, font=self.f_btn,
                                         fg_color=GREEN, hover_color=GREEN_H, state="disabled",
                                         command=self._open_data_previewer)
        self.btn_preview.grid(row=0, column=1, sticky="ew", padx=(0, GAP))

        self.btn_copy_list = ctk.CTkButton(bar, text="📋 Salin Daftar (Share WA)", height=46, font=self.f_btn,
                                          fg_color=("#cbd5e1", "#334155"), hover_color=INDIGO_H,
                                          text_color=("#64748b", "#94a3b8"), state="disabled",
                                          command=self._copy_main_share_list)
        self.btn_copy_list.grid(row=0, column=2, sticky="ew")

        foot = ctk.CTkFrame(bar, fg_color="transparent")
        foot.grid(row=1, column=0, columnspan=3, sticky="ew", pady=(GAP + 2, 0))
        foot.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(foot, text="Output: output/  (format: JSFS_<keyword>_<timestamp>)",
                     font=self.f_s, text_color=MUTED, anchor="w").grid(row=0, column=0, sticky="w")
        ctk.CTkButton(foot, text="Buka Folder Output", height=28, font=self.f_s,
                      width=fit_w(self.f_s, "Buka Folder Output", 0, 28),
                      command=self._open_output).grid(row=0, column=1, sticky="e")

    def _copy_main_share_list(self):
        """Menyalin seluruh daftar jurnal terkini ke clipboard (Aksi 1-Klik Utama v1.7.0)."""
        if not self._latest_results:
            self.log("⚠ Belum ada hasil scraping untuk disalin.")
            return
        share_text = exporter.generate_copypaste_list(self._latest_results)
        try:
            self.clipboard_clear()
            self.clipboard_append(share_text)
            self.update()
            self.btn_copy_list.configure(
                text="✅ Daftar Tersalin!",
                fg_color=OK_GREEN,
            )
            self.after(2500, self._reset_copy_main_btn)
        except Exception:
            pass

    def _reset_copy_main_btn(self):
        try:
            if self.winfo_exists():
                self.btn_copy_list.configure(
                    text="📋 Salin Daftar (Share WA)",
                    fg_color=INDIGO,
                )
        except Exception:
            pass

    # ───────────────────────── Util GUI ─────────────────────────
    def _insert_log_batch(self, lines: list[str]):
        """Menyisipkan batch baris log sekaligus untuk mencegah WM_PAINT starvation."""
        if not hasattr(self, "log_box") or self.log_box is None:
            for l in lines:
                print(f"[JSFS LOG] {l}")
            return
        if not lines:
            return
        self.log_box.configure(state="normal")
        joined = "\n".join(lines) + "\n"
        self.log_box.insert("end", joined)

        # Batasi log box maksimal 500 baris agar rendering Tkinter tetap instan
        content = self.log_box.get("1.0", "end")
        all_lines = content.splitlines()
        if len(all_lines) > 500:
            truncated = "\n".join(all_lines[-500:]) + "\n"
            self.log_box.delete("1.0", "end")
            self.log_box.insert("1.0", truncated)

        self.log_box.see("end")
        self.log_box.configure(state="disabled")

    def log(self, msg: str):
        """Hanya dari thread GUI. Dari thread lain: ui_q.put(('log', msg))."""
        if not hasattr(self, "log_box") or self.log_box is None:
            print(f"[JSFS LOG] {msg}")
            return
        self._insert_log_batch([f"[{time.strftime('%H:%M:%S')}] {msg}"])

    def _clear_log(self):
        self.log_box.configure(state="normal")
        self.log_box.delete("1.0", "end")
        self.log_box.configure(state="disabled")

    def _update_sinta(self):
        self.lbl_sinta.configure(text=fmt_ranges([n for n, v in self.sinta_vars.items() if v.get()]))

    def _on_upload(self):
        p = filedialog.askopenfilename(filetypes=[("Naskah Ilmiah", "*.docx *.pdf")])
        if p:
            self.manuscript_path = p
            self.lbl_ms.configure(text=os.path.basename(p), text_color="#d0d0d0")
            self.var_match.set(True)
            self.log(f"📄 Naskah diunggah: {os.path.basename(p)}")

    def _open_output(self):
        os.makedirs("output", exist_ok=True)
        try:
            os.startfile(os.path.abspath("output"))
        except Exception:
            pass

    def _open_data_previewer(self):
        if not self._latest_results:
            self.log("⚠ Belum ada hasil scraping untuk ditampilkan.")
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

    def _set_running(self, running: bool):
        if running:
            self.btn_start.configure(text="HENTIKAN", fg_color=RED, hover_color=RED_H, command=self._on_stop)
            self.btn_preview.configure(state="disabled")
            if hasattr(self, "btn_copy_list"):
                self.btn_copy_list.configure(
                    state="disabled",
                    fg_color=("#cbd5e1", "#334155"),
                    text_color=("#64748b", "#94a3b8"),
                )
        else:
            self.btn_start.configure(text="MULAI SCRAPING", fg_color=BLUE, hover_color=BLUE_H,
                                     command=self._on_start)
            if self._latest_results:
                self.btn_preview.configure(state="normal")
                if hasattr(self, "btn_copy_list"):
                    self.btn_copy_list.configure(
                        state="normal",
                        fg_color=INDIGO,
                        text_color="#ffffff",
                    )
            elif hasattr(self, "btn_copy_list"):
                self.btn_copy_list.configure(
                    state="disabled",
                    fg_color=("#cbd5e1", "#334155"),
                    text_color=("#64748b", "#94a3b8"),
                )

    def _collect_params(self) -> dict:
        digits = lambda s: int(re.sub(r"\D", "", s) or 0)
        return {
            "keyword": self.entry_kw.get().strip(),
            "target": digits(self.cmb_target.get()) or 50,
            "garuda": self.var_garuda.get(),
            "sinta_src": self.var_sinta_src.get(),
            "sinta": {n for n, v in self.sinta_vars.items() if v.get()},
            "months": self.month_filter.selected_months(),
            "include_unknown": self.month_filter.include_unknown(),
            "only_free": self.var_free.get(),
            "include_na": self.var_na.get(),
            "min_rp": digits(self.ent_min.get()),
            "max_rp": digits(self.ent_max.get()) or 15_000_000,
            "turbo": self.var_turbo.get(),
            "selenium": self.var_selenium.get(),
            "match": self.var_match.get(),
            "formats": [f for f, v in ((".md", self.var_md), (".xlsx", self.var_xlsx), (".csv", self.var_csv)) if v.get()],
            "manuscript": self.manuscript_path,
        }

    # ───────────────────────── Badge Ollama (loop 30 dtk) ─────────────────────────
    def _refresh_badge(self):
        threading.Thread(
            target=lambda: self.ui_q.put(("ollama", ai_engine.check_ollama_active(force=True))),
            daemon=True).start()
        self.after(30_000, self._refresh_badge)

    def _apply_badge(self, active: bool):
        if active:
            self.badge.configure(
                text=f"●  AI Aktif · {ai_engine.MODEL}",
                fg_color=OK_GREEN,
                hover_color=GREEN_H,
                text_color="white",
            )
            if self._ai_auto_off:
                self.var_ai.set(True)
                self._ai_auto_off = False
            self.sw_ai.configure(state="normal")
        else:
            self.badge.configure(
                text="○  Mode Regex (Klik untuk Setup)",
                fg_color="#4a4a4a",
                hover_color="#5a5a5a",
                text_color="#d0d0d0",
            )
            if self.var_ai.get():
                self._ai_auto_off = True
            self.var_ai.set(False)
            self.sw_ai.configure(state="disabled")

    def _open_ai_setup_manager(self):
        if self._ai_setup_dialog is not None and self._ai_setup_dialog.winfo_exists():
            self._ai_setup_dialog.lift()
            self._ai_setup_dialog.focus_force()
            return
        self._ai_setup_dialog = AISetupDialog(self)

    # ───────────────────────── Threading: Start / Stop ─────────────────────────
    def _on_start(self):
        if self._worker and self._worker.is_alive():
            return
        params = self._collect_params()
        if not params["keyword"]:
            self.log("⚠ Masukkan kata kunci terlebih dahulu.")
            return
        if not params["garuda"] and not params["sinta_src"]:
            self.log("⚠ Pilih setidaknya satu sumber (Garuda atau SINTA).")
            return

        self.cancel_evt.clear()
        ai_engine.set_enabled(self.var_ai.get())
        ai_engine.set_workers(int(self.seg_workers.get()))
        self.progress.reset()
        self._reset_kpi()
        self._set_running(True)
        self._worker = threading.Thread(target=self._pipeline, args=(params,), daemon=True)
        self._worker.start()

    def _on_stop(self):
        self.cancel_evt.set()
        self.log("Menghentikan proses… (tugas yang sedang berjalan diselesaikan dulu)")

    def _pipeline(self, p: dict):
        """THREAD LATAR — dilarang menyentuh widget. Semua pelaporan lewat self.ui_q."""
        q = self.ui_q
        try:
            # FASE 1: Fast Broad Scraping (128 Workers)
            q.put(("log", f"Fase 1: Scraping massal untuk '{p['keyword']}' (Target: {p['target']} jurnal)..."))
            records = run_scraper(
                p,
                lambda d, t: q.put(("scrape", d, t)),
                self.cancel_evt,
                on_log=lambda msg: q.put(("log", msg)),
            )
            if self.cancel_evt.is_set():
                q.put(("done", None))
                return

            q.put(("log", f"Fase 1 selesai: {len(records)} jurnal berhasil di-crawl."))
            q.put(("kpi", {"total": len(records), "free": 0, "paid": 0, "fast": 0}))

            # FASE 2: Pre-Filter (Rank SINTA & Bulan Terbit)
            survivors: list[JournalEntry] = []
            for r in records:
                if r["sinta"] not in p["sinta"]:
                    continue
                if not lolos_filter_bulan(r.get("bulan_terbit", []), p["months"], p["include_unknown"]):
                    continue
                survivors.append(r)

            # Jaga pool kandidat tetap kaya untuk deep scrape & filter biaya (v1.6.1)
            pool_limit = max(p["target"] * 2, 40)
            if len(survivors) > pool_limit:
                survivors = survivors[:pool_limit]

            q.put(("log", f"Fase 2: Pre-filter -> {len(survivors)}/{len(records)} jurnal lolos seleksi SINTA & Bulan."))
            q.put(("kpi", {"total": len(survivors), "free": 0, "paid": 0, "fast": 0}))

            if not survivors:
                q.put(("log", "⚠ Tidak ada jurnal yang lolos pre-filter. Selesai."))
                q.put(("done", 0))
                return

            # FASE 3: AI Enrichment (Focus & Scope + APC Submit vs Publikasi)
            mode_desc = f"AI Ollama ({ai_engine.MODEL})" if (self.var_ai.get() and ai_engine.check_ollama_active()) else "Regex Legacy v1.3.8"
            q.put(("log", f"Fase 3: Ekstraksi data mendalam [{mode_desc}] untuk {len(survivors)} jurnal lolos filter..."))
            q.put(("ai", 0, len(survivors) * 2))

            ai_engine.enrich_records(
                survivors,
                on_progress=lambda d, t: q.put(("ai", d, t)),
                cancel=self.cancel_evt,
            )
            if self.cancel_evt.is_set():
                q.put(("done", None))
                return

            # Analisis Naskah & Skor Kesesuaian (v1.5.0 / v1.6.1)
            if p.get("match") and p.get("manuscript") and os.path.exists(p["manuscript"]):
                q.put(("log", f"Menganalisis kesesuaian naskah: {os.path.basename(p['manuscript'])}..."))
                ms_data = parse_manuscript(p["manuscript"])
                if ms_data:
                    for s in survivors:
                        match_res = calculate_manuscript_match(ms_data, s)
                        s.match_score = match_res.get("score", 40)
                        s.kesesuaian_naskah = match_res.get("level", "Cukup Sesuai")
            else:
                # Mode Pencocokan Berbasis Kata Kunci (Keyword-Mode Scoring - v1.5.0 / v1.6.1)
                q.put(("log", f"Menghitung skor kesesuaian berbasis kata kunci '{p['keyword']}' (Kamus Dwibahasa)..."))
                for s in survivors:
                    kw_res = calculate_keyword_match_score(p["keyword"], s)
                    s.match_score = kw_res.get("score", 40)
                    s.kesesuaian_naskah = kw_res.get("level", "Cukup Sesuai")

            n_ai = sum(1 for r in survivors if r.get("sumber_focus") == "AI")
            n_ai_apc = sum(1 for r in survivors if r.get("sumber_apc") == "AI")
            q.put(("log", f"Hasil AI Enrichment: Focus via AI {n_ai}/{len(survivors)}, APC via AI {n_ai_apc}/{len(survivors)}."))

            # Filter biaya pengguna jika diminta (v1.6.1 High-Yield Recall)
            free_survivors: list[JournalEntry] = []
            unknown_survivors: list[JournalEntry] = []
            paid_survivors: list[JournalEntry] = []

            for r in survivors:
                b_pub = r.get("biaya_publikasi")
                if b_pub is None and r.get("apc_amount") is not None:
                    b_pub = r.get("apc_amount")
                apc_status = str(r.get("apc_status", "")).lower()

                is_free = (b_pub == 0) or ("gratis" in apc_status) or ("rp 0" in apc_status) or ("bebas biaya" in apc_status)
                is_unk = (b_pub is None) or ("cek web" in apc_status) or ("belum terdata" in apc_status) or (r.get("apc_status") in (INFO_NOT_FOUND, "Cek Web / Belum Terdata"))

                if is_free:
                    free_survivors.append(r)
                elif is_unk:
                    unknown_survivors.append(r)
                else:
                    if p.get("min_rp", 0) <= b_pub <= p.get("max_rp", 15_000_000):
                        paid_survivors.append(r)

            final_survivors: list[JournalEntry] = []
            if p.get("only_free"):
                # Prioritaskan jurnal murni Gratis (Rp 0)
                final_survivors.extend(free_survivors)
                # Jika include_na aktif ATAU kuota target belum tercapai, sertakan Cek Web / Belum Terdata
                if p.get("include_na", True) or len(final_survivors) < p["target"]:
                    final_survivors.extend(unknown_survivors)
            else:
                final_survivors.extend(free_survivors)
                final_survivors.extend(paid_survivors)
                # Cek Web / Belum Terdata WAJIB lolos jika include_na=True atau sedang memfilter max_rp
                if p.get("include_na", True) or p.get("max_rp", 15_000_000) < 15_000_000:
                    final_survivors.extend(unknown_survivors)

            if not final_survivors and survivors:
                final_survivors = survivors

            if len(final_survivors) > p["target"]:
                final_survivors = final_survivors[:p["target"]]

            self._latest_results = final_survivors

            # Hitung & kirimkan Live KPI Pills (v1.7.0 F3)
            cur_m = time.localtime().tm_mon
            near_months = {cur_m, ((cur_m) % 12) + 1, ((cur_m + 1) % 12) + 1}
            kpi_free = sum(1 for r in final_survivors if (r.get("biaya_publikasi") == 0 or "gratis" in str(r.get("apc_status", "")).lower() or "rp 0" in str(r.get("apc_status", "")).lower()))
            kpi_paid = sum(1 for r in final_survivors if (r.get("biaya_publikasi") or 0) > 0)
            kpi_fast = sum(1 for r in final_survivors if any(m in r.get("bulan_terbit", []) for m in near_months))
            q.put(("kpi", {"total": len(final_survivors), "free": kpi_free, "paid": kpi_paid, "fast": kpi_fast}))

            # FASE 4: Export Data
            q.put(("log", f"Fase 4: Mengekspor {len(final_survivors)} jurnal ke berkas output..."))
            exported = exporter.write_exports(
                final_survivors,
                keyword=p["keyword"],
                formats=p["formats"],
                sinta_ranks=sorted(list(p["sinta"])),
            )
            for ef in exported:
                q.put(("log", f"💾 Berkas tersimpan → {ef}"))
                if ef.endswith(".xlsx"):
                    self._latest_excel_path = ef

            q.put(("done", len(final_survivors)))
        except Exception as e:
            q.put(("error", repr(e)))

    def _poll_queue(self):
        """Dijalankan di thread GUI tiap 80 ms. Batch processing untuk mencegah UI blackout."""
        if not self.winfo_exists():
            return
        latest, others = {}, []
        log_lines = []
        max_batch_size = 25
        count = 0

        try:
            while count < max_batch_size:
                msg = self.ui_q.get_nowait()
                count += 1
                kind = msg[0]
                if kind in ("scrape", "ai"):
                    latest[kind] = msg[1:]
                elif kind == "log":
                    log_lines.append(f"[{time.strftime('%H:%M:%S')}] {msg[1]}")
                else:
                    others.append(msg)
        except queue.Empty:
            pass

        for key, (done, total) in latest.items():
            self.progress.update_bar(key, done, total)

        if log_lines:
            self._insert_log_batch(log_lines)

        for msg in others:
            kind = msg[0]
            if kind == "ollama":
                self._apply_badge(msg[1])
            elif kind == "kpi":
                self._update_kpi(msg[1])
            elif kind == "error":
                self._insert_log_batch([f"[{time.strftime('%H:%M:%S')}] ✖ Error: {msg[1]}"])
                self._set_running(False)
            elif kind == "done":
                self._set_running(False)
                if msg[1] is None:
                    self._insert_log_batch([f"[{time.strftime('%H:%M:%S')}] Proses dihentikan oleh pengguna."])
                else:
                    self._insert_log_batch([f"[{time.strftime('%H:%M:%S')}] ✔ Selesai. {msg[1]} jurnal berhasil diekspor ke folder output/."])
                    if self._latest_results:
                        if hasattr(self, "lbl_empty_banner") and self.lbl_empty_banner:
                            self.lbl_empty_banner.grid_remove()
                        self.btn_preview.configure(
                            state="normal",
                            text=f"PREVIEW HASIL ({len(self._latest_results)})",
                        )
                        if hasattr(self, "btn_copy_list"):
                            self.btn_copy_list.configure(
                                state="normal",
                                fg_color=INDIGO,
                                text_color="#ffffff",
                            )
                    else:
                        if hasattr(self, "lbl_empty_banner") and self.lbl_empty_banner:
                            self.lbl_empty_banner.grid()
        self.after(80, self._poll_queue)

    def _on_close(self):
        self.cancel_evt.set()
        ai_engine.shutdown()
        cache_engine.close_all_connections()
        self.destroy()
        self.destroy()


if __name__ == "__main__":
    app = JSFSApp()
    app.mainloop()
