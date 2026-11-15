//! Quick sharing, Excel/folder launchers, SQLite cache maintenance, and live diagnostic logs.

use crate::engine::cache_ops::clear_cache_async;
use crate::engine::file_ops::{
    format_editor_draft, format_wa_share_from_ui, open_excel_async, open_logs_folder_async,
    open_output_folder_async,
};
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::models::UiJournalItem;
use crate::ui::theme::{
    accent_button, card, card_divider, page_header, pill_badge, secondary_button, Palette,
};
use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn render_history_page(
    ui: &mut Ui,
    p: &Palette,
    results: &[UiJournalItem],
    selected_idx: Option<usize>,
    keyword: &str,
    last_excel_path: Option<&str>,
    cache_count: usize,
    logs: &mut Vec<String>,
    engine: &Arc<EngineState>,
    sink: &EventSink,
) {
    page_header(
        ui,
        p,
        "Riwayat & Ekspor",
        "Salin daftar jurnal siap kirim ke WhatsApp, buka laporan Excel/Markdown, atau kelola cache lokal.",
    );

    ScrollArea::vertical().show(ui, |ui| {
        card(ui, p, |ui| {
            ui.label(
                RichText::new("Berbagi & Ekspor Cepat")
                    .size(13.5)
                    .strong()
                    .color(p.text),
            );
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                let has_rows = !results.is_empty();
                if ui
                    .add_enabled_ui(has_rows, |ui| accent_button(ui, p, "Salin Daftar WA"))
                    .inner
                    .clicked()
                {
                    let refs: Vec<&UiJournalItem> = results.iter().collect();
                    let text = format_wa_share_from_ui(&refs);
                    ui.ctx().copy_text(text);
                    sink.send(EngineEvent::Toast(format!(
                        "Daftar WA untuk {} jurnal disalin ke clipboard.",
                        refs.len()
                    )));
                }

                if ui
                    .add_enabled_ui(has_rows, |ui| {
                        secondary_button(ui, p, "Salin Draf Chat Editor")
                    })
                    .inner
                    .clicked()
                {
                    let idx = selected_idx.unwrap_or(0).min(results.len() - 1);
                    let target = &results[idx];
                    let draft = format_editor_draft(&target.nama, keyword);
                    ui.ctx().copy_text(draft);
                    sink.send(EngineEvent::Toast(format!(
                        "Draf pesan editor untuk '{}' disalin.",
                        target.nama
                    )));
                }

                if secondary_button(ui, p, "Buka File Excel Terakhir").clicked() {
                    open_excel_async(
                        engine.clone(),
                        sink.clone(),
                        last_excel_path.map(ToString::to_string),
                    );
                }
            });

            if let Some(path) = last_excel_path {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Excel terakhir: {path}")).size(11.5).color(p.muted));
            }
        });

        ui.add_space(12.0);

        card(ui, p, |ui| {
            ui.label(
                RichText::new("Folder Output & Cache SQLite")
                    .size(13.5)
                    .strong()
                    .color(p.text),
            );
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if secondary_button(ui, p, "Buka Folder Output").clicked() {
                    open_output_folder_async(engine, sink.clone());
                }
                if secondary_button(ui, p, "Buka Folder Logs").clicked() {
                    open_logs_folder_async(engine, sink.clone());
                }
                if secondary_button(ui, p, "Bersihkan Cache SQLite").clicked() {
                    clear_cache_async(engine.clone(), sink.clone());
                }
                pill_badge(
                    ui,
                    &format!("{cache_count} jurnal di cache (TTL 14 hari)"),
                    p.accent,
                    p.accent_soft,
                );
            });
        });

        ui.add_space(12.0);

        card(ui, p, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Log Aktivitas & Diagnostik")
                        .size(13.5)
                        .strong()
                        .color(p.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if secondary_button(ui, p, "Bersihkan Tampilan Log").clicked() {
                        logs.clear();
                    }
                });
            });
            card_divider(ui, p);

            Frame::new()
                .fill(p.content_bg)
                .stroke(Stroke::new(1.0, p.line))
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ScrollArea::vertical()
                        .max_height(260.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if logs.is_empty() {
                                ui.label(
                                    RichText::new("Belum ada aktivitas scraping di sesi ini.")
                                        .size(12.0)
                                        .color(p.muted),
                                );
                            } else {
                                for line in logs.iter() {
                                    ui.label(
                                        RichText::new(line)
                                            .monospace()
                                            .size(11.5)
                                            .color(p.text),
                                    );
                                }
                            }
                        });
                });
        });
    });
}
