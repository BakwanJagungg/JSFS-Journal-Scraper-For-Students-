//! Pure-Rust OOXML (`.docx`) writer for academic submission letters and supervisor reports.
//! Uses only the existing `zip` crate with A4 page geometry and zero external dependencies.

use crate::config::output_dir;
use crate::engine::manuscript::ManuscriptSummary;
use crate::error::JsfsError;
use crate::exporter::sanitize_slug;
use crate::exporter::templates::{
    generate_cover_letter_with_profile, generate_originality_statement,
};
use crate::models::UiJournalItem;
use crate::ui::i18n::{AppLanguage, AppSettings};
use chrono::Local;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#;

const ROOT_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

const DOC_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:rPr>
      <w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/>
      <w:sz w:val="22"/>
      <w:szCs w:val="22"/>
    </w:rPr>
  </w:style>
</w:styles>"#;

fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' => out.push_str("    "),
            c if c.is_control() && c != '\n' => {}
            c => out.push(c),
        }
    }
    out
}

fn ooxml_paragraph(text: &str, bold: bool, size_hp: u16, align: &str, after_twips: u16) -> String {
    let escaped = xml_escape(text);
    let b_tag = if bold { "<w:b/>" } else { "" };
    format!(
        r#"<w:p><w:pPr><w:jc w:val="{align}"/><w:spacing w:after="{after_twips}" w:line="276" w:lineRule="auto"/></w:pPr><w:r><w:rPr>{b_tag}<w:sz w:val="{size_hp}"/><w:szCs w:val="{size_hp}"/></w:rPr><w:t xml:space="preserve">{escaped}</w:t></w:r></w:p>"#
    )
}

fn ooxml_materai_box(is_en: bool) -> String {
    let label_top = if is_en { "Stamp Duty" } else { "Materai" };
    let label_bot = if is_en { "IDR 10,000" } else { "Rp 10.000" };
    format!(
        r#"<w:tbl>
  <w:tblPr>
    <w:tblW w:w="2300" w:type="dxa"/>
    <w:tblBorders>
      <w:top w:val="dashed" w:sz="6" w:space="0" w:color="555555"/>
      <w:left w:val="dashed" w:sz="6" w:space="0" w:color="555555"/>
      <w:bottom w:val="dashed" w:sz="6" w:space="0" w:color="555555"/>
      <w:right w:val="dashed" w:sz="6" w:space="0" w:color="555555"/>
    </w:tblBorders>
  </w:tblPr>
  <w:tr>
    <w:trPr><w:trHeight w:val="860" w:hRule="atLeast"/></w:trPr>
    <w:tc>
      <w:tcPr>
        <w:tcW w:w="2300" w:type="dxa"/>
        <w:vAlign w:val="center"/>
      </w:tcPr>
      {}
      {}
    </w:tc>
  </w:tr>
</w:tbl>"#,
        ooxml_paragraph(label_top, true, 18, "center", 20),
        ooxml_paragraph(label_bot, true, 18, "center", 0),
    )
}

fn ooxml_table(headers: &[&str], rows: &[Vec<String>], col_widths: &[u32]) -> String {
    let mut xml = String::from(
        r#"<w:tbl><w:tblPr><w:tblW w:w="9026" w:type="dxa"/><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="B8C4D4"/><w:left w:val="single" w:sz="4" w:space="0" w:color="B8C4D4"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="B8C4D4"/><w:right w:val="single" w:sz="4" w:space="0" w:color="B8C4D4"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="D8E0EB"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="D8E0EB"/></w:tblBorders><w:tblCellMar><w:top w:w="90" w:type="dxa"/><w:left w:w="100" w:type="dxa"/><w:bottom w:w="90" w:type="dxa"/><w:right w:w="100" w:type="dxa"/></w:tblCellMar></w:tblPr>"#,
    );

    // Header Row
    xml.push_str("<w:tr>");
    for (idx, h) in headers.iter().enumerate() {
        let w = col_widths.get(idx).copied().unwrap_or(1400);
        xml.push_str(&format!(
            r#"<w:tc><w:tcPr><w:tcW w:w="{w}" w:type="dxa"/><w:shd w:val="clear" w:color="auto" w:fill="E8EFF8"/></w:tcPr>{}</w:tc>"#,
            ooxml_paragraph(h, true, 19, "left", 0)
        ));
    }
    xml.push_str("</w:tr>");

    // Data Rows
    for row in rows {
        xml.push_str("<w:tr>");
        for (idx, val) in row.iter().enumerate() {
            let w = col_widths.get(idx).copied().unwrap_or(1400);
            xml.push_str(&format!(
                r#"<w:tc><w:tcPr><w:tcW w:w="{w}" w:type="dxa"/></w:tcPr>{}</w:tc>"#,
                ooxml_paragraph(val, false, 18, "left", 0)
            ));
        }
        xml.push_str("</w:tr>");
    }
    xml.push_str("</w:tbl>");
    xml
}

/// Assemble a complete OOXML `.docx` archive in memory from raw `<w:body>` elements (A4 page).
fn build_docx_from_body_xml(body_xml: &str) -> Result<Vec<u8>, JsfsError> {
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    {body_xml}
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/>
    </w:sectPr>
  </w:body>
</w:document>"#
    );

    let mut buf = Vec::with_capacity(4096);
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        for (name, bytes) in [
            ("[Content_Types].xml", CONTENT_TYPES_XML.as_bytes()),
            ("_rels/.rels", ROOT_RELS_XML.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS_XML.as_bytes()),
            ("word/styles.xml", STYLES_XML.as_bytes()),
            ("word/document.xml", document_xml.as_bytes()),
        ] {
            zip.start_file(name, opts)
                .map_err(|e| JsfsError::Document(format!("DOCX ZIP error: {e}")))?;
            zip.write_all(bytes)?;
        }
        zip.finish()
            .map_err(|e| JsfsError::Document(format!("DOCX finish error: {e}")))?;
    }
    Ok(buf)
}

fn build_plain_letter_body_xml(text: &str, title_line_idx: Option<usize>) -> String {
    let mut body = String::new();
    for (idx, line) in text.replace("\r\n", "\n").lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            body.push_str(&ooxml_paragraph("", false, 22, "left", 80));
        } else if title_line_idx == Some(idx) {
            body.push_str(&ooxml_paragraph(trimmed, true, 26, "center", 140));
        } else if trimmed.starts_with("Perihal:") || trimmed.starts_with("Subject:") {
            body.push_str(&ooxml_paragraph(trimmed, true, 22, "left", 100));
        } else {
            body.push_str(&ooxml_paragraph(trimmed, false, 22, "left", 60));
        }
    }
    body
}

fn write_bytes_to_path(dest_path: &Path, bytes: &[u8]) -> Result<String, JsfsError> {
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(dest_path)?;
    file.write_all(bytes)?;
    Ok(dest_path.to_string_lossy().into_owned())
}

/// Build an in-memory `.docx` Cover Letter ready for standalone save or ZIP bundling.
pub fn build_cover_letter_docx_bytes(
    journal: &UiJournalItem,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> Result<Vec<u8>, JsfsError> {
    let letter = generate_cover_letter_with_profile(journal, keyword, manuscript, profile, lang);
    let body = build_plain_letter_body_xml(&letter, None);
    build_docx_from_body_xml(&body)
}

/// Write a standard A4 `.docx` Cover Letter to `dest_path`.
pub fn write_cover_letter_docx(
    dest_path: &Path,
    journal: &UiJournalItem,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> Result<String, JsfsError> {
    let bytes = build_cover_letter_docx_bytes(journal, keyword, manuscript, profile, lang)?;
    write_bytes_to_path(dest_path, &bytes)
}

/// Write a standard A4 `.docx` Cover Letter into `output/` and return the saved file path.
pub fn save_cover_letter_docx(
    journal: &UiJournalItem,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> Result<String, JsfsError> {
    let slug = sanitize_slug(&journal.nama);
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let dest = output_dir().join(format!("Cover_Letter_{slug}_{ts}.docx"));
    write_cover_letter_docx(&dest, journal, keyword, manuscript, profile, lang)
}

/// Build an in-memory `.docx` Statement of Originality with bordered Materai Rp 10.000 box.
pub fn build_originality_statement_docx_bytes(
    journal: &UiJournalItem,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> Result<Vec<u8>, JsfsError> {
    let is_en = lang.eq_ignore_ascii_case("en");
    let raw = generate_originality_statement(journal, manuscript, profile, lang);
    let mut body = String::new();
    for (idx, line) in raw.replace("\r\n", "\n").lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("Materai Rp 10.000") || trimmed.contains("Stamp Duty / Materai") {
            body.push_str(&ooxml_materai_box(is_en));
            body.push_str(&ooxml_paragraph("", false, 20, "left", 80));
        } else if idx == 0 {
            body.push_str(&ooxml_paragraph(trimmed, true, 26, "center", 160));
        } else if trimmed.is_empty() {
            body.push_str(&ooxml_paragraph("", false, 22, "left", 60));
        } else {
            body.push_str(&ooxml_paragraph(trimmed, false, 22, "left", 50));
        }
    }
    build_docx_from_body_xml(&body)
}

/// Write a standard A4 `.docx` Statement of Originality (`Surat Keaslian`) with Materai Rp 10.000 box.
pub fn write_originality_statement_docx(
    dest_path: &Path,
    journal: &UiJournalItem,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> Result<String, JsfsError> {
    let bytes = build_originality_statement_docx_bytes(journal, manuscript, profile, lang)?;
    write_bytes_to_path(dest_path, &bytes)
}

/// Write a standard A4 `.docx` Statement of Originality into `output/` and return the path.
pub fn save_originality_statement_docx(
    journal: &UiJournalItem,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> Result<String, JsfsError> {
    let slug = sanitize_slug(&journal.nama);
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let dest = output_dir().join(format!("Surat_Keaslian_{slug}_{ts}.docx"));
    write_originality_statement_docx(&dest, journal, manuscript, profile, lang)
}

/// Write an academic advisor consultation report (`Laporan Konsultasi Dospen`) `.docx` with summary table.
pub fn write_dosbing_report_docx(
    dest_path: &Path,
    candidates: &[&UiJournalItem],
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: AppLanguage,
) -> Result<String, JsfsError> {
    let is_en = matches!(lang, AppLanguage::En);
    let today = Local::now().format("%d %B %Y").to_string();
    let author_line = profile
        .map(|p| {
            let name = if p.author_name.trim().is_empty() {
                "-"
            } else {
                p.author_name.trim()
            };
            let id = p.author_id.trim();
            let aff = p.author_affiliation.trim();
            match (id.is_empty(), aff.is_empty()) {
                (false, false) => format!("{name} ({id}) – {aff}"),
                (false, true) => format!("{name} ({id})"),
                (true, false) => format!("{name} – {aff}"),
                (true, true) => name.to_string(),
            }
        })
        .unwrap_or_else(|| "-".to_string());

    let mut body = String::new();
    body.push_str(&ooxml_paragraph(
        if is_en {
            "TARGET JOURNAL CANDIDATE REPORT FOR ADVISOR CONSULTATION"
        } else {
            "LAPORAN KANDIDAT JURNAL TARGET UNTUK KONSULTASI DOSBING"
        },
        true,
        26,
        "center",
        120,
    ));
    body.push_str(&ooxml_paragraph(
        &format!("{}: {today}", if is_en { "Date" } else { "Tanggal" }),
        false,
        21,
        "left",
        40,
    ));
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}: {author_line}",
            if is_en {
                "Student / Author"
            } else {
                "Mahasiswa / Penulis"
            }
        ),
        false,
        21,
        "left",
        40,
    ));
    if let Some(ms) = manuscript {
        body.push_str(&ooxml_paragraph(
            &format!(
                "{}: {} ({} {})",
                if is_en { "Manuscript" } else { "Naskah" },
                if ms.extracted_title.trim().is_empty() {
                    &ms.name
                } else {
                    &ms.extracted_title
                },
                ms.word_count,
                if is_en { "words" } else { "kata" }
            ),
            false,
            21,
            "left",
            100,
        ));
    }

    let headers: [&str; 6] = if is_en {
        [
            "No",
            "Journal & Publisher",
            "Rank",
            "Match",
            "APC / Fee",
            "Review & Next Issue",
        ]
    } else {
        [
            "No",
            "Jurnal & Penerbit",
            "Indeks",
            "Match",
            "Biaya APC",
            "Review & Terbit",
        ]
    };
    let rows: Vec<Vec<String>> = candidates
        .iter()
        .enumerate()
        .map(|(idx, j)| {
            let mut rank = Vec::new();
            if !j.sinta.trim().is_empty() && j.sinta != "-" {
                rank.push(format!("SINTA {}", j.sinta.trim()));
            }
            if let Some(ref q) = j.quartile {
                if !q.trim().is_empty() {
                    rank.push(q.trim().to_string());
                }
            }
            let rank_str = if rank.is_empty() {
                j.sumber.clone()
            } else {
                rank.join(" / ")
            };
            let match_str = if j.match_score > 0.0 {
                format!("{:.0}%", j.match_score)
            } else {
                "-".to_string()
            };
            let sched_info = format!(
                "{} | Next: {} ({})",
                if j.turnaround_label.trim().is_empty() {
                    "-"
                } else {
                    j.turnaround_label.trim()
                },
                j.terbit_terdekat.trim(),
                j.url.trim()
            );
            vec![
                (idx + 1).to_string(),
                format!("{} ({})", j.nama.trim(), j.penerbit.trim()),
                rank_str,
                match_str,
                j.apc_label.trim().to_string(),
                sched_info,
            ]
        })
        .collect();

    body.push_str(&ooxml_table(
        &headers,
        &rows,
        &[450, 2500, 1100, 800, 1500, 2676],
    ));
    body.push_str(&ooxml_paragraph("", false, 20, "left", 100));
    body.push_str(&ooxml_paragraph(
        if is_en {
            "Prepared automatically by JSFS Submission Copilot for thesis supervisor review."
        } else {
            "Disusun otomatis oleh JSFS Submission Copilot sebagai bahan pertimbangan dosen pembimbing."
        },
        false,
        19,
        "left",
        0,
    ));

    let bytes = build_docx_from_body_xml(&body)?;
    write_bytes_to_path(dest_path, &bytes)
}

/// Write advisor consultation `.docx` report directly inside `output/` and return its path.
pub fn save_dosbing_report_docx(
    candidates: &[&UiJournalItem],
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: AppLanguage,
) -> Result<String, JsfsError> {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let dest = output_dir().join(format!("Laporan_Konsultasi_Dospen_{ts}.docx"));
    write_dosbing_report_docx(&dest, candidates, manuscript, profile, lang)
}

/// Write a standard 3-column academic Response to Reviewers Matrix (`Matriks Revisi`) `.docx`.
pub fn write_revision_matrix_docx(
    dest_path: &Path,
    journal: &UiJournalItem,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: AppLanguage,
) -> Result<String, JsfsError> {
    let is_en = matches!(lang, AppLanguage::En);
    let today = Local::now().format("%d %B %Y").to_string();
    let author_line = profile
        .map(|p| {
            let name = if p.author_name.trim().is_empty() {
                "-"
            } else {
                p.author_name.trim()
            };
            let id = p.author_id.trim();
            let aff = p.author_affiliation.trim();
            match (id.is_empty(), aff.is_empty()) {
                (false, false) => format!("{name} ({id}) – {aff}"),
                (false, true) => format!("{name} ({id})"),
                (true, false) => format!("{name} – {aff}"),
                (true, true) => name.to_string(),
            }
        })
        .unwrap_or_else(|| "-".to_string());

    let mut body = String::new();
    body.push_str(&ooxml_paragraph(
        if is_en {
            "RESPONSE TO REVIEWERS & REVISION MATRIX"
        } else {
            "MATRIKS REVISI & TANGGAPAN TERHADAP KOMENTAR REVIEWER"
        },
        true,
        26,
        "center",
        120,
    ));
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}: {}",
            if is_en {
                "Target Journal"
            } else {
                "Jurnal Target"
            },
            journal.nama.trim()
        ),
        true,
        21,
        "left",
        40,
    ));
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}: {author_line}",
            if is_en {
                "Authors / Researchers"
            } else {
                "Penulis / Peneliti"
            }
        ),
        false,
        21,
        "left",
        40,
    ));
    if let Some(ms) = manuscript {
        body.push_str(&ooxml_paragraph(
            &format!(
                "{}: {} ({} {})",
                if is_en {
                    "Manuscript Title"
                } else {
                    "Judul Naskah"
                },
                if ms.extracted_title.trim().is_empty() {
                    &ms.name
                } else {
                    &ms.extracted_title
                },
                ms.word_count,
                if is_en { "words" } else { "kata" }
            ),
            false,
            21,
            "left",
            40,
        ));
    }
    body.push_str(&ooxml_paragraph(
        &format!("{}: {today}", if is_en { "Date" } else { "Tanggal" }),
        false,
        21,
        "left",
        100,
    ));

    // Introductory polite note
    body.push_str(&ooxml_paragraph(
        if is_en {
            "Dear Editor and Reviewers,\nWe sincerely appreciate the constructive feedback and valuable suggestions provided. Below is our point-by-point response and detailed summary of changes incorporated into the revised manuscript."
        } else {
            "Kepada Yth. Editor dan Tim Reviewer,\nPenulis mengucapkan terima kasih yang sebesar-besarnya atas saran, koreksi, dan masukan berharga yang diberikan. Berikut kami sampaikan tanggapan butir-per-butir serta rincian perbaikan yang telah dilakukan pada naskah revisi."
        },
        false,
        21,
        "both",
        120,
    ));

    let headers: [&str; 3] = if is_en {
        [
            "No & Reviewer Comments",
            "Author's Response",
            "Changes in Manuscript & Page",
        ]
    } else {
        [
            "No & Komentar Reviewer",
            "Tanggapan Penulis",
            "Perubahan pada Naskah & Halaman",
        ]
    };

    let sample_rows: Vec<Vec<String>> = if is_en {
        vec![
            vec![
                "Reviewer #1 - Comment 1:\nThe abstract should clarify the key novelty and quantitative benchmark results.".to_string(),
                "Thank you for the suggestion. We have revised the abstract to clearly state the novelty and empirical improvements.".to_string(),
                "Page 1, Abstract, Lines 18-24.".to_string(),
            ],
            vec![
                "Reviewer #1 - Comment 2:\nPlease provide details of baseline comparison and evaluation metrics.".to_string(),
                "We have added a comprehensive comparison table with Accuracy, Precision, Recall, and F1-score.".to_string(),
                "Page 6, Section 4.2, Table 3.".to_string(),
            ],
            vec![
                "Reviewer #2 - Comment 1:\nReferences should be updated with recent literature from the past 5 years.".to_string(),
                "We updated the references, incorporating 8 high-impact publications from 2022-2026.".to_string(),
                "Pages 9-11, References section.".to_string(),
            ],
        ]
    } else {
        vec![
            vec![
                "Reviewer #1 - Butir 1:\nAbstrak dan pendahuluan perlu mempertegas kebaruan (novelty) serta hasil evaluasi kuantitatif penelitian.".to_string(),
                "Terima kasih atas masukan berharga. Penulis telah merevisi alinea abstrak dan paragraf penutup Pendahuluan untuk memperjelas kontribusi kebaruan metode.".to_string(),
                "Halaman 1 (Abstrak) dan Halaman 2 (Pendahuluan, Alinea Terakhir).".to_string(),
            ],
            vec![
                "Reviewer #1 - Butir 2:\nMetodologi evaluasi perlu menyertakan metrik perbandingan yang jelas terhadap metode baseline.".to_string(),
                "Telah ditambahkan tabel perbandingan komprehensif mencakup metrik Akurasi, Presisi, Recall, dan F1-Score.".to_string(),
                "Halaman 6, Bagian 4.2, Tabel 3.".to_string(),
            ],
            vec![
                "Reviewer #2 - Butir 1:\nDaftar pustaka disarankan diperbarui dengan rujukan primer bereputasi 5 tahun terakhir.".to_string(),
                "Penulis telah memperbarui daftar pustaka dengan menyitir 8 rujukan mutakhir terbitan 2022–2026.".to_string(),
                "Halaman 9–11, Daftar Pustaka.".to_string(),
            ],
        ]
    };

    body.push_str(&ooxml_table(&headers, &sample_rows, &[3500, 3200, 2326]));
    body.push_str(&ooxml_paragraph("", false, 20, "left", 100));
    body.push_str(&ooxml_paragraph(
        if is_en {
            "Generated via JSFS Academic Copilot – Format compliant with OJS/Taylor&Francis/Elsevier revision guidelines."
        } else {
            "Dibuat melalui JSFS Academic Copilot – Format matriks sesuai standar pengajuan revisi OJS / jurnal ilmiah."
        },
        false,
        18,
        "left",
        0,
    ));

    let bytes = build_docx_from_body_xml(&body)?;
    write_bytes_to_path(dest_path, &bytes)
}

pub fn save_revision_matrix_docx(
    journal: &UiJournalItem,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: AppLanguage,
) -> Result<String, JsfsError> {
    let slug = sanitize_slug(&journal.nama);
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let dest = output_dir().join(format!("Matriks_Revisi_{slug}_{ts}.docx"));
    write_revision_matrix_docx(&dest, journal, manuscript, profile, lang)
}

/// Write an official Supervisor Candidate Approval Sheet (`Lembar Persetujuan Dosen Pembimbing`) `.docx`.
pub fn write_dospen_approval_docx(
    dest_path: &Path,
    candidates: &[&UiJournalItem],
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: AppLanguage,
) -> Result<String, JsfsError> {
    let is_en = matches!(lang, AppLanguage::En);
    let today = Local::now().format("%d %B %Y").to_string();

    let student_name = profile
        .map(|p| {
            if p.author_name.trim().is_empty() {
                "-"
            } else {
                p.author_name.trim()
            }
        })
        .unwrap_or("-");
    let student_id = profile
        .map(|p| {
            if p.author_id.trim().is_empty() {
                "-"
            } else {
                p.author_id.trim()
            }
        })
        .unwrap_or("-");
    let student_aff = profile
        .map(|p| {
            if p.author_affiliation.trim().is_empty() {
                "-"
            } else {
                p.author_affiliation.trim()
            }
        })
        .unwrap_or("-");
    let student_email = profile
        .map(|p| {
            if p.author_email.trim().is_empty() {
                "-"
            } else {
                p.author_email.trim()
            }
        })
        .unwrap_or("-");

    let mut body = String::new();
    body.push_str(&ooxml_paragraph(
        if is_en {
            "THESIS SUPERVISOR APPROVAL SHEET\nFOR TARGET SCIENTIFIC JOURNAL SUBMISSION"
        } else {
            "LEMBAR PERSETUJUAN DOSEN PEMBIMBING\nSUBMISI PUBLIKASI KARYA ILMIAH KE JURNAL TARGET"
        },
        true,
        26,
        "center",
        140,
    ));

    // Student Metadata
    body.push_str(&ooxml_paragraph(
        if is_en {
            "A. STUDENT & MANUSCRIPT IDENTIFICATION"
        } else {
            "A. IDENTITAS MAHASISWA & NASKAH"
        },
        true,
        22,
        "left",
        60,
    ));
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}: {student_name}",
            if is_en {
                "Student Name"
            } else {
                "Nama Mahasiswa"
            }
        ),
        false,
        21,
        "left",
        30,
    ));
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}: {student_id}",
            if is_en {
                "Student ID (NIM)"
            } else {
                "Nomor Induk Mahasiswa (NIM)"
            }
        ),
        false,
        21,
        "left",
        30,
    ));
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}: {student_aff}",
            if is_en {
                "Department / Study Program"
            } else {
                "Program Studi / Jurusan"
            }
        ),
        false,
        21,
        "left",
        30,
    ));
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}: {student_email}",
            if is_en {
                "Correspondence Email"
            } else {
                "Email Korespondensi"
            }
        ),
        false,
        21,
        "left",
        30,
    ));

    let ms_title = manuscript
        .map(|m| {
            if m.extracted_title.trim().is_empty() {
                m.name.as_str()
            } else {
                m.extracted_title.as_str()
            }
        })
        .unwrap_or("-");
    let ms_words = manuscript.map(|m| m.word_count).unwrap_or(0);
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}: {ms_title} ({} {})",
            if is_en {
                "Manuscript Title"
            } else {
                "Judul Naskah"
            },
            ms_words,
            if is_en { "words" } else { "kata" }
        ),
        false,
        21,
        "left",
        100,
    ));

    // Section B: Target Journal Candidates
    body.push_str(&ooxml_paragraph(
        if is_en {
            "B. APPROVED TARGET JOURNAL CANDIDATES"
        } else {
            "B. DAFTAR KANDIDAT JURNAL TARGET YANG DISETUJUI"
        },
        true,
        22,
        "left",
        60,
    ));

    let headers: [&str; 5] = if is_en {
        [
            "No",
            "Target Journal & Publisher",
            "Index / SINTA",
            "APC Fee",
            "Turnaround & Status",
        ]
    } else {
        [
            "No",
            "Nama Jurnal & Penerbit",
            "Akreditasi / SINTA",
            "Biaya APC",
            "Keaktifan & Jadwal Terbit",
        ]
    };

    let rows: Vec<Vec<String>> = candidates
        .iter()
        .take(4)
        .enumerate()
        .map(|(idx, j)| {
            let rank = if let Some(ref q) = j.quartile {
                if !q.trim().is_empty() {
                    q.trim().to_string()
                } else if !j.sinta.trim().is_empty() && j.sinta != "-" {
                    format!("SINTA {}", j.sinta.trim())
                } else {
                    j.sumber.clone()
                }
            } else if !j.sinta.trim().is_empty() && j.sinta != "-" {
                format!("SINTA {}", j.sinta.trim())
            } else {
                j.sumber.clone()
            };
            vec![
                (idx + 1).to_string(),
                format!("{} ({})", j.nama.trim(), j.penerbit.trim()),
                rank,
                j.apc_label.trim().to_string(),
                format!(
                    "{} (Terbit: {})",
                    if j.turnaround_label.is_empty() {
                        "-"
                    } else {
                        &j.turnaround_label
                    },
                    j.terbit_terdekat.trim()
                ),
            ]
        })
        .collect();

    body.push_str(&ooxml_table(
        &headers,
        &rows,
        &[450, 3400, 1500, 1600, 2076],
    ));
    body.push_str(&ooxml_paragraph("", false, 20, "left", 80));

    // Declaration statement
    body.push_str(&ooxml_paragraph(
        if is_en {
            "Approval Statement:\nWith this document, the supervisor confirms that the manuscript specified above has been duly reviewed, complies with ethical publication standards, and is approved for submission to one of the approved target journals listed above. Simultaneous multiple submissions (dual submission) are strictly prohibited."
        } else {
            "Pernyataan Persetujuan:\nDengan ini Dosen Pembimbing menyatakan bahwa naskah ilmiah di atas telah melalui proses pembimbingan dan telaah substantif, memenuhi kaidah etika publikasi ilmiah, dan disetujui untuk diajukan (submit) ke salah satu jurnal target yang tertera di atas. Pengiriman naskah secara bersamaan ke lebih dari 1 jurnal (dual submission) dilarang keras."
        },
        false,
        21,
        "both",
        120,
    ));

    // Signatures Block
    body.push_str(&ooxml_paragraph(
        &format!(
            "{}, {today}",
            if is_en { "Date" } else { "Tanggal Persetujuan" }
        ),
        false,
        21,
        "right",
        80,
    ));

    let sig_headers = ["", ""];
    let sig_rows = vec![
        vec![
            if is_en {
                "Student / First Author,".to_string()
            } else {
                "Mahasiswa / Penulis Pertama,".to_string()
            },
            if is_en {
                "Approved by,\nThesis Supervisor,".to_string()
            } else {
                "Menyetujui,\nDosen Pembimbing,".to_string()
            },
        ],
        vec![
            format!("\n\n\n\n({student_name})\nNIM: {student_id}"),
            "\n\n\n\n(......................................................)\nNIDN / NIP: .......................................".to_string(),
        ],
    ];
    body.push_str(&ooxml_table(&sig_headers, &sig_rows, &[4513, 4513]));

    let bytes = build_docx_from_body_xml(&body)?;
    write_bytes_to_path(dest_path, &bytes)
}

pub fn save_dospen_approval_docx(
    candidates: &[&UiJournalItem],
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: AppLanguage,
) -> Result<String, JsfsError> {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let dest = output_dir().join(format!("Lembar_Persetujuan_Dospen_{ts}.docx"));
    write_dospen_approval_docx(&dest, candidates, manuscript, profile, lang)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    fn read_docx_document_xml(bytes: &[u8]) -> String {
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("valid docx zip");
        for req in [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/_rels/document.xml.rels",
            "word/styles.xml",
            "word/document.xml",
        ] {
            assert!(zip.by_name(req).is_ok(), "missing docx part {req}");
        }
        let mut doc = zip.by_name("word/document.xml").unwrap();
        let mut xml = String::new();
        doc.read_to_string(&mut xml).unwrap();
        xml
    }

    #[test]
    fn test_docx_cover_letter_and_originality_materai_valid_ooxml() {
        let dir = std::env::temp_dir().join("jsfs_v49_docx_test");
        let _ = std::fs::remove_dir_all(&dir);
        let item = UiJournalItem {
            nama: "Jurnal Informatika & Rekayasa".to_string(),
            penerbit: "Universitas Merdeka".to_string(),
            sinta: "2".to_string(),
            issn: "2502-2377".to_string(),
            apc_label: "Rp 500.000".to_string(),
            terbit_terdekat: "Desember 2026".to_string(),
            editor_in_chief: Some("Prof. Dr. Santoso".to_string()),
            url: "https://ojs.merdeka.ac.id/jir".to_string(),
            ..Default::default()
        };
        let profile = AppSettings {
            author_name: "Budi Pratama".to_string(),
            author_id: "22010099".to_string(),
            ..Default::default()
        };

        let cl_path = dir.join("cover.docx");
        write_cover_letter_docx(
            &cl_path,
            &item,
            "AI Sistem Informasi",
            None,
            Some(&profile),
            "id",
        )
        .unwrap();
        let cl_xml = read_docx_document_xml(&std::fs::read(&cl_path).unwrap());
        assert!(cl_xml.contains("Prof. Dr. Santoso"));
        assert!(cl_xml.contains("Jurnal Informatika &amp; Rekayasa"));

        let orig_path = dir.join("originality.docx");
        write_originality_statement_docx(&orig_path, &item, None, Some(&profile), "id").unwrap();
        let orig_xml = read_docx_document_xml(&std::fs::read(&orig_path).unwrap());
        assert!(orig_xml.contains("SURAT PERNYATAAN KEASLIAN NASKAH"));
        assert!(orig_xml.contains("Materai"));
        assert!(orig_xml.contains("Rp 10.000"));
        assert!(orig_xml.contains("w:tblBorders"));

        let rep_path = dir.join("dospen.docx");
        write_dosbing_report_docx(&rep_path, &[&item], None, Some(&profile), AppLanguage::Id)
            .unwrap();
        let rep_xml = read_docx_document_xml(&std::fs::read(&rep_path).unwrap());
        assert!(rep_xml.contains("LAPORAN KANDIDAT JURNAL TARGET"));
        assert!(rep_xml.contains("SINTA 2"));

        let rev_path = dir.join("revision_matrix.docx");
        write_revision_matrix_docx(&rev_path, &item, None, Some(&profile), AppLanguage::Id)
            .unwrap();
        let rev_xml = read_docx_document_xml(&std::fs::read(&rev_path).unwrap());
        assert!(rev_xml.contains("MATRIKS REVISI"));
        assert!(rev_xml.contains("Reviewer"));
        assert!(rev_xml.contains("Tanggapan Penulis"));
        assert!(rev_xml.contains("w:tbl"));

        let appr_path = dir.join("dospen_approval.docx");
        write_dospen_approval_docx(&appr_path, &[&item], None, Some(&profile), AppLanguage::Id)
            .unwrap();
        let appr_xml = read_docx_document_xml(&std::fs::read(&appr_path).unwrap());
        assert!(appr_xml.contains("LEMBAR PERSETUJUAN DOSEN PEMBIMBING"));
        assert!(appr_xml.contains("Budi Pratama"));
        assert!(appr_xml.contains("22010099"));
        assert!(appr_xml.contains("Mahasiswa / Penulis Pertama"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
