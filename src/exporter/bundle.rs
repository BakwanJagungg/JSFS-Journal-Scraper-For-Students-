//! 1-Click Submission Bundle Exporter (`.zip`) for JSFS v4.7:
//! Packages personalized Cover Letter, Statement of Originality (Materai Rp 10.000),
//! and Journal Author Guidelines / Pre-Flight Readiness Summary into a single ZIP archive.

use crate::config::output_dir;
use crate::engine::manuscript::ManuscriptSummary;
use crate::error::JsfsError;
use crate::exporter::sanitize_slug;
use crate::exporter::templates::{
    generate_cover_letter_with_profile, generate_originality_statement,
};
use crate::models::{compute_submission_readiness, FitStatus, UiJournalItem};
use crate::parser::doc_matcher::check_manuscript_word_fit;
use crate::ui::i18n::AppSettings;
use chrono::Local;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Generate a concise text cheat-sheet of the target journal's rules, fees, indexes, and readiness checklist.
pub fn generate_journal_rules_summary(
    journal: &UiJournalItem,
    manuscript: Option<&ManuscriptSummary>,
    lang: &str,
) -> String {
    let is_en = lang.eq_ignore_ascii_case("en");
    let readiness = compute_submission_readiness(
        journal,
        manuscript.map(|m| m.word_count),
        manuscript.map(|m| m.imrad_complete),
        manuscript.and_then(|m| m.detected_citation_style.as_deref()),
        is_en,
    );
    let issn_status = if journal.issn_valid {
        "Valid (ISO 3297 Modulo 11)"
    } else if !journal.issn.trim().is_empty() || !journal.eissn.trim().is_empty() {
        "Perlu Verifikasi Manual (Checksum/Format)"
    } else {
        "Tidak terdata"
    };
    let word_limit_disp = match journal.word_limit {
        Some((0, max_w)) => format!("Maksimal {max_w} kata"),
        Some((min_w, max_w)) => format!("{min_w} - {max_w} kata"),
        None => "Belum terdeteksi di halaman OJS".to_string(),
    };
    let word_fit_note = manuscript
        .and_then(|ms| check_manuscript_word_fit(ms.word_count as u32, journal.word_limit))
        .map(|st| match st {
            FitStatus::Sesuai => "Sesuai (OK)",
            FitStatus::TerlaluPendek => "Terlalu Pendek (!)",
            FitStatus::MelebihiBatas => "Melebihi Batas (!)",
        })
        .unwrap_or("-");
    let indexes = if journal.extra_indexes.is_empty() {
        "-".to_string()
    } else {
        journal.extra_indexes.join(", ")
    };
    let checklist_lines: String = readiness
        .checklist
        .iter()
        .map(|(ok, msg)| {
            let mark = if *ok { "[✓]" } else { "[!]" };
            format!("  {mark} {msg}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "============================================================\n\
RINGKASAN ATURAN JURNAL & CHECKLIST PENGIRIMAN (JSFS v4.7)\n\
============================================================\n\
Nama Jurnal         : {name}\n\
Penerbit            : {pub_name}\n\
Peringkat / Portal  : SINTA {sinta} ({portal})\n\
ISSN / e-ISSN       : {issn} / {eissn} [{issn_status}]\n\
Indeks Sekunder     : {indexes}\n\
Website OJS         : {url}\n\
Email Editor        : {email}\n\
Telepon / WA        : {phone}\n\
Template Naskah     : {tpl}\n\
------------------------------------------------------------\n\
BIAYA & JADWAL\n\
------------------------------------------------------------\n\
Biaya Publikasi APC : {apc}\n\
Biaya Submit (Sub)  : {sub_fee}\n\
Jadwal Terbit       : {sched}\n\
Terbit Terdekat     : {next}\n\
Estimasi Review     : {turnaround}\n\
------------------------------------------------------------\n\
SYARAT NASKAH (AUTHOR GUIDELINES)\n\
------------------------------------------------------------\n\
Gaya Sitasi         : {cit}\n\
Batas Panjang Kata  : {word_limit_disp} (Status Naskah: {word_fit_note})\n\
Batas Similarity    : {similarity}\n\
Bahasa Artikel      : {langs}\n\
Focus & Scope       : {scope}\n\
------------------------------------------------------------\n\
PRE-SUBMISSION READINESS SCORE: {score}%\n\
------------------------------------------------------------\n\
{checklist_lines}\n",
        name = journal.nama.trim(),
        pub_name = journal.penerbit.trim(),
        sinta = if journal.sinta.trim().is_empty() {
            "-"
        } else {
            journal.sinta.trim()
        },
        portal = journal.sumber.trim(),
        issn = if journal.issn.trim().is_empty() {
            "-"
        } else {
            journal.issn.trim()
        },
        eissn = if journal.eissn.trim().is_empty() {
            "-"
        } else {
            journal.eissn.trim()
        },
        url = journal.url.trim(),
        email = journal.email.trim(),
        phone = journal.telepon.trim(),
        tpl = journal.template_url.as_deref().unwrap_or("-"),
        apc = journal.apc_label.trim(),
        sub_fee = journal.submit_label.trim(),
        sched = journal.jadwal_terbit.trim(),
        next = journal.terbit_terdekat.trim(),
        turnaround = if journal.turnaround_label.trim().is_empty() {
            "-"
        } else {
            journal.turnaround_label.trim()
        },
        cit = journal
            .citation_style
            .as_deref()
            .unwrap_or("Belum terdeteksi di halaman OJS"),
        similarity = journal
            .max_similarity_pct
            .map(|pct| format!("Maks. {pct}% (Turnitin / iThenticate)"))
            .unwrap_or_else(|| "Belum terdata di halaman OJS".to_string()),
        langs = if journal.article_languages.trim().is_empty() {
            "-"
        } else {
            journal.article_languages.trim()
        },
        scope = journal.focus_scope.trim(),
        score = readiness.score,
    )
}

/// Write a 3-document submission package `.zip` directly to `dest_path`.
pub fn write_submission_bundle_zip_to(
    dest_path: &Path,
    journal: &UiJournalItem,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    settings: &AppSettings,
    lang: &str,
) -> Result<String, JsfsError> {
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let slug = sanitize_slug(&journal.nama);
    let cover_letter =
        generate_cover_letter_with_profile(journal, keyword, manuscript, Some(settings), lang);
    let originality = generate_originality_statement(journal, manuscript, Some(settings), lang);
    let rules = generate_journal_rules_summary(journal, manuscript, lang);
    let cover_docx = crate::exporter::docx_writer::build_cover_letter_docx_bytes(
        journal,
        keyword,
        manuscript,
        Some(settings),
        lang,
    )?;
    let orig_docx = crate::exporter::docx_writer::build_originality_statement_docx_bytes(
        journal,
        manuscript,
        Some(settings),
        lang,
    )?;

    let file = File::create(dest_path)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file(format!("Cover_Letter_{slug}.txt"), opts)
        .map_err(|e| JsfsError::Document(format!("ZIP start_file error: {e}")))?;
    zip.write_all(cover_letter.as_bytes())?;

    zip.start_file(format!("Cover_Letter_{slug}.docx"), opts)
        .map_err(|e| JsfsError::Document(format!("ZIP start_file error: {e}")))?;
    zip.write_all(&cover_docx)?;

    zip.start_file(format!("Surat_Pernyataan_Keaslian_{slug}.txt"), opts)
        .map_err(|e| JsfsError::Document(format!("ZIP start_file error: {e}")))?;
    zip.write_all(originality.as_bytes())?;

    zip.start_file(format!("Surat_Pernyataan_Keaslian_{slug}.docx"), opts)
        .map_err(|e| JsfsError::Document(format!("ZIP start_file error: {e}")))?;
    zip.write_all(&orig_docx)?;

    zip.start_file(format!("Ringkasan_Aturan_Jurnal_{slug}.txt"), opts)
        .map_err(|e| JsfsError::Document(format!("ZIP start_file error: {e}")))?;
    zip.write_all(rules.as_bytes())?;

    zip.finish()
        .map_err(|e| JsfsError::Document(format!("ZIP finish error: {e}")))?;

    Ok(dest_path.to_string_lossy().into_owned())
}

/// Write the 1-Click Submission Bundle (`.zip`) inside `output/`.
pub fn write_submission_bundle_zip(
    journal: &UiJournalItem,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    settings: &AppSettings,
    lang: &str,
) -> Result<String, JsfsError> {
    let out = output_dir();
    let slug = sanitize_slug(&journal.nama);
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let dest = out.join(format!("JSFS_Paket_Pengajuan_{slug}_{ts}.zip"));
    write_submission_bundle_zip_to(&dest, journal, keyword, manuscript, settings, lang)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    #[test]
    fn test_submission_bundle_zip_contains_all_three_documents() {
        let dir = std::env::temp_dir().join("jsfs_v47_bundle_test");
        let _ = std::fs::remove_dir_all(&dir);
        let zip_path = dir.join("test_bundle.zip");

        let item = UiJournalItem {
            nama: "Jurnal Rekayasa Perangkat Lunak".to_string(),
            penerbit: "Universitas Teknologi Nusantara".to_string(),
            sinta: "3".to_string(),
            issn: "2502-2377".to_string(),
            issn_valid: true,
            apc_label: "Rp 450.000".to_string(),
            submit_label: "Gratis (Submission)".to_string(),
            citation_style: Some("IEEE".to_string()),
            word_limit: Some((4000, 7000)),
            extra_indexes: vec!["DOAJ".to_string(), "Copernicus".to_string()],
            match_score: 84.0,
            ..Default::default()
        };
        let ms = ManuscriptSummary {
            name: "skripsi_nlp.docx".to_string(),
            extracted_title: "Deteksi Plagiarisme Berbasis IndoBERT".to_string(),
            word_count: 5100,
            imrad_complete: true,
            ..Default::default()
        };
        let cfg = AppSettings {
            author_name: "Budi Santoso".to_string(),
            author_id: "210411100042".to_string(),
            author_affiliation: "Teknik Informatika, Universitas Nusantara".to_string(),
            author_email: "budi@nusantara.ac.id".to_string(),
            ..Default::default()
        };

        let saved =
            write_submission_bundle_zip_to(&zip_path, &item, "indobert", Some(&ms), &cfg, "id")
                .expect("bundle zip should succeed");
        assert!(Path::new(&saved).exists());

        let f = File::open(&zip_path).unwrap();
        let mut archive = ZipArchive::new(f).unwrap();
        assert_eq!(archive.len(), 5);

        let mut names = Vec::new();
        let mut combined_txt = String::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            let name = entry.name().to_string();
            if name.ends_with(".docx") {
                let mut raw = Vec::new();
                entry.read_to_end(&mut raw).unwrap();
                assert!(raw.starts_with(b"PK"));
            } else {
                let mut s = String::new();
                entry.read_to_string(&mut s).unwrap();
                combined_txt.push_str(&s);
            }
            names.push(name);
        }

        assert!(names
            .iter()
            .any(|n| n.starts_with("Cover_Letter_") && n.ends_with(".txt")));
        assert!(names
            .iter()
            .any(|n| n.starts_with("Cover_Letter_") && n.ends_with(".docx")));
        assert!(names
            .iter()
            .any(|n| n.starts_with("Surat_Pernyataan_Keaslian_") && n.ends_with(".txt")));
        assert!(names
            .iter()
            .any(|n| n.starts_with("Surat_Pernyataan_Keaslian_") && n.ends_with(".docx")));
        assert!(names
            .iter()
            .any(|n| n.starts_with("Ringkasan_Aturan_Jurnal_")));
        assert!(combined_txt.contains("Budi Santoso"));
        assert!(combined_txt.contains("Materai"));
        assert!(combined_txt.contains("IEEE"));
        assert!(combined_txt.contains("DOAJ, Copernicus"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
