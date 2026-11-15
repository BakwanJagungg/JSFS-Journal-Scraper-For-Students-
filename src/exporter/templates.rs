//! Cover Letter, Editor Pre-Submission Inquiry Email, and Supervisor Brief Generators (JSFS v4.5).

use crate::engine::manuscript::ManuscriptSummary;
use crate::models::UiJournalItem;
use crate::ui::i18n::AppLanguage;
use chrono::Local;

fn resolve_manuscript_fields<'a>(
    fallback_title: &'a str,
    manuscript: Option<&'a ManuscriptSummary>,
    lang: &str,
) -> (String, String, String) {
    let raw_title = manuscript
        .map(|m| m.extracted_title.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_title.trim());

    let title = if raw_title.is_empty() {
        if lang.eq_ignore_ascii_case("en") {
            "[Manuscript Title]".to_string()
        } else {
            "[Judul Naskah Anda]".to_string()
        }
    } else {
        raw_title.to_string()
    };

    let keywords = manuscript
        .map(|m| m.suggested_keyword.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let abstract_snippet = manuscript
        .map(|m| m.abstract_snippet.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_default();

    (title, keywords, abstract_snippet)
}

/// Generate a formal academic cover letter for submitting a manuscript to `journal` (`lang` = `"id"` or `"en"`).
/// Auto-fills title, keywords, and abstract summary when `manuscript` is loaded.
pub fn generate_cover_letter(
    journal: &UiJournalItem,
    manuscript_title: &str,
    manuscript: Option<&ManuscriptSummary>,
    lang: &str,
) -> String {
    let today = Local::now().format("%d %B %Y").to_string();
    let (title, keywords, abstract_snippet) =
        resolve_manuscript_fields(manuscript_title, manuscript, lang);
    let publisher_line = if !journal.penerbit.trim().is_empty() && journal.penerbit != "-" {
        format!("\n{}", journal.penerbit.trim())
    } else {
        String::new()
    };
    let scope_snippet = if !journal.focus_scope.trim().is_empty() && journal.focus_scope != "-" {
        let s: String = journal.focus_scope.chars().take(160).collect();
        if journal.focus_scope.chars().count() > 160 {
            format!("{s}...")
        } else {
            s
        }
    } else if lang.eq_ignore_ascii_case("en") {
        "the scholarly focus and scope of your journal".to_string()
    } else {
        "fokus dan ruang lingkup keilmuan jurnal Bapak/Ibu".to_string()
    };

    let abstract_block = if abstract_snippet.is_empty() {
        String::new()
    } else if lang.eq_ignore_ascii_case("en") {
        format!("\n\nBrief Summary:\n{abstract_snippet}")
    } else {
        format!("\n\nRingkasan Abstrak:\n{abstract_snippet}")
    };

    let kw_block = if keywords.is_empty() {
        String::new()
    } else if lang.eq_ignore_ascii_case("en") {
        format!("\nKeywords: {keywords}")
    } else {
        format!("\nKata Kunci: {keywords}")
    };

    if lang.eq_ignore_ascii_case("en") {
        format!(
            "{today}\n\n\
To:\n\
Editor-in-Chief\n\
{journal_name}{publisher_line}\n\n\
Subject: Manuscript Submission – \"{title}\"\n\n\
Dear Editor-in-Chief,\n\n\
We would like to submit our original research manuscript entitled \"{title}\" for consideration for publication in {journal_name}.{abstract_block}{kw_block}\n\n\
Our study aligns closely with the scope of {journal_name} ({scope_snippet}). We confirm that this manuscript is our original work, has not been published previously, and is not currently under consideration for publication elsewhere.\n\n\
All authors have approved the manuscript and agree with its submission to {journal_name}. The authors declare no conflict of interest.\n\n\
Thank you very much for your time and consideration. We look forward to hearing from you.\n\n\
Sincerely,\n\n\
[Nama Penulis Korespondensi / Corresponding Author]\n\
[Afiliasi / Universitas / Institution]\n\
[Email Korespondensi / Email Address]",
            journal_name = journal.nama.trim()
        )
    } else {
        format!(
            "{today}\n\n\
Kepada Yth.\n\
Dewan Redaksi / Editor-in-Chief\n\
{journal_name}{publisher_line}\n\n\
Perihal: Permohonan Publikasi Naskah Ilmiah – \"{title}\"\n\n\
Dengan hormat,\n\n\
Melalui surat pengantar ini, kami bermaksud mengirimkan naskah artikel ilmiah hasil penelitian kami yang berjudul:\n\
\"{title}\"\n\
untuk dipertimbangkan dan ditinjau (peer-review) guna publikasi pada {journal_name}.{abstract_block}{kw_block}\n\n\
Topik penelitian kami selaras dengan fokus dan ruang lingkup {journal_name} ({scope_snippet}). Kami menyatakan bahwa naskah ini adalah karya asli (orisinal), belum pernah dipublikasikan sebelumnya, dan tidak sedang dalam proses peninjauan pada jurnal atau penerbit lain.\n\n\
Seluruh penulis telah membaca dan menyetujui pengiriman naskah ini serta menyatakan tidak terdapat konflik kepentingan (conflict of interest).\n\n\
Atas perhatian dan kesempatan yang Bapak/Ibu berikan, kami ucapkan terima kasih.\n\n\
Hormat kami,\n\n\
[Nama Penulis Korespondensi]\n\
[Program Studi / Fakultas / Universitas]\n\
[Email & Nomor Kontak]",
            journal_name = journal.nama.trim()
        )
    }
}

/// Generate a polite pre-submission scope, schedule, and APC inquiry email for the journal editor.
pub fn generate_editor_inquiry_email(
    journal: &UiJournalItem,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    lang: &str,
) -> String {
    let (title, keywords, abstract_snippet) = resolve_manuscript_fields(keyword, manuscript, lang);
    let journal_name = journal.nama.trim();
    let issue_hint = if !journal.terbit_terdekat.trim().is_empty() && journal.terbit_terdekat != "-"
    {
        journal.terbit_terdekat.trim()
    } else if lang.eq_ignore_ascii_case("en") {
        "the upcoming issue"
    } else {
        "edisi terbitan mendatang"
    };

    let abstract_block = if abstract_snippet.is_empty() {
        String::new()
    } else if lang.eq_ignore_ascii_case("en") {
        format!("\n\nManuscript Summary:\n{abstract_snippet}")
    } else {
        format!("\n\nRingkasan Naskah:\n{abstract_snippet}")
    };

    let kw_line = if keywords.is_empty() {
        String::new()
    } else if lang.eq_ignore_ascii_case("en") {
        format!("\nKeywords: {keywords}")
    } else {
        format!("\nKata Kunci: {keywords}")
    };

    if lang.eq_ignore_ascii_case("en") {
        format!(
            "Subject: Pre-Submission Inquiry – \"{title}\" ({journal_name})\n\n\
Dear Editor-in-Chief of {journal_name},\n\n\
I hope this email finds you well. My co-authors and I are preparing a manuscript entitled:\n\
\"{title}\"{abstract_block}{kw_line}\n\n\
Before submitting formally through your OJS portal, I would appreciate your brief guidance on two points:\n\
1. Does this manuscript align with the current scope and priority for {issue_hint}?\n\
2. Could you confirm the estimated peer-review turnaround time and applicable publication fee (APC: {apc})?\n\n\
Thank you very much for your time and kind assistance.\n\n\
Best regards,\n\
[Nama Lengkap / Full Name]\n\
[Program Studi / Institution]\n\
[Email & Nomor WhatsApp]",
            apc = journal.apc_label.trim()
        )
    } else {
        format!(
            "Subjek: Konsultasi Pra-Submit Naskah – \"{title}\" ({journal_name})\n\n\
Kepada Yth.\n\
Dewan Redaksi / Editor-in-Chief {journal_name},\n\n\
Dengan hormat,\n\
Saya bermaksud mengajukan konsultasi singkat pra-submit terkait rencana pengiriman naskah artikel ilmiah kami yang berjudul:\n\
\"{title}\"{abstract_block}{kw_line}\n\n\
Sebelum melakukan submit resmi melalui laman OJS, mohon konfirmasi Bapak/Ibu terkait hal berikut:\n\
1. Apakah topik naskah kami sesuai dengan fokus jurnal untuk terbitan terdekat ({issue_hint})?\n\
2. Mohon informasi perkiraan durasi proses review serta rincian biaya publikasi (data awal: {apc}).\n\n\
Atas perhatian dan kesediaan Bapak/Ibu meluangkan waktu untuk membalas email ini, saya ucapkan terima kasih.\n\n\
Hormat saya,\n\
[Nama Lengkap]\n\
[Program Studi / Universitas]\n\
[Email & Nomor WhatsApp]",
            apc = journal.apc_label.trim()
        )
    }
}

/// Format top-5 scoped journals into a crisp WhatsApp/Email brief for a thesis supervisor (Dosbing).
pub fn generate_dosbing_brief(records: &[&UiJournalItem], lang: AppLanguage) -> String {
    if records.is_empty() {
        return match lang {
            AppLanguage::Id => {
                "Belum ada jurnal terpilih untuk diringkas ke dosen pembimbing.".to_string()
            }
            AppLanguage::En => {
                "No journals selected to summarize for your supervisor yet.".to_string()
            }
        };
    }

    let is_en = matches!(lang, AppLanguage::En);
    let shown: Vec<&UiJournalItem> = records.iter().copied().take(5).collect();
    let mut lines = Vec::new();
    lines.push(if is_en {
        format!(
            "📋 *Target Journal Recommendations for Supervisor Review* (Top {} of {})",
            shown.len(),
            records.len()
        )
    } else {
        format!(
            "📋 *Rekomendasi Target Jurnal untuk Konsultasi Dosen Pembimbing* ({} Teratas dari {} Pilihan)",
            shown.len(),
            records.len()
        )
    });
    lines.push(String::new());

    for (idx, j) in shown.iter().enumerate() {
        let mut rank_tags = Vec::new();
        if !j.sinta.trim().is_empty() && j.sinta != "-" {
            let s = j.sinta.trim();
            if s.to_ascii_uppercase().starts_with("SINTA") {
                rank_tags.push(s.to_string());
            } else {
                rank_tags.push(format!("SINTA {s}"));
            }
        }
        if let Some(ref q) = j.quartile {
            if !q.trim().is_empty() {
                rank_tags.push(q.trim().to_string());
            }
        }
        let badge = if rank_tags.is_empty() {
            j.sumber.trim().to_string()
        } else {
            rank_tags.join(" / ")
        };

        let match_str = if j.match_score > 0.0 {
            format!("{:.0}% ({})", j.match_score, j.kesesuaian_naskah)
        } else {
            "-".to_string()
        };
        let turnaround = if j.turnaround_label.trim().is_empty() {
            "-".to_string()
        } else {
            j.turnaround_label.trim().to_string()
        };
        let next_issue = if j.terbit_terdekat.trim().is_empty() {
            "-"
        } else {
            j.terbit_terdekat.trim()
        };
        let link = if j.url.starts_with("http") {
            j.url.trim()
        } else if j.garuda_url.starts_with("http") {
            j.garuda_url.trim()
        } else {
            j.sinta_url.trim()
        };

        if is_en {
            lines.push(format!("{}. *{}* [{}]", idx + 1, j.nama.trim(), badge));
            lines.push(format!(
                "   • Match: {match_str} | APC: {} | Review: {turnaround} | Next Issue: {next_issue}",
                j.apc_label.trim()
            ));
            if !link.is_empty() {
                lines.push(format!("   • Link: {link}"));
            }
        } else {
            lines.push(format!("{}. *{}* [{}]", idx + 1, j.nama.trim(), badge));
            lines.push(format!(
                "   • Kecocokan: {match_str} | Biaya APC: {} | Review: {turnaround} | Terbit: {next_issue}",
                j.apc_label.trim()
            ));
            if !link.is_empty() {
                lines.push(format!("   • Link OJS: {link}"));
            }
        }
    }

    if is_en {
        lines.push(String::new());
        lines.push("Mohon arahan / Please advise which target journal we should prioritize for submission.".to_string());
    } else {
        lines.push(String::new());
        lines.push("Mohon arahan Bapak/Ibu terkait target jurnal prioritas untuk proses submit naskah. Terima kasih.".to_string());
    }

    lines.join("\n")
}

/// Generate a polite follow-up email asking the editor about review status based on `submitted_date`.
pub fn generate_followup_email(
    journal: &UiJournalItem,
    manuscript_title: &str,
    manuscript: Option<&ManuscriptSummary>,
    submitted_date: &str,
    lang: &str,
) -> String {
    let (title, _, _) = resolve_manuscript_fields(manuscript_title, manuscript, lang);
    let journal_name = journal.nama.trim();
    let date_label = if submitted_date.trim().is_empty() {
        if lang.eq_ignore_ascii_case("en") {
            "[Submission Date]".to_string()
        } else {
            "[Tanggal Submit]".to_string()
        }
    } else {
        submitted_date.trim().to_string()
    };

    if lang.eq_ignore_ascii_case("en") {
        format!(
            "Subject: Manuscript Review Status Inquiry – \"{title}\" ({journal_name})\n\n\
Dear Editor-in-Chief of {journal_name},\n\n\
I hope you are doing well. I am writing to respectfully follow up on the review status of our manuscript entitled:\n\
\"{title}\"\n\
which was submitted via your OJS portal on {date_label}.\n\n\
Could you kindly provide a brief update on whether the manuscript is currently in peer review or awaiting an editorial decision? We remain enthusiastic about publishing with {journal_name} and are happy to supply any additional information needed.\n\n\
Thank you very much for your time and assistance.\n\n\
Best regards,\n\
[Nama Lengkap / Corresponding Author]\n\
[ID Naskah OJS (jika ada)]\n\
[Email & Nomor WhatsApp]"
        )
    } else {
        format!(
            "Subjek: Follow-Up Perkembangan Review Naskah – \"{title}\" ({journal_name})\n\n\
Kepada Yth.\n\
Dewan Redaksi / Editor-in-Chief {journal_name},\n\n\
Dengan hormat,\n\
Melalui email ini, saya bermaksud menanyakan perkembangan proses peninjauan (peer-review) atas naskah artikel ilmiah kami yang berjudul:\n\
\"{title}\"\n\
yang telah kami kirimkan melalui sistem OJS pada tanggal {date_label}.\n\n\
Mohon berkenan Bapak/Ibu memberikan informasi singkat mengenai status terkini naskah tersebut agar kami dapat menyesuaikan jadwal penyempurnaan atau revisi bila diperlukan.\n\n\
Atas perhatian dan bantuan Bapak/Ibu, kami ucapkan terima kasih banyak.\n\n\
Hormat saya,\n\
[Nama Lengkap / Penulis Korespondensi]\n\
[ID Submission OJS (jika ada)]\n\
[Email & Nomor WhatsApp]"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cover_letter_and_inquiry_autofill() {
        let item = UiJournalItem {
            nama: "Jurnal Sistem Informasi Nasional".to_string(),
            penerbit: "Universitas Contoh".to_string(),
            focus_scope: "Sistem informasi bisnis, tata kelola TI, dan kecerdasan buatan"
                .to_string(),
            apc_label: "Rp 500.000".to_string(),
            terbit_terdekat: "November 2026".to_string(),
            ..Default::default()
        };
        let ms = ManuscriptSummary {
            extracted_title: "Deteksi Hoaks Otomatis Berbasis Transformer".to_string(),
            suggested_keyword: "transformer, nlp, hoaks".to_string(),
            abstract_snippet: "Penelitian ini mengevaluasi akurasi model Transformer untuk klasifikasi berita berbahasa Indonesia.".to_string(),
            ..Default::default()
        };

        let id_letter = generate_cover_letter(&item, "Fallback Title", Some(&ms), "id");
        assert!(id_letter.contains("Deteksi Hoaks Otomatis Berbasis Transformer"));
        assert!(id_letter.contains("Penelitian ini mengevaluasi akurasi"));
        assert!(id_letter.contains("transformer, nlp, hoaks"));

        let en_email = generate_editor_inquiry_email(&item, "Fallback", Some(&ms), "en");
        assert!(en_email.contains("Pre-Submission Inquiry"));
        assert!(en_email.contains("Deteksi Hoaks Otomatis Berbasis Transformer"));
        assert!(en_email.contains("APC: Rp 500.000"));
        assert!(en_email.contains("November 2026"));

        let empty_letter = generate_cover_letter(&item, "", None, "id");
        assert!(empty_letter.contains("[Judul Naskah Anda]"));
    }

    #[test]
    fn test_generate_dosbing_brief() {
        let j1 = UiJournalItem {
            nama: "Jurnal Informatika Unggul".to_string(),
            sinta: "2".to_string(),
            apc_label: "Gratis (Rp 0)".to_string(),
            terbit_terdekat: "Desember 2026".to_string(),
            turnaround_label: "~8 mgg".to_string(),
            match_score: 88.0,
            kesesuaian_naskah: "Sangat Cocok".to_string(),
            url: "https://ojs.kampus.ac.id/jiu".to_string(),
            ..Default::default()
        };
        let j2 = UiJournalItem {
            nama: "Indonesian Journal of AI".to_string(),
            sinta: "1".to_string(),
            quartile: Some("Q2".to_string()),
            apc_label: "Rp 2.475.000".to_string(),
            terbit_terdekat: "Januari 2027".to_string(),
            turnaround_label: "~3 bln".to_string(),
            match_score: 79.0,
            kesesuaian_naskah: "Cocok".to_string(),
            url: "https://ijai.org".to_string(),
            ..Default::default()
        };

        let brief = generate_dosbing_brief(&[&j1, &j2], AppLanguage::Id);
        assert!(brief.contains("Jurnal Informatika Unggul"));
        assert!(brief.contains("[SINTA 2]"));
        assert!(brief.contains("Gratis (Rp 0)"));
        assert!(brief.contains("88% (Sangat Cocok)"));
        assert!(brief.contains("[SINTA 1 / Q2]"));
        assert!(brief.contains("https://ijai.org"));
    }

    #[test]
    fn test_generate_followup_email_id_and_en() {
        let item = UiJournalItem {
            nama: "Jurnal Teknologi Nusantara".to_string(),
            ..Default::default()
        };
        let ms = ManuscriptSummary {
            extracted_title: "Optimasi Model CNN pada Citra Medis".to_string(),
            ..Default::default()
        };

        let id_msg = generate_followup_email(&item, "", Some(&ms), "2026-09-15", "id");
        assert!(id_msg.contains("Follow-Up Perkembangan Review Naskah"));
        assert!(id_msg.contains("Optimasi Model CNN pada Citra Medis"));
        assert!(id_msg.contains("2026-09-15"));

        let en_msg = generate_followup_email(&item, "Fallback Paper", None, "", "en");
        assert!(en_msg.contains("Manuscript Review Status Inquiry"));
        assert!(en_msg.contains("Fallback Paper"));
        assert!(en_msg.contains("[Submission Date]"));
    }
}
