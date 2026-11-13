"""
tests/test_jsfs_core.py — Unit Test Suite JSFS v1.7.0
Menguji fungsi kritis:
1. Anti-Email Flood (Filter NIM Mahasiswa & Whitelist Redaksi)
2. Validasi Nomor Telepon & Whitelist Kode Area Telkom
3. Sinkronisasi Bulan Terbit & Frekuensi (Arsip Grouping)
4. Deep APC Hunter: Anti-OA-Trap & Paid Priority Rule
5. Deduplikasi Cerdas (Canonical Journal Key & Cross-Source Merge)
6. Generator Copy-Paste List WA
7. Incomplete Cache Bypass & Cache Engine Versioning
"""
import sys
import os
import unittest

# Pastikan root direktori ada di sys.path
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

from scraper_garuda import (
    decode_cfemail,
    is_valid_indonesian_phone,
    canonical_journal_key,
    rank_and_filter_emails,
    parse_apc_from_context,
    JournalEntry,
    merge_cross_source_entries,
    APC_UNKNOWN,
)
from month_parser import (
    resolve_schedule_and_frequency,
    extract_months_from_archive_titles,
)
from exporter import generate_copypaste_list
import cache_engine


class TestJSFSCore(unittest.TestCase):

    def test_01_email_filtering_and_ranking(self):
        """Memastikan email mahasiswa (NIM & domain student) dibuang dan email redaksi diprioritaskan."""
        raw_candidates = [
            "2203035@student.tbu.ac.id",          # NIM 7 digit mahasiswa -> Tolak
            "sitinurhaliza070904@gmail.com",       # NIM / tanggal lahir >= 6 digit -> Tolak
            "mahasiswa@mhs.kampus.ac.id",          # Domain mhs -> Tolak
            "alumni@alumni.univ.ac.id",            # Domain alumni -> Tolak
            "author1@gmail.com",                   # Author generic -> Tolak
            "jurnal@plb.ac.id",                    # Email resmi OJS kampus -> Prioritas 1
            "redaksi.ekobima@plb.ac.id",           # Prefix redaksi resmi kampus -> Prioritas 2
            "ekobima.official@gmail.com",          # Gmail redaksi jurnal -> Prioritas 3
        ]
        ojs_url = "https://jurnal.plb.ac.id/index.php/ekobima"
        ranked = rank_and_filter_emails(raw_candidates, ojs_url=ojs_url)

        # Harus membuang email mahasiswa dan author generic
        self.assertNotIn("2203035@student.tbu.ac.id", ranked)
        self.assertNotIn("sitinurhaliza070904@gmail.com", ranked)
        self.assertNotIn("mahasiswa@mhs.kampus.ac.id", ranked)
        self.assertNotIn("alumni@alumni.univ.ac.id", ranked)

        # Harus memuat email resmi redaksi
        self.assertIn("jurnal@plb.ac.id", ranked)
        self.assertIn("redaksi.ekobima@plb.ac.id", ranked)

        # Hard cap maksimal 3 email
        self.assertLessEqual(len(ranked), 3)

    def test_02_cloudflare_email_decoder(self):
        """Menguji dekoder Cloudflare Protected Email (__cf_email__)."""
        # Test encode sample
        decoded = decode_cfemail("f0e1e2e3f1")
        self.assertTrue(isinstance(decoded, str))

    def test_03_phone_validation_telkom_whitelist(self):
        """Menguji validasi nomor telepon kabel Telkom & WhatsApp Indonesia."""
        # 1. Nomor WA valid
        ok, num = is_valid_indonesian_phone("+6281234567890", has_contact_context=True)
        self.assertTrue(ok)
        self.assertTrue(num.startswith("+628"))

        # 2. Nomor telepon kabel valid (0271 = Surakarta)
        ok_telkom, num_t = is_valid_indonesian_phone("(0271) 632450", has_contact_context=True)
        self.assertTrue(ok_telkom)

        # 3. Nomor palsu dari DOI / URL (Kasus UNESA: +6221005162 - digit setelah 021 adalah 00)
        bad_unesa, _ = is_valid_indonesian_phone("+6221005162", has_contact_context=True)
        self.assertFalse(bad_unesa, "Nomor palsu UNESA 02100... seharusnya ditolak!")

        # 4. Nomor terlalu panjang (16 digit DOI/rekening)
        bad_long, _ = is_valid_indonesian_phone("+6249124113500475", has_contact_context=True)
        self.assertFalse(bad_long, "Nomor 16 digit seharusnya ditolak!")

    def test_04_month_schedule_and_frequency_sync(self):
        """Menguji konsistensi bulan terbit dan frekuensi."""
        r = resolve_schedule_and_frequency("Terbit dua kali setahun pada bulan Juni dan Desember")
        self.assertEqual(len(r["months_num"]), 2)
        self.assertIn("2 kali setahun", r["frequency_str"])
        self.assertIn(6, r["months_num"])
        self.assertIn(12, r["months_num"])

    def test_05_archive_year_grouping(self):
        """Memastikan ekstraksi dari arsip mengambil 1 tahun terbaru tanpa akumulasi anomali 7-8 bulan."""
        sample_titles = [
            "Vol. 10 No. 2 (2026): Desember",
            "Vol. 10 No. 1 (2026): Juni",
            "Vol. 9 No. 2 (2025): Desember",
            "Vol. 9 No. 1 (2025): Juni",
            "Vol. 8 No. 2 (2024): Desember",
            "Vol. 8 No. 1 (2024): Juni",
        ]
        months = extract_months_from_archive_titles(sample_titles)
        self.assertEqual(months, [6, 12])

    def test_06_deep_apc_anti_oa_trap(self):
        """Memastikan kebijakan Open Access pembaca TIDAK dianggap sebagai Gratis (Rp 0)."""
        oa_text = (
            "This journal provides immediate open access to its content on the principle "
            "that making research freely available to the public supports a greater global exchange of knowledge."
        )
        status, amount = parse_apc_from_context(oa_text)
        self.assertEqual(status, APC_UNKNOWN)
        self.assertIsNone(amount)

    def test_07_deep_apc_paid_priority_rule(self):
        """Memastikan jika ada Submission = 0 dan Publication > 0, status berbayar yang menang."""
        mixed_text = "Article Submission: 0.00 IDR  |  Article Publication: 500,000.00 IDR"
        status, amount = parse_apc_from_context(mixed_text)
        self.assertEqual(status, "Rp 500.000")
        self.assertEqual(amount, 500000)

    def test_08_deep_apc_explicit_free(self):
        """Memastikan pernyataan eksplisit bebas biaya terbit diakui sebagai Gratis (Rp 0)."""
        free_text = "Penulis tidak dikenakan biaya publikasi (Article Publication Fee is Free)."
        status, amount = parse_apc_from_context(free_text)
        self.assertEqual(status, "Gratis (Rp 0)")
        self.assertEqual(amount, 0)

    def test_09_canonical_deduplication(self):
        """Menguji deduplikasi cerdas canonical_journal_key."""
        k1 = canonical_journal_key("Jurnal Manajemen & Bisnis", "Universitas Indonesia", "2345-6789")
        k2 = canonical_journal_key("JURNAL MANAJEMEN DAN BISNIS", "universitas indonesia", "23456789")
        self.assertEqual(k1, k2)

    def test_10_cross_source_merge(self):
        """Menguji penggabungan entri Garuda dan SINTA."""
        j1 = JournalEntry(
            nama_jurnal="Jurnal Sains Terapan",
            penerbit="Politeknik Negeri",
            peringkat_sinta="Sinta 3",
            link_ojs="http://jurnal.poltek.ac.id/index.php/jst",
            email="jst@poltek.ac.id",
            biaya_publikasi=350000,
            apc_status="Rp 350.000",
            bulan_terbit=[4, 10],
            bidang="Teknologi dan Rekayasa",
        )
        j2 = JournalEntry(
            nama_jurnal="Jurnal Sains Terapan",
            penerbit="Politeknik Negeri",
            peringkat_sinta="Sinta 3",
            sinta_profile_url="https://sinta.kemdiktisaintek.go.id/journals/profile/9999",
            link_ojs="http://jurnal.poltek.ac.id/index.php/jst",
            telepon="+6281234567890",
            bidang="Engineering, Computer Science",
        )
        merged = merge_cross_source_entries([j1, j2])
        self.assertEqual(len(merged), 1)
        res = merged[0]
        self.assertEqual(res.email, "jst@poltek.ac.id")
        self.assertEqual(res.telepon, "+6281234567890")
        self.assertEqual(res.biaya_publikasi, 350000)
        self.assertIn("sinta.kemdiktisaintek.go.id", res.link_sinta)

    def test_11_generate_copypaste_list(self):
        """Menguji formatting output share WA."""
        j = JournalEntry(
            nama_jurnal="Jurnal Akuntansi dan Bisnis",
            peringkat_sinta="Sinta 2",
            penerbit="Universitas Brawijaya",
            email="jab@ub.ac.id",
            telepon="+6281234567890",
            biaya_publikasi=500000,
            apc_status="Rp 500.000",
            bulan_terbit=[3, 9],
            bidang="Akuntansi Keuangan, Manajemen Bisnis",
            link_ojs="https://jab.ub.ac.id",
        )
        text = generate_copypaste_list([j])
        self.assertIn("1. Jurnal Akuntansi dan Bisnis (Sinta 2 - Universitas Brawijaya)", text)
        self.assertIn("Email: jab@ub.ac.id", text)
        self.assertIn("Telepon/WA: +6281234567890", text)
        self.assertIn("Biaya Publikasi: Rp 500.000", text)
        self.assertIn("Maret, September", text)
        self.assertIn("Link OJS: https://jab.ub.ac.id", text)

    def test_12_incomplete_cache_bypass(self):
        """Memastikan data cache dengan APC kosong di-bypass."""
        fake_url = "https://jurnal.test.ac.id/index.php/test"
        # Upsert entri incomplete
        cache_engine.cache_engine.upsert_journal(fake_url, {
            "nama_jurnal": "Test Journal Incomplete",
            "apc_publish": "Cek Web / Belum Terdata",
            "source_apc": "Kosong",
        })
        # get_journal default (allow_incomplete=False) harus return None
        retrieved = cache_engine.cache_engine.get_journal(fake_url, allow_incomplete=False)
        self.assertIsNone(retrieved, "Entri incomplete harus di-bypass (return None)!")

    def test_13_typo_correction_and_fallback(self):
        """Memastikan koreksi typo kata kunci akademik dan fallback sinonim v1.7.1 berjalan akurat."""
        from doc_reader import correct_keyword_typo, get_fallback_keywords

        cases = [
            ("bisins", "bisnis"),
            ("akutansi", "akuntansi"),
            ("manajmen", "manajemen"),
            ("informtika", "informatika"),
            ("pendidkan", "pendidikan"),
            ("hukm", "hukum"),
        ]
        for typo, expected in cases:
            corrected, changed = correct_keyword_typo(typo)
            self.assertTrue(changed, f"Harus mendeteksi typo untuk '{typo}'")
            self.assertEqual(corrected, expected)

        unchanged, flag = correct_keyword_typo("manajemen")
        self.assertFalse(flag)
        self.assertEqual(unchanged, "manajemen")

        fallbacks = get_fallback_keywords("bisins")
        self.assertIn("bisnis", fallbacks)
        self.assertTrue(len(fallbacks) >= 1)

    def test_14_mixed_separator_apc(self):
        """Memastikan ekstraksi nominal campuran '1.000,000 (IDR)' (Benefit UMS) -> Rp 1.000.000."""
        cases = [
            "Article Publication: 1.000,000 (IDR)",
            "Article Publication: 1,000.000 (IDR)",
            "Article Submission: 0.00 (IDR). Article Publication: 1.000,000 (IDR)",
            "Publication Fee: Rp 1.000,000,00",
        ]
        for text in cases:
            disp, val = parse_apc_from_context(text)
            self.assertEqual(disp, "Rp 1.000.000", f"Gagal format untuk: {text}")
            self.assertEqual(val, 1_000_000, f"Gagal nominal untuk: {text}")


if __name__ == "__main__":
    unittest.main()
