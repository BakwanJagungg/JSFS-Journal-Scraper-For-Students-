pub mod csv_writer;
pub mod list_txt;
pub mod markdown;
pub mod xlsx;

use crate::config::output_dir;
use crate::error::JsfsError;
use crate::models::JournalRecord;
use chrono::Local;
use std::path::PathBuf;

pub struct ExportBundle {
    pub excel_path: Option<String>,
    pub markdown_path: Option<String>,
    pub csv_path: Option<String>,
    pub list_path: Option<String>,
}

fn sanitize_slug(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        "jurnal".to_string()
    } else {
        trimmed.chars().take(36).collect()
    }
}

/// Synchronously write requested export formats into `output/` (call via `tokio::task::spawn_blocking`).
pub fn write_all_exports_sync(
    records: &[JournalRecord],
    keyword: &str,
    formats: &[String],
    is_checkpoint: bool,
) -> Result<ExportBundle, JsfsError> {
    let out = output_dir();
    std::fs::create_dir_all(&out)?;
    let slug = sanitize_slug(keyword);
    let ts = if is_checkpoint {
        "AUTOSAVE".to_string()
    } else {
        Local::now().format("%Y%m%d_%H%M%S").to_string()
    };
    let stem = format!("JSFS_{slug}_{ts}");

    let want =
        |ext: &str| formats.is_empty() || formats.iter().any(|f| f.eq_ignore_ascii_case(ext));

    let mut bundle = ExportBundle {
        excel_path: None,
        markdown_path: None,
        csv_path: None,
        list_path: None,
    };

    if want(".xlsx") {
        let p: PathBuf = out.join(format!("{stem}.xlsx"));
        xlsx::write_xlsx(&p, records)?;
        bundle.excel_path = Some(p.to_string_lossy().into_owned());
    }
    if want(".md") && !is_checkpoint {
        let p: PathBuf = out.join(format!("{stem}.md"));
        markdown::write_markdown(&p, records, keyword)?;
        bundle.markdown_path = Some(p.to_string_lossy().into_owned());
    }
    if want(".csv") && !is_checkpoint {
        let p: PathBuf = out.join(format!("{stem}.csv"));
        csv_writer::write_csv(&p, records)?;
        bundle.csv_path = Some(p.to_string_lossy().into_owned());
    }
    if !is_checkpoint {
        let p: PathBuf = out.join(format!("{stem}_LIST.txt"));
        list_txt::write_list_txt(&p, records)?;
        bundle.list_path = Some(p.to_string_lossy().into_owned());
    }

    Ok(bundle)
}

/// Synchronously write a single targeted export format (`.xlsx`, `.md`, or `.csv`) into `output/`.
pub fn write_single_export_sync(
    records: &[JournalRecord],
    keyword: &str,
    ext: &str,
    scope_slug: &str,
) -> Result<String, JsfsError> {
    let out = output_dir();
    std::fs::create_dir_all(&out)?;
    let slug = sanitize_slug(keyword);
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let tag = if scope_slug.trim().is_empty() {
        String::new()
    } else {
        format!("_{}", sanitize_slug(scope_slug))
    };
    let clean_ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();
    let path: PathBuf = out.join(format!("JSFS_{slug}{tag}_{ts}.{clean_ext}"));
    match clean_ext.as_str() {
        "xlsx" => xlsx::write_xlsx(&path, records)?,
        "md" => markdown::write_markdown(&path, records, keyword)?,
        "csv" => csv_writer::write_csv(&path, records)?,
        other => {
            return Err(JsfsError::InvalidInput(format!(
                "Format ekspor tidak didukung: .{other}"
            )));
        }
    }
    Ok(path.to_string_lossy().into_owned())
}
