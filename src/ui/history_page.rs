//! Quick targeted sharing/exporting, search & filter session history, SQLite cache maintenance, and logs.

use crate::engine::cache_ops::clear_cache_async;
use crate::engine::file_ops::{
    collect_valid_emails, export_targeted_async, format_editor_draft, format_wa_share_from_ui,
    open_excel_async, open_logs_folder_async, open_output_folder_async,
};
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::models::UiJournalItem;
use crate::ui::state::{
    select_records_for_scope, ApcFilter, AppState, ExportScope, SearchHistoryEntry, SortColumn,
};
use crate::ui::theme::{
    accent_button, card, card_divider, page_header, pill_badge, secondary_button, Palette,
};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use std::sync::Arc;

pub enum HistoryPageAction {
    None,
    LoadHistoryResults(usize),
    ApplyHistoryFilter(usize),
}

#[allow(clippy::too_many_arguments)]
pub fn render_history_page(
    ui: &mut Ui,
    p: &Palette,
    results: &[UiJournalItem],
    selected_idx: Option<usize>,
    keyword: &str,
    app_state: &mut AppState,
    table_filter: &str,
    apc_filter: ApcFilter,
    sort_col: SortColumn,
    sort_desc: bool,
    search_history: &[SearchHistoryEntry],
    last_excel_path: Option<&str>,
    cache_count: usize,
    logs: &mut Vec<String>,
    engine: &Arc<EngineState>,
    sink: &EventSink,
) -> HistoryPageAction {
    let mut action = HistoryPageAction::None;

    page_header(
        ui,
        p,
        "Riwayat & Ekspor",
        "Ekspor selektif, salin email redaksi & WhatsApp, serta muat ulang riwayat sesi & filter.",
    );

    ScrollArea::vertical().show(ui, |ui| {
        let scoped = select_records_for_scope(
            results,
            app_state.export_scope,
            app_state,
            table_filter,
            apc_filter,
            sort_col,
            sort_desc,
        );
        let has_scoped = !scoped.is_empty();

        card(ui, p, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Berbagi & Ekspor Selektif")
                        .size(13.5)
                        .strong()
                        .color(p.text),
                );
                pill_badge(
                    ui,
                    &format!("{} jurnal siap ekspor", scoped.len()),
                    p.accent,
                    p.accent_soft,
                );
            });
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new("Cakupan Data:").size(12.0).color(p.muted));
                for scope in ExportScope::ALL {
                    let active = app_state.export_scope == scope;
                    let (fg, fill, border) = if active {
                        (p.accent, p.accent_soft, p.accent)
                    } else {
                        (p.text, p.control_bg, p.line)
                    };
                    let chip = egui::Button::new(RichText::new(scope.label()).size(11.5).strong().color(fg))
                        .fill(fill)
                        .stroke(Stroke::new(if active { 1.4 } else { 1.0 }, border))
                        .corner_radius(CornerRadius::same(14))
                        .min_size(Vec2::new(0.0, 26.0));
                    if ui.add(chip).clicked() {
                        app_state.export_scope = scope;
                    }
                }
            });

            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                let scope = app_state.export_scope;
                if ui
                    .add_enabled_ui(has_scoped, |ui| accent_button(ui, p, "Ekspor Excel (.xlsx)"))
                    .inner
                    .clicked()
                {
                    let owned: Vec<UiJournalItem> = scoped.iter().map(|it| (*it).clone()).collect();
                    export_targeted_async(
                        engine.clone(),
                        sink.clone(),
                        owned,
                        keyword.to_string(),
                        "xlsx",
                        scope.slug(),
                        scope.label(),
                    );
                }

                if ui
                    .add_enabled_ui(has_scoped, |ui| secondary_button(ui, p, "Ekspor Markdown (.md)"))
                    .inner
                    .clicked()
                {
                    let owned: Vec<UiJournalItem> = scoped.iter().map(|it| (*it).clone()).collect();
                    export_targeted_async(
                        engine.clone(),
                        sink.clone(),
                        owned,
                        keyword.to_string(),
                        "md",
                        scope.slug(),
                        scope.label(),
                    );
                }

                if ui
                    .add_enabled_ui(has_scoped, |ui| secondary_button(ui, p, "Ekspor CSV (.csv)"))
                    .inner
                    .clicked()
                {
                    let owned: Vec<UiJournalItem> = scoped.iter().map(|it| (*it).clone()).collect();
                    export_targeted_async(
                        engine.clone(),
                        sink.clone(),
                        owned,
                        keyword.to_string(),
                        "csv",
                        scope.slug(),
                        scope.label(),
                    );
                }

                if ui
                    .add_enabled_ui(has_scoped, |ui| secondary_button(ui, p, "Salin Email Redaksi"))
                    .inner
                    .clicked()
                {
                    let (emails, count) = collect_valid_emails(&scoped);
                    if count == 0 {
                        sink.send(EngineEvent::Toast(
                            "Tidak ada email redaksi valid pada cakupan terpilih.".to_string(),
                        ));
                    } else {
                        ui.ctx().copy_text(emails);
                        sink.send(EngineEvent::Toast(format!(
                            "{count} alamat email redaksi ({}) disalin ke clipboard.",
                            scope.label()
                        )));
                    }
                }
            });

            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled_ui(has_scoped, |ui| secondary_button(ui, p, "Salin Daftar WA"))
                    .inner
                    .clicked()
                {
                    let text = format_wa_share_from_ui(&scoped);
                    ui.ctx().copy_text(text);
                    sink.send(EngineEvent::Toast(format!(
                        "Daftar WA untuk {} jurnal ({}) disalin ke clipboard.",
                        scoped.len(),
                        app_state.export_scope.label()
                    )));
                }

                if ui
                    .add_enabled_ui(!results.is_empty(), |ui| {
                        secondary_button(ui, p, "Salin Draf Chat Editor")
                    })
                    .inner
                    .clicked()
                {
                    let target = selected_idx
                        .and_then(|idx| results.get(idx))
                        .or_else(|| scoped.first().copied())
                        .unwrap_or(&results[0]);
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
                ui.label(
                    RichText::new(format!("Excel terakhir: {path}"))
                        .size(11.5)
                        .color(p.muted),
                );
            }
        });

        ui.add_space(12.0);

        card(ui, p, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Riwayat Pencarian & Filter")
                        .size(13.5)
                        .strong()
                        .color(p.text),
                );
                pill_badge(
                    ui,
                    &format!("{} sesi tersimpan", search_history.len()),
                    p.accent,
                    p.accent_soft,
                );
            });
            card_divider(ui, p);

            if search_history.is_empty() {
                ui.label(
                    RichText::new(
                        "Belum ada riwayat sesi. Hasil pencarian & snapshot filter akan tersimpan otomatis di .cache/search_history.json.",
                    )
                    .size(12.0)
                    .color(p.muted),
                );
            } else {
                for (idx, entry) in search_history.iter().enumerate() {
                    Frame::new()
                        .fill(p.control_bg)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(if entry.keyword.is_empty() {
                                        "(Tanpa Kata Kunci)"
                                    } else {
                                        &entry.keyword
                                    })
                                    .size(13.0)
                                    .strong()
                                    .color(p.text),
                                );
                                ui.label(
                                    RichText::new(&entry.timestamp)
                                        .size(11.0)
                                        .color(p.muted),
                                );
                                pill_badge(
                                    ui,
                                    &format!("{} jurnal", entry.total_found),
                                    p.accent,
                                    p.accent_soft,
                                );
                                pill_badge(
                                    ui,
                                    &format!("{} gratis", entry.free_count),
                                    p.good,
                                    p.good_soft,
                                );

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if secondary_button(ui, p, "Terapkan Filter").clicked() {
                                        action = HistoryPageAction::ApplyHistoryFilter(idx);
                                    }
                                    if ui
                                        .add_enabled_ui(!entry.records.is_empty(), |ui| {
                                            accent_button(ui, p, "Muat Hasil")
                                        })
                                        .inner
                                        .clicked()
                                    {
                                        action = HistoryPageAction::LoadHistoryResults(idx);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            pill_badge(ui, &entry.filter_summary, p.muted, p.card_bg);
                        });
                    if idx + 1 < search_history.len() {
                        ui.add_space(6.0);
                    }
                }
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
                                        RichText::new(line).monospace().size(11.5).color(p.text),
                                    );
                                }
                            }
                        });
                });
        });
    });

    action
}
