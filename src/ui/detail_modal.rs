//! Native modal inspector showing complete journal accuracy evidence, activity year, Focus & Scope,
//! Editor contact links (Email / WhatsApp), and 1-click OJS open or WA inquiry draft copy.

use crate::engine::file_ops::{format_editor_draft, open_url_with_feedback};
use crate::engine::{EngineEvent, EventSink};
use crate::models::UiJournalItem;
use crate::ui::theme::{accent_button, card, pill_badge, secondary_button, Palette};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui};

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

pub fn render_detail_modal(
    ctx: &egui::Context,
    p: &Palette,
    item: Option<&UiJournalItem>,
    keyword: &str,
    sink: &EventSink,
) -> bool {
    let Some(j) = item else {
        return false;
    };

    let mut close_requested = false;
    let modal_resp = egui::Modal::new(egui::Id::unique("journal_detail_modal")).show(ctx, |ui| {
        ui.set_width(640.0_f32.min(ctx.content_rect().width() - 48.0));
        ui.set_max_height(ctx.content_rect().height() - 64.0);

        // Header
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width(540.0);
                ui.label(RichText::new(&j.nama).size(17.5).strong().color(p.text));
                ui.horizontal_wrapped(|ui| {
                    let sinta_lbl = if j.sinta.is_empty() {
                        "Garuda Non-SINTA".to_string()
                    } else {
                        format!("SINTA {}", j.sinta)
                    };
                    pill_badge(ui, &sinta_lbl, p.accent, p.accent_soft);

                    let act_fg = if j.is_dormant {
                        p.bad
                    } else if j.activity_status.contains("Aktif") {
                        p.good
                    } else {
                        p.warn
                    };
                    let act_bg = if j.is_dormant {
                        p.bad_soft
                    } else if j.activity_status.contains("Aktif") {
                        p.good_soft
                    } else {
                        p.warn_soft
                    };
                    pill_badge(ui, &j.activity_status, act_fg, act_bg);

                    let match_lbl =
                        format!("Match {:.0}% • {}", j.match_score, j.kesesuaian_naskah);
                    pill_badge(ui, &match_lbl, p.good, p.good_soft);
                });
            });
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                if secondary_button(ui, p, "Tutup").clicked() {
                    close_requested = true;
                }
            });
        });

        ui.add_space(10.0);

        ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
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
                info_kv(ui, p, "ISSN", &issn_disp);
                let year_disp = j
                    .last_issue_year
                    .map(|y| format!("{y} ({})", j.activity_status))
                    .unwrap_or_else(|| j.activity_status.clone());
                info_kv(ui, p, "Tahun Edisi Terakhir", &year_disp);
                info_kv(ui, p, "Biaya Publikasi (APC)", &j.apc_label);
                if j.submit_label != "-" {
                    info_kv(ui, p, "Submission Fee", &j.submit_label);
                }
                info_kv(ui, p, "Jadwal Terbit", &j.jadwal_terbit);
                info_kv(ui, p, "Terbit Terdekat", &j.terbit_terdekat);
            });

            if !j.apc_evidence.trim().is_empty() {
                ui.add_space(8.0);
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

            ui.add_space(8.0);
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

            ui.add_space(8.0);
            card(ui, p, |ui| {
                ui.label(
                    RichText::new("Kontak Redaksi Editor")
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
            });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.set_min_width(ui.available_width());
            if secondary_button(ui, p, "Salin Draf Pesan WA").clicked() {
                let draft = format_editor_draft(&j.nama, keyword);
                ctx.copy_text(draft);
                sink.send(EngineEvent::Toast(
                    "Draf pesan sopan ke editor disalin ke clipboard.".to_string(),
                ));
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

    modal_resp.should_close() || close_requested
}
