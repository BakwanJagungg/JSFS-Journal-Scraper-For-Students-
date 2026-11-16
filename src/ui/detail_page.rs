//! Dedicated "Detail & Persiapan" workspace: Quick 1-URL OJS Analyzer + Recent Inspected list (left)
//! and full journal inspection, manuscript readiness & citation density check, guidelines, and `.docx`/`.zip`/`.ics` exports (right).

use crate::cache::SubmissionTrack;
use crate::config::SavedManuscriptEntry;
use crate::engine::file_ops::{format_editor_draft, open_url_with_feedback};
use crate::engine::manuscript::ManuscriptSummary;
use crate::engine::{EngineEvent, EventSink};
use crate::exporter::templates::{
    generate_cover_letter_with_profile, generate_editor_inquiry_email_with_profile,
    generate_originality_statement,
};
use crate::models::{compute_submission_readiness, FitStatus, UiJournalItem};
use crate::parser::doc_matcher::check_manuscript_word_fit;
use crate::ui::i18n::{AppLanguage, AppSettings};
use crate::ui::state::AppState;
use crate::ui::theme::{
    accent_button, card, card_divider, page_header, pill_badge, quartile_pill_colors,
    secondary_button, secondary_button_h, Palette,
};
use egui::{
    Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};

pub const SUBMISSION_STATUS_OPTIONS: [&str; 7] = [
    "Belum Dikirim",
    "Draft Siap",
    "Submitted",
    "Under Review",
    "Revision",
    "Accepted",
    "Rejected",
];

pub enum DetailPageAction {
    None,
    AnalyzeUrl(String),
    SelectInspected(UiJournalItem),
    LoadSavedManuscript(SavedManuscriptEntry),
    FindSimilar(UiJournalItem),
    ExportCoverDocx(UiJournalItem),
    ExportOriginalityDocx(UiJournalItem),
    ExportBundleZip(UiJournalItem),
    ExportIcs(UiJournalItem),
}

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

pub fn format_word_limit_label(word_limit: Option<(u32, u32)>, is_en: bool) -> String {
    match word_limit {
        Some((0, max_w)) => {
            if is_en {
                format!("Max {max_w} words")
            } else {
                format!("Maks. {max_w} kata")
            }
        }
        Some((min_w, max_w)) => {
            if is_en {
                format!("{min_w}–{max_w} words")
            } else {
                format!("{min_w}–{max_w} kata")
            }
        }
        None => {
            if is_en {
                "Not specified on OJS".to_string()
            } else {
                "Belum terdeteksi di OJS".to_string()
            }
        }
    }
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
fn render_left_panel(
    ui: &mut Ui,
    p: &Palette,
    is_en: bool,
    quick_url_input: &mut String,
    quick_url_loading: bool,
    inspected: Option<&UiJournalItem>,
    recent_inspected: &[UiJournalItem],
    manuscript: Option<&ManuscriptSummary>,
    saved_manuscripts: &[SavedManuscriptEntry],
    action: &mut DetailPageAction,
) {
    card(ui, p, |ui| {
        ui.label(
            RichText::new(if is_en {
                "Quick 1-URL OJS Analyzer"
            } else {
                "Cek Cepat 1 URL OJS"
            })
            .size(13.5)
            .strong()
            .color(p.text),
        );
        ui.label(
            RichText::new(if is_en {
                "Paste any OJS journal link directly without running a full search."
            } else {
                "Tempel link web OJS mana saja untuk audit APC, Editor-in-Chief & jadwal terbit instan."
            })
            .size(11.5)
            .color(p.muted),
        );
        ui.add_space(6.0);
        let resp = ui.add_sized(
            [ui.available_width(), 30.0],
            egui::TextEdit::singleline(quick_url_input)
                .hint_text("https://ejournal.kampus.ac.id/index.php/...")
                .margin(Margin::symmetric(8, 5)),
        );
        let enter_pressed = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.add_space(6.0);
        let btn_lbl = if quick_url_loading {
            if is_en {
                "Analyzing OJS..."
            } else {
                "Menganalisis URL..."
            }
        } else if is_en {
            "Analyze URL"
        } else {
            "Analisis URL"
        };
        let clicked = ui
            .add_enabled_ui(!quick_url_loading, |ui| {
                ui.set_min_width(ui.available_width());
                accent_button(ui, p, btn_lbl)
            })
            .inner
            .clicked();
        if (clicked || enter_pressed) && !quick_url_loading && !quick_url_input.trim().is_empty() {
            *action = DetailPageAction::AnalyzeUrl(quick_url_input.trim().to_string());
        }
    });

    ui.add_space(10.0);

    card(ui, p, |ui| {
        ui.label(
            RichText::new(if is_en {
                "Recent Inspected Journals"
            } else {
                "Riwayat Inspeksi Terakhir"
            })
            .size(13.0)
            .strong()
            .color(p.text),
        );
        card_divider(ui, p);

        if recent_inspected.is_empty() {
            ui.label(
                RichText::new(if is_en {
                    "Click \"Detail\" on any search result or analyze a URL above."
                } else {
                    "Belum ada jurnal dibuka. Klik \"Detail\" pada tabel hasil atau analisis URL di atas."
                })
                .size(11.5)
                .color(p.muted),
            );
        } else {
            ScrollArea::vertical()
                .id_salt("recent_inspected_scroll")
                .max_height(240.0)
                .show(ui, |ui| {
                    let active_key = inspected.map(AppState::bookmark_key);
                    for item in recent_inspected {
                        let is_active =
                            active_key.as_deref() == Some(AppState::bookmark_key(item).as_str());
                        let (bg, border) = if is_active {
                            (p.accent_soft, p.accent)
                        } else {
                            (p.control_bg, p.line)
                        };
                        let frame_resp = Frame::new()
                            .fill(bg)
                            .stroke(Stroke::new(1.0, border))
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&item.nama).size(12.0).strong().color(p.text),
                                    )
                                    .truncate(),
                                );
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    let rank = item
                                        .quartile
                                        .as_deref()
                                        .filter(|q| !q.trim().is_empty())
                                        .map(|q| q.to_string())
                                        .unwrap_or_else(|| {
                                            if item.sinta.trim().is_empty() || item.sinta == "-" {
                                                item.sumber.clone()
                                            } else {
                                                format!("S{}", item.sinta.trim())
                                            }
                                        });
                                    pill_badge(ui, &rank, p.accent, p.accent_soft);
                                    pill_badge(ui, &item.apc_label, p.muted, p.card_bg);
                                });
                            })
                            .response
                            .interact(Sense::click());
                        if frame_resp.clicked() {
                            *action = DetailPageAction::SelectInspected(item.clone());
                        }
                        ui.add_space(5.0);
                    }
                });
        }
    });

    ui.add_space(10.0);

    card(ui, p, |ui| {
        ui.label(
            RichText::new(if is_en {
                "My Manuscripts"
            } else {
                "Koleksi Naskah Saya"
            })
            .size(13.0)
            .strong()
            .color(p.text),
        );
        card_divider(ui, p);

        if saved_manuscripts.is_empty() {
            ui.label(
                RichText::new(if is_en {
                    "Upload a manuscript (.docx/.pdf) in Journal Search to save quick-switch slots here (up to 5)."
                } else {
                    "Unggah naskah (.docx/.pdf) di Pencarian Jurnal agar tersimpan untuk pindah naskah 1-klik (maks. 5)."
                })
                .size(11.5)
                .color(p.muted),
            );
        } else {
            let active_path = manuscript.map(|m| m.path.as_str());
            for entry in saved_manuscripts {
                let is_active = active_path
                    .map(|cur| cur.eq_ignore_ascii_case(&entry.file_path))
                    .unwrap_or(false);
                let (bg, border) = if is_active {
                    (p.good_soft, p.good)
                } else {
                    (p.control_bg, p.line)
                };
                let frame_resp = Frame::new()
                    .fill(bg)
                    .stroke(Stroke::new(1.0, border))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.add(
                            egui::Label::new(
                                RichText::new(&entry.file_name)
                                    .size(12.0)
                                    .strong()
                                    .color(p.text),
                            )
                            .truncate(),
                        );
                        let sub_txt = if entry.suggested_keywords.trim().is_empty() {
                            if is_en {
                                format!("{} words", entry.word_count)
                            } else {
                                format!("{} kata", entry.word_count)
                            }
                        } else if is_en {
                            format!("{} words • {}", entry.word_count, entry.suggested_keywords)
                        } else {
                            format!("{} kata • {}", entry.word_count, entry.suggested_keywords)
                        };
                        ui.add(
                            egui::Label::new(RichText::new(sub_txt).size(11.0).color(p.muted))
                                .truncate(),
                        );
                    })
                    .response
                    .on_hover_text(&entry.file_path)
                    .interact(Sense::click());
                if frame_resp.clicked() {
                    *action = DetailPageAction::LoadSavedManuscript(entry.clone());
                }
                ui.add_space(5.0);
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn render_right_detail_cards(
    ui: &mut Ui,
    p: &Palette,
    j: &UiJournalItem,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    selected_copy_idx: &mut usize,
    app_settings: &AppSettings,
    app_state: &mut AppState,
    sink: &EventSink,
    action: &mut DetailPageAction,
) {
    let is_en = app_settings.language == AppLanguage::En;
    let ctx = ui.ctx().clone();

    // Header Card
    card(ui, p, |ui| {
        ui.label(RichText::new(&j.nama).size(17.5).strong().color(p.text));
        ui.label(
            RichText::new(format!("{} • {}", j.penerbit, j.sumber))
                .size(12.0)
                .color(p.muted),
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if let Some(ref q) = j.quartile {
                if !q.trim().is_empty() {
                    let (q_fg, q_bg) = quartile_pill_colors(p, q);
                    pill_badge(ui, &format!("Intl {q}"), q_fg, q_bg);
                }
            }
            if !j.sinta.trim().is_empty() && j.sinta != "-" {
                pill_badge(
                    ui,
                    &format!("SINTA {}", j.sinta.trim()),
                    p.accent,
                    p.accent_soft,
                );
            }
            if j.match_score > 0.0 {
                let (fg, bg) = if j.match_score >= 70.0 {
                    (p.good, p.good_soft)
                } else {
                    (p.warn, p.warn_soft)
                };
                pill_badge(ui, &format!("Match {:.0}%", j.match_score), fg, bg);
            }
            if let Some(open) = j.submission_open {
                if open {
                    pill_badge(ui, "Open Submit / CFP", p.good, p.good_soft);
                } else {
                    pill_badge(ui, "Registrasi Tutup", p.bad, p.bad_soft);
                }
            }
            if j.issn_valid {
                pill_badge(ui, "ISSN Valid", p.good, p.good_soft);
            }
        });

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let target_url = if j.url.starts_with("http") {
                j.url.as_str()
            } else {
                j.garuda_url.as_str()
            };
            if accent_button(
                ui,
                p,
                if is_en {
                    "Open OJS Web"
                } else {
                    "Buka Web OJS"
                },
            )
            .clicked()
            {
                open_url_with_feedback(target_url, sink);
            }
            if !target_url.is_empty()
                && secondary_button(
                    ui,
                    p,
                    if is_en {
                        "Copy OJS Link"
                    } else {
                        "Salin Link OJS"
                    },
                )
                .clicked()
            {
                ctx.copy_text(target_url.to_string());
                sink.send(EngineEvent::Toast(
                    if is_en {
                        "Journal URL copied to clipboard."
                    } else {
                        "Link web OJS jurnal disalin ke clipboard."
                    }
                    .to_string(),
                ));
            }
            let starred = app_state.is_bookmarked(j);
            let star_lbl = if starred {
                if is_en {
                    "★ Shortlisted"
                } else {
                    "★ Pilihan Dospen"
                }
            } else if is_en {
                "☆ Add Shortlist"
            } else {
                "☆ Tandai Pilihan"
            };
            if secondary_button(ui, p, star_lbl).clicked() {
                let added = app_state.toggle_bookmark(j);
                sink.send(EngineEvent::Toast(if added {
                    format!("'{}' ditandai ke Shortlist & Dospen ★.", j.nama)
                } else {
                    format!("'{}' dihapus dari shortlist.", j.nama)
                }));
            }
            if secondary_button(
                ui,
                p,
                if is_en {
                    "Find Similar Journals"
                } else {
                    "Cari Jurnal Mirip"
                },
            )
            .clicked()
            {
                *action = DetailPageAction::FindSimilar(j.clone());
            }
        });
    });

    if let Some(ref warn) = j.risk_warning {
        ui.add_space(10.0);
        Frame::new()
            .fill(p.bad_soft)
            .stroke(Stroke::new(1.0, p.bad))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    RichText::new("⚠ Peringatan Audit Keamanan / Predatory & Cloned Detector")
                        .size(12.5)
                        .strong()
                        .color(p.bad),
                );
                ui.label(RichText::new(warn).size(12.0).color(p.text));
            });
    }

    ui.add_space(10.0);

    // Card 1: Kesiapan Naskah & Audit Pra-Submit (Readiness + Citation Density)
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
                    "Manuscript Readiness & Pre-Submit Audit"
                } else {
                    "Kesiapan Naskah & Audit Pra-Submit"
                })
                .size(13.5)
                .strong()
                .color(p.text),
            );
            pill_badge(ui, &format!("{}%", readiness.score), rd_fg, rd_bg);
        });
        card_divider(ui, p);
        for (ok, msg) in &readiness.checklist {
            let (mark, col) = if *ok {
                ("[✓]", p.good)
            } else {
                ("[!]", p.warn)
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(mark).size(12.0).strong().color(col));
                ui.label(RichText::new(msg).size(12.0).color(p.text));
            });
        }
        if let Some(pct) = j.max_similarity_pct {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("[✓]").size(12.0).strong().color(p.good));
                ui.label(
                    RichText::new(if is_en {
                        format!(
                            "Plagiarism / Similarity limit detected: Max {pct}% (check Turnitin/iThenticate before upload)."
                        )
                    } else {
                        format!(
                            "Batas Similarity / Turnitin terdeteksi: Maks. {pct}% (pastikan hasil cek plagiasi di bawah batas ini)."
                        )
                    })
                    .size(12.0)
                    .color(p.text),
                );
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("[•]").size(12.0).strong().color(p.muted));
                ui.label(
                    RichText::new(if is_en {
                        "Similarity / Turnitin limit: Not recorded on OJS."
                    } else {
                        "Batas Similarity / Turnitin: Belum terdata di halaman OJS."
                    })
                    .size(12.0)
                    .color(p.muted),
                );
            });
        }

        if let Some(ms) = manuscript {
            ui.add_space(4.0);
            if ms.uncited_long_paragraphs == 0 {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("[✓]").size(12.0).strong().color(p.good));
                    ui.label(
                        RichText::new(if is_en {
                            "Citation density OK: every long paragraph (>=110 words) cites a source."
                        } else {
                            "Kepadatan sitasi baik: seluruh paragraf panjang (>=110 kata) memuat sitasi."
                        })
                        .size(12.0)
                        .color(p.text),
                    );
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("[!]").size(12.0).strong().color(p.warn));
                    ui.label(
                        RichText::new(if is_en {
                            format!(
                                "{} long body paragraph(s) (>=110 words) lack an in-text citation [1]/(Author, Year).",
                                ms.uncited_long_paragraphs
                            )
                        } else {
                            format!(
                                "Terdeteksi {} paragraf panjang (>=110 kata) tanpa sitasi [1]/(Penulis, Tahun) — pertimbangkan tambah referensi.",
                                ms.uncited_long_paragraphs
                            )
                        })
                        .size(12.0)
                        .color(p.warn),
                    );
                });
            }

            if let Some(status) = check_manuscript_word_fit(ms.word_count as u32, j.word_limit) {
                let (badge, fg, bg) = match status {
                    FitStatus::Sesuai => ("Word Count Fit ✓", p.good, p.good_soft),
                    FitStatus::TerlaluPendek => ("Naskah Terlalu Pendek ⚠", p.warn, p.warn_soft),
                    FitStatus::MelebihiBatas => ("Melebihi Batas Kata ⚠", p.bad, p.bad_soft),
                };
                ui.horizontal(|ui| {
                    pill_badge(ui, badge, fg, bg);
                    ui.label(
                        RichText::new(format!("{} ({} kata)", ms.name, ms.word_count))
                            .size(11.5)
                            .color(p.muted),
                    );
                });
            }
        }
    });

    ui.add_space(10.0);

    // Card 2: Aturan Penulisan, Editor-in-Chief, Biaya APC & Kalender
    card(ui, p, |ui| {
        ui.label(
            RichText::new(if is_en {
                "Editorial Team, Guidelines, APC & Schedule"
            } else {
                "Informasi Redaksi, Aturan Penulisan, Biaya & Jadwal"
            })
            .size(13.5)
            .strong()
            .color(p.text),
        );
        card_divider(ui, p);

        if let Some(ref eic) = j.editor_in_chief {
            info_kv(ui, p, "Editor-in-Chief", eic);
        }
        info_kv(
            ui,
            p,
            if is_en {
                "Citation Style"
            } else {
                "Gaya Sitasi"
            },
            j.citation_style.as_deref().unwrap_or(if is_en {
                "Not specified on OJS"
            } else {
                "Belum terdeteksi di OJS"
            }),
        );
        info_kv(
            ui,
            p,
            if is_en {
                "Word Limit"
            } else {
                "Batas Panjang Kata"
            },
            &format_word_limit_label(j.word_limit, is_en),
        );
        let sim_disp = match j.max_similarity_pct {
            Some(pct) => {
                if is_en {
                    format!("Max. {pct}%")
                } else {
                    format!("Maks. {pct}%")
                }
            }
            None => {
                if is_en {
                    "Not recorded".to_string()
                } else {
                    "Belum terdata".to_string()
                }
            }
        };
        info_kv(
            ui,
            p,
            if is_en {
                "Similarity Limit / Turnitin"
            } else {
                "Batas Similarity / Turnitin"
            },
            &sim_disp,
        );
        let sub_disp = match j.submission_fee_idr {
            Some(0) => "Gratis (Rp 0 — tidak ada biaya submit)".to_string(),
            Some(v) => format!(
                "{} (terpisah dari APC)",
                crate::models::format_idr_short(v as i64)
            ),
            None if j.submit_label != "-" => j.submit_label.clone(),
            None => "Tidak dipungut / Tidak dicantumkan".to_string(),
        };
        info_kv(ui, p, "Biaya Submit (Submission Fee)", &sub_disp);
        info_kv(ui, p, "Biaya Publikasi (APC)", &j.apc_label);
        if !j.apc_evidence.trim().is_empty() {
            info_kv(ui, p, "Bukti Halaman Biaya", &j.apc_evidence);
        }
        let issn_disp = match (j.issn.is_empty(), j.eissn.is_empty()) {
            (true, true) => "-".to_string(),
            (false, true) => j.issn.clone(),
            (true, false) => format!("e-ISSN {}", j.eissn),
            (false, false) => format!("{} / e-ISSN {}", j.issn, j.eissn),
        };
        info_kv(
            ui,
            p,
            "ISSN / Validasi ISO 3297",
            &if j.issn_valid {
                format!("{issn_disp} (Valid)")
            } else {
                issn_disp
            },
        );
        if !j.extra_indexes.is_empty() {
            info_kv(ui, p, "Indeks Sekunder", &j.extra_indexes.join(", "));
        }
        let year_disp = j
            .last_issue_year
            .map(|y| format!("{y} ({})", j.activity_status))
            .unwrap_or_else(|| j.activity_status.clone());
        info_kv(ui, p, "Keaktifan Edisi", &year_disp);
        info_kv(ui, p, "Jadwal Terbit", &j.jadwal_terbit);
        info_kv(ui, p, "Terbit Terdekat", &j.terbit_terdekat);
        if !j.turnaround_label.trim().is_empty() {
            info_kv(ui, p, "Kecepatan Review", &j.turnaround_label);
        }
        if !j.article_languages.trim().is_empty() {
            info_kv(ui, p, "Bahasa Artikel", &j.article_languages);
        }
        if j.cited_count > 0 {
            info_kv(
                ui,
                p,
                "Sitasi di Daftar Pustaka",
                &format!("Dikutip {}x di naskah Anda", j.cited_count),
            );
        }
        let not_recorded = if is_en {
            "Not recorded"
        } else {
            "Belum terdata"
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Email Editor:")
                    .size(12.0)
                    .strong()
                    .color(p.muted),
            );
            if j.email.trim().is_empty() {
                ui.label(RichText::new(not_recorded).size(12.0).color(p.muted));
            } else {
                ui.label(RichText::new(j.email.trim()).size(12.0).color(p.text));
                if j.email.contains('@') && secondary_button_h(ui, p, "Salin Email", 24.0).clicked()
                {
                    ctx.copy_text(j.email.clone());
                    sink.send(EngineEvent::Toast("Email editor disalin.".to_string()));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Telepon / WA:")
                    .size(12.0)
                    .strong()
                    .color(p.muted),
            );
            if j.telepon.trim().is_empty() {
                ui.label(RichText::new(not_recorded).size(12.0).color(p.muted));
            } else {
                ui.label(RichText::new(j.telepon.trim()).size(12.0).color(p.text));
                if let Some(wa_url) = extract_whatsapp_url(&j.telepon) {
                    if secondary_button_h(ui, p, "Buka Chat WA", 24.0).clicked() {
                        open_url_with_feedback(&wa_url, sink);
                    }
                }
            }
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new("Focus & Scope:")
                .size(12.0)
                .strong()
                .color(p.muted),
        );
        let focus_txt = if j.focus_scope.trim().is_empty() {
            "Belum diambil dari halaman OJS (cek website resmi)."
        } else {
            j.focus_scope.trim()
        };
        ui.label(RichText::new(focus_txt).size(12.0).color(p.text));
        if !j.match_reasons.is_empty() {
            ui.label(
                RichText::new(format!("Alasan Kecocokan: {}", j.match_reasons.join(" • ")))
                    .size(11.5)
                    .color(p.muted),
            );
        }

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if secondary_button(
                ui,
                p,
                if is_en {
                    "Save Calendar Reminder (.ics)"
                } else {
                    "Simpan Pengingat Kalender (.ics)"
                },
            )
            .clicked()
            {
                *action = DetailPageAction::ExportIcs(j.clone());
            }
            if let Some(ref tpl_url) = j.template_url {
                if secondary_button(
                    ui,
                    p,
                    if is_en {
                        "Download Journal Template"
                    } else {
                        "Unduh Template Naskah"
                    },
                )
                .on_hover_text(tpl_url)
                .clicked()
                {
                    open_url_with_feedback(tpl_url, sink);
                }
            }
        });
    });

    ui.add_space(10.0);

    // Card 3: Generator Berkas Siap Kirim (.docx / .zip / Draf Teks)
    card(ui, p, |ui| {
        ui.label(
            RichText::new(if is_en {
                "Ready-to-Send Documents (.docx, .zip & Quick Copy)"
            } else {
                "Generator Berkas Siap Kirim (.docx, .zip & Salin Cepat)"
            })
            .size(13.5)
            .strong()
            .color(p.text),
        );
        ui.label(
            RichText::new(if is_en {
                "Auto-fills Author Profile from Settings and Editor-in-Chief greeting when detected."
            } else {
                "Mengisi otomatis Profil Penulis dari Pengaturan dan sapaan Editor-in-Chief bila terdeteksi."
            })
            .size(11.5)
            .color(p.muted),
        );
        card_divider(ui, p);

        ui.horizontal_wrapped(|ui| {
            if accent_button(
                ui,
                p,
                if is_en {
                    "Download Cover Letter (.docx)"
                } else {
                    "Unduh Cover Letter (.docx)"
                },
            )
            .clicked()
            {
                *action = DetailPageAction::ExportCoverDocx(j.clone());
            }
            if accent_button(
                ui,
                p,
                if is_en {
                    "Download Originality Statement (.docx)"
                } else {
                    "Unduh Surat Keaslian (.docx)"
                },
            )
            .clicked()
            {
                *action = DetailPageAction::ExportOriginalityDocx(j.clone());
            }
            if secondary_button(
                ui,
                p,
                if is_en {
                    "Download Submission Bundle (.zip)"
                } else {
                    "Unduh Paket Pengajuan (.zip)"
                },
            )
            .clicked()
            {
                *action = DetailPageAction::ExportBundleZip(j.clone());
            }
        });

        ui.add_space(10.0);
        const COPY_TEMPLATES: [&str; 6] = [
            "Cover Letter (ID)",
            "Cover Letter (EN)",
            "Surat Keaslian (ID)",
            "Email Inquiry Editor (ID)",
            "Email Inquiry Editor (EN)",
            "Draf Pesan WA",
        ];
        let safe_idx = (*selected_copy_idx).min(COPY_TEMPLATES.len() - 1);
        *selected_copy_idx = safe_idx;

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(
                RichText::new(if is_en {
                    "Quick Text Draft:"
                } else {
                    "Salin Draf Teks:"
                })
                .size(12.0)
                .strong()
                .color(p.muted),
            );
            egui::ComboBox::from_id_salt("detail_quick_copy_combo")
                .selected_text(COPY_TEMPLATES[safe_idx])
                .width(210.0)
                .show_ui(ui, |ui| {
                    for (idx, label) in COPY_TEMPLATES.iter().enumerate() {
                        ui.selectable_value(selected_copy_idx, idx, *label);
                    }
                });
            let copy_lbl = if is_en { "Copy Text" } else { "Salin Teks" };
            if secondary_button(ui, p, copy_lbl).clicked() {
                let (content, toast_msg) = match *selected_copy_idx {
                    0 => (
                        generate_cover_letter_with_profile(
                            j,
                            keyword,
                            manuscript,
                            Some(app_settings),
                            "id",
                        ),
                        "Cover Letter (ID) disalin ke clipboard.",
                    ),
                    1 => (
                        generate_cover_letter_with_profile(
                            j,
                            keyword,
                            manuscript,
                            Some(app_settings),
                            "en",
                        ),
                        "Cover Letter (EN) copied to clipboard.",
                    ),
                    2 => (
                        generate_originality_statement(j, manuscript, Some(app_settings), "id"),
                        "Surat Keaslian bermaterai disalin ke clipboard.",
                    ),
                    3 => (
                        generate_editor_inquiry_email_with_profile(
                            j,
                            keyword,
                            manuscript,
                            Some(app_settings),
                            "id",
                        ),
                        "Draf Email Inquiry Editor (ID) disalin ke clipboard.",
                    ),
                    4 => (
                        generate_editor_inquiry_email_with_profile(
                            j,
                            keyword,
                            manuscript,
                            Some(app_settings),
                            "en",
                        ),
                        "English Editor Inquiry Email copied to clipboard.",
                    ),
                    _ => (
                        format_editor_draft(&j.nama, keyword),
                        "Draf pesan WA ke editor disalin ke clipboard.",
                    ),
                };
                ctx.copy_text(content);
                sink.send(EngineEvent::Toast(toast_msg.to_string()));
            }
        });
    });

    // Keep track helper aligned silently if user inspects another page later
    let _canonical = SubmissionTrack::canonical_key(&j.url, &j.nama);
}

#[allow(clippy::too_many_arguments)]
pub fn render_detail_page(
    ui: &mut Ui,
    p: &Palette,
    inspected: Option<&UiJournalItem>,
    recent_inspected: &[UiJournalItem],
    quick_url_input: &mut String,
    quick_url_loading: bool,
    keyword: &str,
    manuscript: Option<&ManuscriptSummary>,
    saved_manuscripts: &[SavedManuscriptEntry],
    selected_copy_idx: &mut usize,
    app_settings: &AppSettings,
    app_state: &mut AppState,
    sink: &EventSink,
) -> DetailPageAction {
    let is_en = app_settings.language == AppLanguage::En;
    page_header(
        ui,
        p,
        if is_en {
            "Journal Workspace & Pre-Submission Prep"
        } else {
            "Detail Jurnal & Persiapan Submit"
        },
        if is_en {
            "Inspect journal guidelines, audit manuscript readiness, or check any single OJS URL."
        } else {
            "Audit kesiapan naskah, cek cepat 1 URL OJS langsung, dan unduh berkas pengajuan (.docx/.zip/.ics)."
        },
    );

    let mut action = DetailPageAction::None;
    let total_w = ui.available_width();
    let left_w = 290.0_f32.min((total_w * 0.34).max(240.0));
    let avail_h = ui.available_height();

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        // Left column (~290px): Quick 1-URL OJS Analyzer, Recent list & Saved Manuscripts
        ui.allocate_ui_with_layout(
            Vec2::new(left_w, avail_h),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_max_width(left_w);
                ScrollArea::vertical()
                    .id_salt("detail_left_col_scroll")
                    .show(ui, |ui| {
                        render_left_panel(
                            ui,
                            p,
                            is_en,
                            quick_url_input,
                            quick_url_loading,
                            inspected,
                            recent_inspected,
                            manuscript,
                            saved_manuscripts,
                            &mut action,
                        );
                    });
            },
        );

        // Right column: Spacious scrollable workspace
        let right_w = (total_w - left_w - 14.0).max(320.0);
        ui.allocate_ui_with_layout(
            Vec2::new(right_w, avail_h),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_max_width(right_w);
                ScrollArea::vertical().id_salt("detail_workspace_scroll").show(ui, |ui| {
                    if let Some(j) = inspected {
                        render_right_detail_cards(
                            ui,
                            p,
                            j,
                            keyword,
                            manuscript,
                            selected_copy_idx,
                            app_settings,
                            app_state,
                            sink,
                            &mut action,
                        );
                    } else {
                        card(ui, p, |ui| {
                            ui.add_space(16.0);
                            ui.label(
                                RichText::new(if is_en {
                                    "No Journal Selected Yet"
                                } else {
                                    "Belum Ada Jurnal Dipilih"
                                })
                                .size(15.5)
                                .strong()
                                .color(p.text),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(if is_en {
                                    "Click \"Detail\" on any journal row in Journal Search, or paste an OJS link on the left panel."
                                } else {
                                    "Klik tombol \"Detail\" pada salah satu jurnal di menu Pencarian Jurnal, atau tempel langsung URL OJS di panel sebelah kiri."
                                })
                                .size(12.5)
                                .color(p.muted),
                            );
                            ui.add_space(16.0);
                        });
                    }
                });
            },
        );
    });

    action
}
