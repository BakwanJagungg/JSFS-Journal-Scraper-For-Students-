#!/usr/bin/env python3
# =============================================================================
# JSFS — Journal Scraper For Students
# jsfs.py — Entry point utama CLI
#
# Cara pakai:
#   py jsfs.py --keyword "bisnis" --sinta 3 4 5
#   py jsfs.py --keyword "e-commerce" --sinta 3 4 --format md csv excel
#   py jsfs.py --keyword "consumer behavior" --source sinta --selenium
#   py jsfs.py --keyword "manajemen" --sinta 5 6 --format md
# =============================================================================

import argparse
import sys
from rich.console import Console
from rich.panel import Panel
from rich.text import Text

from config import DEFAULT_SINTA_RANKS
from scraper_garuda import scrape_garuda, JournalEntry, merge_cross_source_entries
from scraper_sinta import scrape_sinta, scrape_sinta_selenium
from exporter import export_markdown, export_csv, export_excel, print_summary_table

console = Console()


def print_banner():
    """Tampilkan banner JSFS."""
    banner = Text()
    banner.append("  ██╗███████╗███████╗███████╗\n", style="bold cyan")
    banner.append("  ██║██╔════╝██╔════╝██╔════╝\n", style="bold cyan")
    banner.append("  ██║███████╗█████╗  ███████╗\n", style="bold cyan")
    banner.append("  ██║╚════██║██╔══╝  ╚════██║\n", style="bold cyan")
    banner.append("  ██║███████║██║     ███████║\n", style="bold cyan")
    banner.append("  ╚═╝╚══════╝╚═╝     ╚══════╝\n", style="bold cyan")
    banner.append("\n")
    banner.append("  Journal Scraper For Students\n", style="bold white")
    banner.append("  Scraping Garuda & SINTA → Markdown/CSV/Excel\n", style="dim")
    banner.append("  by Rakyat Biasa | v1.5.1 (Stability, Anti-Blackout & UX Presets)\n", style="dim")

    console.print(Panel(banner, border_style="cyan", expand=False))


def parse_args():
    parser = argparse.ArgumentParser(
        prog="jsfs",
        description="JSFS — Journal Scraper For Students. Cari jurnal Garuda/SINTA otomatis.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Contoh penggunaan:
  py jsfs.py --keyword "bisnis"
  py jsfs.py --keyword "e-commerce" --sinta 3 4 5 6
  py jsfs.py --keyword "consumer behavior" --source garuda sinta --format md excel
  py jsfs.py --keyword "manajemen" --sinta 5 6 --selenium
        """,
    )

    parser.add_argument(
        "--keyword", "-k",
        type=str,
        required=False,
        help="Kata kunci pencarian jurnal (mis: 'bisnis', 'e-commerce')",
    )
    parser.add_argument(
        "--sinta", "-s",
        type=int,
        nargs="+",
        default=DEFAULT_SINTA_RANKS,
        metavar="RANK",
        help=f"Filter peringkat SINTA (default: {DEFAULT_SINTA_RANKS}). Contoh: --sinta 3 4 5",
    )
    parser.add_argument(
        "--source",
        choices=["garuda", "sinta", "both"],
        default="garuda",
        help="Sumber scraping: garuda | sinta | both (default: garuda)",
    )
    parser.add_argument(
        "--format", "-f",
        nargs="+",
        choices=["md", "csv", "excel"],
        default=["md"],
        dest="formats",
        help="Format output: md | csv | excel (default: md)",
    )
    parser.add_argument(
        "--selenium",
        action="store_true",
        help="Gunakan Selenium headless Chrome (untuk bypass JS/Cloudflare)",
    )
    parser.add_argument(
        "--no-filter",
        action="store_true",
        help="Tampilkan semua peringkat SINTA tanpa filter",
    )
    parser.add_argument(
        "--interactive", "-i",
        action="store_true",
        help="Mode interaktif: isi keyword dan opsi lewat prompt",
    )
    parser.add_argument(
        "--deep", "-d",
        action="store_true",
        help="Mode Deep Scraping OJS (APC, Kontak, Frekuensi)",
    )
    parser.add_argument(
        "--limit", "-l",
        type=int,
        default=50,
        help="Batas target jumlah jurnal (default: 50)",
    )
    parser.add_argument(
        "--turbo",
        action="store_true",
        default=True,
        help="Hardware Turbo Mode: paralelisme throughput tinggi (default: True)",
    )
    parser.add_argument(
        "--manuscript", "-m",
        type=str,
        default=None,
        help="Path berkas naskah artikel (.docx/.pdf) untuk rekomendasi kesesuaian jurnal",
    )

    return parser.parse_args()


def interactive_mode() -> argparse.Namespace:
    """Mode interaktif untuk pengguna yang tidak familiar dengan CLI."""
    console.print("\n[bold cyan]═══ Mode Interaktif JSFS ═══[/bold cyan]\n")

    # Keyword
    keyword = ""
    while not keyword.strip():
        keyword = console.input("[bold]🔍 Masukkan keyword pencarian[/bold] (mis: bisnis, e-commerce): ").strip()
        if not keyword:
            console.print("[red]Keyword tidak boleh kosong.[/red]")

    # Sumber
    console.print("\n[bold]📡 Pilih sumber scraping:[/bold]")
    console.print("  [1] Garuda saja (default, lebih stabil)")
    console.print("  [2] SINTA saja")
    console.print("  [3] Keduanya (lebih lama)")
    src_input = console.input("Pilihan [1/2/3] (Enter = 1): ").strip()
    source_map = {"1": "garuda", "2": "sinta", "3": "both", "": "garuda"}
    source = source_map.get(src_input, "garuda")

    # Filter SINTA
    console.print("\n[bold]📊 Filter Peringkat SINTA:[/bold]")
    console.print(f"  Default: Sinta 3, 4, 5, 6")
    console.print("  Contoh input: 3 4 5  (atau Enter untuk default)")
    ranks_input = console.input("Peringkat SINTA: ").strip()
    if ranks_input:
        try:
            sinta_ranks = [int(r) for r in ranks_input.split()]
        except ValueError:
            console.print("[yellow]Input tidak valid, menggunakan default.[/yellow]")
            sinta_ranks = DEFAULT_SINTA_RANKS
    else:
        sinta_ranks = DEFAULT_SINTA_RANKS

    # Format output
    console.print("\n[bold]💾 Format output:[/bold]")
    console.print("  md    — Markdown (default)")
    console.print("  csv   — CSV")
    console.print("  excel — Excel .xlsx")
    console.print("  Bisa pilih lebih dari satu, pisah spasi (mis: md csv)")
    fmt_input = console.input("Format [md/csv/excel] (Enter = md): ").strip()
    valid_formats = {"md", "csv", "excel"}
    if fmt_input:
        formats = [f for f in fmt_input.split() if f in valid_formats]
        if not formats:
            formats = ["md"]
    else:
        formats = ["md"]

    # Selenium
    use_selenium = False
    if source in ("sinta", "both"):
        sel_input = console.input("\n[bold]🤖 Gunakan Selenium untuk bypass JS? [y/N]:[/bold] ").strip().lower()
        use_selenium = sel_input == "y"

    # Buat namespace palsu agar kompatibel dengan flow utama
    args = argparse.Namespace(
        keyword=keyword,
        sinta=sinta_ranks,
        source=source,
        formats=formats,
        selenium=use_selenium,
        no_filter=False,
        interactive=False,
    )
    return args


def run(args):
    """Jalankan pipeline scraping dan export."""
    keyword = args.keyword
    sinta_ranks = None if args.no_filter else args.sinta
    all_results: list[JournalEntry] = []

    # ── Scraping ──────────────────────────────────────────────────────────────
    deep_mode = getattr(args, "deep", False)
    limit_val = getattr(args, "limit", 50)
    turbo_val = getattr(args, "turbo", True)
    candidate_limit = max(limit_val * 3, 60)

    def _scrape_pass(kw_query: str) -> list[JournalEntry]:
        pass_results: list[JournalEntry] = []
        if args.source in ("garuda", "both"):
            garuda_results = scrape_garuda(kw_query, sinta_ranks, deep_detail=deep_mode, max_results=candidate_limit, turbo_mode=turbo_val)
            pass_results.extend(garuda_results)

        if args.source in ("sinta", "both"):
            if args.selenium:
                sinta_results = scrape_sinta_selenium(kw_query, sinta_ranks, deep_detail=deep_mode, max_results=candidate_limit, turbo_mode=turbo_val)
            else:
                sinta_results = scrape_sinta(kw_query, sinta_ranks, deep_detail=deep_mode, max_results=candidate_limit, turbo_mode=turbo_val, allow_selenium_fallback=False)
            pass_results.extend(sinta_results)
        return pass_results

    all_results.extend(_scrape_pass(keyword))

    if not all_results:
        import doc_reader
        fallback_kws = doc_reader.get_fallback_keywords(keyword)
        if fallback_kws:
            console.print(f"[yellow]⚠️ Kata kunci '{keyword}' tidak menemukan hasil. Mencoba perluasan kata kunci / koreksi ejaan...[/yellow]")
            for fb_kw in fallback_kws:
                console.print(f"[cyan]   ↻ Mencoba kata kunci alternatif: '{fb_kw}'[/cyan]")
                fb_results = _scrape_pass(fb_kw)
                if fb_results:
                    all_results.extend(fb_results)
                    break

    # Deduplikasi & Penggabungan Data Lintas Sumber (Cross-Source Fallback Merging)
    unique_results = merge_cross_source_entries(all_results)
    if len(unique_results) > limit_val:
        unique_results = unique_results[:limit_val]
    console.print(f"\n[bold]📦 Total unik: {len(unique_results)} jurnal[/bold]")

    # Zero-Empty Focus & Scope Synthesizer & Anti-Noise (v1.6.2)
    try:
        from scraper_garuda import synthesize_focus_scope, is_focus_scope_noise
        for j in unique_results:
            f_sc = getattr(j, "focus_scope", "")
            if not f_sc or f_sc == "Informasi tidak ditemukan dalam situs jurnal" or "tidak ditemukan" in str(f_sc).lower() or is_focus_scope_noise(str(f_sc)):
                synth_text, synth_src = synthesize_focus_scope(j)
                j.focus_scope = synth_text
                j.sumber_focus = synth_src
    except Exception:
        pass

    # ── Analisis Naskah & Rekomendasi Jurnal (v1.3.5) ──────────────────────────
    if getattr(args, "manuscript", None):
        import doc_reader
        console.print(f"\n[bold cyan]📄 Menganalisis naskah ilmiah: {args.manuscript}...[/bold cyan]")
        try:
            ms_meta = doc_reader.extract_manuscript_metadata(args.manuscript)
            console.print(f"   [dim]Judul   : {ms_meta.get('title')}[/dim]")
            console.print(f"   [dim]Keywords: {', '.join(ms_meta.get('keywords', []))}[/dim]")
            for j in unique_results:
                sc = doc_reader.calculate_match_score(ms_meta, j)
                j.match_score = sc["score"]
                j.kesesuaian_naskah = sc["level"]
            unique_results.sort(key=lambda x: getattr(x, "match_score", 0), reverse=True)
            console.print(f"[bold green]✓ Selesai menghitung match score untuk {len(unique_results)} jurnal.[/bold green]")
        except Exception as e:
            console.print(f"[red]⚠️ Gagal menganalisis naskah: {e}[/red]")
    else:
        import doc_reader
        for j in unique_results:
            sc = doc_reader.calculate_keyword_match_score(keyword, j)
            j.match_score = sc["score"]
            j.kesesuaian_naskah = sc["level"]
        unique_results.sort(key=lambda x: getattr(x, "match_score", 0), reverse=True)

    # ── Tampilkan di terminal ─────────────────────────────────────────────────
    print_summary_table(unique_results)

    # ── Export ────────────────────────────────────────────────────────────────
    console.print("\n[bold]💾 Menyimpan output...[/bold]")
    exported_files = []

    if "md" in args.formats:
        path = export_markdown(unique_results, keyword, sinta_ranks)
        exported_files.append(path)

    if "csv" in args.formats:
        path = export_csv(unique_results, keyword)
        exported_files.append(path)

    if "excel" in args.formats:
        path = export_excel(unique_results, keyword)
        exported_files.append(path)

    # ── Ringkasan akhir ───────────────────────────────────────────────────────
    console.print("\n")
    console.print(Panel(
        f"[bold green]✅ Selesai![/bold green]\n\n"
        f"  Keyword   : [cyan]{keyword}[/cyan]\n"
        f"  Hasil     : [cyan]{len(unique_results)} jurnal[/cyan]\n"
        f"  File      : [cyan]{', '.join(exported_files)}[/cyan]\n\n"
        f"[dim]Buka file output di folder [bold]output/[/bold] untuk melihat hasil lengkap.[/dim]",
        border_style="green",
        title="JSFS Done",
    ))

    return unique_results


def main():
    print_banner()
    args = parse_args()

    # Mode interaktif jika tidak ada keyword atau flag -i
    if args.interactive or not args.keyword:
        args = interactive_mode()

    run(args)


if __name__ == "__main__":
    main()
