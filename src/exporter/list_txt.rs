//! Clean WhatsApp / plain-text numbered share list generator (`VISION.md` §5 & Pantangan V3-5).

use crate::config::APC_UNKNOWN;
use crate::error::JsfsError;
use crate::models::JournalRecord;
use std::path::Path;

fn is_missing(val: &str) -> bool {
    let t = val.trim();
    t.is_empty()
        || t == "-"
        || t.eq_ignore_ascii_case("nan")
        || t.to_ascii_lowercase().contains("tidak ditemukan")
}

/// Format clean shareable text list (`VISION.md` §5):
/// omits Email & Telepon lines when empty/not found so the list stays concise and readable.
pub fn generate_copypaste_list(records: &[JournalRecord]) -> String {
    if records.is_empty() {
        return "Tidak ada daftar jurnal untuk dibagikan.".to_string();
    }

    let mut blocks = Vec::with_capacity(records.len() + 1);
    blocks.push(format!(
        "📚 *DAFTAR JURNAL REKOMENDASI ({} Jurnal)*",
        records.len()
    ));

    for (idx, r) in records.iter().enumerate() {
        let num = idx + 1;
        let mut lines = Vec::new();
        let sinta = r
            .sinta_rank_num()
            .map(|n| format!("SINTA {n}"))
            .unwrap_or_else(|| "Garuda".to_string());
        lines.push(format!("{num}. *{}* ({sinta})", r.title.trim()));

        let apc_disp = if r.apc_status.trim().is_empty() {
            APC_UNKNOWN
        } else {
            r.apc_status.trim()
        };
        lines.push(format!("   Biaya Publikasi: {apc_disp}"));

        let sched = if is_missing(&r.publish_schedule) {
            "Belum terdata"
        } else {
            r.publish_schedule.trim()
        };
        lines.push(format!("   Jadwal Terbit: {sched}"));

        if !is_missing(&r.next_issue_estimate) {
            lines.push(format!(
                "   Terbit Terdekat: {}",
                r.next_issue_estimate.trim()
            ));
        }
        if !is_missing(&r.email) {
            lines.push(format!("   Email: {}", r.email.trim()));
        }
        if !is_missing(&r.phone) {
            lines.push(format!("   Telepon/WA: {}", r.phone.trim()));
        }
        let link = if r.website_url.starts_with("http") {
            r.website_url.trim()
        } else if r.garuda_url.starts_with("http") {
            r.garuda_url.trim()
        } else {
            "-"
        };
        lines.push(format!("   Link Web: {link}"));

        blocks.push(lines.join("\n"));
    }

    blocks.join("\n\n")
}

pub fn write_list_txt(path: &Path, records: &[JournalRecord]) -> Result<(), JsfsError> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let content = generate_copypaste_list(records);
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::INFO_NOT_FOUND;

    #[test]
    fn test_list_txt_hides_missing_contacts_per_vision_v3_5() {
        let rec = JournalRecord {
            title: "Jurnal Tes Mahasiswa".to_string(),
            sinta_score: "Sinta 4".to_string(),
            apc_status: "Gratis (Rp 0)".to_string(),
            publish_schedule: "2 kali setahun (Juni & Desember)".to_string(),
            next_issue_estimate: "Juni".to_string(),
            email: INFO_NOT_FOUND.to_string(),
            phone: String::new(),
            website_url: "https://ojs.kampus.ac.id/jtm".to_string(),
            ..Default::default()
        };
        let txt = generate_copypaste_list(&[rec]);
        assert!(txt.contains("Jurnal Tes Mahasiswa"));
        assert!(!txt.contains("Email:"));
        assert!(!txt.contains("Telepon/WA:"));
        assert!(txt.contains("Link Web: https://ojs.kampus.ac.id/jtm"));
    }
}
