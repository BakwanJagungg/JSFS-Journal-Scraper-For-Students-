//! Native modal inspector showing complete journal accuracy evidence, activity year, Focus & Scope,
//! Editor contact links (Email / WhatsApp), and 1-click OJS open or WA inquiry draft copy.

use crate::cache::SubmissionTrack;
use crate::engine::file_ops::{format_editor_draft, open_url_with_feedback};
use crate::engine::manuscript::ManuscriptSummary;
use crate::engine::{EngineEvent, EventSink};
use crate::exporter::templates::{
    generate_cover_letter_with_profile, generate_editor_inquiry_email_with_profile,
    generate_followup_email_with_profile, generate_originality_statement,
};
use crate::models::{compute_submission_readiness, FitStatus, UiJournalItem};
use crate::parser::doc_matcher::check_manuscript_word_fit;
use crate::ui::i18n::{AppLanguage, AppSettings};
use crate::ui::state::AppState;
use crate::ui::theme::{
    accent_button, card, pill_badge, quartile_pill_colors, secondary_button, Palette,
};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui};

pub enum DetailModalAction {
    None,
    Close,
    FindSimilar(usize),
    SaveTrack(SubmissionTrack),
    ExportBundle(usize),
}

pub const SUBMISSION_STATUS_OPTIONS: [&str; 7] = [
    "Belum Dikirim",
    "Draft Siap",
    "Submitted",
    "Under Review",
    "Revision",
    "Accepted",
    "Rejected",
];

pub fn extract_whatsapp_url(phone: &str) -> Option<String> {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return None;
    }
    let normalized = if let Some(rest) = digits.strip_prefix('0') {
        format!("62{rest}")
    } else {
        digits
    };
    Some(format!("https://wa.me/{normalized}"))
}

fn info_kv(ui: &mut Ui, p: &Palette, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("{label}:"))
                .size(12.0)
                .strong()
                .color(p.muted),
        );
        ui.label(
            RichText::new(if value.trim().is_empty() {
                "-"
            } else {
                value.trim()
            })
            .size(12.5)
            .color(p.text),
        );
    });
}

#[allow(clippy::too_many_arguments)]
pub fn render_detail_modal(
    ctx: &egui::Context,
    p: &Palette,
    item_idx: usize,
    item: Option<&UiJournalItem>,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    app_settings: &AppSettings,
    app_state: &mut AppState,
    sink: &EventSink,
) -> DetailModalAction {
    let Some(j) = item else {
        return DetailModalAction::Close;
    };
    let is_en = app_settings.language == AppLanguage::En;

    let canonical_key = SubmissionTrack::canonical_key(&j.url, &j.nama);
    if app_state.active_track_edit.journal_key != canonical_key {
        app_state.active_track_edit = app_state
            .submission_tracks
            .get(&canonical_key)
            .cloned()
            .unwrap_or_else(|| SubmissionTrack {
                journal_key: canonical_key.clone(),
                status: "Belum Dikirim".to_string(),
                submitted_date: String::new(),
                notes: String::new(),
                updated_at: String::new(),
            });
    }

    let mut action = DetailModalAction::None;
    let mut close_requested = false;
    let modal_resp = egui::Modal::new(egui::Id::unique("journal_detail_modal")).show(ctx, |ui| {
        ui.set_width(680.0_f32.min(ctx.content_rect().width() - 48.0));
        ui.set_max_height(ctx.content_rect().height() - 56.0);

        // Header
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width(560.0);
                ui.label(RichText::new(&j.nama).size(17.5).strong().color(p.text));
                ui.horizontal_wrapped(|ui| {
                    if let Some(q) = j
                        .quartile
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        let (q_fg, q_bg) = quartile_pill_colors(p, q);
                        pill_badge(ui, &format!("Intl {}", q.to_ascii_uppercase()), q_fg, q_bg);
                    } else if !j.sinta.is_empty() {
                        pill_badge(ui, &format!("SINTA {}", j.sinta), p.accent, p.accent_soft);
                    } else if j.is_international {
                        pill_badge(ui, "Intl OA", p.good, p.good_soft);
                    } else {
                        pill_badge(ui, "Garuda Non-SINTA", p.accent, p.accent_soft);
                    }

                    let act_fg = if j.is_dormant {
                        p.bad
                    } else if j.bot_protected && j.last_issue_year.is_none() {
                        p.warn
                    } else if j.activity_status.contains("Aktif") {
                        p.good
                    } else {
                        p.warn
                    };
                    let act_bg = if j.is_dormant {
                        p.bad_soft
                    } else if j.bot_protected && j.last_issue_year.is_none() {
                        p.warn_soft
                    } else if j.activity_status.contains("Aktif") {
                        p.good_soft
                    } else {
                        p.warn_soft
                    };
                    pill_badge(ui, &j.activity_status, act_fg, act_bg);

                    if !j.issn.trim().is_empty() || !j.eissn.trim().is_empty() {
                        if j.issn_valid {
                            pill_badge(ui, "ISSN Valid", p.good, p.good_soft);
                        } else {
                            pill_badge(ui, "ISSN Cek Checksum", p.warn, p.warn_soft);
                        }
                    }

                    for idx_badge in &j.extra_indexes {
                        pill_badge(ui, idx_badge, p.accent, p.accent_soft);
                    }

                    if !j.turnaround_label.trim().is_empty() {
                        pill_badge(
                            ui,
                            &format!("Review {}", j.turnaround_label),
                            p.accent,
                            p.accent_soft,
                        );
                    }
                    if !j.article_languages.trim().is_empty() {
                        pill_badge(
                            ui,
                            &format!("Bahasa: {}", j.article_languages),
                            p.muted,
                            p.control_bg,
                        );
                    }

                    let match_lbl = format!("Match {:.0}%", j.match_score);
                    pill_badge(ui, &match_lbl, p.good, p.good_soft);

                    if j.cited_count > 0 {
                        pill_badge(
                            ui,
                            &format!("Dikutip {}x di Naskah", j.cited_count),
                            p.accent,
                            p.accent_soft,
                        );
                    }
                    if let Some(open) = j.submission_open {
                        if open {
                            pill_badge(ui, "Open Submit / CFP", p.good, p.good_soft);
                        } else {
                            pill_badge(ui, "Registrasi Tutup", p.bad, p.bad_soft);
                        }
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                if secondary_button(ui, p, "Tutup").clicked() {
                    close_requested = true;
                }
            });
        });

        ui.add_space(10.0);

        ScrollArea::vertical().max_height(430.0).show(ui, |ui| {
            // Pre-Submission Readiness Score (0-100%) Card
            let readiness = compute_submission_readiness(
                j,
                manuscript.map(|m| m.word_count),
                manuscript.map(|m| m.imrad_complete),
                is_en,
            );
            let (rd_fg, rd_bg) = if readiness.score >= 80 {
                (p.good, p.good_soft)
            } else if readiness.score >= 55 {
                (p.warn, p.warn_soft)
            } else {
                (p.bad, p.bad_soft)
            };
            card(ui, p, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if is_en {
                            "Pre-Submission Readiness Score"
                        } else {
                            "Skor Kesiapan Pengajuan (Pre-Submission Readiness)"
                        })
                        .size(13.0)
                        .strong()
                        .color(p.text),
                    );
                    pill_badge(ui, &format!("{}%", readiness.score), rd_fg, rd_bg);
                });
                ui.add_space(4.0);
                for (ok, msg) in &readiness.checklist {
                    let (mark, col) = if *ok { ("[✓]", p.good) } else { ("[!]", p.warn) };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(mark).size(12.0).strong().color(col));
                        ui.label(RichText::new(msg).size(12.0).color(p.text));
                    });
                }
            });
            ui.add_space(10.0);

            if let Some(ref warn) = j.risk_warning {
                Frame::new()
                    .fill(p.bad_soft)
                    .stroke(Stroke::new(1.0, p.bad))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(
                            RichText::new("⚠ Peringatan Audit Keamanan / Predatory & Cloned Detector")
                                .size(12.0)
                                .strong()
                                .color(p.bad),
                        );
                        ui.label(RichText::new(warn).size(12.0).color(p.text));
                    });
                ui.add_space(10.0);
            }

            card(ui, p, |ui| {
                ui.label(
                    RichText::new("Ringkasan Akurasi & Jadwal")
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);
                info_kv(ui, p, "Penerbit", &j.penerbit);
                info_kv(ui, p, "Sumber Portal", &j.sumber);
                let issn_disp = match (j.issn.is_empty(), j.eissn.is_empty()) {
                    (true, true) => "-".to_string(),
                    (false, true) => j.issn.clone(),
                    (true, false) => format!("e-ISSN {}", j.eissn),
                    (false, false) => format!("{} / e-ISSN {}", j.issn, j.eissn),
                };
                let issn_note = if j.issn_valid {
                    format!("{issn_disp} (Checksum ISO 3297 Valid)")
                } else if issn_disp != "-" {
                    format!("{issn_disp} (Cek Validitas Manual)")
                } else {
                    issn_disp
                };
                info_kv(ui, p, "ISSN", &issn_note);
                if !j.extra_indexes.is_empty() {
                    info_kv(ui, p, "Indeks Sekunder", &j.extra_indexes.join(", "));
                }
                let year_disp = j
                    .last_issue_year
                    .map(|y| format!("{y} ({})", j.activity_status))
                    .unwrap_or_else(|| j.activity_status.clone());
                info_kv(ui, p, "Tahun Edisi Terakhir", &year_disp);
                info_kv(ui, p, "Biaya Publikasi (APC)", &j.apc_label);
                let sub_disp = match j.submission_fee_idr {
                    Some(0) => "Gratis (Rp 0 — tidak ada biaya submit)".to_string(),
                    Some(v) => format!(
                        "{} (terpisah dari APC publikasi)",
                        crate::models::format_idr_short(v as i64)
                    ),
                    None if j.submit_label != "-" => j.submit_label.clone(),
                    None => "Tidak dicantumkan / Tidak dipungut".to_string(),
                };
                info_kv(ui, p, "Biaya Submit (Submission Fee)", &sub_disp);
                if let Some(open) = j.submission_open {
                    info_kv(
                        ui,
                        p,
                        "Status Pendaftaran",
                        if open {
                            "Open Submit / Menerima Naskah"
                        } else {
                            "Registrasi Tutup (Cek OJS)"
                        },
                    );
                }
                info_kv(ui, p, "Jadwal Terbit", &j.jadwal_terbit);
                info_kv(ui, p, "Terbit Terdekat", &j.terbit_terdekat);
                if !j.turnaround_label.trim().is_empty() {
                    info_kv(ui, p, "Estimasi Kecepatan Review", &j.turnaround_label);
                }
                let wisuda_tip = if let Some(w) = j.turnaround_weeks {
                    let rec_months = ((w as f32 / 4.3).ceil() as u16).max(1);
                    format!(
                        "Estimasi proses ~{w} minggu — disarankan submit minimal {rec_months} bulan sebelum target sidang/wisuda"
                    )
                } else {
                    "Disarankan submit minimal 2–3 bulan sebelum target sidang/wisuda (termasuk waktu revisi)".to_string()
                };
                info_kv(ui, p, "Saran Target Wisuda", &wisuda_tip);
                if !j.article_languages.trim().is_empty() {
                    info_kv(ui, p, "Bahasa Naskah Diterima", &j.article_languages);
                }
            });

            if j.citation_style.is_some()
                || j.word_limit.is_some()
                || j.cited_count > 0
                || manuscript.is_some()
            {
                ui.add_space(10.0);
                card(ui, p, |ui| {
                    ui.label(
                        RichText::new("Syarat Naskah & Pre-Flight Fit")
                            .size(13.0)
                            .strong()
                            .color(p.text),
                    );
                    ui.add_space(4.0);
                    info_kv(
                        ui,
                        p,
                        "Gaya Sitasi",
                        j.citation_style.as_deref().unwrap_or("Belum terdeteksi di OJS"),
                    );
                    let limit_text = match j.word_limit {
                        Some((0, max_w)) => format!("Maksimal {max_w} kata"),
                        Some((min_w, max_w)) => format!("{min_w} – {max_w} kata"),
                        None => "Belum terdeteksi di OJS".to_string(),
                    };
                    info_kv(ui, p, "Batas Panjang Kata", &limit_text);
                    if j.cited_count > 0 {
                        info_kv(
                            ui,
                            p,
                            "Sitasi di Daftar Pustaka",
                            &format!("Dikutip {}x di naskah Anda", j.cited_count),
                        );
                    }
                    if let Some(ms) = manuscript {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!("Naskah Anda ({}):", ms.name))
                                    .size(12.0)
                                    .strong()
                                    .color(p.muted),
                            );
                            ui.label(
                                RichText::new(format!("{} kata", ms.word_count))
                                    .size(12.0)
                                    .color(p.text),
                            );
                            if let Some(status) =
                                check_manuscript_word_fit(ms.word_count as u32, j.word_limit)
                            {
                                let (lbl, fg, bg) = match status {
                                    FitStatus::Sesuai => {
                                        ("✓ Sesuai Batas Kata", p.good, p.good_soft)
                                    }
                                    FitStatus::TerlaluPendek => {
                                        ("⚠ Terlalu Pendek", p.warn, p.warn_soft)
                                    }
                                    FitStatus::MelebihiBatas => {
                                        ("⚠ Melebihi Batas", p.bad, p.bad_soft)
                                    }
                                };
                                pill_badge(ui, lbl, fg, bg);
                            }
                        });
                    }
                });
            }

            if !j.apc_evidence.trim().is_empty() {
                ui.add_space(10.0);
                Frame::new()
                    .fill(p.accent_soft)
                    .stroke(Stroke::new(1.0, p.accent))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(
                            RichText::new("Bukti Teks APC yang Ditemukan")
                                .size(12.0)
                                .strong()
                                .color(p.accent),
                        );
                        ui.label(RichText::new(&j.apc_evidence).size(12.0).color(p.text));
                    });
            }

            ui.add_space(10.0);
            card(ui, p, |ui| {
                ui.label(
                    RichText::new("Pelacak Status Pengiriman Naskah (Tersimpan Lokal)")
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Status:").size(12.0).color(p.muted));
                    egui::ComboBox::from_id_salt("sub_track_status_combo")
                        .selected_text(&app_state.active_track_edit.status)
                        .show_ui(ui, |ui| {
                            for st in SUBMISSION_STATUS_OPTIONS {
                                ui.selectable_value(
                                    &mut app_state.active_track_edit.status,
                                    st.to_string(),
                                    st,
                                );
                            }
                        });

                    ui.label(RichText::new("Tgl Kirim:").size(12.0).color(p.muted));
                    ui.add_sized(
                        [110.0, 26.0],
                        egui::TextEdit::singleline(&mut app_state.active_track_edit.submitted_date)
                            .hint_text("YYYY-MM-DD"),
                    );

                    if accent_button(ui, p, "Simpan Status").clicked() {
                        action = DetailModalAction::SaveTrack(app_state.active_track_edit.clone());
                    }
                    if secondary_button(ui, p, "Follow-Up (ID)")
                        .on_hover_text("Salin draf email sopan menanyakan status review ke editor")
                        .clicked()
                    {
                        let body = generate_followup_email_with_profile(
                            j,
                            keyword,
                            manuscript,
                            &app_state.active_track_edit.submitted_date,
                            Some(app_settings),
                            "id",
                        );
                        ctx.copy_text(body);
                        sink.send(EngineEvent::Toast(
                            "Email Follow-Up Editor (ID) disalin ke clipboard.".to_string(),
                        ));
                    }
                    if secondary_button(ui, p, "Follow-Up (EN)")
                        .on_hover_text("Copy polite English review-status follow-up email")
                        .clicked()
                    {
                        let body = generate_followup_email_with_profile(
                            j,
                            keyword,
                            manuscript,
                            &app_state.active_track_edit.submitted_date,
                            Some(app_settings),
                            "en",
                        );
                        ctx.copy_text(body);
                        sink.send(EngineEvent::Toast(
                            "English Follow-Up Email copied to clipboard.".to_string(),
                        ));
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Catatan:").size(12.0).color(p.muted));
                    ui.add_sized(
                        [ui.available_width(), 26.0],
                        egui::TextEdit::singleline(&mut app_state.active_track_edit.notes)
                            .hint_text("Catatan untuk jurnal ini (misal: batas revisi, link OJS, nama reviewer)..."),
                    );
                });
            });

            ui.add_space(10.0);
            card(ui, p, |ui| {
                ui.label(
                    RichText::new("Focus & Scope Jurnal")
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);
                let focus_txt = if j.focus_scope.trim().is_empty() {
                    "Belum diambil dari halaman OJS (cek website resmi)."
                } else {
                    j.focus_scope.trim()
                };
                ui.label(RichText::new(focus_txt).size(12.0).color(p.text));
                if !j.match_reasons.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("Alasan Kecocokan: {}", j.match_reasons.join(" • ")))
                            .size(11.5)
                            .color(p.muted),
                    );
                }
            });

            ui.add_space(10.0);
            card(ui, p, |ui| {
                ui.label(
                    RichText::new("Draf Surat & Email ke Editor (Auto-Fill Profil)")
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if secondary_button(ui, p, "Cover Letter (ID)").clicked() {
                        let letter = generate_cover_letter_with_profile(
                            j,
                            keyword,
                            manuscript,
                            Some(app_settings),
                            "id",
                        );
                        ctx.copy_text(letter);
                        sink.send(EngineEvent::Toast(
                            "Surat Pengantar (Cover Letter Bahasa Indonesia) disalin ke clipboard."
                                .to_string(),
                        ));
                    }
                    if secondary_button(ui, p, "Cover Letter (EN)").clicked() {
                        let letter = generate_cover_letter_with_profile(
                            j,
                            keyword,
                            manuscript,
                            Some(app_settings),
                            "en",
                        );
                        ctx.copy_text(letter);
                        sink.send(EngineEvent::Toast(
                            "English Cover Letter copied to clipboard.".to_string(),
                        ));
                    }
                    if secondary_button(ui, p, "Surat Keaslian")
                        .on_hover_text("Salin Surat Pernyataan Keaslian Naskah bermaterai Rp 10.000")
                        .clicked()
                    {
                        let stmt = generate_originality_statement(
                            j,
                            manuscript,
                            Some(app_settings),
                            if is_en { "en" } else { "id" },
                        );
                        ctx.copy_text(stmt);
                        sink.send(EngineEvent::Toast(
                            "Surat Pernyataan Keaslian Naskah disalin ke clipboard.".to_string(),
                        ));
                    }
                    if secondary_button(ui, p, "Email Editor (ID)")
                        .on_hover_text("Salin email konsultasi pra-submit (Subjek + Isi) siap kirim")
                        .clicked()
                    {
                        let email = generate_editor_inquiry_email_with_profile(
                            j,
                            keyword,
                            manuscript,
                            Some(app_settings),
                            "id",
                        );
                        ctx.copy_text(email);
                        sink.send(EngineEvent::Toast(
                            "Draf Email Inquiry Editor (ID) disalin ke clipboard.".to_string(),
                        ));
                    }
                    if secondary_button(ui, p, "Email Editor (EN)")
                        .on_hover_text(
                            "Copy pre-submission inquiry email (Subject + Body) to clipboard",
                        )
                        .clicked()
                    {
                        let email = generate_editor_inquiry_email_with_profile(
                            j,
                            keyword,
                            manuscript,
                            Some(app_settings),
                            "en",
                        );
                        ctx.copy_text(email);
                        sink.send(EngineEvent::Toast(
                            "English Editor Inquiry Email copied to clipboard.".to_string(),
                        ));
                    }
                });
            });

            ui.add_space(10.0);
            card(ui, p, |ui| {
                ui.label(
                    RichText::new("Kontak Redaksi Editor & Dokumen")
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Email:").size(12.0).strong().color(p.muted));
                    ui.label(RichText::new(&j.email).size(12.0).color(p.text));
                    if j.email.contains('@') && secondary_button(ui, p, "Salin Email").clicked() {
                        ctx.copy_text(j.email.clone());
                        sink.send(EngineEvent::Toast("Email editor disalin.".to_string()));
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Telepon / WA:")
                            .size(12.0)
                            .strong()
                            .color(p.muted),
                    );
                    ui.label(RichText::new(&j.telepon).size(12.0).color(p.text));
                    if let Some(wa_url) = extract_whatsapp_url(&j.telepon) {
                        if secondary_button(ui, p, "Buka Chat WA").clicked() {
                            open_url_with_feedback(&wa_url, sink);
                        }
                    }
                });
                if !j.url.trim().is_empty() {
                    info_kv(ui, p, "Website OJS", &j.url);
                }
                if let Some(ref tpl_url) = j.template_url {
                    ui.add_space(4.0);
                    if secondary_button(ui, p, "📄 Unduh Template Naskah (.docx / Panduan)")
                        .on_hover_text(tpl_url)
                        .clicked()
                    {
                        open_url_with_feedback(tpl_url, sink);
                    }
                }
            });
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.set_min_width(ui.available_width());
            if secondary_button(ui, p, "Salin Draf WA").clicked() {
                let draft = format_editor_draft(&j.nama, keyword);
                ctx.copy_text(draft);
                sink.send(EngineEvent::Toast(
                    "Draf pesan sopan ke editor disalin ke clipboard.".to_string(),
                ));
            }
            if secondary_button(ui, p, "Paket Pengajuan (.zip)")
                .on_hover_text("Bundel .zip berisi Cover Letter, Surat Pernyataan Keaslian, dan Ringkasan Aturan Jurnal")
                .clicked()
            {
                action = DetailModalAction::ExportBundle(item_idx);
            }
            if secondary_button(ui, p, "Cari Jurnal Mirip")
                .on_hover_text("Urutkan tabel berdasarkan kemiripan topik dengan jurnal ini")
                .clicked()
            {
                action = DetailModalAction::FindSimilar(item_idx);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if accent_button(ui, p, "Buka Web OJS").clicked() {
                    let target = if j.url.starts_with("http") {
                        &j.url
                    } else {
                        &j.garuda_url
                    };
                    open_url_with_feedback(target, sink);
                }
            });
        });
    });

    if modal_resp.should_close() || close_requested {
        DetailModalAction::Close
    } else {
        action
    }
}
