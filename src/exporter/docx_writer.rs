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

        let _ = std::fs::remove_dir_all(&dir);
    }
}
