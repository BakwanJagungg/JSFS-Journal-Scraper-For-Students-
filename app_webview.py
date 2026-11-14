"""
app_webview.py
Desktop UI WebView2 (pywebview + HTML5/CSS3/Vanilla JS) untuk JSFS v2.0.0.
Menggantikan lapisan CustomTkinter dengan rendering GPU 60 FPS yang bebas Not Responding,
sambil mempertahankan 100% mesin scraping, presisi APC, cache SQLite, AI Ollama, dan ekspor.
"""

from __future__ import annotations

import ctypes
import os
import re
import sys
import threading
import time
import webbrowser
from concurrent.futures import ThreadPoolExecutor
from typing import Any, Optional

import webview

from config import APP_VERSION, INFO_NOT_FOUND, OUTPUT_DIR


def _get_bundle_dir() -> str:
    """Direktori aset UI (mendukung PyInstaller _MEIPASS dan mode sumber)."""
    if getattr(sys, "frozen", False) and hasattr(sys, "_MEIPASS"):
        return sys._MEIPASS
    return os.path.dirname(os.path.abspath(__file__))


def _copy_to_clipboard(text: str) -> bool:
    """Salin teks Unicode ke clipboard OS tanpa dependensi Tkinter."""
    if not text:
        return False
    if sys.platform != "win32":
        return False

    CF_UNICODETEXT = 13
    GMEM_MOVEABLE = 0x0002

    user32 = ctypes.windll.user32
    kernel32 = ctypes.windll.kernel32
    kernel32.GlobalAlloc.argtypes = [ctypes.c_uint, ctypes.c_size_t]
    kernel32.GlobalAlloc.restype = ctypes.c_void_p
    kernel32.GlobalLock.argtypes = [ctypes.c_void_p]
    kernel32.GlobalLock.restype = ctypes.c_void_p
    kernel32.GlobalUnlock.argtypes = [ctypes.c_void_p]
    user32.SetClipboardData.argtypes = [ctypes.c_uint, ctypes.c_void_p]
    user32.SetClipboardData.restype = ctypes.c_void_p

    data = (text + "\0").encode("utf-16-le")
    if not user32.OpenClipboard(None):
        return False
    try:
        user32.EmptyClipboard()
        h_mem = kernel32.GlobalAlloc(GMEM_MOVEABLE, len(data))
        if not h_mem:
            return False
        ptr = kernel32.GlobalLock(h_mem)
        if not ptr:
            return False
        try:
            ctypes.memmove(ptr, data, len(data))
        finally:
            kernel32.GlobalUnlock(h_mem)
        user32.SetClipboardData(CF_UNICODETEXT, h_mem)
        return True
    finally:
        user32.CloseClipboard()


def _run_scraper(params: dict, on_progress, cancel: threading.Event, on_log=None, on_telemetry=None, on_checkpoint=None) -> list:
    """
    Menjalankan Fase 1 Garuda & SINTA dengan Pre-Deep Deduplication lintas kata kunci & lintas portal (v2.1.0),
    Smart Typo Fallback, Adaptive Pool Multiplier, serta Deep Enrich terpadu.
    """
    from config import compute_candidate_pool_limit
    from doc_reader import get_fallback_keywords
    from scraper_garuda import deep_enrich_entries, merge_cross_source_entries, scrape_garuda
    from scraper_sinta import scrape_sinta, scrape_sinta_selenium

    keyword = params["keyword"]
    sinta_ranks = sorted(list(params["sinta"]))
    target = max(1, min(1000, int(params.get("target", 25))))
    candidate_target = compute_candidate_pool_limit(target)
    use_garuda = params["garuda"]
    use_sinta = params["sinta_src"]
    turbo = bool(params.get("turbo", True)) or target >= 100
    use_selenium = params.get("selenium", False)

    def _scrape_pass(kw_query: str) -> list:
        pass_records = []
        if use_garuda and not cancel.is_set():
            g_results = scrape_garuda(
                keyword=kw_query,
                sinta_ranks=sinta_ranks,
                deep_detail=False,
                max_results=candidate_target,
                turbo_mode=turbo,
                free_only=False,
                include_unknown_apc=True,
                progress_cb=on_telemetry,
            )
            pass_records.extend(g_results)
            on_progress(len(pass_records), target)

        if use_sinta and not cancel.is_set():
            if use_selenium:
                s_results = scrape_sinta_selenium(
                    keyword=kw_query,
                    sinta_ranks=sinta_ranks,
                    deep_detail=False,
                    max_results=candidate_target,
                    turbo_mode=turbo,
                    free_only=False,
                    include_unknown_apc=True,
                )
            else:
                s_results = scrape_sinta(
                    keyword=kw_query,
                    sinta_ranks=sinta_ranks,
                    deep_detail=False,
                    max_results=candidate_target,
                    turbo_mode=turbo,
                    free_only=False,
                    include_unknown_apc=True,
                    allow_selenium_fallback=False,
                    log_cb=on_log,
                    progress_cb=on_telemetry,
                )
            pass_records.extend(s_results)
            on_progress(len(pass_records), target)
        return pass_records

    records = _scrape_pass(keyword)

    if not records and not cancel.is_set():
        fallback_kws = get_fallback_keywords(keyword)
        if fallback_kws:
            if on_log:
                on_log(
                    f"⚠️ Kata kunci '{keyword}' tidak menemukan hasil. "
                    "Mencoba perluasan kata kunci / koreksi ejaan..."
                )
            for fb_kw in fallback_kws:
                if cancel.is_set():
                    break
                if on_log:
                    on_log(f"↻ Mencoba kata kunci alternatif: '{fb_kw}'")
                fb_records = _scrape_pass(fb_kw)
                if fb_records:
                    records.extend(fb_records)
                    break

    # Tahap Pre-Deep Deduplication: gabungkan duplikat lintas kata kunci & lintas portal SEBELUM deep OJS crawl
    raw_count = len(records)
    if records:
        records = merge_cross_source_entries(records)
    if len(records) > candidate_target:
        records = records[:candidate_target]

    if on_log and raw_count > 0:
        dedup_saved = max(0, raw_count - len(records))
        on_log(
            f"🔎 Pre-Deep Deduplikasi: {len(records)} jurnal unik kandidat "
            f"(pangkas {dedup_saved} duplikat lintas portal/kata kunci)."
        )

    if records and not cancel.is_set():
        records = deep_enrich_entries(
            records,
            keyword=keyword,
            max_results=target,
            turbo_mode=turbo,
            free_only=False,
            include_unknown_apc=True,
            progress_cb=on_telemetry,
            checkpoint_cb=on_checkpoint,
            cancel_event=cancel,
        )

    on_progress(len(records), target)
    return records


class JSFSApi:
    """Bridge Python <-> JavaScript yang dipanggil WebView2 via window.pywebview.api."""

    def __init__(self):
        self._window = None
        self._lock = threading.Lock()
        self._cancel_evt = threading.Event()
        self._ai_cancel_evt = threading.Event()
        self._worker: Optional[threading.Thread] = None

        self._running = False
        self._phase = "idle"
        self._phase_status = "Siap mencari jurnal"
        self._current_item = ""
        self._active_keyword = ""
        self._run_started_at = 0.0
        self._elapsed_seconds = 0.0
        self._seen_deep_names: set[str] = set()
        self._autosave_executor = ThreadPoolExecutor(max_workers=1)
        self._logs: list[str] = []
        self._progress = {"scrape_done": 0, "scrape_target": 0, "ai_done": 0, "ai_target": 0}
        self._kpi = {"total": 0, "free": 0, "paid": 0, "fast": 0}
        self._latest_entries: list = []
        self._serialized_results: list[dict[str, Any]] = []
        self._results_version = 0
        self._latest_excel_path: str = ""
        self._ollama_active = False
        self._cache_count = 0

        self._ai_task_running = False
        self._ai_task_percent = 0.0
        self._ai_task_message = ""

    def attach_window(self, window):
        self._window = window

    def startup_background_checks(self):
        """Dipanggil 600ms setelah jendela tampil agar first paint instan (<1.5 detik)."""
        def _bg():
            try:
                import cache_engine
                cache_engine.cache_engine.cleanup_expired()
                stats = cache_engine.cache_engine.get_stats()
                with self._lock:
                    self._cache_count = int(stats.get("valid_entries", 0))
            except Exception:
                pass
            try:
                import ai_engine
                active = bool(ai_engine.check_ollama_active(force=True))
                with self._lock:
                    self._ollama_active = active
            except Exception:
                pass

        threading.Timer(0.6, _bg).start()

    def shutdown(self):
        """Bersihkan worker, AI engine, dan koneksi SQLite saat jendela ditutup."""
        self._cancel_evt.set()
        self._ai_cancel_evt.set()
        try:
            import ai_engine
            ai_engine.shutdown()
        except Exception:
            pass
        try:
            import cache_engine
            cache_engine.close_all_connections()
        except Exception:
            pass
        try:
            self._autosave_executor.shutdown(wait=False, cancel_futures=True)
        except Exception:
            pass

    def _log(self, msg: str):
        ts = time.strftime("%H:%M:%S")
        with self._lock:
            self._logs.append(f"[{ts}] {msg}")
            if len(self._logs) > 600:
                self._logs = self._logs[-400:]

    def _serialize_results(self, entries: list) -> list[dict[str, Any]]:
        import exporter
        import month_parser

        out: list[dict[str, Any]] = []
        for idx, j in enumerate(entries):
            b_pub = getattr(j, "biaya_publikasi", None)
            if b_pub is None:
                b_pub = getattr(j, "apc_amount", None)
            b_sub = getattr(j, "biaya_submit", None)
            cur = getattr(j, "mata_uang", None) or "IDR"
            apc_status = str(getattr(j, "apc_status", INFO_NOT_FOUND) or INFO_NOT_FOUND)
            apc_low = apc_status.lower()

            is_free = (b_pub == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("bebas biaya" in apc_low)
            if is_free:
                apc_cat = "free"
                apc_label = "Gratis (Rp 0)"
                biaya_sort = 0
            elif isinstance(b_pub, (int, float)) and b_pub > 0:
                apc_cat = "paid"
                apc_label = exporter._format_amount(int(b_pub), cur)
                biaya_sort = int(b_pub)
            else:
                apc_cat = "unknown"
                apc_label = apc_status if apc_status != INFO_NOT_FOUND else "Cek Web / Belum Terdata"
                biaya_sort = 999_999_999

            submit_label = exporter._format_amount(int(b_sub), cur) if isinstance(b_sub, (int, float)) else "-"
            bulan_list = getattr(j, "bulan_terbit", None) or []
            jadwal = month_parser.format_bulan_terbit(bulan_list) if bulan_list else INFO_NOT_FOUND
            terdekat = exporter.get_next_issue_estimate(bulan_list)

            email_val = str(getattr(j, "email", INFO_NOT_FOUND) or INFO_NOT_FOUND)
            telp_val = str(getattr(j, "telepon", INFO_NOT_FOUND) or INFO_NOT_FOUND)
            url_val = str(getattr(j, "link_ojs", "") or getattr(j, "sinta_profile_url", "") or getattr(j, "garuda_profile_url", "") or "")
            sinta_m = re.search(r"(\d)", str(getattr(j, "peringkat_sinta", "")))
            sinta_int = int(sinta_m.group(1)) if sinta_m else 0

            health_warning = (not bulan_list) and (email_val == INFO_NOT_FOUND) and (telp_val == INFO_NOT_FOUND)
            email_short = email_val.split(",")[0].strip() if email_val != INFO_NOT_FOUND else "Email belum terdata"

            out.append({
                "index": idx,
                "nama": str(getattr(j, "nama_jurnal", "") or ""),
                "penerbit": str(getattr(j, "penerbit", "") or "-"),
                "sinta": sinta_int,
                "sumber": str(getattr(j, "sumber", "Portal") or "Portal"),
                "match_score": int(getattr(j, "match_score", 0) or 40),
                "kesesuaian_naskah": str(getattr(j, "kesesuaian_naskah", "Cukup Sesuai") or "Cukup Sesuai"),
                "apc_category": apc_cat,
                "apc_label": apc_label,
                "biaya_publikasi": int(b_pub) if isinstance(b_pub, (int, float)) else None,
                "biaya_sort": biaya_sort,
                "submit_label": submit_label,
                "jadwal_terbit": jadwal if jadwal != INFO_NOT_FOUND else "-",
                "terbit_terdekat": terdekat,
                "email": email_val,
                "email_short": email_short,
                "telepon": telp_val,
                "url": url_val,
                "apc_url": str(getattr(j, "apc_url", "") or url_val),
                "focus_scope": str(getattr(j, "focus_scope", INFO_NOT_FOUND) or INFO_NOT_FOUND),
                "health_warning": health_warning,
            })
        return out

    def get_status(self, log_offset: int = 0) -> dict[str, Any]:
        with self._lock:
            safe_off = max(0, min(int(log_offset), len(self._logs)))
            elapsed = (
                round(max(0.0, time.monotonic() - self._run_started_at), 1)
                if self._running and self._run_started_at > 0
                else self._elapsed_seconds
            )
            items_prog = max(
                int(self._progress.get("scrape_done", 0)),
                int(self._kpi.get("total", 0)),
            )
            rate = round(items_prog / elapsed, 1) if elapsed >= 0.5 else 0.0
            kpi_snap = dict(self._kpi)
            prog_snap = dict(self._progress)
            return {
                "logs": self._logs[safe_off:],
                "next_log_offset": len(self._logs),
                "progress": prog_snap,
                "scrape_done": prog_snap.get("scrape_done", 0),
                "scrape_total": prog_snap.get("scrape_target", 0),
                "ai_done": prog_snap.get("ai_done", 0),
                "ai_total": prog_snap.get("ai_target", 0),
                "kpi": kpi_snap,
                "live_counts": kpi_snap,
                "results": list(self._serialized_results),
                "results_version": self._results_version,
                "excel_path": self._latest_excel_path,
                "running": self._running,
                "phase": self._phase,
                "phase_label": self._phase_status,
                "phase_status": self._phase_status,
                "current_item": self._current_item,
                "elapsed_seconds": elapsed,
                "rate_per_sec": rate,
                "ollama_active": self._ollama_active,
                "cache_count": self._cache_count,
            }

    def start_search(self, params: dict[str, Any]) -> dict[str, Any]:
        return self.start_scrape(params)

    def start_scrape(self, params: dict[str, Any]) -> dict[str, Any]:
        with self._lock:
            if self._running:
                return {"ok": False, "message": "Proses pencarian masih berlangsung."}
            self._running = True
            self._cancel_evt.clear()
            try:
                raw_target = int(params.get("target", 25))
            except (TypeError, ValueError):
                raw_target = 25
            target = max(1, min(1000, raw_target))
            params = dict(params)
            params["target"] = target
            if target >= 100:
                params["turbo"] = True
            self._phase = "fase1_portal"
            self._phase_status = "Menjelajahi halaman portal SINTA & Garuda..."
            self._active_keyword = str(params.get("keyword", "") or "")
            self._current_item = f"Memulai pencarian '{self._active_keyword}'..."
            self._run_started_at = time.monotonic()
            self._elapsed_seconds = 0.0
            self._seen_deep_names = set()
            self._progress = {"scrape_done": 0, "scrape_target": 0, "ai_done": 0, "ai_target": 0}
            self._kpi = {"total": 0, "free": 0, "paid": 0, "fast": 0}
            self._latest_excel_path = ""

        self._worker = threading.Thread(target=self._pipeline, args=(params,), daemon=True)
        self._worker.start()
        return {"ok": True}

    def stop_scrape(self) -> dict[str, Any]:
        self._cancel_evt.set()
        self._log("Menghentikan proses... (tugas aktif diselesaikan dengan aman)")
        return {"ok": True}

    def _pipeline(self, p: dict[str, Any]):
        import ai_engine
        import cache_engine
        import exporter
        import month_parser

        try:
            keyword = str(p.get("keyword", "")).strip()
            target = max(1, min(1000, int(p.get("target", 25))))
            turbo_mode = bool(p.get("turbo", True)) or target >= 100
            sinta_set = {int(x) for x in p.get("sinta", [1, 2, 3, 4, 5, 6])}
            months_set = {int(x) for x in p.get("months", list(range(1, 13)))}
            include_unknown_month = bool(p.get("include_unknown", True))

            ai_engine.set_enabled(bool(p.get("ai_enabled", True)))
            ai_engine.set_workers(int(p.get("ai_workers", 2)))
            with self._lock:
                self._ollama_active = bool(ai_engine.check_ollama_active())
                self._phase = "fase1_portal"
                self._phase_status = "Menjelajahi halaman portal SINTA & Garuda..."

            if target >= 100:
                self._log(f"⚡ Target {target} jurnal: Turbo Mode diaktifkan otomatis untuk kecepatan maksimal.")
            self._log(f"Fase 1: Scraping massal untuk '{keyword}' (Target: {target} jurnal)...")
            records = _run_scraper(
                {
                    "keyword": keyword,
                    "sinta": sinta_set,
                    "target": target,
                    "garuda": bool(p.get("garuda", True)),
                    "sinta_src": bool(p.get("sinta_src", True)),
                    "turbo": turbo_mode,
                    "selenium": bool(p.get("selenium", False)),
                },
                on_progress=lambda d, t: self._set_scrape_prog(d, t),
                cancel=self._cancel_evt,
                on_log=self._log,
                on_telemetry=self._on_scraper_telemetry,
                on_checkpoint=self._on_checkpoint,
            )
            if self._cancel_evt.is_set():
                self._finish_run()
                return

            self._log(f"Fase 1 selesai: {len(records)} jurnal berhasil di-crawl.")

            # FASE 2: Pre-Filter SINTA & Bulan Terbit
            survivors = []
            for r in records:
                if r["sinta"] not in sinta_set:
                    continue
                if not month_parser.lolos_filter_bulan(r.get("bulan_terbit", []), months_set, include_unknown_month):
                    continue
                survivors.append(r)

            pool_limit = max(target * 2, 40)
            if len(survivors) > pool_limit:
                survivors = survivors[:pool_limit]

            self._log(f"Fase 2: Pre-filter -> {len(survivors)}/{len(records)} jurnal lolos seleksi SINTA & Bulan.")
            cur_m_f2 = time.localtime().tm_mon
            near_m_f2 = {cur_m_f2, (cur_m_f2 % 12) + 1, ((cur_m_f2 + 1) % 12) + 1}
            if survivors:
                inter_ser = self._serialize_results(survivors[:target])
                with self._lock:
                    self._latest_entries = survivors[:target]
                    self._serialized_results = inter_ser
                    self._results_version += 1
                    self._kpi = {
                        "total": len(survivors),
                        "free": sum(1 for r in survivors if r.get("biaya_publikasi") == 0 or "gratis" in str(r.get("apc_status", "")).lower()),
                        "paid": sum(1 for r in survivors if (r.get("biaya_publikasi") or 0) > 0),
                        "fast": sum(1 for r in survivors if any(m in r.get("bulan_terbit", []) for m in near_m_f2)),
                    }
            else:
                with self._lock:
                    self._kpi = {"total": 0, "free": 0, "paid": 0, "fast": 0}

            if not survivors:
                self._log("⚠️ Tidak ada jurnal yang lolos pre-filter. Selesai.")
                with self._lock:
                    self._latest_entries = []
                    self._serialized_results = []
                    self._results_version += 1
                self._finish_run()
                return

            # FASE 3: Regex-First, AI-Second
            with self._lock:
                self._phase = "fase3_ai"
                self._phase_status = "Ekstraksi biaya & jadwal terbit..."
                self._current_item = f"Memvalidasi {len(survivors)} jurnal kandidat..."
            self._log(
                f"Fase 3: Ekstraksi Regex-First (AI selektif maks {ai_engine.MAX_AI_QUEUE_JOURNALS} "
                f"jurnal jika kosong) untuk {len(survivors)} jurnal..."
            )
            self._set_ai_prog(0, len(survivors))

            def _on_ai_telemetry(done: int, total: int, entry=None):
                name = getattr(entry, "nama_jurnal", "") or ""
                with self._lock:
                    self._phase = "fase3_ai"
                    self._phase_status = f"Validasi cerdas AI ({done}/{total} jurnal)..."
                    if name:
                        self._current_item = name
                    self._progress["ai_done"] = int(done)
                    self._progress["ai_target"] = int(total)

            ai_engine.enrich_records(
                survivors,
                on_progress=lambda d, t: self._set_ai_prog(d, t),
                cancel=self._cancel_evt,
                on_ai_progress=_on_ai_telemetry,
            )
            if self._cancel_evt.is_set():
                self._finish_run()
                return

            # Analisis Naskah & Match Score
            if p.get("match", True):
                from doc_reader import (
                    calculate_keyword_match_score,
                    calculate_match_score,
                    extract_manuscript_metadata,
                )

                ms_path = str(p.get("manuscript", "") or "")
                if ms_path and os.path.exists(ms_path):
                    self._log(f"Menganalisis kesesuaian naskah: {os.path.basename(ms_path)}...")
                    ms_data = extract_manuscript_metadata(ms_path)
                    if ms_data:
                        for s in survivors:
                            match_res = calculate_match_score(ms_data, s)
                            s.match_score = match_res.get("score", 40)
                            s.kesesuaian_naskah = match_res.get("level", "Cukup Sesuai")
                else:
                    self._log(f"Menghitung skor kesesuaian berbasis kata kunci '{keyword}'...")
                    for s in survivors:
                        kw_res = calculate_keyword_match_score(keyword, s)
                        s.match_score = kw_res.get("score", 40)
                        s.kesesuaian_naskah = kw_res.get("level", "Cukup Sesuai")

            # Filter Biaya & High-Yield Recall (v1.6.1)
            min_rp = int(p.get("min_rp", 0))
            max_rp = int(p.get("max_rp", 15_000_000))
            only_free = bool(p.get("only_free", False))
            include_na = bool(p.get("include_na", True))

            free_list, unk_list, paid_list = [], [], []
            for r in survivors:
                b_pub = r.get("biaya_publikasi")
                if b_pub is None and r.get("apc_amount") is not None:
                    b_pub = r.get("apc_amount")
                apc_status = str(r.get("apc_status", "")).lower()

                is_free = (b_pub == 0) or ("gratis" in apc_status) or ("rp 0" in apc_status) or ("bebas biaya" in apc_status)
                is_unk = (
                    (b_pub is None)
                    or ("cek web" in apc_status)
                    or ("belum terdata" in apc_status)
                    or (r.get("apc_status") in (INFO_NOT_FOUND, "Cek Web / Belum Terdata"))
                )
                if is_free:
                    free_list.append(r)
                elif is_unk:
                    unk_list.append(r)
                elif min_rp <= b_pub <= max_rp:
                    paid_list.append(r)

            final_survivors = []
            if only_free:
                final_survivors.extend(free_list)
                if include_na or len(final_survivors) < target:
                    final_survivors.extend(unk_list)
            else:
                final_survivors.extend(free_list)
                final_survivors.extend(paid_list)
                if include_na or max_rp < 15_000_000:
                    final_survivors.extend(unk_list)

            if not final_survivors and survivors:
                final_survivors = survivors
            if len(final_survivors) > target:
                final_survivors = final_survivors[:target]

            cur_m = time.localtime().tm_mon
            near_months = {cur_m, (cur_m % 12) + 1, ((cur_m + 1) % 12) + 1}
            kpi_free = sum(
                1 for r in final_survivors
                if r.get("biaya_publikasi") == 0 or "gratis" in str(r.get("apc_status", "")).lower()
            )
            kpi_paid = sum(1 for r in final_survivors if (r.get("biaya_publikasi") or 0) > 0)
            kpi_fast = sum(1 for r in final_survivors if any(m in r.get("bulan_terbit", []) for m in near_months))

            # FASE 4: Export Output
            with self._lock:
                self._phase = "fase4_export"
                self._phase_status = f"Menyimpan berkas Excel & Markdown ({len(final_survivors)} jurnal)..."
                self._current_item = "Menulis berkas output (.xlsx / .md)"
            self._log(f"Fase 4: Mengekspor {len(final_survivors)} jurnal ke berkas output...")
            formats = list(p.get("formats", [".xlsx", ".md"]))
            exported = exporter.write_exports(
                final_survivors,
                keyword=keyword,
                formats=formats,
                sinta_ranks=sorted(list(sinta_set)),
            )
            excel_path = ""
            for ef in exported:
                self._log(f"💾 Berkas tersimpan → {ef}")
                if ef.endswith(".xlsx"):
                    excel_path = ef

            serialized = self._serialize_results(final_survivors)
            cache_stats = cache_engine.cache_engine.get_stats()

            with self._lock:
                self._latest_entries = final_survivors
                self._serialized_results = serialized
                self._results_version += 1
                self._latest_excel_path = excel_path
                self._kpi = {
                    "total": len(final_survivors),
                    "free": kpi_free,
                    "paid": kpi_paid,
                    "fast": kpi_fast,
                }
                self._cache_count = int(cache_stats.get("valid_entries", self._cache_count))

            self._log(f"✅ Selesai! {len(final_survivors)} jurnal siap ditinjau di tabel hasil.")
        except Exception as exc:
            self._log(f"❌ Error pipeline: {exc!r}")
        finally:
            self._finish_run()

    def _on_checkpoint(self, done: int = 0, total: int = 0, entries: Optional[list] = None):
        """Streaming batch ke tabel UI + trigger auto-save latar belakang setiap 50 jurnal selesai."""
        if not entries or self._cancel_evt.is_set():
            return
        snapshot = list(entries)
        kw = self._active_keyword
        try:
            ser_batch = self._serialize_results(snapshot)
            with self._lock:
                self._latest_entries = snapshot
                self._serialized_results = ser_batch
                self._results_version += 1
        except Exception:
            pass

        def _bg_save():
            try:
                import exporter
                xlsx_p, _ = exporter.export_autosave_checkpoint(snapshot, keyword=kw)
                if xlsx_p:
                    with self._lock:
                        if not self._latest_excel_path:
                            self._latest_excel_path = xlsx_p
                    self._log(f"💾 Auto-save checkpoint ({done}/{total} jurnal) → {xlsx_p}")
            except Exception:
                pass

        try:
            self._autosave_executor.submit(_bg_save)
        except Exception:
            pass

    def _on_scraper_telemetry(self, phase: str = "fase1_portal", done: int = 0, total: int = 0, current_item: str = "", entry=None):
        """Callback thread-safe per-halaman / per-jurnal untuk telemetry UI real-time."""
        with self._lock:
            self._phase = phase
            if phase == "fase1_portal":
                self._phase_status = "Menjelajahi halaman portal SINTA & Garuda..."
            elif phase == "fase2_deep":
                self._phase_status = f"Memeriksa biaya & jadwal ({done}/{total} jurnal)..."
            if current_item:
                self._current_item = str(current_item)
            self._progress["scrape_done"] = int(done)
            self._progress["scrape_target"] = int(total)

            if entry is not None:
                key = (getattr(entry, "nama_jurnal", "") or getattr(entry, "link_ojs", "") or "").strip().lower()
                if key and key not in self._seen_deep_names:
                    self._seen_deep_names.add(key)
                    b_pub = getattr(entry, "biaya_publikasi", None)
                    if b_pub is None:
                        b_pub = getattr(entry, "apc_amount", None)
                    apc_low = str(getattr(entry, "apc_status", "") or "").lower()
                    is_free = (b_pub == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("bebas biaya" in apc_low)
                    cur_m = time.localtime().tm_mon
                    near_months = {cur_m, (cur_m % 12) + 1, ((cur_m + 1) % 12) + 1}
                    bulan_list = getattr(entry, "bulan_terbit", None) or []
                    is_fast = any(m in bulan_list for m in near_months)
                    self._kpi["total"] = len(self._seen_deep_names)
                    if is_free:
                        self._kpi["free"] += 1
                    elif isinstance(b_pub, (int, float)) and b_pub > 0:
                        self._kpi["paid"] += 1
                    if is_fast:
                        self._kpi["fast"] += 1

    def _set_scrape_prog(self, done: int, total: int):
        with self._lock:
            self._progress["scrape_done"] = int(done)
            self._progress["scrape_target"] = int(total)

    def _set_ai_prog(self, done: int, total: int):
        with self._lock:
            self._progress["ai_done"] = int(done)
            self._progress["ai_target"] = int(total)

    def _finish_run(self):
        with self._lock:
            self._running = False
            if self._run_started_at > 0:
                self._elapsed_seconds = round(max(0.0, time.monotonic() - self._run_started_at), 1)
            self._phase = "done" if self._serialized_results else "idle"
            self._phase_status = "Pencarian selesai" if self._serialized_results else "Siap mencari jurnal"
            self._current_item = ""

    def upload_manuscript(self) -> dict[str, Any]:
        return self.pick_manuscript()

    def pick_manuscript(self) -> dict[str, Any]:
        if not self._window:
            return {"path": ""}
        dialog_type = getattr(getattr(webview, "FileDialog", None), "OPEN", None)
        if dialog_type is None:
            dialog_type = getattr(webview, "OPEN_DIALOG", 10)

        picked = self._window.create_file_dialog(
            dialog_type,
            allow_multiple=False,
            file_types=("Dokumen Naskah (*.docx;*.pdf)", "Semua Berkas (*.*)"),
        )
        if not picked:
            return {"path": ""}
        path = picked[0] if isinstance(picked, (list, tuple)) else str(picked)
        suggested = ""
        try:
            from doc_reader import extract_manuscript_metadata
            meta = extract_manuscript_metadata(path)
            kws = meta.get("keywords") or []
            if kws:
                suggested = ", ".join(kws[:3])
        except Exception:
            pass
        return {"path": path, "name": os.path.basename(path), "suggested_keyword": suggested}

    def copy_share_list(self, indices: Optional[list[int]] = None) -> dict[str, Any]:
        with self._lock:
            entries = list(self._latest_entries)
        if not entries:
            return {"ok": False}
        if indices:
            subset = [entries[i] for i in indices if 0 <= int(i) < len(entries)]
        else:
            subset = entries
        if not subset:
            subset = entries
        import exporter
        text = exporter.generate_copypaste_list(subset)
        ok = _copy_to_clipboard(text)
        return {"ok": ok, "text": text}

    def copy_editor_draft(self, index: int, keyword: str = "") -> dict[str, Any]:
        return self.copy_editor_chat(index, keyword)

    def copy_editor_chat(self, index: int, keyword: str = "") -> dict[str, Any]:
        with self._lock:
            entries = list(self._latest_entries)
        if not (0 <= int(index) < len(entries)):
            return {"ok": False}
        j = entries[int(index)]
        j_name = getattr(j, "nama_jurnal", "Jurnal Tujuan")
        kw_text = keyword.strip() or "bidang penelitian saya"
        draft = (
            f"Selamat pagi/siang Bapak/Ibu Tim Editor {j_name},\n\n"
            f"Perkenalkan saya mahasiswa yang sedang menyusun artikel ilmiah di bidang {kw_text}. "
            f"Melalui pesan ini saya ingin menanyakan apakah masih tersedia kuota/slot publikasi "
            f"untuk edisi terbit terdekat serta konfirmasi mengenai estimasi biaya publikasi (APC) "
            f"di {j_name}.\n\n"
            f"Terima kasih banyak atas perhatian dan informasinya."
        )
        ok = _copy_to_clipboard(draft)
        return {"ok": ok, "text": draft}

    def open_excel(self) -> dict[str, Any]:
        path = self._latest_excel_path
        if not path or not os.path.exists(path):
            return {"ok": False, "message": "File Excel hasil pencarian belum tersedia."}
        try:
            if sys.platform == "win32":
                os.startfile(path)
            else:
                webbrowser.open(f"file://{os.path.abspath(path)}")
            return {"ok": True}
        except Exception as e:
            return {"ok": False, "message": str(e)}

    def open_output_folder(self) -> dict[str, Any]:
        os.makedirs(OUTPUT_DIR, exist_ok=True)
        try:
            if sys.platform == "win32":
                os.startfile(OUTPUT_DIR)
            else:
                webbrowser.open(f"file://{os.path.abspath(OUTPUT_DIR)}")
            return {"ok": True}
        except Exception as e:
            return {"ok": False, "message": str(e)}

    def open_url(self, url: str) -> dict[str, Any]:
        clean = str(url or "").strip()
        if clean.startswith(("http://", "https://")):
            webbrowser.open(clean)
            return {"ok": True}
        return {"ok": False}

    def clear_cache(self) -> dict[str, Any]:
        try:
            import cache_engine
            cache_engine.cache_engine.clear_cache()
            stats = cache_engine.cache_engine.get_stats()
            with self._lock:
                self._cache_count = int(stats.get("valid_entries", 0))
            self._log("🗑️ Cache lokal SQLite berhasil dibersihkan.")
            return {"ok": True, "message": "Cache lokal berhasil dibersihkan."}
        except Exception as e:
            return {"ok": False, "message": f"Gagal membersihkan cache: {e}"}

    def get_ai_setup_status(self) -> dict[str, Any]:
        import ai_installer
        running = ai_installer.is_ollama_running(timeout=1.2)
        installed = ai_installer.is_model_installed(ai_installer.DEFAULT_MODEL, timeout=1.2) if running else False
        with self._lock:
            self._ollama_active = running
            return {
                "model": ai_installer.DEFAULT_MODEL,
                "running": running,
                "model_installed": installed,
                "task_running": self._ai_task_running,
                "task_percent": self._ai_task_percent,
                "task_message": self._ai_task_message,
            }

    def launch_ollama(self) -> dict[str, Any]:
        import ai_installer
        ok = ai_installer.launch_ollama_service()
        msg = "Layanan Ollama berhasil dijalankan." if ok else "Aplikasi Ollama tidak ditemukan di komputer ini."
        return {"ok": ok, "message": msg}

    def install_ollama(self) -> dict[str, Any]:
        import ai_installer
        with self._lock:
            if self._ai_task_running:
                return {"ok": False}
            self._ai_task_running = True
            self._ai_task_percent = 0.0
            self._ai_task_message = "Menyiapkan pengunduhan OllamaSetup.exe..."
        self._ai_cancel_evt.clear()

        def _prog(_dl, _tot, pct, msg):
            with self._lock:
                self._ai_task_percent = float(pct)
                self._ai_task_message = msg

        def _done(_ok, msg):
            with self._lock:
                self._ai_task_running = False
                self._ai_task_message = msg

        threading.Thread(
            target=ai_installer.download_and_install_ollama,
            args=(_prog, _done, self._ai_cancel_evt),
            daemon=True,
        ).start()
        return {"ok": True}

    def pull_ai_model(self) -> dict[str, Any]:
        import ai_installer
        with self._lock:
            if self._ai_task_running:
                return {"ok": False}
            self._ai_task_running = True
            self._ai_task_percent = 0.0
            self._ai_task_message = f"Mengunduh model {ai_installer.DEFAULT_MODEL}..."
        self._ai_cancel_evt.clear()

        def _prog(_dl, _tot, pct, msg):
            with self._lock:
                self._ai_task_percent = float(pct)
                self._ai_task_message = msg

        def _done(_ok, msg):
            with self._lock:
                self._ai_task_running = False
                self._ai_task_message = msg

        threading.Thread(
            target=ai_installer.pull_model,
            args=(ai_installer.DEFAULT_MODEL, _prog, _done, self._ai_cancel_evt),
            daemon=True,
        ).start()
        return {"ok": True}


def main():
    ui_path = os.path.join(_get_bundle_dir(), "ui_web", "index.html")
    api = JSFSApi()
    window = webview.create_window(
        f"JSFS v{APP_VERSION} — Journal Scraper For Students",
        url=ui_path,
        js_api=api,
        width=1120,
        height=760,
        min_size=(960, 680),
        text_select=True,
    )
    api.attach_window(window)
    window.events.closed += api.shutdown
    api.startup_background_checks()
    webview.start(debug=False)


if __name__ == "__main__":
    main()

