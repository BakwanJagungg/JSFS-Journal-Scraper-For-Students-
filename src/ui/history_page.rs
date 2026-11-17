//! Quick targeted sharing/exporting, search & filter session history, SQLite cache maintenance, and collapsible logs.

use crate::engine::cache_ops::clear_cache_async;
use crate::engine::file_ops::{
    collect_valid_emails, export_targeted_async, format_editor_draft, format_wa_share_from_ui,
    open_excel_async, open_logs_folder_async, open_output_folder_async,
};
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::exporter::templates::generate_dosbing_brief;
use crate::models::UiJournalItem;
use crate::ui::i18n::{t, AppLanguage};
use crate::ui::state::{
    select_records_for_scope, ApcFilter, AppState, ExportScope, SearchHistoryEntry, SortColumn,
};
use crate::ui::theme::{
    accent_button, card_divider, card_padded, page_header, pill_badge, secondary_button, Palette,
};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use std::path::Path;
use std::sync::Arc;

pub enum HistoryPageAction {
    None,
    LoadHistoryResults(usize),
    ApplyHistoryFilter(usize),
    DeleteHistoryEntry(usize),
}

fn scope_label_localized(scope: ExportScope, lang: AppLanguage) -> &'static str {
    match lang {
        AppLanguage::Id => match scope {
            ExportScope::All => "Semua Hasil",
            ExportScope::BookmarkedOnly => "Hanya ★ Pilihan",
            ExportScope::FreeOnly => "Hanya Gratis (Rp 0)",
            ExportScope::TableFiltered => "Sesuai Filter Tabel",
        },
        AppLanguage::En => match scope {
            ExportScope::All => "All Results",
            ExportScope::BookmarkedOnly => "★ Shortlist Only",
            ExportScope::FreeOnly => "Free Only (Rp 0)",
            ExportScope::TableFiltered => "Current Table Filter",
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_history_page(
    ui: &mut Ui,
    p: &Palette,
    lang: AppLanguage,
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
    show_history_logs: &mut bool,
    engine: &Arc<EngineState>,
    sink: &EventSink,
) -> HistoryPageAction {
    let mut action = HistoryPageAction::None;
    let txt = t(lang);

    page_header(ui, p, txt.history_title, txt.history_subtitle);

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

        card_padded(ui, p, 12, |ui| {
            ui.label(
                RichText::new(txt.export_card_title)
                    .size(13.5)
                    .strong()
                    .color(p.text),
            );
            ui.add_space(6.0);

            // Sub-section 1: Data Scope + right-aligned ready badge
            ui.horizontal(|ui| {
                ui.label(RichText::new(txt.sub_sec_1).size(11.5).strong().color(p.muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    pill_badge(
                        ui,
                        &format!("{} {}", scoped.len(), txt.export_ready_suffix),
                        p.accent,
                        p.accent_soft,
                    );
                });
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for scope in ExportScope::ALL {
                    let active = app_state.export_scope == scope;
                    let (fg, fill, border) = if active {
                        (p.accent, p.accent_soft, p.accent)
                    } else {
                        (p.text, p.control_bg, p.line)
                    };
                    let lbl = scope_label_localized(scope, lang);
                    let chip = egui::Button::new(RichText::new(lbl).size(11.5).strong().color(fg))
                        .fill(fill)
                        .stroke(Stroke::new(if active { 1.4 } else { 1.0 }, border))
                        .corner_radius(CornerRadius::same(14))
                        .min_size(Vec2::new(0.0, 26.0));
                    if ui.add(chip).clicked() {
                        app_state.export_scope = scope;
                    }
                }
            });

            card_divider(ui, p);

            // Sub-section 2: Save & Open Report Files
            ui.label(RichText::new(txt.sub_sec_2).size(11.5).strong().color(p.muted));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let scope = app_state.export_scope;
                let scope_lbl = scope_label_localized(scope, lang);

                if ui
                    .add_enabled_ui(has_scoped, |ui| accent_button(ui, p, txt.btn_export_xlsx))
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
                        scope_lbl,
                    );
                }

                if ui
                    .add_enabled_ui(has_scoped, |ui| secondary_button(ui, p, txt.btn_export_md))
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
                        scope_lbl,
                    );
                }

                if ui
                    .add_enabled_ui(has_scoped, |ui| secondary_button(ui, p, txt.btn_export_csv))
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
                        scope_lbl,
                    );
                }

                if ui
                    .add_enabled_ui(has_scoped, |ui| {
                        secondary_button(ui, p, "Ekspor Sitasi (.ris)")
                    })
                    .inner
                    .on_hover_text("Ekspor format RIS untuk Zotero / Mendeley")
                    .clicked()
                {
                    let owned: Vec<UiJournalItem> = scoped.iter().map(|it| (*it).clone()).collect();
                    export_targeted_async(
                        engine.clone(),
                        sink.clone(),
                        owned,
                        keyword.to_string(),
                        "ris",
                        scope.slug(),
                        scope_lbl,
                    );
                }

                if ui
                    .add_enabled_ui(has_scoped, |ui| {
                        secondary_button(ui, p, "Ekspor BibTeX (.bib)")
                    })
                    .inner
                    .on_hover_text("Ekspor format BibTeX untuk LaTeX / Overleaf / Zotero")
                    .clicked()
                {
                    let owned: Vec<UiJournalItem> = scoped.iter().map(|it| (*it).clone()).collect();
                    export_targeted_async(
                        engine.clone(),
                        sink.clone(),
                        owned,
                        keyword.to_string(),
                        "bib",
                        scope.slug(),
                        scope_lbl,
                    );
                }

                if secondary_button(ui, p, txt.btn_open_last_xlsx).clicked() {
                    open_excel_async(
                        engine.clone(),
                        sink.clone(),
                        last_excel_path.map(ToString::to_string),
                    );
                }
            });

            if let Some(path) = last_excel_path {
                let fname = Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path);
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Excel: {fname}"))
                        .size(11.5)
                        .color(p.muted),
                )
                .on_hover_text(path);
            }

            card_divider(ui, p);

            // Sub-section 3: Quick Copy to Clipboard (WhatsApp & Email)
            ui.label(RichText::new(txt.sub_sec_3).size(11.5).strong().color(p.muted));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if ui
                    .add_enabled_ui(has_scoped, |ui| secondary_button(ui, p, txt.btn_copy_wa))
                    .inner
                    .clicked()
                {
                    let text = format_wa_share_from_ui(&scoped);
                    ui.ctx().copy_text(text);
                    sink.send(EngineEvent::Toast(format!(
                        "{} {} ({}) disalin ke clipboard.",
                        scoped.len(),
                        txt.results_unit,
                        scope_label_localized(app_state.export_scope, lang)
                    )));
                }

                if ui
                    .add_enabled_ui(has_scoped, |ui| secondary_button(ui, p, txt.btn_copy_emails))
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
                            "{count} email redaksi disalin ke clipboard."
                        )));
                    }
                }

                if ui
                    .add_enabled_ui(!results.is_empty(), |ui| {
                        secondary_button(ui, p, txt.btn_copy_draft)
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

                if ui
                    .add_enabled_ui(!results.is_empty(), |ui| {
                        secondary_button(ui, p, txt.btn_copy_dosbing)
                    })
                    .inner
                    .clicked()
                {
                    let bookmarked_scoped = select_records_for_scope(
                        results,
                        ExportScope::BookmarkedOnly,
                        app_state,
                        table_filter,
                        apc_filter,
                        sort_col,
                        sort_desc,
                    );
                    let target_slice = if !bookmarked_scoped.is_empty() {
                        bookmarked_scoped
                    } else {
                        scoped.clone()
                    };
                    let brief = generate_dosbing_brief(&target_slice, lang);
                    ui.ctx().copy_text(brief);
                    let shown_n = target_slice.len().min(5);
                    sink.send(EngineEvent::Toast(format!(
                        "Ringkasan Konsultasi Dosbing ({shown_n} jurnal) disalin ke clipboard."
                    )));
                }
            });
        });

        ui.add_space(10.0);

        card_padded(ui, p, 12, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(txt.history_card_title)
                        .size(13.5)
                        .strong()
                        .color(p.text),
                );
                pill_badge(
                    ui,
                    &format!("{} {}", search_history.len(), txt.sessions_saved_suffix),
                    p.accent,
                    p.accent_soft,
                );
            });
            card_divider(ui, p);

            let is_en = matches!(lang, AppLanguage::En);
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔍").size(12.0).color(p.muted));
                ui.add_sized(
                    [ui.available_width().min(260.0), 26.0],
                    egui::TextEdit::singleline(&mut app_state.history_filter)
                        .hint_text(if is_en { "Search history topics..." } else { "Cari riwayat topik..." }),
                );
                if !app_state.history_filter.is_empty() && secondary_button(ui, p, "✕").clicked() {
                    app_state.history_filter.clear();
                }
            });
            ui.add_space(6.0);

            let filter_query = app_state.history_filter.trim().to_ascii_lowercase();
            let matched_entries: Vec<(usize, &SearchHistoryEntry)> = search_history
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    if filter_query.is_empty() {
                        true
                    } else {
                        entry.keyword.to_ascii_lowercase().contains(&filter_query)
                            || entry.filter_summary.to_ascii_lowercase().contains(&filter_query)
                            || entry.timestamp.to_ascii_lowercase().contains(&filter_query)
                    }
                })
                .collect();

            if search_history.is_empty() {
                ui.label(
                    RichText::new(
                        "Belum ada riwayat sesi. Hasil pencarian & snapshot filter akan tersimpan otomatis di .cache/search_history.json.",
                    )
                    .size(12.0)
                    .color(p.muted),
                );
            } else if matched_entries.is_empty() {
                ui.label(
                    RichText::new(if is_en {
                        "No search history matches your filter."
                    } else {
                        "Tidak ada riwayat pencarian yang cocok dengan kata kunci pencarian Anda."
                    })
                    .size(12.0)
                    .color(p.muted),
                );
            } else {
                for (real_idx, entry) in matched_entries {
                    Frame::new()
                        .fill(p.control_bg)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(10, 7))
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
                                    &format!("{} {}", entry.total_found, txt.results_unit),
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
                                    if secondary_button(ui, p, if is_en { "Delete" } else { "Hapus" })
                                        .on_hover_text(if is_en { "Remove this search session from history" } else { "Hapus sesi pencarian ini dari riwayat" })
                                        .clicked()
                                    {
                                        action = HistoryPageAction::DeleteHistoryEntry(real_idx);
                                    }
                                    if secondary_button(ui, p, txt.apply_filter_btn).clicked() {
                                        action = HistoryPageAction::ApplyHistoryFilter(real_idx);
                                    }
                                    if ui
                                        .add_enabled_ui(!entry.records.is_empty(), |ui| {
                                             accent_button(ui, p, txt.load_results_btn)
                                        })
                                        .inner
                                        .clicked()
                                    {
                                        action = HistoryPageAction::LoadHistoryResults(real_idx);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            pill_badge(ui, &entry.filter_summary, p.muted, p.card_bg);
                        });
                    ui.add_space(6.0);
                }
            }
        });

        ui.add_space(10.0);

        card_padded(ui, p, 12, |ui| {
            ui.label(
                RichText::new(txt.folder_cache_card)
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

        ui.add_space(10.0);

        card_padded(ui, p, 12, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(txt.logs_card_title)
                        .size(13.5)
                        .strong()
                        .color(p.text),
                );
                pill_badge(
                    ui,
                    &format!("{} baris", logs.len()),
                    p.muted,
                    p.control_bg,
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if secondary_button(ui, p, "Bersihkan Log").clicked() {
                        logs.clear();
                    }
                    let toggle_lbl = if *show_history_logs {
                        txt.hide_full_log
                    } else {
                        txt.show_full_log
                    };
                    if secondary_button(ui, p, toggle_lbl).clicked() {
                        *show_history_logs = !*show_history_logs;
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
                    if logs.is_empty() {
                        ui.label(
                            RichText::new("Belum ada aktivitas scraping di sesi ini.")
                                .size(12.0)
                                .color(p.muted),
                        );
                    } else if *show_history_logs {
                        ScrollArea::vertical()
                            .max_height(260.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for line in logs.iter() {
                                    ui.label(
                                        RichText::new(line).monospace().size(11.5).color(p.text),
                                    );
                                }
                            });
                    } else {
                        let start = logs.len().saturating_sub(5);
                        for line in &logs[start..] {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(line).monospace().size(11.5).color(p.text),
                                )
                                .truncate(),
                            );
                        }
                    }
                });
        });

        ui.add_space(24.0);
    });

    action
}
