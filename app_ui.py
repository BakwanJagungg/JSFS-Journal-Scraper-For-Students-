"""
app_ui.py — JSFS v1.4.0 (Hybrid AI Engine & CustomTkinter Desktop GUI)
Antarmuka desktop dengan responsivitas tinggi (Pixel-Perfect, Zero Clipping),
Badge Ollama real-time, Dual Progress Bar (Scrape & AI), dan 4-Fase Pipeline.
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

# ───────────────────────── Konstanta Desain ─────────────────────────
PAD, GAP = 12, 6                      # padding kartu, jarak antar elemen
BG_CARD = "#262626"
BLUE, BLUE_H = "#1f6aa5", "#144870"
GREEN, GREEN_H = "#13a06f", "#0e7a55"
RED, RED_H = "#c0392b", "#962d22"
MUTED = "#8a8a8a"
OK_GREEN = "#22a45d"

ctk.set_appearance_mode("dark")
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
    card = ctk.CTkFrame(parent, corner_radius=10, fg_color=BG_CARD)
    card.grid_columnconfigure(0, weight=1)
    card.grid_rowconfigure(1, weight=1)
    header = ctk.CTkFrame(card, fg_color="transparent")
    header.grid(row=0, column=0, sticky="ew", padx=PAD, pady=(PAD, GAP))
    header.grid_columnconfigure(0, weight=1)
    ctk.CTkLabel(header, text=title, font=font_h, anchor="w").grid(row=0, column=0, sticky="w")
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
    ROWS = (("Scraping Massal (HTTP)", "scrape", BLUE), ("Analisis AI (Ollama)", "ai", "#e08a1e"))

    def __init__(self, master, font: ctk.CTkFont, **kw):
        super().__init__(master, fg_color=BG_CARD, corner_radius=10, **kw)
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
def run_scraper(params: dict, on_progress, cancel: threading.Event) -> list[JournalEntry]:
    """
    Fase 1: Fast Broad Scraping (128 workers HTTP/Selenium).
    Mengumpulkan data mentah (raw_about, raw_submissions, bulan_terbit) tanpa menyentuh LLM.
    """
    keyword = params["keyword"]
    target = int(params["target"])
    sinta_ranks = sorted(list(params["sinta"]))
    use_garuda = params.get("garuda", True)
    use_sinta = params.get("sinta_src", True)
    turbo = params.get("turbo", True)
    use_selenium = params.get("selenium", False)

    records: list[JournalEntry] = []

    if use_garuda and not cancel.is_set():
        g_results = scrape_garuda(
            keyword=keyword,
            sinta_ranks=sinta_ranks,
            deep_detail=True,
            max_results=target,
            turbo_mode=turbo,
            free_only=False,
            include_unknown_apc=True,
        )
        records.extend(g_results)
        on_progress(len(records), target)

    if use_sinta and not cancel.is_set():
        if use_selenium:
            s_results = scrape_sinta_selenium(
                keyword=keyword,
                sinta_ranks=sinta_ranks,
                deep_detail=True,
                max_results=target,
                turbo_mode=turbo,
                free_only=False,
                include_unknown_apc=True,
            )
        else:
            s_results = scrape_sinta(
                keyword=keyword,
                sinta_ranks=sinta_ranks,
                deep_detail=True,
                max_results=target,
                turbo_mode=turbo,
                free_only=False,
                include_unknown_apc=True,
            )
        records.extend(s_results)
        on_progress(len(records), target)

    if use_garuda and use_sinta:
        records = merge_cross_source_entries(records)

    on_progress(len(records), target)
    return records


# ───────────────────────── Data Previewer Interaktif ─────────────────────────
class JSFSDataPreviewer(ctk.CTkToplevel):
    """
    Jendela Pop-up Interaktif Data Table Previewer v1.4.0.
    Menampilkan data terstruktur dengan kolom Submit, Publikasi, Bulan Terbit, dan Sumber (AI/Regex).
    """

    def __init__(self, master, journals: list[JournalEntry], excel_path: Optional[str] = None):
        super().__init__(master)

        self.title("👁️ JSFS Data Table Previewer v1.4.0 — Hasil Scraping Jurnal")
        self.geometry("1180x720")
        self.minsize(980, 600)

        self._all_journals: list[JournalEntry] = list(journals)
        self._filtered_journals: list[JournalEntry] = list(journals)
        self._excel_path: Optional[str] = excel_path

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
            text="📊 Data Table Previewer v1.4.0",
            font=ctk.CTkFont(family="Segoe UI", size=18, weight="bold"),
            text_color="#38bdf8",
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

        # 3. Treeview Table Container with Dark Styling
        tree_container = ctk.CTkFrame(container, corner_radius=8)
        tree_container.pack(fill="both", expand=True, pady=(0, 8))

        style = ttk.Style()
        style.theme_use("clam")
        style.configure(
            "JSFS.Treeview",
            background="#1e1e1e",
            foreground="#f8fafc",
            rowheight=26,
            fieldbackground="#1e1e1e",
            font=("Segoe UI", 10),
            borderwidth=0,
        )
        style.configure(
            "JSFS.Treeview.Heading",
            background="#2d3748",
            foreground="#38bdf8",
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
            b_pub = exporter._format_amount(b_pub_num, getattr(j, "mata_uang", None))

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
            self.lbl_detail_title.configure(text=f"📑 {j.nama_jurnal} ({j.peringkat_sinta}) • {j.penerbit}")
            self.btn_open_ojs.configure(state="normal" if (j.link_ojs and j.link_ojs.startswith("http")) else "disabled")

            b_sub = exporter._format_amount(getattr(j, "biaya_submit", None), getattr(j, "mata_uang", None))
            b_pub_num = getattr(j, "biaya_publikasi", None)
            if b_pub_num is None and getattr(j, "apc_amount", None) is not None:
                b_pub_num = getattr(j, "apc_amount", None)
            b_pub = exporter._format_amount(b_pub_num, getattr(j, "mata_uang", None))
            bulan = format_bulan_terbit(getattr(j, "bulan_terbit", []))
            terbit_est = exporter.get_next_issue_estimate(getattr(j, "bulan_terbit", []))
            s_focus = getattr(j, "sumber_focus", "Regex") or "Regex"
            s_apc = getattr(j, "sumber_apc", "Regex") or "Regex"

            txt = (
                f"• Focus & Scope [{s_focus}]: {getattr(j, 'focus_scope', INFO_NOT_FOUND)}\n"
                f"• Biaya Penulis [{s_apc}]: Submit: {b_sub} | Publikasi: {b_pub} | Mata Uang: {getattr(j, 'mata_uang', 'IDR') or 'IDR'}\n"
                f"• Jadwal Terbit: {bulan} (Terdekat: {terbit_est}) | Frekuensi: {getattr(j, 'frekuensi', INFO_NOT_FOUND)}\n"
                f"• Kontak: Email: {getattr(j, 'email', INFO_NOT_FOUND)} | Telepon/WA: {getattr(j, 'telepon', INFO_NOT_FOUND)} | Server: {getattr(j, 'status_server', 'Aktif')}\n"
                f"• Link Website: {j.link_ojs or INFO_NOT_FOUND}"
            )
            self.detail_textbox.configure(state="normal")
            self.detail_textbox.delete("1.0", "end")
            self.detail_textbox.insert("1.0", txt)
            self.detail_textbox.configure(state="disabled")

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
        self.title("⚙️ 1-Click AI Setup Manager — JSFS v1.4.0")
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
        self.configure(fg_color="#181818")

        f_pad = 16
        # Header
        hdr = ctk.CTkFrame(self, fg_color="transparent")
        hdr.grid(row=0, column=0, sticky="ew", padx=f_pad, pady=(16, 8))
        hdr.grid_columnconfigure(0, weight=1)

        ctk.CTkLabel(
            hdr,
            text="🤖 1-Click AI Setup Manager",
            font=ctk.CTkFont(size=18, weight="bold"),
            text_color="#38bdf8",
            anchor="w",
        ).grid(row=0, column=0, sticky="w")

        ctk.CTkLabel(
            hdr,
            text="JSFS membutuhkan Ollama & Model Qwen2.5 3B (Total ~2.2 GB)\nuntuk ekstraksi cerdas Focus & Scope dan Biaya Penulis (APC).",
            font=ctk.CTkFont(size=11),
            text_color="#a0a0a0",
            justify="left",
            anchor="w",
        ).grid(row=1, column=0, sticky="w", pady=(4, 0))

        # Status Card
        sc = ctk.CTkFrame(self, fg_color="#242424", corner_radius=10)
        sc.grid(row=1, column=0, sticky="ew", padx=f_pad, pady=8)
        sc.grid_columnconfigure(1, weight=1)

        ctk.CTkLabel(sc, text="Status Ollama:", font=ctk.CTkFont(size=12, weight="bold"), text_color="#d0d0d0").grid(row=0, column=0, sticky="w", padx=12, pady=(10, 4))
        self.lbl_ollama_status = ctk.CTkLabel(sc, text="Memeriksa...", font=ctk.CTkFont(size=12), text_color="#eab308")
        self.lbl_ollama_status.grid(row=0, column=1, sticky="w", padx=12, pady=(10, 4))

        ctk.CTkLabel(sc, text="Status Model (qwen2.5:3b):", font=ctk.CTkFont(size=12, weight="bold"), text_color="#d0d0d0").grid(row=1, column=0, sticky="w", padx=12, pady=(4, 10))
        self.lbl_model_status = ctk.CTkLabel(sc, text="Memeriksa...", font=ctk.CTkFont(size=12), text_color="#eab308")
        self.lbl_model_status.grid(row=1, column=1, sticky="w", padx=12, pady=(4, 10))

        # Action Buttons Frame
        btn_frame = ctk.CTkFrame(self, fg_color="#242424", corner_radius=10)
        btn_frame.grid(row=2, column=0, sticky="ew", padx=f_pad, pady=8)
        btn_frame.grid_columnconfigure(0, weight=1)

        self.btn_download_ollama = ctk.CTkButton(
            btn_frame,
            text="1. Unduh & Pasang Ollama (~65 MB)",
            font=ctk.CTkFont(size=13, weight="bold"),
            height=38,
            corner_radius=8,
            fg_color="#1f6aa5",
            hover_color="#144870",
            command=self._on_click_download_ollama,
        )
        self.btn_download_ollama.grid(row=0, column=0, sticky="ew", padx=12, pady=(12, 6))

        self.btn_pull_model = ctk.CTkButton(
            btn_frame,
            text="2. Unduh Model AI (qwen2.5:3b) (~2.0 GB)",
            font=ctk.CTkFont(size=13, weight="bold"),
            height=38,
            corner_radius=8,
            fg_color="#13a06f",
            hover_color="#0e7a55",
            state="disabled",
            command=self._on_click_pull_model,
        )
        self.btn_pull_model.grid(row=1, column=0, sticky="ew", padx=12, pady=(6, 12))

        # Progress Frame
        p_frame = ctk.CTkFrame(self, fg_color="#242424", corner_radius=10)
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


# ───────────────────────── Aplikasi Utama ─────────────────────────
class JSFSApp(ctk.CTk):
    def __init__(self):
        super().__init__()
        self.title("JSFS v1.5.0 — Journal Scraper For Students (Hybrid AI Engine)")
        self.geometry("1040x860")
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

        # Grid Window: baris 1 (body) fleksibel, baris 0 & 2 dipatok
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(1, weight=1)

        self._build_fonts()
        self._build_header()
        self._build_body()
        self._build_actionbar()

        self.protocol("WM_DELETE_WINDOW", self._on_close)
        self.after(100, self._poll_queue)            # pompa pesan thread -> GUI (100 ms)
        self.after(300, self._refresh_badge)         # cek status Ollama (loop 30 dtk)

    # ── fonts ──
    def _build_fonts(self):
        self.f_title = ctk.CTkFont(size=20, weight="bold")
        self.f_h = ctk.CTkFont(size=13, weight="bold")
        self.f_n = ctk.CTkFont(size=12)
        self.f_s = ctk.CTkFont(size=11)
        self.f_mono = ctk.CTkFont(family="Consolas", size=12)
        self.f_btn = ctk.CTkFont(size=14, weight="bold")

    # ── header + badge Ollama ──
    def _build_header(self):
        hdr = ctk.CTkFrame(self, fg_color="transparent")
        hdr.grid(row=0, column=0, sticky="ew", padx=PAD + 4, pady=(PAD, GAP))
        hdr.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(hdr, text="JSFS v1.5.0 — Journal Scraper For Students",
                     font=self.f_title, text_color="#38bdf8", anchor="w").grid(row=0, column=0, sticky="w")
        ctk.CTkLabel(hdr, text="Hybrid AI Engine • Smart SQLite WAL Cache • Top 3 AI Advisor • Schedule Estimator",
                     font=self.f_s, text_color=MUTED, anchor="w").grid(row=1, column=0, sticky="w", pady=(2, 0))

        texts = (f"●  AI Aktif · {ai_engine.MODEL}", "○  Mode Regex (Klik untuk Setup)")
        self.badge = ctk.CTkButton(
            hdr,
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
        self.badge.grid(row=0, column=1, rowspan=2, sticky="e", padx=(GAP, 0))

    # ── body (scrollable: Zero Clipping aturan #4) ──
    def _build_body(self):
        body = ctk.CTkScrollableFrame(self, fg_color="transparent")
        body.grid(row=1, column=0, sticky="nsew", padx=PAD, pady=0)
        body.grid_columnconfigure(0, weight=1)
        self.body = body

        # Baris 0: kata kunci
        kw = ctk.CTkFrame(body, fg_color=BG_CARD, corner_radius=10)
        kw.grid(row=0, column=0, sticky="ew", pady=(0, GAP + 2))
        kw.grid_columnconfigure(1, weight=1)
        ctk.CTkLabel(kw, text="Kata Kunci:", font=self.f_h).grid(row=0, column=0, padx=(PAD, GAP), pady=PAD)
        self.entry_kw = ctk.CTkEntry(kw, height=34, font=self.f_n,
                                     placeholder_text="Topik riset (mis. bisnis, akuntansi, machine learning, manajemen)")
        self.entry_kw.grid(row=0, column=1, sticky="ew", pady=PAD)
        self.entry_kw.bind("<Return>", lambda _e: self._on_start())
        ctk.CTkButton(kw, text="Cari", width=76, height=34, font=self.f_h,
                      command=self._on_start).grid(row=0, column=2, padx=(GAP, PAD), pady=PAD)

        # Baris 1: naskah
        ms = ctk.CTkFrame(body, fg_color=BG_CARD, corner_radius=10)
        ms.grid(row=1, column=0, sticky="ew", pady=(0, GAP + 2))
        ms.grid_columnconfigure(3, weight=1)
        ctk.CTkLabel(ms, text="Naskah Ilmiah:", font=self.f_h, text_color="#38bdf8").grid(
            row=0, column=0, padx=(PAD, GAP), pady=PAD)
        ctk.CTkButton(ms, text="Unggah (.docx/.pdf)", height=32, font=self.f_n,
                      width=fit_w(self.f_n, "Unggah (.docx/.pdf)", 0, 28),
                      command=self._on_upload).grid(row=0, column=1, padx=GAP, pady=PAD)
        self.var_match = ctk.BooleanVar(value=False)
        FitCheckBox(ms, "Match Score (0–100%)", self.f_h, variable=self.var_match).grid(
            row=0, column=2, padx=GAP, pady=PAD)
        self.lbl_ms = ctk.CTkLabel(ms, text="Belum ada naskah (opsional)", font=self.f_s,
                                   text_color=MUTED, anchor="w")
        self.lbl_ms.grid(row=0, column=3, sticky="ew", padx=(GAP, PAD))

        # Baris 2: kartu 2x2 (kolom 3:2 seragam)
        grid = ctk.CTkFrame(body, fg_color="transparent")
        grid.grid(row=2, column=0, sticky="nsew", pady=(0, GAP + 2))
        grid.grid_columnconfigure(0, weight=3, uniform="cards")
        grid.grid_columnconfigure(1, weight=2, uniform="cards")
        for r in (0, 1):
            grid.grid_rowconfigure(r, weight=1)
        self._card_source(grid).grid(row=0, column=0, sticky="nsew", padx=(0, GAP), pady=(0, GAP))
        self._card_sinta(grid).grid(row=0, column=1, sticky="nsew", padx=(GAP, 0), pady=(0, GAP))
        self._card_month(grid).grid(row=1, column=0, sticky="nsew", padx=(0, GAP), pady=(GAP, 0))
        self._card_ai(grid).grid(row=1, column=1, sticky="nsew", padx=(GAP, 0), pady=(GAP, 0))

        # Baris 3: progress ganda
        self.progress = DualProgress(body, self.f_n)
        self.progress.grid(row=3, column=0, sticky="ew", pady=(0, GAP + 2))

        # Baris 4-5: log
        lh = ctk.CTkFrame(body, fg_color="transparent")
        lh.grid(row=4, column=0, sticky="ew")
        lh.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(lh, text="Pemantauan Proses & Riwayat Log:", font=self.f_h, anchor="w").grid(
            row=0, column=0, sticky="w")
        ctk.CTkButton(lh, text="Bersihkan Log", width=fit_w(self.f_s, "Bersihkan Log", 0, 24), height=24,
                      font=self.f_s, fg_color="#3a3a3a", hover_color="#4a4a4a",
                      command=self._clear_log).grid(row=0, column=1, sticky="e")
        self.log_box = ctk.CTkTextbox(body, height=190, font=self.f_mono, wrap="word", state="disabled")
        self.log_box.grid(row=5, column=0, sticky="nsew", pady=(GAP, PAD))
        self.log("Selamat datang di JSFS v1.4.0 (Hybrid AI Engine).")

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

    # ── action bar (dipatok di bawah) ──
    def _build_actionbar(self):
        bar = ctk.CTkFrame(self, fg_color="transparent")
        bar.grid(row=2, column=0, sticky="ew", padx=PAD + 4, pady=(GAP, PAD))
        bar.grid_columnconfigure(0, weight=3, uniform="act")
        bar.grid_columnconfigure(1, weight=2, uniform="act")

        self.btn_start = ctk.CTkButton(bar, text="MULAI SCRAPING", height=46, font=self.f_btn,
                                       fg_color=BLUE, hover_color=BLUE_H, command=self._on_start)
        self.btn_start.grid(row=0, column=0, sticky="ew", padx=(0, GAP))
        self.btn_preview = ctk.CTkButton(bar, text="PREVIEW HASIL DATA", height=46, font=self.f_btn,
                                         fg_color=GREEN, hover_color=GREEN_H, state="disabled",
                                         command=self._open_data_previewer)
        self.btn_preview.grid(row=0, column=1, sticky="ew", padx=(GAP, 0))

        foot = ctk.CTkFrame(bar, fg_color="transparent")
        foot.grid(row=1, column=0, columnspan=2, sticky="ew", pady=(GAP + 2, 0))
        foot.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(foot, text="Output: output/  (format: JSFS_<keyword>_<timestamp>)",
                     font=self.f_s, text_color=MUTED, anchor="w").grid(row=0, column=0, sticky="w")
        ctk.CTkButton(foot, text="Buka Folder Output", height=28, font=self.f_s,
                      width=fit_w(self.f_s, "Buka Folder Output", 0, 28),
                      command=self._open_output).grid(row=0, column=1, sticky="e")

    # ───────────────────────── Util GUI ─────────────────────────
    def log(self, msg: str):
        """Hanya dari thread GUI. Dari thread lain: ui_q.put(('log', msg))."""
        self.log_box.configure(state="normal")
        self.log_box.insert("end", f"[{time.strftime('%H:%M:%S')}] {msg}\n")
        self.log_box.see("end")
        self.log_box.configure(state="disabled")

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
        else:
            self.btn_start.configure(text="MULAI SCRAPING", fg_color=BLUE, hover_color=BLUE_H,
                                     command=self._on_start)

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
            records = run_scraper(p, lambda d, t: q.put(("scrape", d, t)), self.cancel_evt)
            if self.cancel_evt.is_set():
                q.put(("done", None))
                return

            q.put(("log", f"Fase 1 selesai: {len(records)} jurnal berhasil di-crawl."))

            # FASE 2: Pre-Filter (Rank SINTA & Bulan Terbit)
            survivors: list[JournalEntry] = []
            for r in records:
                if r["sinta"] not in p["sinta"]:
                    continue
                if not lolos_filter_bulan(r.get("bulan_terbit", []), p["months"], p["include_unknown"]):
                    continue
                survivors.append(r)

            if len(survivors) > p["target"]:
                survivors = survivors[:p["target"]]

            q.put(("log", f"Fase 2: Pre-filter -> {len(survivors)}/{len(records)} jurnal lolos seleksi SINTA & Bulan."))

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

            # Analisis Naskah & Skor Kesesuaian (v1.5.0)
            if p.get("match") and p.get("manuscript") and os.path.exists(p["manuscript"]):
                q.put(("log", f"Menganalisis kesesuaian naskah: {os.path.basename(p['manuscript'])}..."))
                ms_data = parse_manuscript(p["manuscript"])
                if ms_data:
                    for s in survivors:
                        match_res = calculate_manuscript_match(ms_data, s)
                        s.match_score = match_res.get("score", 0)
                        s.kesesuaian_naskah = match_res.get("level", "Kurang Sesuai")
            else:
                # Mode Pencocokan Berbasis Kata Kunci (Keyword-Mode Scoring - v1.5.0)
                q.put(("log", f"Menghitung skor kesesuaian berbasis kata kunci '{p['keyword']}' (Kamus Dwibahasa)..."))
                for s in survivors:
                    kw_res = calculate_keyword_match_score(p["keyword"], s)
                    s.match_score = kw_res.get("score", 0)
                    s.kesesuaian_naskah = kw_res.get("level", "Kurang Sesuai")

            n_ai = sum(1 for r in survivors if r.get("sumber_focus") == "AI")
            n_ai_apc = sum(1 for r in survivors if r.get("sumber_apc") == "AI")
            q.put(("log", f"Hasil AI Enrichment: Focus via AI {n_ai}/{len(survivors)}, APC via AI {n_ai_apc}/{len(survivors)}."))

            # Filter biaya pengguna jika diminta
            final_survivors: list[JournalEntry] = []
            for r in survivors:
                b_pub = r.get("biaya_publikasi")
                if p.get("only_free"):
                    if b_pub == 0:
                        final_survivors.append(r)
                    elif b_pub is None and p.get("include_na"):
                        final_survivors.append(r)
                else:
                    if b_pub is not None and b_pub > 0:
                        if p.get("min_rp", 0) <= b_pub <= p.get("max_rp", 15_000_000):
                            final_survivors.append(r)
                    elif b_pub == 0:
                        if p.get("min_rp", 0) == 0:
                            final_survivors.append(r)
                    else:
                        if p.get("include_na", True):
                            final_survivors.append(r)

            if not final_survivors and survivors:
                final_survivors = survivors

            self._latest_results = final_survivors

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
        """Dijalankan di thread GUI tiap 100 ms. Coalesce progress pesan agar GUI anti-freeze."""
        latest, others = {}, []
        try:
            while True:
                msg = self.ui_q.get_nowait()
                if msg[0] in ("scrape", "ai"):
                    latest[msg[0]] = msg[1:]
                else:
                    others.append(msg)
        except queue.Empty:
            pass

        for key, (done, total) in latest.items():
            self.progress.update_bar(key, done, total)
        for msg in others:
            kind = msg[0]
            if kind == "log":
                self.log(msg[1])
            elif kind == "ollama":
                self._apply_badge(msg[1])
            elif kind == "error":
                self.log(f"✖ Error: {msg[1]}")
                self._set_running(False)
            elif kind == "done":
                self._set_running(False)
                if msg[1] is None:
                    self.log("Proses dihentikan oleh pengguna.")
                else:
                    self.log(f"✔ Selesai. {msg[1]} jurnal berhasil diekspor ke folder output/.")
                    if self._latest_results:
                        self.btn_preview.configure(
                            state="normal",
                            text=f"PREVIEW HASIL ({len(self._latest_results)})",
                        )
        self.after(100, self._poll_queue)

    def _on_close(self):
        self.cancel_evt.set()
        ai_engine.shutdown()
        self.destroy()


if __name__ == "__main__":
    app = JSFSApp()
    app.mainloop()
