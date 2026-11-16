//! Pure RFC 5545 (`.ics`) calendar reminder exporter for target journal submission deadlines.

use crate::config::output_dir;
use crate::error::JsfsError;
use crate::exporter::sanitize_slug;
use crate::models::UiJournalItem;
use chrono::{Datelike, Days, Local, NaiveDate, Utc};
use std::fs;
use std::path::Path;

fn parse_indonesian_or_english_month(token: &str) -> Option<u32> {
    match token.trim().to_ascii_lowercase().as_str() {
        "jan" | "januari" | "january" => Some(1),
        "feb" | "februari" | "february" => Some(2),
        "mar" | "maret" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "mei" | "may" => Some(5),
        "jun" | "juni" | "june" => Some(6),
        "jul" | "juli" | "july" => Some(7),
        "agu" | "ags" | "agustus" | "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "okt" | "oktober" | "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "des" | "desember" | "dec" | "december" => Some(12),
        _ => None,
    }
}

/// Infer an actionable submission reminder date ~35 days ahead of the next journal issue
/// (clamped to at least 7 days from today so reminders are always upcoming).
pub fn infer_submission_target_date(terbit_terdekat: &str, today: NaiveDate) -> NaiveDate {
    let min_future = today.checked_add_days(Days::new(7)).unwrap_or(today);

    let tokens: Vec<&str> = terbit_terdekat
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let parsed_month = tokens
        .iter()
        .find_map(|tok| parse_indonesian_or_english_month(tok));
    let parsed_year = tokens.iter().find_map(|tok| {
        tok.parse::<i32>()
            .ok()
            .filter(|&y| (2024..=2035).contains(&y))
    });

    if let Some(m) = parsed_month {
        let y = parsed_year.unwrap_or_else(|| {
            if m >= today.month() {
                today.year()
            } else {
                today.year() + 1
            }
        });
        if let Some(issue_date) = NaiveDate::from_ymd_opt(y, m, 1) {
            if let Some(pre_submit) = issue_date.checked_sub_days(Days::new(35)) {
                return pre_submit.max(min_future);
            }
        }
    }

    today.checked_add_days(Days::new(14)).unwrap_or(min_future)
}

fn escape_ics_text(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace(['\r', '\n'], "\\n")
}

/// Export a standard `.ics` calendar reminder for submitting to `journal` directly at `dest_path`.
pub fn export_submission_ics_to(
    dest_path: &Path,
    journal: &UiJournalItem,
) -> Result<String, JsfsError> {
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let today = Local::now().date_naive();
    let target_date = infer_submission_target_date(&journal.terbit_terdekat, today);
    let end_date = target_date
        .checked_add_days(Days::new(1))
        .unwrap_or(target_date);
    let dt_start = target_date.format("%Y%m%d");
    let dt_end = end_date.format("%Y%m%d");
    let dt_stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let slug = sanitize_slug(&journal.nama);
    let uid = format!("jsfs-{slug}-{dt_start}@jsfs.local");

    let summary = escape_ics_text(&format!("Submit Naskah ke {}", journal.nama.trim()));
    let desc = escape_ics_text(&format!(
        "Target Edisi: {} | APC: {} | Estimasi Review: {} | Gaya Sitasi: {} | URL OJS: {}",
        journal.terbit_terdekat.trim(),
        journal.apc_label.trim(),
        journal.turnaround_label.trim(),
        journal.citation_style.as_deref().unwrap_or("-"),
        journal.url.trim(),
    ));
    let url = journal.url.trim();

    let ics = format!(
        "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//JSFS//Journal Scraper For Students//EN\r\n\
CALSCALE:GREGORIAN\r\n\
METHOD:PUBLISH\r\n\
BEGIN:VEVENT\r\n\
UID:{uid}\r\n\
DTSTAMP:{dt_stamp}\r\n\
DTSTART;VALUE=DATE:{dt_start}\r\n\
DTEND;VALUE=DATE:{dt_end}\r\n\
SUMMARY:{summary}\r\n\
DESCRIPTION:{desc}\r\n\
URL:{url}\r\n\
BEGIN:VALARM\r\n\
TRIGGER:-P3D\r\n\
ACTION:DISPLAY\r\n\
DESCRIPTION:Pengingat 3 hari sebelum target submit naskah\r\n\
END:VALARM\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n"
    );

    fs::write(dest_path, ics.as_bytes())?;
    Ok(dest_path.to_string_lossy().into_owned())
}

/// Export a standard `.ics` submission reminder to `output/` and return the saved path.
pub fn export_submission_ics(journal: &UiJournalItem) -> Result<String, JsfsError> {
    let slug = sanitize_slug(&journal.nama);
    let dest = output_dir().join(format!("Pengingat_Submit_{slug}.ics"));
    export_submission_ics_to(&dest, journal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_submission_ics_creation_and_content() {
        let dir = std::env::temp_dir().join("jsfs_v49_ics_test");
        let _ = fs::remove_dir_all(&dir);
        let dest = dir.join("reminder.ics");

        let item = UiJournalItem {
            nama: "Jurnal Teknik Informatika".to_string(),
            apc_label: "Gratis (Rp 0)".to_string(),
            terbit_terdekat: "Desember 2028".to_string(),
            turnaround_label: "~6 mgg".to_string(),
            url: "https://ojs.kampus.ac.id/jti".to_string(),
            ..Default::default()
        };

        export_submission_ics_to(&dest, &item).unwrap();
        let content = fs::read_to_string(&dest).unwrap();
        assert!(content.contains("BEGIN:VCALENDAR"));
        assert!(content.contains("BEGIN:VEVENT"));
        assert!(content.contains("SUMMARY:Submit Naskah ke Jurnal Teknik Informatika"));
        assert!(content.contains("DTSTART;VALUE=DATE:20281027"));
        assert!(content.contains("END:VCALENDAR"));

        let _ = fs::remove_dir_all(&dir);
    }
}
