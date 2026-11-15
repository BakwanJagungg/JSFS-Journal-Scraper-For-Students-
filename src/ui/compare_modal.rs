//! Side-by-side Journal Comparison Modal (2..=4 shortlisted journals across 8 key criteria).

use crate::engine::manuscript::ManuscriptSummary;
use crate::models::{FitStatus, UiJournalItem};
use crate::parser::doc_matcher::check_manuscript_word_fit;
use crate::ui::i18n::AppLanguage;
use crate::ui::theme::{
    accent_button, pill_badge, quartile_pill_colors, secondary_button, Palette,
};
use egui::{Align, CornerRadius, Frame, Grid, Layout, Margin, RichText, ScrollArea, Stroke, Ui};

pub enum CompareModalAction {
    None,
    Close,
    OpenDetail(usize),
}

fn format_word_limit_label(word_limit: Option<(u32, u32)>, is_en: bool) -> String {
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
        None => "-".to_string(),
    }
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

fn metric_row_label(ui: &mut Ui, p: &Palette, title: &str) {
    ui.label(RichText::new(title).size(12.0).strong().color(p.muted));
}

/// Render an in-app side-by-side comparison modal for 2..=4 bookmarked journals.
pub fn render_compare_modal(
    ctx: &egui::Context,
    p: &Palette,
    compared: &[(usize, UiJournalItem)],
    manuscript: Option<&ManuscriptSummary>,
    lang: AppLanguage,
) -> CompareModalAction {
    if compared.len() < 2 {
        return CompareModalAction::Close;
    }

    let is_en = matches!(lang, AppLanguage::En);
    let mut action = CompareModalAction::None;
    let mut close_requested = false;

    let modal_resp = egui::Modal::new(egui::Id::unique("journal_compare_modal")).show(ctx, |ui| {
        let max_w = (ctx.content_rect().width() - 40.0).min(980.0);
        ui.set_width(max_w);
        ui.set_max_height(ctx.content_rect().height() - 60.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(if is_en {
                        "Side-by-Side Journal Comparison"
                    } else {
                        "Bandingkan Jurnal Pilihan (Side-by-Side)"
                    })
                    .size(17.0)
                    .strong()
                    .color(p.text),
                );
                ui.label(
                    RichText::new(if is_en {
                        "Comparing shortlisted journals across accreditation, match score, APC, review speed, and manuscript fit."
                    } else {
                        "Perbandingan langsung jurnal bertanda ★ berdasarkan akreditasi, skor kecocokan, biaya APC, kecepatan review, dan syarat naskah."
                    })
                    .size(12.0)
                    .color(p.muted),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                if secondary_button(ui, p, if is_en { "Close" } else { "Tutup" }).clicked() {
                    close_requested = true;
                }
            });
        });

        ui.add_space(10.0);

        let col_count = compared.len().min(4);
        let col_width = ((max_w - 175.0) / col_count as f32).clamp(155.0, 215.0);

        ScrollArea::both().max_height(460.0).show(ui, |ui| {
            Frame::new()
                .fill(p.card_bg)
                .stroke(Stroke::new(1.0, p.line))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    Grid::new("compare_journals_grid")
                        .num_columns(1 + col_count)
                        .spacing([12.0, 10.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Row 0: Journal Headers
                            metric_row_label(
                                ui,
                                p,
                                if is_en { "Journal Name" } else { "Nama Jurnal" },
                            );
                            for (_, j) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 46.0].into(), |ui| {
                                    ui.vertical(|ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&j.nama)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(p.text),
                                            )
                                            .wrap(),
                                        );
                                        ui.label(
                                            RichText::new(&j.penerbit)
                                                .size(11.0)
                                                .color(p.muted),
                                        );
                                    });
                                });
                            }
                            ui.end_row();

                            // Row 1: Accreditation / Quartile
                            metric_row_label(
                                ui,
                                p,
                                if is_en {
                                    "Accreditation"
                                } else {
                                    "Akreditasi / Indeks"
                                },
                            );
                            for (_, j) in compared.iter().take(4) {
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

                            // Row 2: Match Score + Cite-to-Target
                            metric_row_label(
                                ui,
                                p,
                                if is_en {
                                    "Match Score"
                                } else {
                                    "Kesesuaian Naskah"
                                },
                            );
                            for (_, j) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 32.0].into(), |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        let m_lbl = if j.match_score > 0.0 {
                                            format!("{:.0}% • {}", j.match_score, j.kesesuaian_naskah)
                                        } else {
                                            "-".to_string()
                                        };
                                        let (fg, bg) = if j.match_score >= 70.0 {
                                            (p.good, p.good_soft)
                                        } else if j.match_score >= 45.0 {
                                            (p.warn, p.warn_soft)
                                        } else {
                                            (p.muted, p.control_bg)
                                        };
                                        pill_badge(ui, &m_lbl, fg, bg);
                                        if j.cited_count > 0 {
                                            pill_badge(
                                                ui,
                                                &format!("Dikutip {}x", j.cited_count),
                                                p.accent,
                                                p.accent_soft,
                                            );
                                        }
                                    });
                                });
                            }
                            ui.end_row();

                            // Row 3: APC Fee (unified IDR/USD)
                            metric_row_label(ui, p, if is_en { "APC Fee" } else { "Biaya APC" });
                            for (_, j) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 24.0].into(), |ui| {
                                    let (fg, bg) = if j.apc_category == "free" {
                                        (p.good, p.good_soft)
                                    } else if j.apc_category == "paid" {
                                        (p.warn, p.warn_soft)
                                    } else {
                                        (p.muted, p.control_bg)
                                    };
                                    pill_badge(ui, &j.apc_label, fg, bg);
                                });
                            }
                            ui.end_row();

                            // Row 4: Turnaround / Review Speed
                            metric_row_label(
                                ui,
                                p,
                                if is_en {
                                    "Review Speed"
                                } else {
                                    "Kecepatan Review"
                                },
                            );
                            for (_, j) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 22.0].into(), |ui| {
                                    let txt = if j.turnaround_label.trim().is_empty() {
                                        "-"
                                    } else {
                                        j.turnaround_label.trim()
                                    };
                                    ui.label(RichText::new(txt).size(12.0).color(p.text));
                                });
                            }
                            ui.end_row();

                            // Row 5: Publish Schedule & Next Issue
                            metric_row_label(
                                ui,
                                p,
                                if is_en {
                                    "Publish Schedule"
                                } else {
                                    "Jadwal & Terbit Terdekat"
                                },
                            );
                            for (_, j) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 36.0].into(), |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&j.jadwal_terbit)
                                                .size(12.0)
                                                .color(p.text),
                                        );
                                        if !j.terbit_terdekat.trim().is_empty()
                                            && j.terbit_terdekat != "-"
                                        {
                                            ui.label(
                                                RichText::new(format!(
                                                    "Next: {}",
                                                    j.terbit_terdekat
                                                ))
                                                .size(11.0)
                                                .color(p.muted),
                                            );
                                        }
                                    });
                                });
                            }
                            ui.end_row();

                            // Row 6: Citation Style
                            metric_row_label(
                                ui,
                                p,
                                if is_en {
                                    "Citation Style"
                                } else {
                                    "Gaya Sitasi"
                                },
                            );
                            for (_, j) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 22.0].into(), |ui| {
                                    let sty = j.citation_style.as_deref().unwrap_or("-");
                                    ui.label(RichText::new(sty).size(12.0).strong().color(p.text));
                                });
                            }
                            ui.end_row();

                            // Row 7: Word Limit & Pre-Flight Fit
                            metric_row_label(
                                ui,
                                p,
                                if is_en {
                                    "Word Limit & Fit"
                                } else {
                                    "Batas Kata & Fit"
                                },
                            );
                            for (_, j) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 38.0].into(), |ui| {
                                    ui.vertical(|ui| {
                                        let limit_lbl = format_word_limit_label(j.word_limit, is_en);
                                        ui.label(RichText::new(limit_lbl).size(12.0).color(p.text));
                                        if let Some(ms) = manuscript {
                                            if let Some(st) = check_manuscript_word_fit(
                                                ms.word_count as u32,
                                                j.word_limit,
                                            ) {
                                                let (lbl, fg, bg) = match st {
                                                    FitStatus::Sesuai => (
                                                        if is_en { "✓ Fits" } else { "✓ Sesuai" },
                                                        p.good,
                                                        p.good_soft,
                                                    ),
                                                    FitStatus::TerlaluPendek => (
                                                        if is_en {
                                                            "⚠ Too Short"
                                                        } else {
                                                            "⚠ Terlalu Pendek"
                                                        },
                                                        p.warn,
                                                        p.warn_soft,
                                                    ),
                                                    FitStatus::MelebihiBatas => (
                                                        if is_en {
                                                            "⚠ Over Limit"
                                                        } else {
                                                            "⚠ Melebihi Batas"
                                                        },
                                                        p.bad,
                                                        p.bad_soft,
                                                    ),
                                                };
                                                pill_badge(ui, lbl, fg, bg);
                                            }
                                        }
                                    });
                                });
                            }
                            ui.end_row();

                            // Row 8: Language
                            metric_row_label(
                                ui,
                                p,
                                if is_en {
                                    "Article Language"
                                } else {
                                    "Bahasa Naskah"
                                },
                            );
                            for (_, j) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 22.0].into(), |ui| {
                                    let l = if j.article_languages.trim().is_empty() {
                                        "-"
                                    } else {
                                        j.article_languages.trim()
                                    };
                                    ui.label(RichText::new(l).size(12.0).color(p.text));
                                });
                            }
                            ui.end_row();

                            // Action row: Open Detail
                            metric_row_label(ui, p, if is_en { "Inspect" } else { "Aksi" });
                            for (orig_idx, _) in compared.iter().take(4) {
                                ui.allocate_ui([col_width, 28.0].into(), |ui| {
                                    if accent_button(
                                        ui,
                                        p,
                                        if is_en { "Open Detail" } else { "Lihat Detail" },
                                    )
                                    .clicked()
                                    {
                                        action = CompareModalAction::OpenDetail(*orig_idx);
                                    }
                                });
                            }
                            ui.end_row();
                        });
                });
        });
    });

    if modal_resp.should_close() || close_requested {
        CompareModalAction::Close
    } else {
        action
    }
}
