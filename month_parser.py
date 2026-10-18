"""
month_parser.py — Parser Bulan Terbit (Regex) v1.4.0
Ekstraksi deterministik jadwal bulan terbit jurnal tanpa beban LLM.
"""
from __future__ import annotations

import re
from typing import Iterable, List, Set

_MONTHS = {
    1: ["januari", "january", "jan"],
    2: ["februari", "pebruari", "february", "feb"],
    3: ["maret", "march", "mar"],
    4: ["april", "apr"],
    5: ["mei", "may"],
    6: ["juni", "june", "jun"],
    7: ["juli", "july", "jul"],
    8: ["agustus", "august", "agu", "agt", "aug"],
    9: ["september", "sept", "sep"],
    10: ["oktober", "october", "okt", "oct"],
    11: ["november", "nopember", "nov"],
    12: ["desember", "december", "des", "dec"],
}

_LOOKUP = {alias: num for num, aliases in _MONTHS.items() for alias in aliases}
_MONTH_RX = re.compile(r"\b(" + "|".join(sorted(_LOOKUP, key=len, reverse=True)) + r")\b", re.I)
_FREQ_RX = re.compile(
    r"terbit|diterbitkan|publish|issued?\b|frekuensi|frequency|setahun|per year|per tahun|annual|edisi",
    re.I,
)

_MONTH_NAMES_ID = {
    1: "Jan", 2: "Feb", 3: "Mar", 4: "Apr", 5: "Mei", 6: "Jun",
    7: "Jul", 8: "Agu", 9: "Sep", 10: "Okt", 11: "Nov", 12: "Des"
}


def parse_bulan_terbit(text: str) -> list[int]:
    """Kembalikan daftar nomor bulan terbit (1-12), mis. [3, 9]. Kosong = tidak terdeteksi."""
    months: set[int] = set()
    for sent in re.split(r"(?<=[.!?])\s+", text or ""):
        if not _FREQ_RX.search(sent):
            continue
        for m in _MONTH_RX.findall(sent):
            if m == "may":  # 'may' huruf kecil = kata kerja bantu Bahasa Inggris, bukan Mei
                continue
            months.add(_LOOKUP[m.lower()])
    return sorted(months)


def lolos_filter_bulan(
    bulan_jurnal: list[int] | None,
    bulan_dipilih: set[int] | Iterable[int],
    sertakan_tak_terdeteksi: bool = True,
) -> bool:
    """Cek apakah jadwal bulan terbit jurnal lolos filter yang dipilih pengguna."""
    if not bulan_jurnal:
        return sertakan_tak_terdeteksi
    return bool(set(bulan_jurnal) & set(bulan_dipilih))


def format_bulan_terbit(months: list[int] | None) -> str:
    """Format daftar integer bulan menjadi representasi teks ringkas, misal 'Mar, Sep'."""
    if not months:
        return "—"
    return ", ".join(_MONTH_NAMES_ID.get(m, str(m)) for m in sorted(set(months)))
