//! RIS (`.ris`) & BibTeX (`.bib`) Citation Exporter for Zotero & Mendeley (Feature #9).

use crate::error::JsfsError;
use crate::models::JournalRecord;
use std::path::Path;

/// Format a slice of `JournalRecord` items as standard RIS records.
pub fn format_ris(records: &[JournalRecord]) -> String {
    let mut out = String::with_capacity(records.len() * 320);
    for rec in records {
        out.push_str("TY  - JOUR\r\n");
        out.push_str(&format!("TI  - {}\r\n", rec.title.trim()));
        out.push_str(&format!("T2  - {}\r\n", rec.title.trim()));
        if !rec.publisher.trim().is_empty() && rec.publisher != "-" {
            out.push_str(&format!("PB  - {}\r\n", rec.publisher.trim()));
        }
        let issn = if !rec.eissn.trim().is_empty() && rec.eissn != "-" {
            rec.eissn.trim()
        } else {
            rec.issn.trim()
        };
        if !issn.is_empty() && issn != "-" {
            out.push_str(&format!("SN  - {issn}\r\n"));
        }
        let url = if !rec.website_url.trim().is_empty() && rec.website_url != "-" {
            rec.website_url.trim()
        } else if !rec.sinta_url.trim().is_empty() {
            rec.sinta_url.trim()
        } else {
            rec.garuda_url.trim()
        };
        if !url.is_empty() && url != "-" {
            out.push_str(&format!("UR  - {url}\r\n"));
        }
        if let Some(yr) = rec.last_issue_year {
            out.push_str(&format!("PY  - {yr}\r\n"));
        }
        let mut notes = Vec::new();
        if !rec.sinta_score.trim().is_empty() {
            notes.push(format!("Tier: {}", rec.sinta_score.trim()));
        }
        if !rec.apc_status.trim().is_empty() {
            notes.push(format!("APC: {}", rec.apc_status.trim()));
        }
        if !rec.turnaround_label.trim().is_empty() {
            notes.push(format!("Review: {}", rec.turnaround_label.trim()));
        }
        if !notes.is_empty() {
            out.push_str(&format!("N1  - {}\r\n", notes.join(" | ")));
        }
        if !rec.focus_and_scope.trim().is_empty() && rec.focus_and_scope != "-" {
            out.push_str(&format!(
                "AB  - {}\r\n",
                rec.focus_and_scope.replace(['\r', '\n'], " ").trim()
            ));
        }
        out.push_str("ER  - \r\n\r\n");
    }
    out
}

/// Format a slice of `JournalRecord` items as standard BibTeX `@periodical` entries.
pub fn format_bibtex(records: &[JournalRecord]) -> String {
    let mut out = String::with_capacity(records.len() * 340);
    for (idx, rec) in records.iter().enumerate() {
        let slug: String = rec
            .title
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(16)
            .collect::<String>()
            .to_ascii_lowercase();
        let key = if slug.is_empty() {
            format!("jsfs_journal_{}", idx + 1)
        } else {
            format!("{slug}_{}", idx + 1)
        };
        let esc = |s: &str| {
            s.replace(['{', '}', '"', '\r', '\n'], " ")
                .trim()
                .to_string()
        };

        out.push_str(&format!("@periodical{{{key},\n"));
        out.push_str(&format!("  title = {{{}}},\n", esc(&rec.title)));
        if !rec.publisher.trim().is_empty() && rec.publisher != "-" {
            out.push_str(&format!("  publisher = {{{}}},\n", esc(&rec.publisher)));
        }
        let issn = if !rec.eissn.trim().is_empty() && rec.eissn != "-" {
            rec.eissn.trim()
        } else {
            rec.issn.trim()
        };
        if !issn.is_empty() && issn != "-" {
            out.push_str(&format!("  issn = {{{}}},\n", esc(issn)));
        }
        let url = if !rec.website_url.trim().is_empty() && rec.website_url != "-" {
            rec.website_url.trim()
        } else if !rec.sinta_url.trim().is_empty() {
            rec.sinta_url.trim()
        } else {
            rec.garuda_url.trim()
        };
        if !url.is_empty() && url != "-" {
            out.push_str(&format!("  url = {{{}}},\n", esc(url)));
        }
        let year = rec.last_issue_year.unwrap_or(2026);
        out.push_str(&format!("  year = {{{year}}},\n"));
        out.push_str(&format!(
            "  note = {{Tier: {} | APC: {}}}\n",
            esc(&rec.sinta_score),
            esc(&rec.apc_status)
        ));
        out.push_str("}\n\n");
    }
    out
}

pub fn write_ris(path: &Path, records: &[JournalRecord]) -> Result<(), JsfsError> {
    let content = format_ris(records);
    std::fs::write(path, content)?;
    Ok(())
}

pub fn write_bibtex(path: &Path, records: &[JournalRecord]) -> Result<(), JsfsError> {
    let content = format_bibtex(records);
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ris_and_bibtex_formatting() {
        let rec = JournalRecord {
            title: "Jurnal Rekayasa Sistem Informasi".to_string(),
            publisher: "Universitas Nusantara".to_string(),
            issn: "1234-5678".to_string(),
            eissn: "8765-4321".to_string(),
            website_url: "https://jrsi.ac.id/index.php/jrsi".to_string(),
            sinta_score: "Sinta 2".to_string(),
            apc_status: "Gratis (Rp 0)".to_string(),
            turnaround_label: "~8 mgg".to_string(),
            last_issue_year: Some(2026),
            ..Default::default()
        };
        let ris = format_ris(std::slice::from_ref(&rec));
        assert!(ris.contains("TY  - JOUR"));
        assert!(ris.contains("TI  - Jurnal Rekayasa Sistem Informasi"));
        assert!(ris.contains("SN  - 8765-4321"));
        assert!(ris.contains("ER  - "));

        let bib = format_bibtex(&[rec]);
        assert!(bib.contains("@periodical{"));
        assert!(bib.contains("title = {Jurnal Rekayasa Sistem Informasi}"));
        assert!(bib.contains("issn = {8765-4321}"));
    }
}
