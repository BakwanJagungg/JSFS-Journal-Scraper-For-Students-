//! Clean Markdown report generator for students and advisors.

use crate::config::{APC_UNKNOWN, INFO_NOT_FOUND};
use crate::error::JsfsError;
use crate::models::JournalRecord;
use chrono::Local;
use std::path::Path;

pub fn write_markdown(path: &Path, records: &[JournalRecord], keyword: &str) -> Result<(), JsfsError> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }

    let now = Local::now().format("%d %B %Y %H:%M").to_string();
    let mut lines = vec![
        format!("# Hasil Pencarian Jurnal Ilmiah — \"{keyword}\""),
        String::new(),
        format!("- **Tanggal:** {now}"),
        format!("- **Total Jurnal Valid:** {}", records.len()),
        format!(
            "- **Dihasilkan oleh:** JSFS v{} (Journal Scraper For Students)",
            crate::config::APP_VERSION
        ),
        String::new(),
        "## Ringkasan Tabel".to_string(),
        String::new(),
        "| No | Nama Jurnal | SINTA | Cocok | Biaya Publikasi | Jadwal Terbit | Link OJS |".to_string(),
        "|---:|---|:---:|:---:|---|---|---|".to_string(),
    ];

    for (i, r) in records.iter().enumerate() {
        let sinta = r
            .sinta_rank_num()
            .map(|n| format!("S{n}"))
            .unwrap_or_else(|| "Garuda".to_string());
        let apc = if r.apc_status.is_empty() {
            APC_UNKNOWN
        } else {
            &r.apc_status
        };
        let sched = if r.publish_schedule.is_empty() {
            "-"
        } else {
            &r.publish_schedule
        };
        let url = if r.website_url.starts_with("http") {
            &r.website_url
        } else {
            &r.garuda_url
        };
        let safe_title = r.title.replace('|', "\\|");
        lines.push(format!(
            "| {} | {} | {} | {:.0}% | {} | {} | [Buka]({}) |",
            i + 1,
            safe_title,
            sinta,
            r.match_score,
            apc.replace('|', "/"),
            sched.replace('|', "/"),
            url
        ));
    }

    lines.push(String::new());
    lines.push("## Detail & Kontak Redaksi".to_string());
    lines.push(String::new());

    for (i, r) in records.iter().enumerate() {
        lines.push(format!("### {}. {}", i + 1, r.title.trim()));
        if !r.publisher.is_empty() {
            lines.push(format!("- **Penerbit:** {}", r.publisher.trim()));
        }
        if !r.email.is_empty() && r.email != INFO_NOT_FOUND {
            lines.push(format!("- **Email Redaksi:** {}", r.email.trim()));
        }
        if !r.phone.is_empty() && r.phone != INFO_NOT_FOUND {
            lines.push(format!("- **Telepon / WhatsApp:** {}", r.phone.trim()));
        }
        if !r.focus_and_scope.is_empty() && r.focus_and_scope != INFO_NOT_FOUND {
            lines.push(format!("- **Focus & Scope:** {}", r.focus_and_scope.trim()));
        }
        lines.push(String::new());
    }

    std::fs::write(path, lines.join("\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_and_csv_export() {
        let dir = std::env::temp_dir().join("jsfs_v3_test_md_csv");
        let md_path = dir.join("out.md");
        let csv_path = dir.join("out.csv");
        let rec = JournalRecord {
            title: "Jurnal Ilmu Komputer".to_string(),
            sinta_score: "Sinta 3".to_string(),
            apc_status: "Rp 350.000".to_string(),
            match_score: 91.0,
            website_url: "https://ojs.tes.ac.id/jik".to_string(),
            ..Default::default()
        };
        write_markdown(&md_path, std::slice::from_ref(&rec), "informatika").unwrap();
        crate::exporter::csv_writer::write_csv(&csv_path, &[rec]).unwrap();
        let md = std::fs::read_to_string(&md_path).unwrap();
        let csv = std::fs::read(&csv_path).unwrap();
        assert!(md.contains("Jurnal Ilmu Komputer"));
        assert!(md.contains(&format!("JSFS v{}", crate::config::APP_VERSION)));
        assert_eq!(&csv[0..3], &[0xEF, 0xBB, 0xBF]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
