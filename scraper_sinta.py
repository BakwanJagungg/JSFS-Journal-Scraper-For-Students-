# =============================================================================
# JSFS - Journal Scraper For Students
# scraper_sinta.py — Modul scraping Direktori SINTA (Kemdiktisaintek) v1.3.8
# =============================================================================

import sys
import time
import random
import re
import threading
import requests
import urllib3
from bs4 import BeautifulSoup
from typing import Optional
from concurrent.futures import ThreadPoolExecutor
from rich.console import Console

# Nonaktifkan peringatan SSL tidak aman
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

# Pastikan console Windows mendukung output UTF-8
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass

import cache_engine

from config import (
    SINTA_JOURNAL_URL,
    HEADERS,
    get_browser_headers,
    REQUEST_DELAY_MIN,
    REQUEST_DELAY_MAX,
    MAX_PAGES_LIMIT,
    MAX_RESULTS,
    INFO_NOT_FOUND,
    DEAD_LINK_TEXT,
    DEEP_SCRAPE_WORKERS,
    TURBO_SCRAPE_WORKERS,
    NORMAL_SCRAPE_WORKERS,
    HTTP_TIMEOUT,
    DEEP_SCRAPE_TIMEOUT,
    FAST_FAIL_TIMEOUT,
)
from scraper_garuda import (
    JournalEntry,
    extract_ojs_deep_info,
    check_ojs_domain_reachable,
    clean_contact_list,
    sanitize_text,
    get_high_capacity_session,
    normalize_issn,
    clean_and_merge_subjects,
    format_frequency,
    parse_search_keywords,
    merge_cross_source_entries,
    build_tier3_scope_fallback,
    resolve_garuda_profile_meta,
    decode_cfemail,
    is_valid_indonesian_phone,
    strip_ojs_profile_table_noise,
    is_focus_scope_noise,
    canonical_journal_key,
    synthesize_focus_scope,
    APC_UNKNOWN,
)

console = Console()

RE_SINTA_RANK = re.compile(r"S(\d)")
RE_P_ISSN = re.compile(r"P-ISSN\s*:\s*([0-9a-zA-Z-]+)", re.I)
RE_E_ISSN = re.compile(r"E-ISSN\s*:\s*([0-9a-zA-Z-]+)", re.I)
RE_SUBJECT = re.compile(r"Subject Area\s*:\s*([^|]+)", re.I)


def _is_cloudflare_blocked(html_text: str, status_code: int = 200) -> bool:
    """
    Mengecek apakah respon web merupakan halaman blokir / tantangan Cloudflare sesungguhnya.
    Membersihkan mention CDN seperti cdnjs.cloudflare.com agar tidak terjadi false positive.
    """
    if status_code in (403, 503):
        return True
    lower = html_text.lower()
    clean_text = re.sub(r"https?://\S*cloudflare\.com", "", lower)
    cf_indicators = [
        "just a moment",
        "attention required! | cloudflare",
        "cf-browser-verification",
        "challenge-running",
        "turnstile",
        "enable javascript and cookies to continue",
        "cloudflare ray id",
    ]
    return any(indicator in clean_text for indicator in cf_indicators)


def resolve_sinta_profile_meta(session: requests.Session, profile_url: str) -> dict:
    """
    SECONDARY REQUEST KE PROFIL SINTA (v1.3.4):
    Mengambil URL OJS resmi dari tombol 'Website' / 'Editor URL', nomor ISSN, E-ISSN,
    dan Bidang Ilmu (Subject Area) pada halaman profil jurnal SINTA.
    """
    meta = {
        "ojs_url": "",
        "issn": INFO_NOT_FOUND,
        "e_issn": INFO_NOT_FOUND,
        "subject": INFO_NOT_FOUND,
        "publisher": "",
        "email": INFO_NOT_FOUND,
        "telepon": INFO_NOT_FOUND,
    }
    if not profile_url or not profile_url.startswith("http"):
        return meta
    try:
        r = session.get(profile_url, timeout=HTTP_TIMEOUT, verify=False, allow_redirects=True)
        if r.status_code == 200:
            soup = BeautifulSoup(r.text, "lxml")
            garuda_link = ""
            for a in soup.find_all("a", href=True):
                txt = a.get_text(strip=True).lower()
                h = a["href"].strip()
                if h.startswith("//"):
                    h = f"https:{h}"
                if txt in ("website", "editor url", "home page", "portal", "ojs") and "kemdiktisaintek" not in h:
                    if not h.startswith("http"):
                        h = f"https://{h}"
                    meta["ojs_url"] = cache_engine.normalize_ojs_root_url(h)
                    break
                if "garuda.kemdiktisaintek.go.id/journal/view" in h:
                    garuda_link = h

            # Ekstraksi ISSN / E-ISSN / Subject dari teks profil SINTA
            prof_text = soup.get_text(" ", strip=True)
            m_p = RE_P_ISSN.search(prof_text)
            if m_p:
                meta["issn"] = normalize_issn(m_p.group(1))
            m_e = RE_E_ISSN.search(prof_text)
            if m_e:
                meta["e_issn"] = normalize_issn(m_e.group(1))
            m_s = RE_SUBJECT.search(prof_text)
            if m_s:
                meta["subject"] = clean_and_merge_subjects(m_s.group(1))

            # Fallback melalui link portal Garuda yang tertera di profil SINTA
            if not meta["ojs_url"] and garuda_link:
                try:
                    r_gar = session.get(garuda_link, timeout=HTTP_TIMEOUT, verify=False, allow_redirects=True)
                    if r_gar.status_code == 200:
                        soup_gar = BeautifulSoup(r_gar.text, "lxml")
                        for ga in soup_gar.find_all("a", href=True):
                            g_txt = ga.get_text(strip=True).lower()
                            g_h = ga["href"].strip()
                            if g_txt == "website" and g_h.startswith("http") and "kemdiktisaintek" not in g_h:
                                meta["ojs_url"] = cache_engine.normalize_ojs_root_url(g_h)
                                break
                except Exception:
                    pass
    except Exception:
        pass
    return meta


def resolve_sinta_ojs_url(session: requests.Session, profile_url: str) -> str:
    """Wrapper kompatibilitas untuk mengambil URL OJS resmi dari tombol profil SINTA."""
    return resolve_sinta_profile_meta(session, profile_url).get("ojs_url", "")


def _process_deep_sinta_entry(session: requests.Session, entry: JournalEntry, stop_event: Optional[threading.Event] = None) -> JournalEntry:
    """
    Worker untuk menuntaskan resolusi URL OJS dan deep detail extraction secara paralel (v1.6.0).
    Dilengkapi Fast-Fail Circuit Breaker, Stop Event short-circuiting, dan Multi-layered Exception Shield.
    """
    if stop_event and stop_event.is_set():
        return entry

    try:
        # 1. Jika link_ojs belum ditemukan atau masih mengarah ke SINTA, ambil dari profil SINTA
        if (not entry.link_ojs or "kemdiktisaintek.go.id" in entry.link_ojs) and entry.sinta_profile_url:
            p_meta = resolve_sinta_profile_meta(session, entry.sinta_profile_url)
            if p_meta.get("ojs_url"):
                entry.link_ojs = p_meta["ojs_url"]
            if entry.issn == INFO_NOT_FOUND and p_meta.get("issn") and p_meta.get("issn") != INFO_NOT_FOUND:
                entry.issn = p_meta["issn"]
            if entry.e_issn == INFO_NOT_FOUND and p_meta.get("e_issn") and p_meta.get("e_issn") != INFO_NOT_FOUND:
                entry.e_issn = p_meta["e_issn"]
            if p_meta.get("subject") and p_meta.get("subject") != INFO_NOT_FOUND:
                entry.bidang = clean_and_merge_subjects(entry.bidang, p_meta["subject"])

        # 2. Jika link OJS valid dan tidak mengarah ke SINTA, lakukan Quad-Pass Deep Scraping
        if entry.link_ojs and "kemdiktisaintek.go.id" not in entry.link_ojs and entry.link_ojs.startswith("http"):
            # Fast-Fail Circuit Breaker: Uji ketersediaan domain induk sebelum crawling sub-jalur
            if not check_ojs_domain_reachable(session, entry.link_ojs):
                entry.apc_status = DEAD_LINK_TEXT
                entry.frekuensi = DEAD_LINK_TEXT
                entry.email = INFO_NOT_FOUND
                entry.telepon = INFO_NOT_FOUND
                entry.status_server = "Non-Aktif"
                entry.transparansi_kontak = "Tidak Transparan"
                if entry.garuda_profile_url:
                    g_meta = resolve_garuda_profile_meta(session, entry.garuda_profile_url)
                    if g_meta.get("focus_scope"):
                        entry.focus_scope = g_meta["focus_scope"]
                if not entry.focus_scope or entry.focus_scope == INFO_NOT_FOUND:
                    entry.focus_scope = build_tier3_scope_fallback(entry)
                return entry

            res = extract_ojs_deep_info(session, entry.link_ojs, stop_event=stop_event)
            if res.get("official_title") and (len(entry.nama_jurnal) > 55 or entry.nama_jurnal.isupper()):
                entry.nama_jurnal = res["official_title"]

            entry.email = clean_contact_list(res.get("email", INFO_NOT_FOUND), contact_type="email", ojs_url=entry.link_ojs)
            entry.telepon = clean_contact_list(res.get("telepon", INFO_NOT_FOUND), contact_type="phone")
            entry.frekuensi = res.get("frekuensi", INFO_NOT_FOUND)
            entry.apc_status = res.get("apc_status", APC_UNKNOWN)
            entry.apc_amount = res.get("apc_amount", None)
            if res.get("biaya_submit"):
                entry.biaya_submit = res["biaya_submit"]

            # Smart Fallback Biaya Submit -> Biaya Publikasi (v1.6.2)
            if (entry.biaya_submit and entry.biaya_submit > 50000) and (entry.biaya_publikasi is None or entry.biaya_publikasi == 0):
                entry.biaya_publikasi = entry.biaya_submit
                if entry.apc_status in (APC_UNKNOWN, INFO_NOT_FOUND, ""):
                    entry.apc_status = f"Rp {entry.biaya_publikasi:,}".replace(",", ".")

            if entry.apc_status == "Gratis (Rp 0)" or entry.apc_amount == 0:
                entry.biaya_publikasi = 0
            elif entry.apc_amount is not None and entry.apc_amount > 0:
                entry.biaya_publikasi = entry.apc_amount
            elif (entry.biaya_submit and entry.biaya_submit > 50000) and (entry.biaya_publikasi is None or entry.biaya_publikasi == 0):
                entry.biaya_publikasi = entry.biaya_submit

            entry.focus_scope = res.get("focus_scope", INFO_NOT_FOUND)
            entry.status_server = res.get("status_server", "Aktif (HTTP)")
            entry.transparansi_kontak = res.get("transparansi_kontak", "Tidak Transparan")
            if entry.issn == INFO_NOT_FOUND and res.get("issn") and res.get("issn") != INFO_NOT_FOUND:
                entry.issn = res.get("issn")
            if entry.e_issn == INFO_NOT_FOUND and res.get("e_issn") and res.get("e_issn") != INFO_NOT_FOUND:
                entry.e_issn = res.get("e_issn")
            if res.get("bidang") and res.get("bidang") != INFO_NOT_FOUND:
                entry.bidang = clean_and_merge_subjects(entry.bidang, res.get("bidang"))
            if not entry.penerbit and res.get("publisher"):
                entry.penerbit = res.get("publisher")
            if entry.link_ojs != res.get("ojs_url", entry.link_ojs) and res.get("ojs_url"):
                entry.link_ojs = res.get("ojs_url", entry.link_ojs)
            entry.raw_about = res.get("raw_about", "")
            entry.raw_submissions = res.get("raw_submissions", "")
            entry.bulan_terbit = res.get("bulan_terbit", [])
            # Jika OJS tidak menyediakan Focus & Scope, periksa profil Garuda
            if (not entry.focus_scope or entry.focus_scope == INFO_NOT_FOUND or "tidak ditemukan" in str(entry.focus_scope).lower() or is_focus_scope_noise(entry.focus_scope)) and entry.garuda_profile_url:
                g_meta = resolve_garuda_profile_meta(session, entry.garuda_profile_url)
                if g_meta.get("focus_scope") and not is_focus_scope_noise(g_meta["focus_scope"]):
                    entry.focus_scope = g_meta["focus_scope"]

        # Fallback jika OJS tidak bisa dihubungi atau profil Garuda tersedia
        if (not entry.focus_scope or entry.focus_scope == INFO_NOT_FOUND or "tidak ditemukan" in str(entry.focus_scope).lower() or is_focus_scope_noise(entry.focus_scope)) and entry.garuda_profile_url:
            g_meta = resolve_garuda_profile_meta(session, entry.garuda_profile_url)
            if g_meta.get("focus_scope") and not is_focus_scope_noise(g_meta["focus_scope"]):
                entry.focus_scope = g_meta["focus_scope"]

        # Zero-Empty Focus & Scope Synthesizer & Profile Table Noise Stripper (v1.6.2)
        if not entry.focus_scope or entry.focus_scope == INFO_NOT_FOUND or "tidak ditemukan" in str(entry.focus_scope).lower() or is_focus_scope_noise(entry.focus_scope):
            synth_text, synth_src = synthesize_focus_scope(entry)
            entry.focus_scope = synth_text
            entry.sumber_focus = synth_src
        else:
            cleaned_scope = strip_ojs_profile_table_noise(entry.focus_scope)
            if is_focus_scope_noise(cleaned_scope):
                synth_text, synth_src = synthesize_focus_scope(entry)
                entry.focus_scope = synth_text
                entry.sumber_focus = synth_src
            else:
                entry.focus_scope = cleaned_scope
                if not getattr(entry, "sumber_focus", ""):
                    entry.sumber_focus = "Regex"
    except Exception:
        pass

    return entry


def _parse_sinta_page(soup: BeautifulSoup, session: requests.Session, deep_detail: bool = False) -> list[JournalEntry]:
    """
    Parse halaman daftar jurnal dari SINTA Kemdiktisaintek dengan standardisasi v1.3.4.
    """
    journals: list[JournalEntry] = []
    items = soup.select("div.list-item") or soup.select("div.card-journal") or soup.select("div.journal-item")
    deep_tasks: list[JournalEntry] = []

    for item in items:
        # 1. Judul Jurnal & Link Profil SINTA
        title_el = item.select_one("div.affil-name a") or item.select_one(".affil-name")
        if not title_el:
            continue
        nama_jurnal = title_el.get_text(" ", strip=True)
        if not nama_jurnal:
            continue

        sinta_url = title_el.get("href", "").strip() if title_el.name == "a" else ""
        if sinta_url.startswith("/"):
            sinta_url = f"https://sinta.kemdiktisaintek.go.id{sinta_url}"

        # 2. Penerbit / Institusi
        pub_el = item.select_one("div.affil-loc a") or item.select_one(".affil-loc")
        penerbit = pub_el.get_text(" ", strip=True) if pub_el else ""

        # 3. Ekstraksi Langsung Tautan OJS dan Profil Garuda dari elemen item pencarian
        ojs_link = ""
        garuda_link = ""
        for a in item.select("a[href]"):
            txt = a.get_text(strip=True).lower()
            href = a.get("href", "").strip()
            if href.startswith("//"):
                href = f"https:{href}"
            if "garuda.kemdiktisaintek.go.id/journal/view" in href:
                garuda_link = href
            if any(t in txt for t in ("website", "editor url", "ojs", "portal", "home page")) and "kemdiktisaintek" not in href and href.startswith("http"):
                ojs_link = href

        # Jika belum ditemukan, periksa div.affil-abbrev a
        if not ojs_link:
            for a in item.select("div.affil-abbrev a[href]"):
                href = a.get("href", "").strip()
                if "kemdiktisaintek" not in href and ("http" in href):
                    ojs_link = href
                    break

        # 4. Peringkat SINTA (S1 sampai S6)
        rank = INFO_NOT_FOUND
        rank_elem = item.select_one("span.num-stat.accredited") or item.select_one(".accredited")
        if rank_elem:
            m = RE_SINTA_RANK.search(rank_elem.get_text())
            if m:
                rank = f"Sinta {m.group(1)}"

        # 5. Metadata Tambahan: ISSN, E-ISSN, Bidang Keilmuan
        issn, e_issn, subject = INFO_NOT_FOUND, INFO_NOT_FOUND, INFO_NOT_FOUND
        prof_id = item.select_one("div.profile-id")
        if prof_id:
            text_id = prof_id.get_text(" ", strip=True)
            m_p = RE_P_ISSN.search(text_id)
            if m_p:
                issn = normalize_issn(m_p.group(1))
            m_e = RE_E_ISSN.search(text_id)
            if m_e:
                e_issn = normalize_issn(m_e.group(1))
            m_s = RE_SUBJECT.search(text_id)
            if m_s:
                subject = clean_and_merge_subjects(m_s.group(1))

        entry = JournalEntry(
            nama_jurnal=sanitize_text(nama_jurnal),
            penerbit=sanitize_text(penerbit),
            link_ojs=ojs_link,
            sinta_profile_url=sinta_url,
            garuda_profile_url=garuda_link,
            peringkat_sinta=sanitize_text(rank),
            issn=normalize_issn(issn),
            e_issn=normalize_issn(e_issn),
            bidang=clean_and_merge_subjects(subject),
            apc_status=INFO_NOT_FOUND,
            apc_amount=None,
            email=INFO_NOT_FOUND,
            telepon=INFO_NOT_FOUND,
            frekuensi=INFO_NOT_FOUND,
            sumber="SINTA",
        )
        journals.append(entry)
        deep_tasks.append(entry)

    # Pastikan seluruh jurnal memiliki link OJS valid via quick resolve profil SINTA jika belum terisi
    if deep_tasks:
        try:
            with ThreadPoolExecutor(max_workers=min(len(deep_tasks), 16)) as executor:
                def _quick_resolve(e):
                    if (not e.link_ojs or "kemdiktisaintek.go.id" in e.link_ojs) and e.sinta_profile_url:
                        res_url = resolve_sinta_ojs_url(session, e.sinta_profile_url)
                        if res_url:
                            e.link_ojs = res_url
                futures = [executor.submit(_quick_resolve, entry) for entry in deep_tasks]
                for f in futures:
                    f.result()
        except Exception:
            pass

    return journals
def _scrape_sinta_single_keyword(
    keyword: str,
    sinta_ranks: Optional[list[int]] = None,
    deep_detail: bool = False,
    max_results: int = MAX_RESULTS,
    turbo_mode: bool = True,
    session: Optional[requests.Session] = None,
    free_only: bool = False,
    include_unknown_apc: bool = True,
    allow_selenium_fallback: bool = False,
    log_cb=None,
) -> list[JournalEntry]:
    """
    Scraping satu kata kunci Direktori SINTA via High-Speed Concurrent HTTP requests (v1.3.8).
    """
    if session is None:
        session = get_high_capacity_session()
    all_journals: list[JournalEntry] = []
    active_workers = TURBO_SCRAPE_WORKERS if turbo_mode else NORMAL_SCRAPE_WORKERS

    console.print(f"\n[bold magenta]🔍 Memulai scraping Direktori SINTA (v1.3.8)...[/bold magenta]")
    console.print(f"[dim]   Keyword      : {keyword}[/dim]")
    console.print(f"[dim]   Filter SINTA : Sinta {sinta_ranks if sinta_ranks else 'semua'}[/dim]")
    console.print(f"[dim]   Target Hasil : {max_results} jurnal | Mode Detail OJS: {'Aktif (Quad-Pass)' if deep_detail else 'Mati (Mode Cepat)'}[/dim]")
    console.print(f"[dim]   Hardware Concurrency: {active_workers} workers {'(Turbo Mode)' if turbo_mode else ''}[/dim]")

    # Pass 1: Uji probe halaman pertama untuk deteksi proteksi Cloudflare
    console.print(f"[yellow]📄 SINTA Halaman 1[/yellow] → {SINTA_JOURNAL_URL}?q={keyword}&page=1")
    try:
        req_headers = get_browser_headers(referer="https://sinta.kemdiktisaintek.go.id/")
        session.headers.update(req_headers)
        r1 = session.get(SINTA_JOURNAL_URL, params={"q": keyword, "page": 1}, headers=req_headers, timeout=HTTP_TIMEOUT, verify=False)
        if _is_cloudflare_blocked(r1.text, r1.status_code):
            if allow_selenium_fallback:
                console.print("[bold red]⚠️ SINTA memblokir request (Cloudflare Challenge Detected).[/bold red]")
                console.print("[dim]   Mengalihkan secara otomatis ke browser automation / fallback...[/dim]")
                return scrape_sinta_selenium(keyword, sinta_ranks, deep_detail=deep_detail, max_results=max_results, turbo_mode=turbo_mode, free_only=free_only, include_unknown_apc=include_unknown_apc)
            diag_msg = "⚠️ Portal SINTA membatasi akses HTTP standar. Mengandalkan Portal Garuda atau aktifkan 'Gunakan Selenium' di Pengaturan Lengkap."
            console.print(f"[bold yellow]{diag_msg}[/bold yellow]")
            if log_cb:
                log_cb(diag_msg)
            return []
        r1.raise_for_status()
    except requests.exceptions.RequestException as e:
        console.print(f"[red]✗ Koneksi SINTA gagal: {e}[/red]")
        return []

    soup1 = BeautifulSoup(r1.text, "lxml")
    p1_journals = _parse_sinta_page(soup1, session, deep_detail=False)
    if not p1_journals:
        console.print("[dim]   Tidak ada jurnal ditemukan untuk kata kunci ini.[/dim]")
        return []

    console.print(f"[green]   ✓ Ditemukan {len(p1_journals)} jurnal di halaman 1[/green]")
    all_journals.extend(p1_journals)

    rank_labels = [f"S{r}" for r in sinta_ranks] + [f"Sinta {r}" for r in sinta_ranks] if sinta_ranks else []
    matching_count = len([j for j in all_journals if any(r in j.peringkat_sinta for r in rank_labels)]) if sinta_ranks else len(all_journals)

    if matching_count < max_results:
        # Taksir halaman yang dibutuhkan berdasarkan target kuota
        if sinta_ranks:
            pages_to_scan = min(MAX_PAGES_LIMIT, max(10, int(max_results * 0.45) + 6))
        else:
            pages_to_scan = min(MAX_PAGES_LIMIT, max(5, ((max_results + 9) // 10) + 2))

        batch_size = 12
        stop_scraping = False

        for batch_start in range(2, pages_to_scan + 1, batch_size):
            if stop_scraping:
                break
            batch_end = min(pages_to_scan + 1, batch_start + batch_size)
            page_numbers = list(range(batch_start, batch_end))
            console.print(f"[yellow]⚡ Memindai SINTA Halaman {batch_start}..{batch_end - 1} secara paralel (Direct Concurrent HTTP)...[/yellow]")

            def _fetch_page_worker(p: int) -> tuple[int, Optional[str], bool]:
                try:
                    resp = session.get(SINTA_JOURNAL_URL, params={"q": keyword, "page": p}, timeout=HTTP_TIMEOUT, verify=False)
                    if resp.status_code == 200:
                        is_cf = _is_cloudflare_blocked(resp.text, resp.status_code)
                        return p, resp.text, is_cf
                except Exception:
                    pass
                return p, None, False

            page_results: dict[int, Optional[str]] = {}
            with ThreadPoolExecutor(max_workers=min(len(page_numbers), 16)) as executor:
                future_to_page = {executor.submit(_fetch_page_worker, p): p for p in page_numbers}
                for fut in future_to_page:
                    p_num, html_txt, is_cf = fut.result()
                    if is_cf:
                        console.print("[bold red]⚠️ SINTA memicu Cloudflare saat pemindaian paralel. Beralih ke browser automation...[/bold red]")
                        return scrape_sinta_selenium(keyword, sinta_ranks, deep_detail=deep_detail, max_results=max_results, turbo_mode=turbo_mode, free_only=free_only, include_unknown_apc=include_unknown_apc)
                    page_results[p_num] = html_txt

            # Parse berurutan sesuai nomor halaman
            for p in sorted(page_results.keys()):
                h_text = page_results[p]
                if not h_text:
                    continue
                p_soup = BeautifulSoup(h_text, "lxml")
                p_journals = _parse_sinta_page(p_soup, session, deep_detail=False)
                if not p_journals:
                    console.print(f"[dim]   Pencarian SINTA berakhir di halaman {p}.[/dim]")
                    stop_scraping = True
                    break

                console.print(f"[green]   ✓ Ditemukan {len(p_journals)} jurnal di halaman {p}[/green]")
                all_journals.extend(p_journals)

                # High-Yield Candidate Pool (v1.6.1): Over-fetch minimal 3x atau 60 jurnal
                pool_target = max(max_results * 3, 60)
                if sinta_ranks:
                    cur_matching = [j for j in all_journals if any(r in j.peringkat_sinta for r in rank_labels)]
                    if len(cur_matching) >= max(max_results * 2, 40) or len(all_journals) >= pool_target:
                        stop_scraping = True
                        break
                else:
                    if len(all_journals) >= pool_target:
                        stop_scraping = True
                        break

    # Filter peringkat SINTA
    if sinta_ranks and all_journals:
        filtered = [j for j in all_journals if any(r in j.peringkat_sinta for r in rank_labels)]
    else:
        filtered = all_journals

    # Pertahankan candidate pool yang melimpah untuk deep scrape & filter biaya hilir
    pool_limit = max(max_results * 2, 40) if deep_detail else max_results
    if len(filtered) > pool_limit:
        filtered = filtered[:pool_limit]

    console.print(f"\n[bold]📊 Hasil SINTA terfilter: {len(filtered)} jurnal[/bold]")

    # Deep Scraping paralel dengan Target Reach Short-Circuiting (v1.3.6)
    if filtered and deep_detail:
        console.print(f"[bold cyan]🔍 Menjalankan Quad-Pass Deep Scraping SINTA ({len(filtered)} jurnal) [{active_workers} workers]...[/bold cyan]")
        stop_event = threading.Event()
        matched_lock = threading.Lock()
        matched_count = 0
        deep_session = get_high_capacity_session(is_deep_scrape=True)

        try:
            with ThreadPoolExecutor(max_workers=active_workers) as executor:
                def _do_sinta_deep(entry: JournalEntry):
                    nonlocal matched_count
                    if stop_event.is_set():
                        return
                    try:
                        _process_deep_sinta_entry(deep_session, entry, stop_event=stop_event)
                        with matched_lock:
                            if free_only:
                                apc_low = entry.apc_status.lower()
                                is_free = (entry.apc_amount == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("free" in apc_low)
                                is_unk = (entry.apc_amount is None) or (entry.apc_status in (INFO_NOT_FOUND, APC_UNKNOWN))
                                if is_free or (is_unk and include_unknown_apc):
                                    matched_count += 1
                            else:
                                matched_count += 1
                            if matched_count >= max_results:
                                stop_event.set()
                    except Exception:
                        pass

                futures = [executor.submit(_do_sinta_deep, entry) for entry in filtered]
                for f in futures:
                    f.result()
        except Exception:
            pass

    # Selalu hitung keyword match score untuk setiap jurnal agar kolom Kesesuaian & Skor terisi (v1.6.1)
    try:
        from doc_reader import calculate_keyword_match_score
        for entry in filtered:
            if entry.match_score <= 0 or not entry.kesesuaian_naskah:
                sc = calculate_keyword_match_score(keyword, entry)
                entry.match_score = sc.get("score", 40)
                entry.kesesuaian_naskah = sc.get("level", "Cukup Sesuai")
    except Exception:
        pass

    return filtered


def scrape_sinta(
    keyword: str,
    sinta_ranks: Optional[list[int]] = None,
    deep_detail: bool = False,
    max_results: int = MAX_RESULTS,
    turbo_mode: bool = True,
    free_only: bool = False,
    include_unknown_apc: bool = True,
    allow_selenium_fallback: bool = False,
    log_cb=None,
) -> list[JournalEntry]:
    """
    Scraping Direktori SINTA dengan dukungan multi-keyword adaptif dan short-circuiting (v1.3.6).
    """
    keywords = parse_search_keywords(keyword)
    session = get_high_capacity_session()

    if len(keywords) > 1:
        console.print(f"\n[bold magenta]🔍 Deteksi multi-keyword adaptif SINTA ({len(keywords)} topik): {', '.join(keywords)}[/bold magenta]")
        all_res: list[JournalEntry] = []
        buffer_ratio = 1.35
        target_per_kw = max(20, int((max_results * buffer_ratio) / len(keywords)) + 15)
        for kw in keywords:
            current_unique = len(merge_cross_source_entries(all_res))
            if current_unique >= max_results:
                break
            remaining_needed = max_results - current_unique
            fetch_limit = min(max_results, max(target_per_kw, remaining_needed + 15))

            kw_journals = _scrape_sinta_single_keyword(
                kw,
                sinta_ranks=sinta_ranks,
                deep_detail=deep_detail,
                max_results=fetch_limit,
                turbo_mode=turbo_mode,
                session=session,
                free_only=free_only,
                include_unknown_apc=include_unknown_apc,
                allow_selenium_fallback=allow_selenium_fallback,
                log_cb=log_cb,
            )
            all_res.extend(kw_journals)
            all_res = merge_cross_source_entries(all_res)
            if len(all_res) >= max_results:
                break
        merged = merge_cross_source_entries(all_res)
        return merged[:max_results]

    return _scrape_sinta_single_keyword(
        keyword,
        sinta_ranks=sinta_ranks,
        deep_detail=deep_detail,
        max_results=max_results,
        turbo_mode=turbo_mode,
        session=session,
        free_only=free_only,
        include_unknown_apc=include_unknown_apc,
        allow_selenium_fallback=allow_selenium_fallback,
        log_cb=log_cb,
    )


def scrape_sinta_selenium(
    keyword: str,
    sinta_ranks: Optional[list[int]] = None,
    deep_detail: bool = False,
    max_results: int = MAX_RESULTS,
    turbo_mode: bool = True,
    free_only: bool = False,
    include_unknown_apc: bool = True,
) -> list[JournalEntry]:
    """
    Fallback browser automation untuk SINTA dengan akselerasi:
    - page_load_strategy = 'eager'
    - Blokir muatan gambar, stylesheet, dan font untuk speedup maksimal.
    """
    driver = None
    all_journals: list[JournalEntry] = []
    active_workers = TURBO_SCRAPE_WORKERS if turbo_mode else NORMAL_SCRAPE_WORKERS

    console.print("\n[bold magenta]🤖 Menjalankan Browser Automation Anti-Cloudflare (Akselerasi Eager Mode)...[/bold magenta]")

    try:
        import undetected_chromedriver as uc
        options = uc.ChromeOptions()
        options.add_argument("--no-sandbox")
        options.add_argument("--disable-dev-shm-usage")
        options.add_argument(f"--user-agent={HEADERS['User-Agent']}")
        options.page_load_strategy = "eager"

        # Blokir muatan media gambar dan font untuk akselerasi
        prefs = {
            "profile.managed_default_content_settings.images": 2,
            "profile.default_content_setting_values.notifications": 2,
            "profile.managed_default_content_settings.stylesheets": 2,
        }
        options.add_experimental_option("prefs", prefs)
        driver = uc.Chrome(options=options, version_main=None, use_subprocess=True)
    except Exception as e:
        console.print(f"[dim]   Info: undetected-chromedriver dialihkan ke selenium standar ({e}).[/dim]")
        try:
            from selenium import webdriver
            from selenium.webdriver.chrome.options import Options
            from selenium.webdriver.chrome.service import Service
            from webdriver_manager.chrome import ChromeDriverManager

            sel_options = Options()
            sel_options.add_argument("--no-sandbox")
            sel_options.add_argument("--disable-dev-shm-usage")
            sel_options.add_argument(f"--user-agent={HEADERS['User-Agent']}")
            sel_options.add_experimental_option("excludeSwitches", ["enable-automation"])
            sel_options.add_experimental_option("useAutomationExtension", False)
            sel_options.page_load_strategy = "eager"

            prefs = {"profile.managed_default_content_settings.images": 2}
            sel_options.add_experimental_option("prefs", prefs)

            service = Service(ChromeDriverManager().install())
            driver = webdriver.Chrome(service=service, options=sel_options)
        except Exception as sel_err:
            console.print(f"[red]✗ Gagal menginisialisasi browser: {sel_err}[/red]")
            return []

    pages_to_scan = min(MAX_PAGES_LIMIT, max(15, (max_results // 4) + 10))
    session = get_high_capacity_session()

    try:
        for page in range(1, pages_to_scan + 1):
            url = f"{SINTA_JOURNAL_URL}?q={keyword}&page={page}"
            console.print(f"[yellow]📄 Selenium → {url}[/yellow]")
            driver.get(url)

            time.sleep(2.0)
            page_source = driver.page_source
            if _is_cloudflare_blocked(page_source):
                console.print(
                    "[bold red]⚠️ SINTA memblokir request (Cloudflare Challenge Detected pada Browser Automation).[/bold red]"
                )
                time.sleep(4.0)

            soup = BeautifulSoup(page_source, "lxml")
            page_journals = _parse_sinta_page(soup, session, deep_detail=False)

            if not page_journals:
                console.print(f"[dim]   Tidak ada data di halaman {page}.[/dim]")
                break

            console.print(f"[green]   ✓ {len(page_journals)} jurnal ditemukan via browser automation[/green]")
            all_journals.extend(page_journals)

            if sinta_ranks:
                rank_labels = [f"S{r}" for r in sinta_ranks] + [f"Sinta {r}" for r in sinta_ranks]
                matching_so_far = [j for j in all_journals if any(r in j.peringkat_sinta for r in rank_labels)]
                if len(matching_so_far) >= max_results:
                    break
            else:
                if len(all_journals) >= max_results:
                    all_journals = all_journals[:max_results]
                    break

    finally:
        if driver:
            try:
                driver.close()
            except Exception:
                pass
            try:
                driver.quit()
            except Exception:
                pass

    if sinta_ranks and all_journals:
        rank_labels = [f"S{r}" for r in sinta_ranks] + [f"Sinta {r}" for r in sinta_ranks]
        filtered = [
            j for j in all_journals
            if any(r in j.peringkat_sinta for r in rank_labels)
        ]
    else:
        filtered = all_journals

    # Pertahankan candidate pool yang melimpah untuk deep scrape & filter biaya hilir
    pool_limit = max(max_results * 2, 40) if deep_detail else max_results
    if len(filtered) > pool_limit:
        filtered = filtered[:pool_limit]

    if filtered and deep_detail:
        console.print(f"[bold cyan]🔍 Menjalankan Quad-Pass Deep Scraping Selenium ({len(filtered)} jurnal) [{active_workers} workers]...[/bold cyan]")
        stop_event = threading.Event()
        matched_lock = threading.Lock()
        matched_count = 0
        deep_session = get_high_capacity_session(is_deep_scrape=True)

        try:
            with ThreadPoolExecutor(max_workers=active_workers) as executor:
                def _do_sel_deep(entry: JournalEntry):
                    nonlocal matched_count
                    if stop_event.is_set():
                        return
                    try:
                        _process_deep_sinta_entry(deep_session, entry, stop_event=stop_event)
                        with matched_lock:
                            if free_only:
                                apc_low = entry.apc_status.lower()
                                is_free = (entry.apc_amount == 0) or ("gratis" in apc_low) or ("rp 0" in apc_low) or ("free" in apc_low)
                                is_unk = (entry.apc_amount is None) or (entry.apc_status in (INFO_NOT_FOUND, APC_UNKNOWN))
                                if is_free or (is_unk and include_unknown_apc):
                                    matched_count += 1
                            else:
                                matched_count += 1
                            if matched_count >= max_results:
                                stop_event.set()
                    except Exception:
                        pass

                futures = [executor.submit(_do_sel_deep, entry) for entry in filtered]
                for f in futures:
                    f.result()
        except Exception:
            pass

    # Selalu hitung keyword match score untuk setiap jurnal agar kolom Kesesuaian & Skor terisi (v1.6.1)
    try:
        from doc_reader import calculate_keyword_match_score
        for entry in filtered:
            if entry.match_score <= 0 or not entry.kesesuaian_naskah:
                sc = calculate_keyword_match_score(keyword, entry)
                entry.match_score = sc.get("score", 40)
                entry.kesesuaian_naskah = sc.get("level", "Cukup Sesuai")
    except Exception:
        pass

    return filtered


