//! Cover Letter, Editor Pre-Submission Inquiry Email, Originality Statement, and Supervisor Brief Generators (JSFS v4.7).

use crate::engine::manuscript::ManuscriptSummary;
use crate::models::UiJournalItem;
use crate::ui::i18n::{AppLanguage, AppSettings};
use chrono::Local;

fn resolve_author_signoff(profile: Option<&AppSettings>, lang: &str) -> (String, String, String) {
    let is_en = lang.eq_ignore_ascii_case("en");
    let raw_name = profile.map(|p| p.author_name.trim()).unwrap_or("");
    let raw_id = profile.map(|p| p.author_id.trim()).unwrap_or("");
    let raw_aff = profile.map(|p| p.author_affiliation.trim()).unwrap_or("");
    let raw_email = profile.map(|p| p.author_email.trim()).unwrap_or("");

    let name = if raw_name.is_empty() {
        if is_en {
            "[Nama Penulis Korespondensi / Corresponding Author]".to_string()
        } else {
            "[Nama Penulis Korespondensi]".to_string()
        }
    } else if raw_id.is_empty() {
        raw_name.to_string()
    } else {
        format!("{raw_name} (ID/NIM: {raw_id})")
    };

    let aff = if raw_aff.is_empty() {
        if is_en {
            "[Afiliasi / Universitas / Institution]".to_string()
        } else {
            "[Program Studi / Fakultas / Universitas]".to_string()
        }
    } else {
        raw_aff.to_string()
    };

    let email = if raw_email.is_empty() {
        if is_en {
            "[Email Korespondensi / Email Address]".to_string()
        } else {
            "[Email & Nomor Kontak]".to_string()
        }
    } else {
        raw_email.to_string()
    };

    (name, aff, email)
}

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

/// Generate a formal academic cover letter with optional author bio auto-fill from `AppSettings`.
pub fn generate_cover_letter_with_profile(
    journal: &UiJournalItem,
    manuscript_title: &str,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> String {
    let today = Local::now().format("%d %B %Y").to_string();
    let (title, keywords, abstract_snippet) =
        resolve_manuscript_fields(manuscript_title, manuscript, lang);
    let (author_name, author_aff, author_email) = resolve_author_signoff(profile, lang);
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
{author_name}\n\
{author_aff}\n\
{author_email}",
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
{author_name}\n\
{author_aff}\n\
{author_email}",
            journal_name = journal.nama.trim()
        )
    }
}

/// Generate a formal academic cover letter for submitting a manuscript to `journal` (`lang` = `"id"` or `"en"`).
/// Auto-fills title, keywords, and abstract summary when `manuscript` is loaded.
pub fn generate_cover_letter(
    journal: &UiJournalItem,
    manuscript_title: &str,
    manuscript: Option<&ManuscriptSummary>,
    lang: &str,
) -> String {
    generate_cover_letter_with_profile(journal, manuscript_title, manuscript, None, lang)
}

/// Generate a polite pre-submission scope, schedule, and APC inquiry email with optional author bio auto-fill.
pub fn generate_editor_inquiry_email_with_profile(
    journal: &UiJournalItem,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> String {
    let (title, keywords, abstract_snippet) = resolve_manuscript_fields(keyword, manuscript, lang);
    let (author_name, author_aff, author_email) = resolve_author_signoff(profile, lang);
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
{author_name}\n\
{author_aff}\n\
{author_email}",
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
{author_name}\n\
{author_aff}\n\
{author_email}",
            apc = journal.apc_label.trim()
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
    generate_editor_inquiry_email_with_profile(journal, keyword, manuscript, None, lang)
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

/// Generate a polite follow-up email with optional author bio auto-fill from `AppSettings`.
pub fn generate_followup_email_with_profile(
    journal: &UiJournalItem,
    manuscript_title: &str,
    manuscript: Option<&ManuscriptSummary>,
    submitted_date: &str,
    profile: Option<&AppSettings>,
    lang: &str,
) -> String {
    let (title, _, _) = resolve_manuscript_fields(manuscript_title, manuscript, lang);
    let (author_name, author_aff, author_email) = resolve_author_signoff(profile, lang);
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
{author_name}\n\
{author_aff}\n\
[Manuscript OJS ID (if any)]\n\
{author_email}"
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
{author_name}\n\
{author_aff}\n\
[ID Submission OJS (jika ada)]\n\
{author_email}"
        )
    }
}

/// Generate a polite follow-up email asking the editor about review status based on `submitted_date`.
pub fn generate_followup_email(
    journal: &UiJournalItem,
    manuscript_title: &str,
    manuscript: Option<&ManuscriptSummary>,
    submitted_date: &str,
    lang: &str,
) -> String {
    generate_followup_email_with_profile(
        journal,
        manuscript_title,
        manuscript,
        submitted_date,
        None,
        lang,
    )
}

/// Generate a formal Statement of Originality & Anti-Plagiarism (`Surat Pernyataan Keaslian Naskah`)
/// ready for Materai Rp 10.000 signature.
pub fn generate_originality_statement(
    journal: &UiJournalItem,
    manuscript: Option<&ManuscriptSummary>,
    profile: Option<&AppSettings>,
    lang: &str,
) -> String {
    let today = Local::now().format("%d %B %Y").to_string();
    let (title, _, _) = resolve_manuscript_fields("", manuscript, lang);
    let raw_name = profile.map(|p| p.author_name.trim()).unwrap_or("");
    let raw_id = profile.map(|p| p.author_id.trim()).unwrap_or("");
    let raw_aff = profile.map(|p| p.author_affiliation.trim()).unwrap_or("");
    let raw_email = profile.map(|p| p.author_email.trim()).unwrap_or("");

    let is_en = lang.eq_ignore_ascii_case("en");
    let author_name = if raw_name.is_empty() {
        if is_en {
            "[Full Name / Author]"
        } else {
            "[Nama Lengkap Penulis]"
        }
    } else {
        raw_name
    };
    let author_id = if raw_id.is_empty() {
        if is_en {
            "[Student/Researcher ID]"
        } else {
            "[NIM / NIDN / NIP]"
        }
    } else {
        raw_id
    };
    let author_aff = if raw_aff.is_empty() {
        if is_en {
            "[Department / Institution]"
        } else {
            "[Program Studi / Afiliasi Kampus]"
        }
    } else {
        raw_aff
    };
    let author_email = if raw_email.is_empty() {
        if is_en {
            "[Email Address]"
        } else {
            "[Email Korespondensi]"
        }
    } else {
        raw_email
    };

    if is_en {
        format!(
            "STATEMENT OF ORIGINALITY AND ETHICAL COMPLIANCE\n\n\
I, the undersigned Corresponding Author:\n\
Name               : {author_name}\n\
Student / ID No.   : {author_id}\n\
Affiliation        : {author_aff}\n\
Email              : {author_email}\n\n\
hereby solemnly declare that the manuscript entitled:\n\
\"{title}\"\n\
submitted for publication to:\n\
Journal            : {journal_name}\n\
ISSN / e-ISSN      : {issn} / {eissn}\n\n\
1. Is the original scientific work of the author(s), written independently without plagiarism, fabrication, or falsification of data.\n\
2. Has not been published previously in any peer-reviewed journal or conference proceedings in whole or in part.\n\
3. Is not currently under consideration or peer-review by any other journal or publisher.\n\
4. Has obtained approval from all co-authors (if applicable) for submission and copyright transfer upon acceptance.\n\n\
This statement is made truthfully for academic and editorial purposes.\n\n\
[City], {today}\n\
Corresponding Author,\n\n\
[ Stamp Duty / Materai Rp 10.000 ]\n\n\
({author_name})\n\
ID: {author_id}",
            journal_name = journal.nama.trim(),
            issn = if journal.issn.trim().is_empty() { "-" } else { journal.issn.trim() },
            eissn = if journal.eissn.trim().is_empty() { "-" } else { journal.eissn.trim() },
        )
    } else {
        format!(
            "SURAT PERNYATAAN KEASLIAN NASKAH & BEBAS PLAGIARISME\n\n\
Yang bertanda tangan di bawah ini (Penulis Korespondensi):\n\
Nama Lengkap       : {author_name}\n\
NIM / NIDN         : {author_id}\n\
Program Studi/Univ : {author_aff}\n\
Email Korespondensi: {author_email}\n\n\
dengan ini menyatakan secara sadar dan sebenar-benarnya bahwa naskah artikel ilmiah berjudul:\n\
\"{title}\"\n\
yang diajukan untuk dipublikasikan pada:\n\
Nama Jurnal        : {journal_name}\n\
ISSN / e-ISSN      : {issn} / {eissn}\n\n\
1. Merupakan karya ilmiah asli (orisinal) penulis, bukan hasil plagiarisme, duplikasi, maupun fabrikasi data.\n\
2. Belum pernah dipublikasikan sebelumnya pada jurnal ilmiah, prosiding, maupun media publikasi lainnya.\n\
3. Tidak sedang dikirimkan atau berada dalam proses peninjauan (under review) di jurnal atau penerbit lain secara bersamaan.\n\
4. Seluruh penulis pendamping (bila ada) telah menyetujui isi naskah dan pengajuan publikasi ini.\n\n\
Apabila di kemudian hari ditemukan pelanggaran etika publikasi akademik, penulis bersedia menerima sanksi pembatalan maupun penarikan artikel sesuai ketentuan redaksi jurnal.\n\n\
[Kota], {today}\n\
Hormat saya, Yang Membuat Pernyataan,\n\n\
[ Materai Rp 10.000 ]\n\n\
({author_name})\n\
NIM/NIDN: {author_id}",
            journal_name = journal.nama.trim(),
            issn = if journal.issn.trim().is_empty() { "-" } else { journal.issn.trim() },
            eissn = if journal.eissn.trim().is_empty() { "-" } else { journal.eissn.trim() },
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

        let profile = AppSettings {
            author_name: "Siti Aminah".to_string(),
            author_id: "2101001122".to_string(),
            author_affiliation: "Ilmu Komputer, Universitas Merdeka".to_string(),
            author_email: "siti@merdeka.ac.id".to_string(),
            ..Default::default()
        };
        let auto_cover =
            generate_cover_letter_with_profile(&item, "Fallback", Some(&ms), Some(&profile), "id");
        assert!(auto_cover.contains("Siti Aminah (ID/NIM: 2101001122)"));
        assert!(auto_cover.contains("Ilmu Komputer, Universitas Merdeka"));
        assert!(auto_cover.contains("siti@merdeka.ac.id"));

        let stmt = generate_originality_statement(&item, Some(&ms), Some(&profile), "id");
        assert!(stmt.contains("SURAT PERNYATAAN KEASLIAN NASKAH"));
        assert!(stmt.contains("Materai Rp 10.000"));
        assert!(stmt.contains("Siti Aminah"));
        assert!(stmt.contains("2101001122"));
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
