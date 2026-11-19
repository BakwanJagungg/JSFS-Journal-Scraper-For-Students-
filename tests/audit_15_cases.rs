use jsfs::models::{ApcConfidence, JournalRecord, ManuscriptMeta, PreSubmitChecklist};
use jsfs::parser::doc_matcher::compute_match_score;
use jsfs::parser::heuristics::{is_acronym_match, jaccard_token_similarity, score_apc_confidence};
use jsfs::scraper::ojs_deep::{extract_year_from_feed_xml, extract_year_from_oai_xml};
use std::collections::HashMap;

#[test]
fn test_stage5_15_case_studies_audit() {
    println!("\n=== AUDIT 15 STUDI KASUS ENGINE VALIDASI JSFS v4.14.1 ===\n");

    let mut apc_correct = 0;
    let mut keaktifan_correct = 0;
    let mut relevance_correct = 0;
    let mut triangulation_correct = 0;

    // --- KELOMPOK 1: 5 Variasi Keyword & Bidang Ilmu ---
    println!("[Kelompok 1] 5 Variasi Kata Kunci & Bidang Ilmu:");

    // Kasus 1: "machine learning" / Informatika
    let rec1_target = JournalRecord {
        title: "Jurnal Ilmu Komputer dan Kecerdasan Buatan".to_string(),
        subject_area: "Informatika".to_string(),
        focus_and_scope: "Fokus pada machine learning, data mining, dan computer vision."
            .to_string(),
        ..Default::default()
    };
    let rec1_unrel = JournalRecord {
        title: "Jurnal Budidaya Pertanian Tropika".to_string(),
        subject_area: "Pertanian".to_string(),
        focus_and_scope: "Fokus pada agronomi dan kesuburan tanah.".to_string(),
        ..Default::default()
    };
    let (sc1_target, _, r1) =
        compute_match_score(&rec1_target, &["machine learning".to_string()], None);
    let (sc1_unrel, _, _) =
        compute_match_score(&rec1_unrel, &["machine learning".to_string()], None);
    assert!(
        sc1_target > sc1_unrel + 30.0,
        "Kasus 1: Target ({sc1_target}) harus jauh lebih tinggi dari unrelated ({sc1_unrel})"
    );
    assert!(!r1.is_empty(), "Kasus 1: Rationale kecocokan harus terisi");
    relevance_correct += 1;
    println!("  Kasus 1: 'machine learning' / Informatika -> Target {sc1_target:.1} vs Unrel {sc1_unrel:.1} [OK]");

    // Kasus 2: "pendidikan matematika" / Pendidikan (Bilingual & Stemmer)
    let rec2_target = JournalRecord {
        title: "Journal of Mathematics Education and Learning".to_string(),
        subject_area: "Pendidikan".to_string(),
        focus_and_scope: "Publishes research on mathematics teaching and curriculum development."
            .to_string(),
        ..Default::default()
    };
    let rec2_unrel = JournalRecord {
        title: "Jurnal Teknik Mesin dan Otomotif".to_string(),
        subject_area: "Teknik".to_string(),
        focus_and_scope: "Penelitian sistem pembakaran motor diesel.".to_string(),
        ..Default::default()
    };
    let (sc2_target, _, r2) = compute_match_score(
        &rec2_target,
        &[
            "pendidikan".to_string(),
            "matematika".to_string(),
            "pembelajaran".to_string(),
        ],
        None,
    );
    let (sc2_unrel, _, _) = compute_match_score(
        &rec2_unrel,
        &[
            "pendidikan".to_string(),
            "matematika".to_string(),
            "pembelajaran".to_string(),
        ],
        None,
    );
    assert!(sc2_target > sc2_unrel + 25.0);
    assert!(r2.iter().any(|r| r.contains("ID-EN")
        || r.contains("ID–EN")
        || r.contains("pendidikan")
        || r.contains("pembelajaran")));
    relevance_correct += 1;
    println!("  Kasus 2: 'pendidikan matematika' / Pendidikan -> Target {sc2_target:.1} vs Unrel {sc2_unrel:.1} [OK]");

    // Kasus 3: "akuntansi keuangan" / Ekonomi
    let rec3_target = JournalRecord {
        title: "Jurnal Riset Akuntansi dan Perpajakan".to_string(),
        subject_area: "Ekonomi dan Bisnis".to_string(),
        focus_and_scope: "Kajian akuntansi keuangan, auditing, dan tata kelola perusahaan."
            .to_string(),
        ..Default::default()
    };
    let rec3_unrel = JournalRecord {
        title: "Jurnal Kelautan dan Perikanan Nusantara".to_string(),
        subject_area: "Kelautan".to_string(),
        focus_and_scope: "Kajian oseanografi dan budidaya udang.".to_string(),
        ..Default::default()
    };
    let (sc3_target, _, _) = compute_match_score(
        &rec3_target,
        &["akuntansi".to_string(), "keuangan".to_string()],
        None,
    );
    let (sc3_unrel, _, _) = compute_match_score(
        &rec3_unrel,
        &["akuntansi".to_string(), "keuangan".to_string()],
        None,
    );
    assert!(sc3_target > sc3_unrel + 30.0);
    relevance_correct += 1;
    println!("  Kasus 3: 'akuntansi keuangan' / Ekonomi -> Target {sc3_target:.1} vs Unrel {sc3_unrel:.1} [OK]");

    // Kasus 4: "hukum pidana" / Hukum
    let rec4_target = JournalRecord {
        title: "Jurnal Konstitusi dan Peradilan".to_string(),
        subject_area: "Ilmu Hukum".to_string(),
        focus_and_scope: "Kajian hukum pidana, pembaharuan hukum pidana materiil dan formil."
            .to_string(),
        ..Default::default()
    };
    let rec4_unrel = JournalRecord {
        title: "Jurnal Desain Komunikasi Visual".to_string(),
        subject_area: "Seni dan Desain".to_string(),
        focus_and_scope: "Kajian tipografi dan ilustrasi digital.".to_string(),
        ..Default::default()
    };
    let (sc4_target, _, _) = compute_match_score(
        &rec4_target,
        &["hukum pidana".to_string(), "pidana".to_string()],
        None,
    );
    let (sc4_unrel, _, _) = compute_match_score(
        &rec4_unrel,
        &["hukum pidana".to_string(), "pidana".to_string()],
        None,
    );
    assert!(sc4_target > sc4_unrel + 30.0);
    relevance_correct += 1;
    println!(
        "  Kasus 4: 'hukum pidana' / Hukum -> Target {sc4_target:.1} vs Unrel {sc4_unrel:.1} [OK]"
    );

    // Kasus 5: "keperawatan anak" / Kesehatan
    let rec5_target = JournalRecord {
        title: "Jurnal Keperawatan dan Kebidanan Tropis".to_string(),
        subject_area: "Kesehatan".to_string(),
        focus_and_scope: "Kajian keperawatan anak, asuhan keperawatan maternitas, dan stunting."
            .to_string(),
        ..Default::default()
    };
    let rec5_unrel = JournalRecord {
        title: "Jurnal Metalurgi dan Material Terbarukan".to_string(),
        subject_area: "Teknik".to_string(),
        focus_and_scope: "Kajian paduan logam titanium dan korosi baja.".to_string(),
        ..Default::default()
    };
    let (sc5_target, _, _) = compute_match_score(
        &rec5_target,
        &["keperawatan".to_string(), "anak".to_string()],
        None,
    );
    let (sc5_unrel, _, _) = compute_match_score(
        &rec5_unrel,
        &["keperawatan".to_string(), "anak".to_string()],
        None,
    );
    assert!(sc5_target > sc5_unrel + 30.0);
    relevance_correct += 1;
    println!("  Kasus 5: 'keperawatan anak' / Kesehatan -> Target {sc5_target:.1} vs Unrel {sc5_unrel:.1} [OK]");

    // --- KELOMPOK 2: 5 Kombinasi Filter Ketat & Engine Validasi ---
    println!("\n[Kelompok 2] 5 Kombinasi Filter Ketat & Validasi Bawaan:");

    // Kasus 6: SINTA 2 + Gratis (APC == 0) + Fast-Track
    let text6 = "Article Submission Fee: 0 (Free). Fast-track Review: IDR 500.000. Article Publication: IDR 0 (Free of charge for all authors).";
    let pos6 = text6.find("Publication: IDR 0").unwrap();
    let (conf6, snip6) = score_apc_confidence(text6, pos6);
    assert_eq!(conf6, ApcConfidence::Medium);
    assert!(snip6.contains("Free of charge"));

    // Kasus 6b: Pure Gratis Publikasi (High Confidence)
    let text6b = "Author Fees & Article Processing Charges (APC): IDR 0 (Free of charge). No publication fee is required.";
    let pos6b = text6b.find("IDR 0").unwrap();
    let (conf6b, _) = score_apc_confidence(text6b, pos6b);
    assert_eq!(conf6b, ApcConfidence::High);
    apc_correct += 1;
    println!(
        "  Kasus 6: SINTA 2 + Gratis APC (0) -> Conf: {:?} (dengan Fast-Track), {:?} (Murni) [OK]",
        conf6, conf6b
    );

    // Kasus 7: SINTA 3-4 + Terbit Bulan Depan (OAI XML Fallback Keaktifan)
    let oai_xml7 = r#"<?xml version="1.0" encoding="UTF-8"?>
    <OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/">
      <ListIdentifiers>
        <header>
          <identifier>oai:ojs.example.ac.id:article/1054</identifier>
          <datestamp>2026-03-15T08:00:00Z</datestamp>
        </header>
      </ListIdentifiers>
    </OAI-PMH>"#;
    let yr7 = extract_year_from_oai_xml(oai_xml7);
    assert_eq!(yr7, Some(2026));
    keaktifan_correct += 1;
    println!("  Kasus 7: Keaktifan via OAI-PMH Fallback -> Terdeteksi Tahun {yr7:?} [OK]");

    // Kasus 8: Terbuka Submisi + Predatory Risk Normal (Atom Feed Fallback Keaktifan)
    let feed_xml8 = r#"<?xml version="1.0" encoding="utf-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
      <title>Jurnal Teknologi Informasi</title>
      <updated>2025-11-20T10:00:00Z</updated>
      <entry>
        <title>Sistem Deteksi Anomali Jaringan</title>
        <published>2025-10-05T00:00:00Z</published>
      </entry>
    </feed>"#;
    let yr8 = extract_year_from_feed_xml(feed_xml8);
    assert_eq!(yr8, Some(2025));
    keaktifan_correct += 1;
    println!("  Kasus 8: Keaktifan via Atom Feed Fallback -> Terdeteksi Tahun {yr8:?} [OK]");

    // Kasus 9: Maksimal APC Rp 500.000 (Ambiguitas Biaya Cetak -> NeedsCheck, Resmi APC -> High)
    let text9_ambig = "Authors are requested to pay towards printing costs of IDR 350.000.";
    let pos9_ambig = text9_ambig.find("350.000").unwrap();
    let (conf9_ambig, _) = score_apc_confidence(text9_ambig, pos9_ambig);
    assert_eq!(conf9_ambig, ApcConfidence::NeedsCheck); // Tepat: biaya cetak diragukan sebagai APC wajib

    let text9_official = "Publication fee (APC) for accepted manuscript: IDR 450.000.";
    let pos9_official = text9_official.find("450.000").unwrap();
    let (conf9_official, _snip9) = score_apc_confidence(text9_official, pos9_official);
    assert_eq!(conf9_official, ApcConfidence::High);
    apc_correct += 1;
    println!(
        "  Kasus 9: APC Rp 450.000 -> Conf: {:?}, Ambiguitas Biaya Cetak -> {:?} [OK]",
        conf9_official, conf9_ambig
    );

    // Kasus 10: ISSN Mismatch / Hijacked Journal Detection (Triangulasi)
    let t_reg10 = "Bulletin of Electrical Engineering and Informatics";
    let t_site10 = "International Journal of Electrical Agriculture and Medical Review";
    let sim10 = jaccard_token_similarity(t_reg10, t_site10);
    let is_acr10 = is_acronym_match(t_reg10, t_site10);
    let mismatch10 = sim10 < 0.35 && !is_acr10;
    assert!(
        mismatch10,
        "Kasus 10: Jurnal dibajak harus terdeteksi mismatch"
    );
    triangulation_correct += 1;

    // Tes akronim jurnal yang sah tidak dianggap mismatch
    let t_acr_reg = "BEEI";
    let t_acr_site = "Bulletin of Electrical Engineering and Informatics";
    assert!(is_acronym_match(t_acr_reg, t_acr_site));
    triangulation_correct += 1;
    println!("  Kasus 10: Deteksi Mismatch Pembajakan ISSN -> Sim={sim10:.2}, Mismatch={mismatch10} [OK]");

    // --- KELOMPOK 3: 5 Manuscript Matcher dengan Naskah Nyata ---
    println!("\n[Kelompok 3] 5 Uji Manuscript Matcher dengan Naskah Nyata:");

    // Kasus 11: Naskah Segmentasi Pelanggan E-Commerce (Informatika)
    let ms11 = ManuscriptMeta {
        filename: "naskah_kmeans_ecommerce.docx".to_string(),
        char_count: 5000,
        extracted_text: "Abstrak: Penelitian ini membahas segmentasi pelanggan e-commerce menggunakan algoritma k-means clustering dan model RFM. Hasil pengujian menunjukkan segmentasi menghasilkan 4 klaster pelanggan optimal. Kata Kunci: e-commerce, k-means, clustering, segmentasi pelanggan, rfm.".to_string(),
        detected_keywords: vec!["e-commerce".to_string(), "k-means".to_string(), "clustering".to_string(), "segmentasi pelanggan".to_string()],
        detected_domain: "Informatika".to_string(),
        extracted_title: "Penerapan Algoritma K-Means untuk Segmentasi Pelanggan E-Commerce".to_string(),
        suggested_keywords: "e-commerce, k-means, clustering".to_string(),
        abstract_snippet: "Penelitian ini membahas segmentasi pelanggan e-commerce menggunakan clustering.".to_string(),
        checklist: PreSubmitChecklist::default(),
        cited_journals: HashMap::new(),
        recommended_topics: Vec::new(),
        ..Default::default()
    };
    let rec11_target = JournalRecord {
        title: "Jurnal Teknologi Informasi dan Sistem Komputer".to_string(),
        subject_area: "Informatika".to_string(),
        focus_and_scope:
            "Fokus pada data mining, kecerdasan buatan, e-commerce, dan sistem informasi."
                .to_string(),
        ..Default::default()
    };
    let rec11_unrel = JournalRecord {
        title: "Jurnal Agroteknologi Tanaman Industri".to_string(),
        subject_area: "Pertanian".to_string(),
        focus_and_scope: "Penelitian kelapa sawit dan pemupukan urea.".to_string(),
        ..Default::default()
    };
    let (sc11_target, _, _) = compute_match_score(&rec11_target, &[], Some(&ms11));
    let (sc11_unrel, _, _) = compute_match_score(&rec11_unrel, &[], Some(&ms11));
    assert!(
        sc11_target > sc11_unrel + 25.0,
        "Kasus 11: target {sc11_target} vs unrel {sc11_unrel}"
    );
    relevance_correct += 1;
    println!(
        "  Kasus 11: K-Means E-Commerce -> Target {sc11_target:.1} vs Unrel {sc11_unrel:.1} [OK]"
    );

    // Kasus 12: Naskah Media Pembelajaran AR (Pendidikan)
    let ms12 = ManuscriptMeta {
        filename: "naskah_ar_pendidikan.docx".to_string(),
        char_count: 4800,
        extracted_text: "Abstrak: Pengembangan media pembelajaran interaktif berbasis augmented reality pada materi geometri siswa sekolah menengah. Model pengembangan menggunakan ADDIE. Hasil validasi ahli media mencapai 91% (sangat layak). Kata Kunci: media pembelajaran, augmented reality, addie, geometri.".to_string(),
        detected_keywords: vec!["media pembelajaran".to_string(), "augmented reality".to_string(), "pembelajaran".to_string(), "pendidikan".to_string()],
        detected_domain: "Pendidikan".to_string(),
        extracted_title: "Pengembangan Media Pembelajaran Interaktif Berbasis Augmented Reality".to_string(),
        suggested_keywords: "media pembelajaran, augmented reality".to_string(),
        abstract_snippet: "Pengembangan media pembelajaran interaktif berbasis augmented reality.".to_string(),
        checklist: PreSubmitChecklist::default(),
        cited_journals: HashMap::new(),
        recommended_topics: Vec::new(),
        ..Default::default()
    };
    let rec12_target = JournalRecord {
        title: "Jurnal Inovasi Pendidikan dan Multimedia Pembelajaran".to_string(),
        subject_area: "Pendidikan".to_string(),
        focus_and_scope: "Fokus pada teknologi pendidikan, media pembelajaran interaktif, augmented reality, dan inovasi kurikulum.".to_string(),
        ..Default::default()
    };
    let rec12_unrel = JournalRecord {
        title: "Jurnal Geologi dan Pertambangan Nusantara".to_string(),
        subject_area: "Geologi".to_string(),
        focus_and_scope: "Penelitian eksplorasi batubara dan mineral logam.".to_string(),
        ..Default::default()
    };
    let (sc12_target, _, _) = compute_match_score(&rec12_target, &[], Some(&ms12));
    let (sc12_unrel, _, _) = compute_match_score(&rec12_unrel, &[], Some(&ms12));
    assert!(
        sc12_target > sc12_unrel + 25.0,
        "Kasus 12: target {sc12_target} vs unrel {sc12_unrel}"
    );
    relevance_correct += 1;
    println!("  Kasus 12: Media Pembelajaran AR -> Target {sc12_target:.1} vs Unrel {sc12_unrel:.1} [OK]");

    // Kasus 13: Naskah Profitabilitas & Leverage Bank (Akuntansi/Keuangan)
    let ms13 = ManuscriptMeta {
        filename: "naskah_keuangan_bank.docx".to_string(),
        char_count: 5200,
        extracted_text: "Abstrak: Penelitian ini menganalisis pengaruh return on assets (ROA) dan debt to equity ratio (DER) terhadap nilai perusahaan (Tobin's Q) pada sub-sektor perbankan di Bursa Efek Indonesia. Kata Kunci: profitabilitas, leverage, nilai perusahaan, perbankan.".to_string(),
        detected_keywords: vec![
            "keuangan".to_string(),
            "profitabilitas".to_string(),
            "leverage".to_string(),
            "nilai perusahaan".to_string(),
            "perbankan".to_string(),
        ],
        detected_domain: "Ekonomi".to_string(),
        extracted_title: "Pengaruh Profitabilitas dan Leverage terhadap Nilai Perusahaan Sektor Perbankan".to_string(),
        suggested_keywords: "keuangan, profitabilitas, leverage, perbankan".to_string(),
        abstract_snippet: "Menganalisis pengaruh return on assets dan debt to equity terhadap nilai perusahaan perbankan.".to_string(),
        checklist: PreSubmitChecklist::default(),
        cited_journals: HashMap::new(),
        recommended_topics: Vec::new(),
        ..Default::default()
    };
    let rec13_target = JournalRecord {
        title: "Jurnal Keuangan dan Perbankan".to_string(),
        subject_area: "Ekonomi dan Keuangan".to_string(),
        focus_and_scope: "Menerbitkan artikel ilmiah di bidang manajemen keuangan, pasar modal, perbankan, dan profitabilitas perusahaan.".to_string(),
        ..Default::default()
    };
    let rec13_unrel = JournalRecord {
        title: "Jurnal Teknik Sipil dan Struktur Beton".to_string(),
        subject_area: "Teknik Sipil".to_string(),
        focus_and_scope: "Analisis kekuatan geser balok beton bertulang.".to_string(),
        ..Default::default()
    };
    let (sc13_target, _, _) = compute_match_score(&rec13_target, &[], Some(&ms13));
    let (sc13_unrel, _, _) = compute_match_score(&rec13_unrel, &[], Some(&ms13));
    assert!(
        sc13_target > sc13_unrel + 20.0,
        "Kasus 13: target {sc13_target} vs unrel {sc13_unrel}"
    );
    relevance_correct += 1;
    println!(
        "  Kasus 13: Keuangan Perbankan -> Target {sc13_target:.1} vs Unrel {sc13_unrel:.1} [OK]"
    );

    // Kasus 14: Naskah Pidana Lingkungan Hidup (Hukum)
    let ms14 = ManuscriptMeta {
        filename: "naskah_pidana_lingkungan.docx".to_string(),
        char_count: 5400,
        extracted_text: "Abstrak: Analisis pertanggungjawaban pidana korporasi terhadap pencemaran limbah industri berdasarkan Undang-Undang Perlindungan dan Pengelolaan Lingkungan Hidup (UU PPLH). Kata Kunci: pertanggungjawaban pidana, korporasi, hukum lingkungan, limbah industri.".to_string(),
        detected_keywords: vec!["pertanggungjawaban pidana".to_string(), "korporasi".to_string(), "hukum lingkungan".to_string(), "pidana".to_string()],
        detected_domain: "Hukum".to_string(),
        extracted_title: "Analisis Yuridis Pertanggungjawaban Pidana Korporasi dalam Tindak Pidana Lingkungan Hidup".to_string(),
        suggested_keywords: "pertanggungjawaban pidana, korporasi, hukum lingkungan".to_string(),
        abstract_snippet: "Analisis pertanggungjawaban pidana korporasi terhadap pencemaran lingkungan hidup.".to_string(),
        checklist: PreSubmitChecklist::default(),
        cited_journals: HashMap::new(),
        recommended_topics: Vec::new(),
        ..Default::default()
    };
    let rec14_target = JournalRecord {
        title: "Jurnal Hukum dan Keadilan Lingkungan".to_string(),
        subject_area: "Ilmu Hukum".to_string(),
        focus_and_scope: "Fokus pada hukum pidana khusus, hukum lingkungan, pertanggungjawaban korporasi, dan peradilan tata usaha negara.".to_string(),
        ..Default::default()
    };
    let rec14_unrel = JournalRecord {
        title: "Jurnal Kimia Terapan dan Katalisis".to_string(),
        subject_area: "Kimia".to_string(),
        focus_and_scope: "Sintesis polimer zeolit dan adsorpsi zat warna metilen biru.".to_string(),
        ..Default::default()
    };
    let (sc14_target, _, _) = compute_match_score(&rec14_target, &[], Some(&ms14));
    let (sc14_unrel, _, _) = compute_match_score(&rec14_unrel, &[], Some(&ms14));
    assert!(
        sc14_target > sc14_unrel + 25.0,
        "Kasus 14: target {sc14_target} vs unrel {sc14_unrel}"
    );
    relevance_correct += 1;
    println!("  Kasus 14: Pidana Lingkungan Korporasi -> Target {sc14_target:.1} vs Unrel {sc14_unrel:.1} [OK]");

    // Kasus 15: Naskah Edukasi Gizi Stunting (Kesehatan Masyarakat)
    let ms15 = ManuscriptMeta {
        filename: "naskah_gizi_stunting.docx".to_string(),
        char_count: 4900,
        extracted_text: "Abstrak: Studi quasi-eksperimen untuk menguji efektivitas edukasi gizi seimbang terhadap peningkatan pengetahuan ibu balita dalam pencegahan stunting di wilayah kerja Puskesmas. Hasil menunjukkan peningkatan rata-rata skor pengetahuan sebesar 35%. Kata Kunci: edukasi gizi, stunting, balita, puskesmas.".to_string(),
        detected_keywords: vec!["edukasi gizi".to_string(), "stunting".to_string(), "balita".to_string(), "puskesmas".to_string()],
        detected_domain: "Kesehatan".to_string(),
        extracted_title: "Efektivitas Edukasi Gizi terhadap Pencegahan Stunting pada Balita di Puskesmas".to_string(),
        suggested_keywords: "edukasi gizi, stunting, balita".to_string(),
        abstract_snippet: "Edukasi gizi seimbang terhadap pencegahan stunting pada balita di puskesmas.".to_string(),
        checklist: PreSubmitChecklist::default(),
        cited_journals: HashMap::new(),
        recommended_topics: Vec::new(),
        ..Default::default()
    };
    let rec15_target = JournalRecord {
        title: "Jurnal Kesehatan Masyarakat dan Gizi Indonesia".to_string(),
        subject_area: "Kesehatan Masyarakat".to_string(),
        focus_and_scope: "Fokus pada gizi kesehatan masyarakat, epidemiologi, stunting, kesehatan ibu dan anak, serta pelayanan puskesmas.".to_string(),
        ..Default::default()
    };
    let rec15_unrel = JournalRecord {
        title: "Jurnal Robotika dan Otomasi Manufaktur".to_string(),
        subject_area: "Teknik Mekatronika".to_string(),
        focus_and_scope: "Perancangan lengan robotik berbasis PLC dan mikrokontroler.".to_string(),
        ..Default::default()
    };
    let (sc15_target, _, _) = compute_match_score(&rec15_target, &[], Some(&ms15));
    let (sc15_unrel, _, _) = compute_match_score(&rec15_unrel, &[], Some(&ms15));
    assert!(
        sc15_target > sc15_unrel + 25.0,
        "Kasus 15: target {sc15_target} vs unrel {sc15_unrel}"
    );
    relevance_correct += 1;
    println!("  Kasus 15: Edukasi Gizi Stunting -> Target {sc15_target:.1} vs Unrel {sc15_unrel:.1} [OK]");

    // --- REKAPITULASI HASIL AUDIT ---
    println!("\n=== RINGKASAN METRIK AKURASI & GROUND-TRUTH (15 STUDI KASUS) ===");
    println!("- Relevansi Match Score BM25 + Bigram: 10/10 naskah/bidang (100%)");
    println!("- Akurasi APC Confidence Scoring:      2/2 skenario uji   (100%)");
    println!("- Deteksi Keaktifan (OAI/Feed Fallback): 2/2 endpoint XML  (100%)");
    println!("- Deteksi Hijacked/ISSN Triangulasi:   2/2 skenario uji   (100%)");
    println!("- Total Keberhasilan:                  100% (Semua Kasus Lulus Sesuai Ground-Truth)");
    println!("- Status Keputusan:                    GO (Siap untuk Rilis Stabil)\n");

    assert_eq!(relevance_correct, 10);
    assert_eq!(apc_correct, 2);
    assert_eq!(keaktifan_correct, 2);
    assert_eq!(triangulation_correct, 2);
}
