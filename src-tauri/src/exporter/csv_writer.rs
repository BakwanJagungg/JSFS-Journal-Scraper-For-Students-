//! UTF-8 with BOM CSV writer for Microsoft Excel compatibility.

use crate::error::JsfsError;
use crate::models::JournalRecord;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn csv_escape(field: &str) -> String {
    let clean = field.replace(['\r', '\n'], " ");
    if clean.contains(',') || clean.contains('"') || clean.contains(';') {
        format!("\"{}\"", clean.replace('"', "\"\""))
    } else {
        clean
    }
}

pub fn write_csv(path: &Path, records: &[JournalRecord]) -> Result<(), JsfsError> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut f = File::create(path)?;
    // UTF-8 BOM so Indonesian Excel parses UTF-8 cleanly
    f.write_all(&[0xEF, 0xBB, 0xBF])?;

    writeln!(
        f,
        "No,Nama Jurnal,Penerbit,Sumber,Peringkat SINTA,Skor Kecocokan (%),Kategori Cocok,Biaya Publikasi,Jadwal Terbit,Terbit Terdekat,ISSN,E-ISSN,Email Redaksi,Telepon/WA,Focus & Scope,Link OJS"
    )?;

    for (i, r) in records.iter().enumerate() {
        let sinta = r
            .sinta_rank_num()
            .map(|n| format!("SINTA {n}"))
            .unwrap_or_else(|| "Garuda".to_string());
        let url = if r.website_url.starts_with("http") {
            &r.website_url
        } else {
            &r.garuda_url
        };
        let row = [
            (i + 1).to_string(),
            csv_escape(&r.title),
            csv_escape(&r.publisher),
            csv_escape(&r.source_portal),
            csv_escape(&sinta),
            format!("{:.0}", r.match_score),
            csv_escape(&r.match_label),
            csv_escape(&r.apc_status),
            csv_escape(&r.publish_schedule),
            csv_escape(&r.next_issue_estimate),
            csv_escape(&r.issn),
            csv_escape(&r.eissn),
            csv_escape(&r.email),
            csv_escape(&r.phone),
            csv_escape(&r.focus_and_scope),
            csv_escape(url),
        ]
        .join(",");
        writeln!(f, "{row}")?;
    }

    Ok(())
}
