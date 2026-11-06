"""
month_parser.py — Parser Bulan Terbit & Unifikasi Frekuensi v1.5.0
Ekstraksi deterministik jadwal bulan terbit jurnal tanpa beban LLM,
penghitungan estimasi terbit terdekat (Next Issue Countdown),
dan unifikasi satu pintu frekuensi agar 0% kontradiksi.
"""
from __future__ import annotations

import re
from datetime import datetime
from typing import Any, Dict, Iterable, List, Optional, Set

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

_MONTH_NAMES_FULL_ID = {
    1: "Januari", 2: "Februari", 3: "Maret", 4: "April", 5: "Mei", 6: "Juni",
    7: "Juli", 8: "Agustus", 9: "September", 10: "Oktober", 11: "November", 12: "Desember"
}


def parse_bulan_terbit(text: str) -> list[int]:
    """Kembalikan daftar nomor bulan terbit (1-12), mis. [3, 9]. Kosong = tidak terdeteksi."""
    if not text:
        return []
    months: set[int] = set()
    cleaned_text = text.strip()

    # Jika teks pendek (mis. field khusus atau daftar bulan langsung), ekstrak langsung
    if len(cleaned_text) < 120:
        for m in _MONTH_RX.findall(cleaned_text):
            if m.lower() == "may" and m == "may":
                continue
            months.add(_LOOKUP[m.lower()])
        if months:
            return sorted(months)

    for sent in re.split(r"(?<=[.!?])\s+", cleaned_text):
        if not _FREQ_RX.search(sent):
            continue
        for m in _MONTH_RX.findall(sent):
            if m.lower() == "may" and m == "may":  # 'may' huruf kecil = kata kerja bantu Bahasa Inggris, bukan Mei
                continue
            months.add(_LOOKUP[m.lower()])
    return sorted(months)


def _normalize_month_list(months: Any) -> list[int]:
    """
    Normalisasi berbagai variasi format data bulan (int, str nama bulan, singkatan, json string)
    menjadi list integer [1..12] terurut dan unik.
    Mencegah bug perbedaan tipe data (set string vs set int) yang menyebabkan drop jurnal.
    """
    if not months:
        return []
    if isinstance(months, str):
        months_str = months.strip()
        if months_str.startswith("[") and months_str.endswith("]"):
            try:
                import json
                parsed = json.loads(months_str)
                return _normalize_month_list(parsed)
            except Exception:
                pass
        return parse_bulan_terbit(months_str)

    if isinstance(months, (int, float)):
        val = int(months)
        return [val] if 1 <= val <= 12 else []

    result: set[int] = set()
    try:
        for item in months:
            if isinstance(item, (int, float)):
                val = int(item)
                if 1 <= val <= 12:
                    result.add(val)
            elif isinstance(item, str):
                s = item.strip().lower()
                if s.isdigit():
                    val = int(s)
                    if 1 <= val <= 12:
                        result.add(val)
                elif s in _LOOKUP:
                    result.add(_LOOKUP[s])
                else:
                    for m_num in parse_bulan_terbit(s):
                        result.add(m_num)
    except Exception:
        pass
    return sorted(result)


def lolos_filter_bulan(
    bulan_jurnal: Any,
    bulan_dipilih: set[int] | Iterable[int] | None,
    sertakan_tak_terdeteksi: bool = True,
) -> bool:
    """
    Cek apakah jadwal bulan terbit jurnal lolos filter yang dipilih pengguna.
    - Jika seluruh 12 bulan dipilih (default), langsung return True agar 0% jurnal terbuang.
    - Jika bulan_jurnal kosong, ikuti flag sertakan_tak_terdeteksi.
    - Normalisasi data bulan ke integer sebelum interseksi set.
    """
    if not bulan_dipilih:
        return True

    selected_set = set(bulan_dipilih)
    if len(selected_set) >= 12:
        return True

    normalized = _normalize_month_list(bulan_jurnal)
    if not normalized:
        return sertakan_tak_terdeteksi
    return bool(set(normalized) & selected_set)


def format_bulan_terbit(months: Any) -> str:
    """Format daftar integer atau string bulan menjadi representasi teks ringkas, misal 'Mar, Sep'."""
    norm = _normalize_month_list(months)
    if not norm:
        return "—"
    return ", ".join(_MONTH_NAMES_ID.get(m, str(m)) for m in norm)


def resolve_schedule_and_frequency(raw_text: str, fallback_freq_num: int = 0) -> dict:
    """
    UNIFIKASI SATU PINTU JADWAL & FREKUENSI (v1.5.0):
    Mengeliminasi 100% kontradiksi antara kolom 'Bulan Terbit' dan 'Frekuensi'.
    
    Output dictionary:
    - months_num: list[int] terurut unik (misal: [4, 8, 12])
    - months_short: str untuk kolom Bulan Terbit (misal: 'Apr, Agu, Des' atau '—')
    - frequency_str: str untuk kolom Frekuensi.
      Jika months_num terisi, jumlah frekuensi WAJIB mengikuti len(months_num):
      f"{len(months_num)} kali setahun ({months_full_joined})"
    """
    months = parse_bulan_terbit(raw_text)
    months_short = format_bulan_terbit(months)

    if months:
        count = len(months)
        full_names = [_MONTH_NAMES_FULL_ID.get(m, str(m)) for m in months]
        if count == 1:
            joined = full_names[0]
        elif count == 2:
            joined = f"{full_names[0]} & {full_names[1]}"
        else:
            joined = ", ".join(full_names[:-1]) + f" & {full_names[-1]}"
        frequency_str = f"{count} kali setahun ({joined})"
    else:
        detected_count = None
        if fallback_freq_num is not None:
            if isinstance(fallback_freq_num, (int, float)):
                if 1 <= int(fallback_freq_num) <= 24:
                    detected_count = int(fallback_freq_num)
            elif isinstance(fallback_freq_num, str):
                m_dig = re.search(r"(\d+)", fallback_freq_num)
                if m_dig:
                    val = int(m_dig.group(1))
                    if 1 <= val <= 24:
                        detected_count = val
        if detected_count is None and raw_text:
            low = raw_text.lower()
            if any(k in low for k in ("semiannual", "biannual", "twice a year", "2 kali setahun", "2 terbitan")):
                detected_count = 2
            elif any(k in low for k in ("triannual", "three times a year", "3 kali setahun", "3 terbitan")):
                detected_count = 3
            elif any(k in low for k in ("quarterly", "four times a year", "4 kali setahun", "4 terbitan")):
                detected_count = 4
            elif any(k in low for k in ("bimonthly", "six times a year", "6 kali setahun", "6 terbitan")):
                detected_count = 6
            elif any(k in low for k in ("monthly", "twelve times a year", "12 kali setahun", "12 terbitan")):
                detected_count = 12
            elif any(k in low for k in ("annual", "once a year", "1 kali setahun", "1 terbitan")):
                detected_count = 1
            else:
                m_gen = re.search(r"(\d+)\s*(?:kali|nomor|terbitan|issues?|times?)\s*(?:setahun|per\s*(?:tahun|year)|\/\s*(?:tahun|year))?", low)
                if not m_gen:
                    m_gen = re.search(r"terbit\s*(\d+)\s*(?:kali|nomor|terbitan)?", low)
                if not m_gen:
                    m_gen = re.search(r"\b(\d+)\s*\/\s*(?:year|tahun)\b", low)
                if m_gen:
                    try:
                        c = int(m_gen.group(1))
                        if 1 <= c <= 24:
                            detected_count = c
                    except ValueError:
                        pass

        if detected_count and 1 <= detected_count <= 24:
            frequency_str = f"{detected_count} kali setahun"
        else:
            frequency_str = "Informasi tidak ditemukan dalam situs jurnal"

    return {
        "months_num": months,
        "months_short": months_short,
        "frequency_str": frequency_str,
    }


def get_next_issue_estimate(months_num: Any, ref_date: Optional[datetime] = None) -> str:
    """
    Kalkulasi 'Estimasi Terbit Terdekat' (Next Issue Countdown) v1.5.0:
    - Ambil curr_month & curr_year dari ref_date (default: datetime.now()).
    - Cari bulan m in months_num terkecil di mana m >= curr_month.
    - Jika ditemukan: delta = m - curr_month, target_year = curr_year.
    - Jika tidak ditemukan (semua bulan terbit tahun ini lewat):
      m = min(months_num), delta = (12 - curr_month) + m, target_year = curr_year + 1.
    - Format output:
      Jika delta == 0: "Des 2026 (Bulan Ini)"
      Jika delta > 0:  "Des 2026 (2 bln lagi)"
    """
    valid_months = _normalize_month_list(months_num)
    if not valid_months:
        return "—"

    if ref_date is None:
        ref_date = datetime.now()

    curr_month = ref_date.month
    curr_year = ref_date.year

    candidates = [m for m in valid_months if m >= curr_month]
    if candidates:
        target_m = min(candidates)
        delta = target_m - curr_month
        target_year = curr_year
    else:
        target_m = min(valid_months)
        delta = (12 - curr_month) + target_m
        target_year = curr_year + 1

    m_str = _MONTH_NAMES_ID.get(target_m, str(target_m))
    if delta == 0:
        return f"{m_str} {target_year} (Bulan Ini)"
    return f"{m_str} {target_year} ({delta} bln lagi)"


def extract_months_from_archive_titles(titles: list[str]) -> list[int]:
    """
    Ekstraksi jadwal bulan terbit nyata dari daftar judul edisi terbitan (Issue Archive) v1.6.0.
    Misal: ['Vol. 10 No. 1 (2026): April - Oktober', 'Vol 5 No 2 (2025): Desember'] -> [4, 10, 12].
    """
    if not titles:
        return []
    found_months: set[int] = set()
    for t in titles:
        if not t or not isinstance(t, str):
            continue
        cleaned = t.strip()
        # Judul edisi umumnya memuat nama bulan (mis: 'Vol 1 No 1: April - Mei 2026')
        ms = parse_bulan_terbit(cleaned)
        found_months.update(ms)
    return sorted(found_months)

