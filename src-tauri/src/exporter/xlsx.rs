//! Styled 3-sheet Excel (.xlsx) generator using pure-Rust `rust_xlsxwriter`.

use crate::config::{APC_UNKNOWN, INFO_NOT_FOUND};
use crate::error::JsfsError;
use crate::models::JournalRecord;
use chrono::{Datelike, Local};
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Url, Workbook};
use std::path::Path;

pub struct TopRecommendation<'a> {
    pub record: &'a JournalRecord,
    pub advisor_score: f64,
    pub reason: String,
}

pub fn select_top_recommendations(records: &[JournalRecord], top_n: usize) -> Vec<TopRecommendation<'_>> {
    let cur_month = Local::now().month() as u8;
    let near = [cur_month, (cur_month % 12) + 1, ((cur_month + 1) % 12) + 1];

    let mut scored: Vec<(f64, &JournalRecord, String)> = records
        .iter()
        .map(|r| {
            let mut score = r.match_score * 0.6;
            let mut reasons = Vec::new();
            match r.apc_category() {
                "free" => {
                    score += 25.0;
                    reasons.push("Gratis (Rp 0)".to_string());
                }
                "paid" if r.publication_fee_idr.is_some_and(|v| v <= 500_000) => {
                    score += 15.0;
                    reasons.push("APC Terjangkau".to_string());
                }
                _ => {}
            }
            if r.publish_month_nums.iter().any(|m| near.contains(m)) {
                score += 12.0;
                reasons.push("Terbit Terdekat".to_string());
            }
            if let Some(rank) = r.sinta_rank_num() {
                score += match rank {
                    1 | 2 => 10.0,
                    3 | 4 => 8.0,
                    _ => 5.0,
                };
                reasons.push(format!("SINTA {rank}"));
            }
            let reason = if reasons.is_empty() {
                "Kecocokan topik & kelengkapan metadata baik".to_string()
            } else {
                reasons.join(" + ")
            };
            (score, r, reason)
        })
        .collect();

    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored
        .into_iter()
        .take(top_n)
        .map(|(advisor_score, record, reason)| TopRecommendation {
            record,
            advisor_score,
            reason,
        })
        .collect()
}

pub fn write_xlsx(path: &Path, records: &[JournalRecord]) -> Result<(), JsfsError> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }

    let mut workbook = Workbook::new();

    let header_fmt = Format::new()
        .set_bold()
        .set_font_name("Segoe UI")
        .set_font_size(10)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x1B365D))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);

    let cell_even = Format::new()
        .set_font_name("Segoe UI")
        .set_font_size(10)
        .set_text_wrap()
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);

    let cell_odd = Format::new()
        .set_font_name("Segoe UI")
        .set_font_size(10)
        .set_background_color(Color::RGB(0xF9FAFB))
        .set_text_wrap()
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);

    let free_fmt = Format::new()
        .set_bold()
        .set_font_name("Segoe UI")
        .set_font_size(10)
        .set_font_color(Color::RGB(0x065F46))
        .set_background_color(Color::RGB(0xD1FAE5))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);

    let paid_fmt = Format::new()
        .set_bold()
        .set_font_name("Segoe UI")
        .set_font_size(10)
        .set_font_color(Color::RGB(0x92400E))
        .set_background_color(Color::RGB(0xFEF3C7))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);

    let unk_fmt = Format::new()
        .set_font_name("Segoe UI")
        .set_font_size(10)
        .set_font_color(Color::RGB(0x4B5563))
        .set_background_color(Color::RGB(0xF3F4F6))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);

    // ── Sheet 1: Data Jurnal ──────────────────────────────────────────────
    let ws1 = workbook.add_worksheet();
    ws1.set_name("Data Jurnal")?;
    let headers1 = [
        "No",
        "Nama Jurnal",
        "Penerbit",
        "SINTA",
        "Skor Cocok",
        "Biaya Publikasi (APC)",
        "Jadwal Terbit",
        "Terbit Terdekat",
        "Email Redaksi",
        "Telepon / WA",
        "Focus & Scope",
        "Link OJS",
    ];
    let widths1 = [5.0, 34.0, 24.0, 10.0, 12.0, 22.0, 26.0, 16.0, 26.0, 18.0, 42.0, 34.0];
    for (c, (h, w)) in headers1.iter().zip(widths1.iter()).enumerate() {
        let col = c as u16;
        ws1.write_string_with_format(0, col, *h, &header_fmt)?;
        ws1.set_column_width(col, *w)?;
    }
    ws1.set_row_height(0, 26)?;
    ws1.set_freeze_panes(1, 0)?;

    for (idx, r) in records.iter().enumerate() {
        let row = (idx + 1) as u32;
        let base_fmt = if idx % 2 == 0 { &cell_even } else { &cell_odd };
        let sinta = r
            .sinta_rank_num()
            .map(|n| format!("SINTA {n}"))
            .unwrap_or_else(|| "Garuda".to_string());
        let apc_fmt = match r.apc_category() {
            "free" => &free_fmt,
            "paid" => &paid_fmt,
            _ => &unk_fmt,
        };
        let apc_txt = if r.apc_status.is_empty() {
            APC_UNKNOWN
        } else {
            &r.apc_status
        };
        let url_str = if r.website_url.starts_with("http") {
            &r.website_url
        } else if r.garuda_url.starts_with("http") {
            &r.garuda_url
        } else {
            INFO_NOT_FOUND
        };

        ws1.write_number_with_format(row, 0, (idx + 1) as f64, base_fmt)?;
        ws1.write_string_with_format(row, 1, &r.title, base_fmt)?;
        ws1.write_string_with_format(row, 2, &r.publisher, base_fmt)?;
        ws1.write_string_with_format(row, 3, &sinta, base_fmt)?;
        ws1.write_string_with_format(row, 4, format!("{:.0}%", r.match_score), base_fmt)?;
        ws1.write_string_with_format(row, 5, apc_txt, apc_fmt)?;
        ws1.write_string_with_format(row, 6, &r.publish_schedule, base_fmt)?;
        ws1.write_string_with_format(row, 7, &r.next_issue_estimate, base_fmt)?;
        ws1.write_string_with_format(row, 8, &r.email, base_fmt)?;
        ws1.write_string_with_format(row, 9, &r.phone, base_fmt)?;
        ws1.write_string_with_format(row, 10, &r.focus_and_scope, base_fmt)?;
        if url_str.starts_with("http") {
            let _ = ws1.write_url_with_format(row, 11, Url::new(url_str), base_fmt);
        } else {
            ws1.write_string_with_format(row, 11, url_str, base_fmt)?;
        }
    }
    if !records.is_empty() {
        ws1.autofilter(0, 0, records.len() as u32, (headers1.len() - 1) as u16)?;
    }

    // ── Sheet 2: Rekomendasi AI (Top 3) ───────────────────────────────────
    let top3 = select_top_recommendations(records, 3);
    let ws2 = workbook.add_worksheet();
    ws2.set_name("Rekomendasi AI")?;
    let title_fmt = Format::new()
        .set_bold()
        .set_font_name("Segoe UI")
        .set_font_size(13)
        .set_font_color(Color::RGB(0x1B365D));
    ws2.write_string_with_format(0, 0, "TOP 3 REKOMENDASI JURNAL TERBAIK (AI ADVISOR)", &title_fmt)?;
    let headers2 = [
        "Peringkat",
        "Nama Jurnal",
        "SINTA",
        "Skor Advisor",
        "Biaya Publikasi",
        "Terbit Terdekat",
        "Alasan Rekomendasi",
        "Link OJS",
    ];
    let widths2 = [11.0, 34.0, 10.0, 14.0, 22.0, 16.0, 36.0, 34.0];
    for (c, (h, w)) in headers2.iter().zip(widths2.iter()).enumerate() {
        let col = c as u16;
        ws2.write_string_with_format(2, col, *h, &header_fmt)?;
        ws2.set_column_width(col, *w)?;
    }
    for (idx, rec) in top3.iter().enumerate() {
        let row = (idx + 3) as u32;
        let r = rec.record;
        let sinta = r
            .sinta_rank_num()
            .map(|n| format!("SINTA {n}"))
            .unwrap_or_else(|| "Garuda".to_string());
        let url_str = if r.website_url.starts_with("http") {
            &r.website_url
        } else {
            &r.garuda_url
        };
        ws2.write_string_with_format(row, 0, format!("#{}", idx + 1), &cell_even)?;
        ws2.write_string_with_format(row, 1, &r.title, &cell_even)?;
        ws2.write_string_with_format(row, 2, &sinta, &cell_even)?;
        ws2.write_string_with_format(row, 3, format!("{:.1} pts", rec.advisor_score), &cell_even)?;
        ws2.write_string_with_format(row, 4, &r.apc_status, &cell_even)?;
        ws2.write_string_with_format(row, 5, &r.next_issue_estimate, &cell_even)?;
        ws2.write_string_with_format(row, 6, &rec.reason, &cell_even)?;
        if url_str.starts_with("http") {
            let _ = ws2.write_url_with_format(row, 7, Url::new(url_str), &cell_even);
        } else {
            ws2.write_string_with_format(row, 7, url_str, &cell_even)?;
        }
    }

    // ── Sheet 3: Direktori Kontak ─────────────────────────────────────────
    let ws3 = workbook.add_worksheet();
    ws3.set_name("Direktori Kontak")?;
    let headers3 = [
        "No",
        "Nama Jurnal",
        "SINTA",
        "Email Redaksi",
        "Telepon / WhatsApp",
        "Link OJS",
    ];
    let widths3 = [5.0, 36.0, 10.0, 30.0, 22.0, 36.0];
    for (c, (h, w)) in headers3.iter().zip(widths3.iter()).enumerate() {
        let col = c as u16;
        ws3.write_string_with_format(0, col, *h, &header_fmt)?;
        ws3.set_column_width(col, *w)?;
    }
    ws3.set_freeze_panes(1, 0)?;
    for (idx, r) in records.iter().enumerate() {
        let row = (idx + 1) as u32;
        let base_fmt = if idx % 2 == 0 { &cell_even } else { &cell_odd };
        let sinta = r
            .sinta_rank_num()
            .map(|n| format!("SINTA {n}"))
            .unwrap_or_else(|| "Garuda".to_string());
        let url_str = if r.website_url.starts_with("http") {
            &r.website_url
        } else {
            &r.garuda_url
        };
        ws3.write_number_with_format(row, 0, (idx + 1) as f64, base_fmt)?;
        ws3.write_string_with_format(row, 1, &r.title, base_fmt)?;
        ws3.write_string_with_format(row, 2, &sinta, base_fmt)?;
        ws3.write_string_with_format(row, 3, &r.email, base_fmt)?;
        ws3.write_string_with_format(row, 4, &r.phone, base_fmt)?;
        if url_str.starts_with("http") {
            let _ = ws3.write_url_with_format(row, 5, Url::new(url_str), base_fmt);
        } else {
            ws3.write_string_with_format(row, 5, url_str, base_fmt)?;
        }
    }

    workbook.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CACHE_VERSION;

    #[test]
    fn test_xlsx_export_generates_valid_workbook() {
        let dir = std::env::temp_dir().join("jsfs_v3_test_exports");
        let path = dir.join("test_journals.xlsx");
        let rec = JournalRecord {
            title: "Jurnal Ekonomi & Bisnis Digital".to_string(),
            publisher: "Universitas Contoh".to_string(),
            sinta_score: "Sinta 4".to_string(),
            apc_status: "Gratis (Rp 0)".to_string(),
            publication_fee_idr: Some(0),
            publish_schedule: "2 kali setahun (Mei & November)".to_string(),
            next_issue_estimate: "Mei".to_string(),
            email: "redaksi@contoh.ac.id".to_string(),
            phone: "+6281234567890".to_string(),
            website_url: "https://ojs.contoh.ac.id/jebd".to_string(),
            match_score: 88.0,
            cache_version: CACHE_VERSION.to_string(),
            ..Default::default()
        };
        write_xlsx(&path, &[rec]).unwrap();
        let meta = std::fs::metadata(&path).unwrap();
        assert!(meta.len() > 2000);
        let _ = std::fs::remove_dir_all(dir);
    }
}
