//! Advanced search criteria arranged in System-Settings grouped cards.

use crate::engine::{EngineEvent, EventSink};
use crate::ui::i18n::{t, AppLanguage};
use crate::ui::state::{FilterSettings, QuickPreset, MONTH_NAMES};
use crate::ui::theme::{card, card_divider, page_header, secondary_button, toggle_row, Palette};
use egui::{Align2, CornerRadius, Frame, Margin, RichText, ScrollArea, Sense, Stroke, Ui};

fn selectable_chip(ui: &mut Ui, p: &Palette, active: bool, text: &str) -> bool {
    let fill = if active { p.accent } else { p.control_bg };
    let border = if active { p.accent } else { p.line };
    let fg = if active { egui::Color32::WHITE } else { p.text };
    let resp = Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(11, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.0).strong().color(fg));
        })
        .response
        .interact(Sense::click());
    resp.clicked()
}

pub fn render_filters_page(
    ui: &mut Ui,
    p: &Palette,
    lang: AppLanguage,
    filters: &mut FilterSettings,
    target_text: &mut String,
    sink: &EventSink,
) {
    let txt = t(lang);
    page_header(ui, p, txt.filters_title, txt.filters_subtitle);

    ScrollArea::vertical().show(ui, |ui| {
        // Card 1: SINTA, Q1–Q4 & Portal Sources
        card(ui, p, |ui| {
            ui.label(
                RichText::new(txt.filters_card1)
                    .size(13.5)
                    .strong()
                    .color(p.text),
            );
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                for rank in 0..6 {
                    if selectable_chip(
                        ui,
                        p,
                        filters.sinta_enabled[rank],
                        &format!("SINTA {}", rank + 1),
                    ) {
                        filters.sinta_enabled[rank] = !filters.sinta_enabled[rank];
                        filters.active_preset = QuickPreset::Custom;
                    }
                }
            });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(txt.filters_intl_section)
                        .size(12.0)
                        .color(p.muted),
                );
                for q_idx in 0..4 {
                    if selectable_chip(
                        ui,
                        p,
                        filters.quartile_enabled[q_idx],
                        &format!("Q{}", q_idx + 1),
                    ) {
                        filters.quartile_enabled[q_idx] = !filters.quartile_enabled[q_idx];
                        filters.active_preset = QuickPreset::Custom;
                    }
                }
            });

            card_divider(ui, p);

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Target Jurnal (1–1000):").size(12.5).color(p.text));
                for preset_t in [10_usize, 25, 50, 100, 250, 500, 1000] {
                    if selectable_chip(
                        ui,
                        p,
                        filters.target_count == preset_t,
                        &preset_t.to_string(),
                    ) {
                        filters.clamp_target(preset_t);
                        *target_text = filters.target_count.to_string();
                    }
                }
            });

            card_divider(ui, p);
            toggle_row(
                ui,
                p,
                &mut filters.garuda,
                "Portal Garuda Kemdiktisaintek",
                "Mencari indeks artikel & profil jurnal nasional langsung dari Garuda.",
            );
            card_divider(ui, p);
            toggle_row(
                ui,
                p,
                &mut filters.sinta_src,
                "Portal SINTA Kemdiktisaintek",
                "Mengambil peringkat akreditasi resmi SINTA 1–6 sebagai prioritas utama.",
            );
            card_divider(ui, p);
            let mut intl_on = filters.openalex || filters.doaj;
            if toggle_row(
                ui,
                p,
                &mut intl_on,
                txt.filters_intl_toggle,
                txt.filters_intl_hint,
            )
            .changed()
            {
                filters.openalex = intl_on;
                filters.doaj = intl_on;
                filters.active_preset = QuickPreset::Custom;
            }
            card_divider(ui, p);
            toggle_row(
                ui,
                p,
                &mut filters.include_non_sinta_garuda,
                "Sertakan jurnal Garuda yang belum terakreditasi SINTA",
                "Nonaktifkan agar hanya jurnal terverifikasi peringkat SINTA pilihan yang lolos.",
            );
        });

        ui.add_space(12.0);

        // Card 2: Publish Months (12-month grid)
        card(ui, p, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(txt.filters_months_card)
                        .size(13.5)
                        .strong()
                        .color(p.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if secondary_button(ui, p, "Kosongkan").clicked() {
                        filters.months = [false; 12];
                        filters.active_preset = QuickPreset::Custom;
                    }
                    if secondary_button(ui, p, "3 Bln Dekat").clicked() {
                        filters.select_near_months();
                        filters.active_preset = QuickPreset::TerbitTerdekat;
                    }
                    if secondary_button(ui, p, "Semua").clicked() {
                        filters.months = [true; 12];
                        filters.active_preset = QuickPreset::Custom;
                    }
                });
            });

            ui.add_space(8.0);
            ui.columns(6, |cols| {
                for (m_idx, m_name) in MONTH_NAMES.iter().enumerate() {
                    let col = &mut cols[m_idx % 6];
                    if m_idx >= 6 {
                        col.add_space(6.0);
                    }
                    if selectable_chip(col, p, filters.months[m_idx], m_name) {
                        filters.months[m_idx] = !filters.months[m_idx];
                        filters.active_preset = QuickPreset::Custom;
                    }
                }
            });

            card_divider(ui, p);
            toggle_row(
                ui,
                p,
                &mut filters.include_unknown_month,
                "Sertakan jurnal yang jadwal bulannya belum terdata",
                "Tetap tampilkan kandidat potensial bila OJS tidak menulis nama bulan secara eksplisit.",
            );
        });

        ui.add_space(12.0);

        // Card 3: Budget APC & Activity Quality Gate
        card(ui, p, |ui| {
            ui.label(
                RichText::new(txt.filters_budget_card)
                    .size(13.5)
                    .strong()
                    .color(p.text),
            );
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                cols[0].label(RichText::new("Minimum Biaya (Rp)").size(12.0).color(p.muted));
                cols[0].add_space(4.0);
                if cols[0]
                    .add_sized(
                        [cols[0].available_width(), 32.0],
                        egui::TextEdit::singleline(&mut filters.min_rp_text)
                            .align(Align2::LEFT_CENTER)
                            .margin(Margin::symmetric(8, 5)),
                    )
                    .changed()
                {
                    filters.sync_budget_from_text();
                    filters.active_preset = QuickPreset::Custom;
                }

                cols[1].label(RichText::new("Maksimum Biaya (Rp)").size(12.0).color(p.muted));
                cols[1].add_space(4.0);
                if cols[1]
                    .add_sized(
                        [cols[1].available_width(), 32.0],
                        egui::TextEdit::singleline(&mut filters.max_rp_text)
                            .align(Align2::LEFT_CENTER)
                            .margin(Margin::symmetric(8, 5)),
                    )
                    .changed()
                {
                    filters.sync_budget_from_text();
                    filters.active_preset = QuickPreset::Custom;
                }
            });

            card_divider(ui, p);
            if toggle_row(
                ui,
                p,
                &mut filters.only_free,
                "Hanya Jurnal Gratis (Rp 0)",
                "Strictly menyaring keluar semua jurnal dengan biaya publikasi > Rp 0.",
            )
            .changed()
            {
                filters.active_preset = QuickPreset::Custom;
            }
            card_divider(ui, p);
            toggle_row(
                ui,
                p,
                &mut filters.include_na,
                "Sertakan biaya \"Cek Web / Belum Terdata\"",
                "Berguna bila jurnal OJS menulis biaya di halaman eksternal atau PDF panduan.",
            );
            card_divider(ui, p);
            toggle_row(
                ui,
                p,
                &mut filters.exclude_dormant,
                "Sembunyikan jurnal tidak aktif / dorman (>2 tahun tidak terbit)",
                "Direkomendasikan agar tidak menyarankan jurnal mati/vakum (2019–2023).",
            );
        });

        ui.add_space(12.0);

        // Card 4: AI & Export Formats
        card(ui, p, |ui| {
            ui.label(
                RichText::new(txt.filters_ai_card)
                    .size(13.5)
                    .strong()
                    .color(p.text),
            );
            ui.add_space(6.0);

            toggle_row(
                ui,
                p,
                &mut filters.match_enabled,
                "Hitung skor kesesuaian topik (Match Score)",
                "Memakai Bilingual BM25 + Sinonim Akademik ID-EN terhadap Focus & Scope.",
            );
            card_divider(ui, p);
            toggle_row(
                ui,
                p,
                &mut filters.use_ai,
                "Aktifkan Pengayaan Match Engine / AI",
                "Mengikuti mode terpilih di halaman Match Engine & AI (Built-in / Cloud / Ollama).",
            );
            card_divider(ui, p);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Format Ekspor Output:").size(12.5).color(p.text));
                ui.checkbox(&mut filters.fmt_xlsx, "Excel (.xlsx)");
                ui.checkbox(&mut filters.fmt_md, "Markdown (.md)");
                ui.checkbox(&mut filters.fmt_csv, "CSV (.csv)");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if secondary_button(ui, p, txt.filters_reset_btn).clicked() {
                        *filters = FilterSettings::default();
                        *target_text = filters.target_count.to_string();
                        sink.send(EngineEvent::Toast("Filter dikembalikan ke default.".to_string()));
                    }
                });
            });
        });
    });
}
