//! Dedicated "Shortlist & Dospen" workspace combining:
//! - Section A: Supervisor Consultation Brief (WA/Email copy & `.docx` report export)
//! - Section B: Side-by-Side Candidate Comparison (2..=4 starred journals)
//! - Section C: Persistent Submission Tracker & Advisor Revision Notes Board

use crate::cache::SubmissionTrack;
use crate::engine::manuscript::ManuscriptSummary;
use crate::engine::{EngineEvent, EventSink};
use crate::exporter::templates::{generate_dosbing_brief, generate_followup_email_with_profile};
use crate::models::{compute_submission_readiness, FitStatus, UiJournalItem};
use crate::parser::doc_matcher::check_manuscript_word_fit;
use crate::ui::detail_page::{format_word_limit_label, SUBMISSION_STATUS_OPTIONS};
use crate::ui::i18n::{AppLanguage, AppSettings};
use crate::ui::state::AppState;
use crate::ui::theme::{
    accent_button, card, card_divider, page_header, pill_badge, quartile_pill_colors,
    secondary_button, secondary_button_h, Palette,
};
use egui::{CornerRadius, Frame, Grid, Margin, RichText, ScrollArea, Stroke, Ui};

pub enum DospenPageAction {
    None,
    OpenDetail(UiJournalItem),
    ExportDosbingDocx(Vec<UiJournalItem>),
    ExportBundleZip(UiJournalItem),
    SaveTrack(SubmissionTrack),
}

fn format_accreditation_text(j: &UiJournalItem) -> String {
    if let Some(q) = j
        .quartile
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        format!("Intl {}", q.to_ascii_uppercase())
    } else if !j.sinta.trim().is_empty() && j.sinta != "-" {
        let s = j.sinta.trim();
        if s.to_ascii_uppercase().starts_with("SINTA") {
            s.to_string()
        } else {
            format!("SINTA {s}")
        }
    } else if j.is_international {
        "Intl OA".to_string()
    } else {
        "Garuda Non-SINTA".to_string()
    }
}

fn render_compare_section(
    ui: &mut Ui,
    p: &Palette,
    shortlisted: &[UiJournalItem],
    manuscript: Option<&ManuscriptSummary>,
    app_state: &mut AppState,
    is_en: bool,
    action: &mut DospenPageAction,
) {
    card(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if is_en {
                    "B. Side-by-Side Candidate Comparison (★)"
                } else {
                    "B. Perbandingan Berdampingan Kandidat ★"
                })
                .size(14.0)
                .strong()
                .color(p.text),
            );
            pill_badge(
                ui,
                &format!(
                    "{} {}",
                    shortlisted.len().min(4),
                    if is_en { "shown" } else { "ditampilkan" }
                ),
                p.accent,
                p.accent_soft,
            );
        });
        card_divider(ui, p);

        if shortlisted.len() < 2 {
            ui.label(
                RichText::new(if is_en {
                    "Star 2 to 4 candidate journals (★) in Journal Search or Detail Workspace to compare them side by side."
                } else {
                    "Tandai 2–4 jurnal kandidat dengan ikon bintang ★ di Pencarian Jurnal atau Detail & Persiapan untuk melihat perbandingan berdampingan."
                })
                .size(12.5)
                .color(p.muted),
            );
            return;
        }

        let compared: Vec<&UiJournalItem> = shortlisted.iter().take(4).collect();
        let col_count = compared.len();
        let col_width = ((ui.available_width() - 185.0) / col_count as f32).clamp(150.0, 230.0);

        ScrollArea::horizontal()
            .id_salt("dospen_compare_hscroll")
            .show(ui, |ui| {
                Grid::new("dospen_compare_grid")
                    .num_columns(1 + col_count)
                    .spacing([12.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Row 0: Journal Header
                        ui.label(
                            RichText::new(if is_en { "Journal" } else { "Nama Jurnal" })
                                .size(12.0)
                                .strong()
                                .color(p.muted),
                        );
                        for j in &compared {
                            ui.allocate_ui([col_width, 46.0].into(), |ui| {
                                ui.vertical(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&j.nama)
                                                .size(12.5)
                                                .strong()
                                                .color(p.text),
                                        )
                                        .wrap(),
                                    );
                                    ui.label(RichText::new(&j.penerbit).size(11.0).color(p.muted));
                                });
                            });
                        }
                        ui.end_row();

                        // Row 1: Accreditation
                        ui.label(
                            RichText::new(if is_en { "Accreditation" } else { "Akreditasi" })
                                .size(12.0)
                                .strong()
                                .color(p.muted),
                        );
                        for j in &compared {
                            ui.allocate_ui([col_width, 24.0].into(), |ui| {
                                let lbl = format_accreditation_text(j);
                                if let Some(q) = j
                                    .quartile
                                    .as_deref()
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                {
                                    let (fg, bg) = quartile_pill_colors(p, q);
                                    pill_badge(ui, &lbl, fg, bg);
                                } else {
                                    pill_badge(ui, &lbl, p.accent, p.accent_soft);
                                }
                            });
                        }
                        ui.end_row();

                        // Row 2: Match & Readiness
                        ui.label(
                            RichText::new(if is_en {
                                "Match & Readiness"
                            } else {
                                "Kecocokan & Siap"
                            })
                            .size(12.0)
                            .strong()
                            .color(p.muted),
                        );
                        for j in &compared {
                            let rd = compute_submission_readiness(
                                j,
                                manuscript.map(|m| m.word_count),
                                manuscript.map(|m| m.imrad_complete),
                                is_en,
                            );
                            ui.allocate_ui([col_width, 28.0].into(), |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    if j.match_score > 0.0 {
                                        pill_badge(
                                            ui,
                                            &format!("Match {:.0}%", j.match_score),
                                            p.accent,
                                            p.accent_soft,
                                        );
                                    }
                                    let (rfg, rbg) = if rd.score >= 80 {
                                        (p.good, p.good_soft)
                                    } else {
                                        (p.warn, p.warn_soft)
                                    };
                                    pill_badge(ui, &format!("Siap {}%", rd.score), rfg, rbg);
                                });
                            });
                        }
                        ui.end_row();

                        // Row 3: Biaya Submit vs APC
                        ui.label(
                            RichText::new(if is_en {
                                "Submit vs APC Fee"
                            } else {
                                "Biaya Submit vs APC"
                            })
                            .size(12.0)
                            .strong()
                            .color(p.muted),
                        );
                        for j in &compared {
                            ui.allocate_ui([col_width, 32.0].into(), |ui| {
                                ui.vertical(|ui| {
                                    let (fg, bg) = match j.apc_category.as_str() {
                                        "free" => (p.good, p.good_soft),
                                        "paid" => (p.warn, p.warn_soft),
                                        _ => (p.muted, p.control_bg),
                                    };
                                    pill_badge(ui, &j.apc_label, fg, bg);
                                    if let Some(sf) = j.submission_fee_idr {
                                        let sf_txt = if sf == 0 {
                                            "Submit: Gratis".to_string()
                                        } else {
                                            format!(
                                                "Submit: {}",
                                                crate::models::format_idr_short(sf as i64)
                                            )
                                        };
                                        ui.label(RichText::new(sf_txt).size(11.0).color(p.muted));
                                    }
                                });
                            });
                        }
                        ui.end_row();

                        // Row 4: Review Speed & Publish Schedule
                        ui.label(
                            RichText::new(if is_en {
                                "Speed & Schedule"
                            } else {
                                "Kecepatan & Jadwal"
                            })
                            .size(12.0)
                            .strong()
                            .color(p.muted),
                        );
                        for j in &compared {
                            ui.allocate_ui([col_width, 36.0].into(), |ui| {
                                ui.vertical(|ui| {
                                    let spd = if j.turnaround_label.trim().is_empty() {
                                        "-"
                                    } else {
                                        j.turnaround_label.trim()
                                    };
                                    ui.label(
                                        RichText::new(format!(
                                            "{spd} • Next: {}",
                                            j.terbit_terdekat
                                        ))
                                        .size(11.5)
                                        .color(p.text),
                                    );
                                    ui.label(
                                        RichText::new(&j.jadwal_terbit).size(11.0).color(p.muted),
                                    );
                                });
                            });
                        }
                        ui.end_row();

                        // Row 5: Citation Style, Word Limit & Fit
                        ui.label(
                            RichText::new(if is_en {
                                "Citation & Word Fit"
                            } else {
                                "Sitasi & Batas Kata"
                            })
                            .size(12.0)
                            .strong()
                            .color(p.muted),
                        );
                        for j in &compared {
                            ui.allocate_ui([col_width, 38.0].into(), |ui| {
                                ui.vertical(|ui| {
                                    let sty = j.citation_style.as_deref().unwrap_or("-");
                                    let wlim = format_word_limit_label(j.word_limit, is_en);
                                    ui.label(
                                        RichText::new(format!("{sty} • {wlim}"))
                                            .size(11.5)
                                            .color(p.text),
                                    );
                                    if let Some(ms) = manuscript {
                                        if let Some(st) = check_manuscript_word_fit(
                                            ms.word_count as u32,
                                            j.word_limit,
                                        ) {
                                            let (lbl, fg, bg) = match st {
                                                FitStatus::Sesuai => {
                                                    ("Kata Sesuai ✓", p.good, p.good_soft)
                                                }
                                                FitStatus::TerlaluPendek => {
                                                    ("Terlalu Pendek ⚠", p.warn, p.warn_soft)
                                                }
                                                FitStatus::MelebihiBatas => {
                                                    ("Melebihi Batas ⚠", p.bad, p.bad_soft)
                                                }
                                            };
                                            pill_badge(ui, lbl, fg, bg);
                                        }
                                    }
                                });
                            });
                        }
                        ui.end_row();

                        // Row 6: Language & Actions
                        ui.label(
                            RichText::new(if is_en {
                                "Language & Action"
                            } else {
                                "Bahasa & Aksi"
                            })
                            .size(12.0)
                            .strong()
                            .color(p.muted),
                        );
                        for j in &compared {
                            ui.allocate_ui([col_width, 32.0].into(), |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    if secondary_button_h(
                                        ui,
                                        p,
                                        if is_en { "Open Detail" } else { "Detail" },
                                        24.0,
                                    )
                                    .clicked()
                                    {
                                        *action = DospenPageAction::OpenDetail((*j).clone());
                                    }
                                    if secondary_button_h(ui, p, "★ Hapus", 24.0).clicked() {
                                        app_state.toggle_bookmark(j);
                                    }
                                });
                            });
                        }
                        ui.end_row();
                    });
            });
    });
}

#[allow(clippy::too_many_arguments)]
pub fn render_dospen_page(
    ui: &mut Ui,
    p: &Palette,
    results: &[UiJournalItem],
    recent_inspected: &[UiJournalItem],
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    app_settings: &AppSettings,
    app_state: &mut AppState,
    sink: &EventSink,
) -> DospenPageAction {
    let is_en = app_settings.language == AppLanguage::En;
    let ctx = ui.ctx().clone();
    let mut action = DospenPageAction::None;

    page_header(
        ui,
        p,
        if is_en {
            "Shortlist, Comparison & Advisor Workspace"
        } else {
            "Shortlist Kandidat, Konsultasi Dospen & Pelacak Kiriman"
        },
        if is_en {
            "Prepare supervisor consultation packages, compare starred journals, and track submission revisions."
        } else {
            "Siapkan paket konsultasi dosen pembimbing, bandingkan kandidat ★, dan catat status submit/revisi naskah."
        },
    );

    let shortlisted = app_state.collect_shortlisted_items(results, recent_inspected);

    ScrollArea::vertical().id_salt("dospen_page_scroll").show(ui, |ui| {
        // Section A: Paket Konsultasi Dosen Pembimbing (Dosbing Brief)
        card(ui, p, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if is_en {
                        "A. Supervisor Consultation Package (Dosbing Brief)"
                    } else {
                        "A. Paket Konsultasi Dosen Pembimbing (Dosbing Brief)"
                    })
                    .size(14.0)
                    .strong()
                    .color(p.text),
                );
                pill_badge(
                    ui,
                    &format!("{} ★ {}", shortlisted.len(), if is_en { "candidates" } else { "kandidat" }),
                    if shortlisted.is_empty() { p.muted } else { p.warn },
                    if shortlisted.is_empty() { p.control_bg } else { p.warn_soft },
                );
            });
            card_divider(ui, p);

            if let Some(ms) = manuscript {
                ui.label(
                    RichText::new(format!(
                        "Naskah aktif: {} ({} kata • {} sitasi tidak bertanda)",
                        if ms.extracted_title.trim().is_empty() {
                            &ms.name
                        } else {
                            &ms.extracted_title
                        },
                        ms.word_count,
                        ms.uncited_long_paragraphs
                    ))
                    .size(12.0)
                    .color(p.muted),
                );
                ui.add_space(4.0);
            }

            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled_ui(!shortlisted.is_empty(), |ui| {
                        accent_button(
                            ui,
                            p,
                            if is_en {
                                "Copy Supervisor Brief (WA / Email)"
                            } else {
                                "Salin Paket Konsultasi WA / Email"
                            },
                        )
                    })
                    .inner
                    .clicked()
                {
                    let refs: Vec<&UiJournalItem> = shortlisted.iter().collect();
                    let brief = generate_dosbing_brief(&refs, app_settings.language);
                    ctx.copy_text(brief);
                    sink.send(EngineEvent::Toast(
                        if is_en {
                            "Supervisor brief copied to clipboard."
                        } else {
                            "Paket konsultasi Dospen (WA/Email) disalin ke clipboard."
                        }
                        .to_string(),
                    ));
                }

                if ui
                    .add_enabled_ui(!shortlisted.is_empty(), |ui| {
                        secondary_button(
                            ui,
                            p,
                            if is_en {
                                "Export Advisor Report (.docx)"
                            } else {
                                "Ekspor Laporan Dospen (.docx)"
                            },
                        )
                    })
                    .inner
                    .clicked()
                {
                    action = DospenPageAction::ExportDosbingDocx(shortlisted.clone());
                }
            });
        });

        ui.add_space(12.0);

        // Section B: Side-by-Side Comparison of 2..=4 candidates
        render_compare_section(
            ui,
            p,
            &shortlisted,
            manuscript,
            app_state,
            is_en,
            &mut action,
        );

        ui.add_space(12.0);

        // Section C: Papan Pelacak Pengiriman & Catatan Bimbingan (Submission Tracker)
        card(ui, p, |ui| {
            ui.label(
                RichText::new(if is_en {
                    "C. Submission Tracker & Advisor Revision Notes"
                } else {
                    "C. Papan Pelacak Pengiriman & Catatan Revisi Dospen"
                })
                .size(14.0)
                .strong()
                .color(p.text),
            );
            ui.label(
                RichText::new(if is_en {
                    "Track submission statuses, sent dates, and advisor feedback saved locally in SQLite."
                } else {
                    "Catat progres submit ke jurnal, tanggal kirim, dan arahan revisi dosen pembimbing (tersimpan permanen di SQLite)."
                })
                .size(11.5)
                .color(p.muted),
            );
            card_divider(ui, p);

            // Build lookup of known items (shortlisted + results + recent) so tracked items stay reachable
            let mut item_by_track_key = std::collections::HashMap::<String, UiJournalItem>::new();
            for it in shortlisted
                .iter()
                .chain(recent_inspected.iter())
                .chain(results.iter())
            {
                let tk = SubmissionTrack::canonical_key(&it.url, &it.nama);
                item_by_track_key.entry(tk).or_insert_with(|| it.clone());
            }

            let mut tracker_keys: Vec<String> = Vec::new();
            for it in &shortlisted {
                let tk = SubmissionTrack::canonical_key(&it.url, &it.nama);
                if !tracker_keys.contains(&tk) {
                    tracker_keys.push(tk);
                }
            }
            for tk in app_state.submission_tracks.keys() {
                if !tracker_keys.contains(tk) {
                    tracker_keys.push(tk.clone());
                }
            }

            if tracker_keys.is_empty() {
                ui.label(
                    RichText::new(if is_en {
                        "No shortlisted or tracked journals yet. Star candidate journals (★) to start tracking."
                    } else {
                        "Belum ada jurnal di shortlist atau pelacak. Tandai jurnal dengan bintang ★ terlebih dahulu."
                    })
                    .size(12.5)
                    .color(p.muted),
                );
            } else {
                for tk in tracker_keys {
                    let known_item = item_by_track_key.get(&tk).cloned();
                    let display_title = known_item
                        .as_ref()
                        .map(|j| j.nama.clone())
                        .unwrap_or_else(|| tk.clone());

                    let entry = app_state
                        .submission_tracks
                        .entry(tk.clone())
                        .or_insert_with(|| SubmissionTrack {
                            journal_key: tk.clone(),
                            status: "Belum Dikirim".to_string(),
                            submitted_date: String::new(),
                            notes: String::new(),
                            updated_at: String::new(),
                        });

                    let mut changed_now = false;
                    Frame::new()
                        .fill(p.control_bg)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(&display_title).size(13.0).strong().color(p.text));
                                if let Some(ref j) = known_item {
                                    pill_badge(ui, &j.apc_label, p.accent, p.accent_soft);
                                }
                            });
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Status:").size(12.0).color(p.muted));
                                egui::ComboBox::from_id_salt(("dospen_track_status", &tk))
                                    .selected_text(&entry.status)
                                    .width(135.0)
                                    .show_ui(ui, |ui| {
                                        for opt in SUBMISSION_STATUS_OPTIONS {
                                            if ui
                                                .selectable_value(
                                                    &mut entry.status,
                                                    opt.to_string(),
                                                    opt,
                                                )
                                                .changed()
                                            {
                                                changed_now = true;
                                            }
                                        }
                                    });
                                ui.label(RichText::new("Tgl Kirim:").size(12.0).color(p.muted));
                                if ui
                                    .add_sized(
                                        [105.0, 26.0],
                                        egui::TextEdit::singleline(&mut entry.submitted_date)
                                            .hint_text("YYYY-MM-DD"),
                                    )
                                    .lost_focus()
                                {
                                    changed_now = true;
                                }
                                if secondary_button_h(ui, p, "Simpan", 26.0).clicked() {
                                    changed_now = true;
                                }
                                if let Some(ref j) = known_item {
                                    if secondary_button_h(ui, p, "Follow-Up (ID)", 26.0).clicked() {
                                        let msg = generate_followup_email_with_profile(
                                            j,
                                            keyword,
                                            manuscript,
                                            &entry.submitted_date,
                                            Some(app_settings),
                                            "id",
                                        );
                                        ctx.copy_text(msg);
                                        sink.send(EngineEvent::Toast(
                                            "Email Follow-Up (ID) disalin.".to_string(),
                                        ));
                                    }
                                    if secondary_button_h(ui, p, "Follow-Up (EN)", 26.0).clicked() {
                                        let msg = generate_followup_email_with_profile(
                                            j,
                                            keyword,
                                            manuscript,
                                            &entry.submitted_date,
                                            Some(app_settings),
                                            "en",
                                        );
                                        ctx.copy_text(msg);
                                        sink.send(EngineEvent::Toast(
                                            "English Follow-Up Email copied.".to_string(),
                                        ));
                                    }
                                    if secondary_button_h(ui, p, "Buka di Detail", 26.0).clicked() {
                                        action = DospenPageAction::OpenDetail(j.clone());
                                    }
                                    if secondary_button_h(ui, p, "Paket (.zip)", 26.0).clicked() {
                                        action = DospenPageAction::ExportBundleZip(j.clone());
                                    }
                                }
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Catatan Dospen:").size(12.0).color(p.muted));
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 26.0],
                                        egui::TextEdit::singleline(&mut entry.notes)
                                            .hint_text("Arahan revisi dosbing, komentar reviewer, link submit OJS..."),
                                    )
                                    .lost_focus()
                                {
                                    changed_now = true;
                                }
                            });
                        });

                    if changed_now {
                        action = DospenPageAction::SaveTrack(entry.clone());
                    }
                    ui.add_space(8.0);
                }
            }
        });
    });

    action
}
