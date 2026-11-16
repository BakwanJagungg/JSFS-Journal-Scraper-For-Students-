//! Primary search workspace: hero keyword bar, manuscript uploader, 4 student presets,
//! dual live progress bars, 4 KPI cards, and high-speed virtualized results table.

use crate::engine::file_ops::open_url_with_feedback;
use crate::engine::manuscript::ManuscriptSummary;
use crate::engine::EventSink;
use crate::models::UiJournalItem;
use crate::ui::i18n::{t, AppLanguage};
use crate::ui::state::{
    activity_badge_label, filtered_sorted_indices, format_duration, save_saved_profiles_to,
    upsert_saved_profile, ApcFilter, AppState, FilterSettings, QuickPreset, SearchProgressView,
    SortColumn,
};
use crate::ui::theme::{
    accent_button_h, card, card_divider, danger_button_h, page_header, pill_badge,
    quartile_pill_colors, secondary_button, secondary_button_h, Palette,
};
use egui::{
    Align, Align2, CornerRadius, Frame, Layout, Margin, ProgressBar, RichText, ScrollArea, Sense,
    Stroke, Ui, Vec2,
};
use egui_extras::{Column, TableBuilder};
use std::path::PathBuf;

fn table_cell(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    Frame::new()
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, add_contents);
}

pub enum SearchPageAction {
    None,
    StartSearch,
    StopSearch,
    PickManuscript(PathBuf),
    ClearManuscript,
    OpenDetail(usize),
    OpenCompare,
    CopyWa(Vec<usize>),
    OpenExcel,
    NavigateFilters,
}

fn preset_chip(
    ui: &mut Ui,
    p: &Palette,
    active: bool,
    title: &str,
    desc: &str,
    accent: egui::Color32,
) -> bool {
    let fill = if active { p.accent_soft } else { p.control_bg };
    let border = if active { p.accent } else { p.line };
    let title_color = if active { p.accent } else { p.text };
    let resp = Frame::new()
        .fill(fill)
        .stroke(Stroke::new(if active { 1.5 } else { 1.0 }, border))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(dot_r.center(), 4.0, accent);
                ui.label(RichText::new(title).size(12.0).strong().color(title_color));
            });
        })
        .response
        .interact(Sense::click())
        .on_hover_text(desc);
    resp.clicked()
}

fn toggle_topic_in_keyword(keyword: &mut String, topic: &str, all_topics: &[String]) {
    let topic_low = topic.trim().to_ascii_lowercase();
    let all_joined = all_topics.join(", ");
    if keyword.trim().eq_ignore_ascii_case(&all_joined) && all_topics.len() > 1 {
        *keyword = topic.trim().to_string();
        return;
    }
    let mut parts: Vec<String> = keyword
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if let Some(pos) = parts
        .iter()
        .position(|p| p.eq_ignore_ascii_case(&topic_low))
    {
        parts.remove(pos);
    } else {
        parts.push(topic.trim().to_string());
    }
    *keyword = parts.join(", ");
}

fn render_kpi_strip(
    ui: &mut Ui,
    p: &Palette,
    max_w: f32,
    items: [(&str, usize, egui::Color32); 4],
) {
    let inner_w = (max_w - 28.0).max(400.0);
    Frame::new()
        .fill(p.card_bg)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.set_max_width(inner_w);
            ui.allocate_ui(Vec2::new(inner_w, 30.0), |ui| {
                ui.columns(4, |cols| {
                    for (idx, (label, value, color)) in items.into_iter().enumerate() {
                        cols[idx].horizontal_centered(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!("{label}:")).size(11.5).color(p.muted),
                                )
                                .truncate(),
                            );
                            ui.label(
                                RichText::new(value.to_string())
                                    .size(15.5)
                                    .strong()
                                    .color(color),
                            );
                        });
                    }
                });
            });
        });
}

#[allow(clippy::too_many_arguments)]
pub fn render_search_page(
    ui: &mut Ui,
    p: &Palette,
    lang: AppLanguage,
    keyword: &mut String,
    target_text: &mut String,
    filters: &mut FilterSettings,
    app_state: &mut AppState,
    manuscript: Option<&ManuscriptSummary>,
    is_searching: bool,
    progress: &SearchProgressView,
    results: &[UiJournalItem],
    table_filter: &mut String,
    apc_filter: &mut ApcFilter,
    sort_col: &mut SortColumn,
    sort_desc: &mut bool,
    sink: &EventSink,
) -> SearchPageAction {
    let mut action = SearchPageAction::None;
    let txt = t(lang);
    let page_w = ui.available_width().max(540.0);
    let is_en = matches!(lang, AppLanguage::En);

    ScrollArea::vertical()
        .id_salt("search_page_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_max_width(page_w);
            page_header(ui, p, txt.search_title, "");

    // Card 1: Clean 2-Row Anti-Overflow Search Header + On-Demand Manuscript Chip Selector
    card(ui, p, |ui| {
        ui.set_max_width((page_w - 24.0).max(480.0));
        let mut enter_pressed = false;

        // Baris 1: Input kata kunci (fleksibel) + Target + Unggah Naskah + Mulai Cari + Filter
        ui.allocate_ui(Vec2::new(ui.available_width(), 36.0), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                if secondary_button_h(ui, p, txt.filter_btn, 36.0).clicked() {
                    action = SearchPageAction::NavigateFilters;
                }

                if is_searching {
                    if danger_button_h(ui, p, txt.stop_search, 36.0).clicked() {
                        action = SearchPageAction::StopSearch;
                    }
                } else if accent_button_h(ui, p, txt.start_search, 36.0).clicked() {
                    if let Ok(n) = target_text.trim().parse::<usize>() {
                        filters.clamp_target(n);
                    }
                    action = SearchPageAction::StartSearch;
                }

                if secondary_button_h(ui, p, txt.upload_ms, 36.0).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Dokumen Naskah", &["docx", "pdf"])
                        .pick_file()
                    {
                        action = SearchPageAction::PickManuscript(path);
                    }
                }

                let tgt_resp = ui.add_sized(
                    [48.0, 36.0],
                    egui::TextEdit::singleline(target_text)
                        .align(Align2::CENTER_CENTER)
                        .margin(Margin::symmetric(4, 7))
                        .hint_text("40"),
                );
                if tgt_resp.changed() {
                    if let Ok(n) = target_text.trim().parse::<usize>() {
                        filters.clamp_target(n);
                        filters.active_preset = QuickPreset::Custom;
                    }
                }
                ui.label(RichText::new(txt.target_label).size(12.5).color(p.muted));

                let input_w = ui.available_width().max(120.0);
                let resp = ui.add_sized(
                    [input_w, 36.0],
                    egui::TextEdit::singleline(keyword)
                        .align(Align2::LEFT_CENTER)
                        .margin(Margin::symmetric(10, 7))
                        .hint_text(txt.keyword_hint),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    enter_pressed = true;
                }
            });
        });
        if enter_pressed && !is_searching {
            if let Ok(n) = target_text.trim().parse::<usize>() {
                filters.clamp_target(n);
            }
            action = SearchPageAction::StartSearch;
        }

        ui.add_space(8.0);

        // Baris 2: Toolbar Ringkas Berbasis Dropdown (Preset & Profil) + Ringkasan Filter
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            let preset_base = match filters.active_preset {
                QuickPreset::SkripsiS1 => format!("Preset: {}", txt.preset_skripsi),
                QuickPreset::JurnalGratis => format!("Preset: {}", txt.preset_free),
                QuickPreset::TerbitTerdekat => format!("Preset: {}", txt.preset_soon),
                QuickPreset::ReputasiTinggi => format!("Preset: {}", txt.preset_high),
                QuickPreset::Custom => if is_en {
                    "Preset: Custom".to_string()
                } else {
                    "Preset: Kustom".to_string()
                },
            };
            let preset_disp = if filters.offline_mode {
                format!("{preset_base} • Offline")
            } else {
                preset_base
            };
            egui::ComboBox::from_id_salt("preset_dropdown_combo")
                .selected_text(preset_disp)
                .show_ui(ui, |ui| {
                    let presets = [
                        (QuickPreset::SkripsiS1, txt.preset_skripsi),
                        (QuickPreset::JurnalGratis, txt.preset_free),
                        (QuickPreset::TerbitTerdekat, txt.preset_soon),
                        (QuickPreset::ReputasiTinggi, txt.preset_high),
                    ];
                    for (preset, title) in presets {
                        if ui
                            .selectable_label(filters.active_preset == preset, title)
                            .clicked()
                        {
                            let msg = filters.apply_preset(preset);
                            *target_text = filters.target_count.to_string();
                            sink.send(crate::engine::EngineEvent::LogLine(msg.to_string()));
                            sink.send(crate::engine::EngineEvent::Toast(msg.to_string()));
                        }
                    }
                    ui.separator();
                    let offline_lbl = if is_en {
                        "Search Local Cache (Offline)"
                    } else {
                        "Cari dari Cache Lokal (Offline)"
                    };
                    if ui
                        .selectable_label(filters.offline_mode, offline_lbl)
                        .clicked()
                    {
                        filters.offline_mode = !filters.offline_mode;
                        let msg = if filters.offline_mode {
                            "Mode Offline diaktifkan: pencarian langsung dari cache SQLite lokal."
                        } else {
                            "Mode Online diaktifkan kembali."
                        };
                        sink.send(crate::engine::EngineEvent::Toast(msg.to_string()));
                    }
                });

            let prof_lbl = if is_en {
                format!("Profiles ({})", app_state.saved_profiles.len())
            } else {
                format!("Profil ({})", app_state.saved_profiles.len())
            };
            egui::ComboBox::from_id_salt("saved_profiles_combo")
                .selected_text(prof_lbl)
                .show_ui(ui, |ui| {
                    let save_lbl = if is_en {
                        "+ Save Current Search & Filters"
                    } else {
                        "+ Simpan Pencarian & Filter Ini"
                    };
                    if ui.selectable_label(false, save_lbl).clicked() {
                        let pname = if !keyword.trim().is_empty() {
                            keyword.trim().chars().take(24).collect::<String>()
                        } else {
                            format!("Profil {}", app_state.saved_profiles.len() + 1)
                        };
                        upsert_saved_profile(&mut app_state.saved_profiles, &pname, keyword, filters);
                        let _ = save_saved_profiles_to(
                            &crate::config::saved_profiles_path(),
                            &app_state.saved_profiles,
                        );
                        sink.send(crate::engine::EngineEvent::Toast(format!(
                            "Profil '{pname}' disimpan."
                        )));
                    }
                    ui.separator();
                    if app_state.saved_profiles.is_empty() {
                        ui.label(
                            RichText::new(if is_en {
                                "No saved profiles yet"
                            } else {
                                "Belum ada profil tersimpan"
                            })
                            .size(11.5)
                            .color(p.muted),
                        );
                    } else {
                        let cloned = app_state.saved_profiles.clone();
                        for prof in cloned {
                            if ui
                                .selectable_label(false, format!("{} ({})", prof.name, prof.keyword))
                                .clicked()
                            {
                                *keyword = prof.keyword.clone();
                                *filters = prof.filters.clone();
                                *target_text = filters.target_count.to_string();
                                sink.send(crate::engine::EngineEvent::Toast(format!(
                                    "Profil '{}' dimuat.",
                                    prof.name
                                )));
                            }
                        }
                    }
                });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(
                    egui::Label::new(RichText::new(filters.summary_text()).size(11.5).color(p.muted))
                        .truncate(),
                );
            });
        });

        // Kartu Naskah & Rekomendasi Chip Topik (Hanya muncul saat naskah diunggah)
        if let Some(ms) = manuscript {
            card_divider(ui, p);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                pill_badge(ui, &format!("📄 {}", ms.name), p.good, p.good_soft);
                let struct_lbl = if !ms.summary_badge.is_empty() {
                    ms.summary_badge.clone()
                } else {
                    format!("{} kata • ~{} ref", ms.word_count, ms.estimated_refs)
                };
                let (cl_fg, cl_bg) = if ms.imrad_complete {
                    (p.good, p.good_soft)
                } else {
                    (p.warn, p.warn_soft)
                };
                pill_badge(ui, &struct_lbl, cl_fg, cl_bg);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if secondary_button(ui, p, txt.remove_ms).clicked() {
                        action = SearchPageAction::ClearManuscript;
                    }
                });
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                pill_badge(
                    ui,
                    if is_en {
                        "🔒 100% Processed Locally in RAM (Zero Upload)"
                    } else {
                        "🔒 100% Diproses Lokal di RAM (Zero Upload)"
                    },
                    p.good,
                    p.good_soft,
                );
            });

            let chips: Vec<String> = if !ms.recommended_topics.is_empty() {
                ms.recommended_topics.clone()
            } else {
                ms.detected_keywords.iter().take(6).cloned().collect()
            };

            if !chips.is_empty() {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 5.0);
                    ui.label(
                        RichText::new(if is_en {
                            "Recommended Topics:"
                        } else {
                            "Rekomendasi Topik Naskah:"
                        })
                        .size(11.5)
                        .strong()
                        .color(p.muted),
                    );
                    for topic in &chips {
                        let is_selected = keyword
                            .split([',', ';'])
                            .any(|part| part.trim().eq_ignore_ascii_case(topic));
                        if preset_chip(
                            ui,
                            p,
                            is_selected,
                            topic,
                            "Klik untuk menambah atau menghapus topik ini di kolom pencarian",
                            p.accent,
                        ) {
                            toggle_topic_in_keyword(keyword, topic, &chips);
                        }
                    }

                    let use_all_lbl = if is_en {
                        "Use All Topics"
                    } else {
                        "Gunakan Semua Topik"
                    };
                    if secondary_button_h(ui, p, use_all_lbl, 26.0).clicked() {
                        *keyword = chips.join(", ");
                        sink.send(crate::engine::EngineEvent::Toast(format!(
                            "Seluruh topik naskah diisi: {}",
                            keyword
                        )));
                    }
                });
            }
        }
    });

    ui.add_space(8.0);

    // Row 2: Slim 44px KPI Strip Bar
    let kpi_total = if progress.kpi_total > 0 {
        progress.kpi_total
    } else {
        results.len()
    };
    let kpi_free = if progress.kpi_total > 0 {
        progress.kpi_free
    } else {
        results.iter().filter(|r| r.apc_category == "free").count()
    };
    let kpi_paid = if progress.kpi_total > 0 {
        progress.kpi_paid
    } else {
        results.iter().filter(|r| r.apc_category == "paid").count()
    };
    let kpi_fast = progress.kpi_fast;

    render_kpi_strip(
        ui,
        p,
        page_w,
        [
            (txt.kpi_total, kpi_total, p.text),
            (txt.kpi_free, kpi_free, p.good),
            (txt.kpi_paid, kpi_paid, p.warn),
            (txt.kpi_soon, kpi_fast, p.accent),
        ],
    );

    ui.add_space(10.0);

    // Row 3: Compact 1-Line Live Progress Strip (shown ONLY while actively searching)
    if is_searching {
        let frac = if progress.total > 0 {
            (progress.current as f32 / progress.total as f32).clamp(0.0, 1.0)
        } else if progress.percent > 0.0 {
            (progress.percent as f32 / 100.0).clamp(0.0, 1.0)
        } else {
            match progress.phase.as_str() {
                "fase1_portal" => 0.15,
                "fase3_ai" => 0.85,
                "fase4_export" => 0.95,
                _ => 0.05,
            }
        };
        let pct = (frac * 100.0).round() as u32;
        let count_txt = if progress.total > 0 {
            format!("{} / {} jurnal", progress.current, progress.total)
        } else {
            format!("{} jurnal", progress.current)
        };
        card(ui, p, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                pill_badge(ui, &format!("{pct}%"), p.accent, p.accent_soft);
                ui.label(RichText::new(count_txt).size(12.0).strong().color(p.text));
                ui.add(
                    ProgressBar::new(frac)
                        .fill(p.accent)
                        .desired_height(7.0)
                        .desired_width(180.0),
                );
                let status_line = if !progress.current_item.trim().is_empty() {
                    &progress.current_item
                } else {
                    &progress.status_text
                };
                ui.add(
                    egui::Label::new(RichText::new(status_line).size(11.5).color(p.muted)).truncate(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(eta) = progress.eta_seconds {
                        pill_badge(ui, &format!("Sisa ~{eta} dtk"), p.accent, p.accent_soft);
                    } else {
                        pill_badge(
                            ui,
                            &format_duration(progress.elapsed_seconds),
                            p.muted,
                            p.control_bg,
                        );
                    }
                });
            });
        });
        ui.add_space(10.0);
    }

    // Row 4: Virtualized Results Table or Empty State
    card(ui, p, |ui| {
        ui.set_max_width((page_w - 24.0).max(480.0));
        if results.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(28.0);
                ui.label(
                    RichText::new("Temukan Jurnal SINTA & Garuda dalam Hitungan Detik")
                        .size(17.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Ketik bidang penelitian di atas, pilih preset mahasiswa, lalu klik Mulai Cari Jurnal.")
                        .size(12.5)
                        .color(p.muted),
                );
                ui.add_space(14.0);
                ui.columns(3, |cols| {
                    for (idx, (num, head, body)) in [
                        (
                            "1",
                            "Topik atau Naskah",
                            "Isi topik riset (target 1-1000) atau lampirkan naskah .docx/.pdf.",
                        ),
                        (
                            "2",
                            "Audit Keaktifan & APC",
                            "Zero-trap scanner memverifikasi halaman biaya OJS & tahun edisi terakhir.",
                        ),
                        (
                            "3",
                            "Ekspor & Kontak Editor",
                            "Buka detail jurnal, salin draf WA ke editor, atau ekspor otomatis ke Excel.",
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        Frame::new()
                            .fill(p.control_bg)
                            .stroke(Stroke::new(1.0, p.line))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(10))
                            .show(&mut cols[idx], |ui| {
                                ui.set_min_width(ui.available_width());
                                pill_badge(ui, &format!("Langkah {num}"), p.accent, p.accent_soft);
                                ui.add_space(4.0);
                                ui.label(RichText::new(head).size(12.5).strong().color(p.text));
                                ui.label(RichText::new(body).size(11.5).color(p.muted));
                            });
                    }
                });
                ui.add_space(24.0);
            });
        } else {

        let bm_filter = app_state.only_bookmarked.then_some(&app_state.bookmarked_ids);
        let filtered = filtered_sorted_indices(
            results,
            table_filter,
            *apc_filter,
            bm_filter,
            *sort_col,
            *sort_desc,
        );
        let bm_count = app_state.bookmarked_count(results);

        // Table Toolbar
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(txt.results_heading).size(14.5).strong().color(p.text));
            pill_badge(
                ui,
                &format!("{} / {} {}", filtered.len(), results.len(), txt.results_unit),
                p.accent,
                p.accent_soft,
            );

            let (bm_fg, bm_fill, bm_border) = if app_state.only_bookmarked {
                (p.warn, p.warn_soft, p.warn)
            } else {
                (p.text, p.control_bg, p.line)
            };
            let bm_btn = egui::Button::new(
                RichText::new(format!("★ {} ({bm_count})", txt.shortlisted_btn))
                    .size(12.0)
                    .strong()
                    .color(bm_fg),
            )
            .fill(bm_fill)
            .stroke(Stroke::new(if app_state.only_bookmarked { 1.4 } else { 1.0 }, bm_border))
            .corner_radius(CornerRadius::same(7))
            .min_size(Vec2::new(0.0, 28.0));
            if ui
                .add(bm_btn)
                .on_hover_text("Tampilkan hanya jurnal yang ditandai bintang")
                .clicked()
            {
                app_state.only_bookmarked = !app_state.only_bookmarked;
            }

            let can_compare = (2..=4).contains(&bm_count);
            let cmp_lbl = if matches!(lang, AppLanguage::En) {
                format!("Compare ({bm_count})")
            } else {
                format!("Bandingkan ({bm_count})")
            };
            if ui
                .add_enabled_ui(can_compare, |ui| secondary_button(ui, p, &cmp_lbl))
                .inner
                .on_hover_text(if can_compare {
                    "Bandingkan jurnal bertanda ★ secara berdampingan (side-by-side)"
                } else {
                    "Tandai 2–4 jurnal dengan bintang ★ untuk mengaktifkan perbandingan"
                })
                .clicked()
            {
                action = SearchPageAction::OpenCompare;
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if secondary_button(ui, p, "Excel")
                    .on_hover_text("Buka file Excel hasil pencarian terakhir")
                    .clicked()
                {
                    action = SearchPageAction::OpenExcel;
                }
                if ui
                    .add_enabled_ui(!filtered.is_empty(), |ui| secondary_button(ui, p, txt.copy_wa_btn))
                    .inner
                    .on_hover_text("Salin daftar jurnal tampil ke clipboard (format WhatsApp)")
                    .clicked()
                {
                    action = SearchPageAction::CopyWa(filtered.clone());
                }

                let dir_lbl = if *sort_desc { "Turun" } else { "Naik" };
                if secondary_button(ui, p, dir_lbl).clicked() {
                    *sort_desc = !*sort_desc;
                }

                egui::ComboBox::from_id_salt("sort_col_combo")
                    .selected_text(match *sort_col {
                        SortColumn::MatchScore => "Urut: Match",
                        SortColumn::Cost => "Urut: Biaya",
                        SortColumn::Sinta => "Urut: SINTA / Q",
                        SortColumn::Activity => "Urut: Keaktifan",
                        SortColumn::Title => "Urut: Nama",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(sort_col, SortColumn::MatchScore, "Match Score");
                        ui.selectable_value(sort_col, SortColumn::Cost, "Biaya (APC)");
                        ui.selectable_value(sort_col, SortColumn::Sinta, "Peringkat SINTA / Q1–Q4");
                        ui.selectable_value(sort_col, SortColumn::Activity, "Tahun Edisi Terakhir");
                        ui.selectable_value(sort_col, SortColumn::Title, "Nama Jurnal (A-Z)");
                    });

                egui::ComboBox::from_id_salt("apc_filter_combo")
                    .selected_text(apc_filter.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(apc_filter, ApcFilter::All, ApcFilter::All.label());
                        ui.selectable_value(apc_filter, ApcFilter::Free, ApcFilter::Free.label());
                        ui.selectable_value(apc_filter, ApcFilter::Paid, ApcFilter::Paid.label());
                        ui.selectable_value(apc_filter, ApcFilter::Unknown, ApcFilter::Unknown.label());
                    });

                ui.add_sized(
                    [138.0, 26.0],
                    egui::TextEdit::singleline(table_filter)
                        .hint_text(txt.filter_table_hint),
                );
            });
        });

        ui.add_space(8.0);

        let avail_h = (ui.available_height() - 40.0).clamp(460.0, 780.0);
        let table_w = (ui.available_width() - 16.0).max(460.0);
        ui.scope(|ui| {
            ui.set_max_width(table_w);
            ScrollArea::horizontal()
                .id_salt("results_h_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let fixed_w = 80.0 + 62.0 + 144.0 + 170.0 + 146.0;
                    let col0_w = (table_w - fixed_w).max(220.0);
                    let table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(false)
                        .min_scrolled_height(460.0)
                        .max_scroll_height(avail_h)
                        .cell_layout(Layout::left_to_right(Align::Center))
                        .column(Column::exact(col0_w).clip(true))
                        .column(Column::exact(80.0))
                        .column(Column::exact(62.0))
                        .column(Column::exact(144.0))
                        .column(Column::exact(170.0))
                        .column(Column::exact(146.0));

            table
                .header(28.0, |mut header| {
                    for title in [
                        txt.col_journal,
                        txt.col_accred,
                        txt.col_match,
                        txt.col_apc,
                        txt.col_activity,
                        txt.col_action,
                    ] {
                        header.col(|ui| {
                            table_cell(ui, |ui| {
                                ui.label(RichText::new(title).size(11.5).strong().color(p.muted));
                            });
                        });
                    }
                })
                .body(|body| {
                    body.rows(62.0, filtered.len(), |mut row| {
                        let real_idx = filtered[row.index()];
                        let item = &results[real_idx];

                        row.col(|ui| {
                            let inner_w = (col0_w - 14.0).max(120.0);
                            table_cell(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_max_width(inner_w);
                                    ui.spacing_mut().item_spacing.y = 3.0;
                                    let link_resp = if item.cited_count == 0
                                        && item.risk_warning.is_none()
                                    {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&item.nama)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(p.text),
                                            )
                                            .truncate()
                                            .sense(Sense::click()),
                                        )
                                    } else {
                                        let reserve = if item.cited_count > 0 { 82.0 } else { 0.0 }
                                            + if item.risk_warning.is_some() {
                                                68.0
                                            } else {
                                                0.0
                                            };
                                        let title_w = (inner_w - reserve).max(90.0);
                                        ui.horizontal(|ui| {
                                            ui.set_max_width(inner_w);
                                            let resp = ui
                                                .vertical(|ui| {
                                                    ui.set_max_width(title_w);
                                                    ui.add(
                                                        egui::Label::new(
                                                            RichText::new(&item.nama)
                                                                .size(13.0)
                                                                .strong()
                                                                .color(p.text),
                                                        )
                                                        .truncate()
                                                        .sense(Sense::click()),
                                                    )
                                                })
                                                .inner;
                                            if item.cited_count > 0 {
                                                ui.scope(|ui| {
                                                    pill_badge(
                                                        ui,
                                                        &format!("Dikutip {}x", item.cited_count),
                                                        p.accent,
                                                        p.accent_soft,
                                                    );
                                                })
                                                .response
                                                .on_hover_text(format!(
                                                    "Jurnal ini dikutip {}x di Daftar Pustaka naskah Anda",
                                                    item.cited_count
                                                ));
                                            }
                                            if let Some(ref warn) = item.risk_warning {
                                                let w_resp = ui.add(
                                                    egui::Label::new(
                                                        RichText::new("⚠ Periksa")
                                                            .size(10.5)
                                                            .strong()
                                                            .color(p.bad),
                                                    )
                                                    .sense(Sense::hover()),
                                                );
                                                w_resp.on_hover_text(warn);
                                            }
                                            resp
                                        })
                                        .inner
                                    };
                                    if link_resp.clicked() {
                                        action = SearchPageAction::OpenDetail(real_idx);
                                    }
                                    let track_key =
                                        crate::cache::SubmissionTrack::canonical_key(
                                            &item.url,
                                            &item.nama,
                                        );
                                    let status_suffix = app_state
                                        .submission_tracks
                                        .get(&track_key)
                                        .filter(|t| {
                                            !t.status.trim().is_empty()
                                                && t.status != "Belum Dikirim"
                                        })
                                        .map(|t| format!(" • [{}]", t.status))
                                        .unwrap_or_default();
                                    let lang_suffix =
                                        if !item.article_languages.trim().is_empty() {
                                            format!(" • {}", item.article_languages)
                                        } else {
                                            String::new()
                                        };
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(format!(
                                                "{} • {}{lang_suffix}{status_suffix}",
                                                item.penerbit, item.email_short
                                            ))
                                            .size(11.5)
                                            .color(p.muted),
                                        )
                                        .truncate(),
                                    );
                                });
                            });
                        });

                        row.col(|ui| {
                            table_cell(ui, |ui| {
                                if let Some(q) = item
                                    .quartile
                                    .as_deref()
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                {
                                    let (q_fg, q_bg) = quartile_pill_colors(p, q);
                                    pill_badge(ui, &q.to_ascii_uppercase(), q_fg, q_bg);
                                } else if !item.sinta.is_empty() {
                                    pill_badge(
                                        ui,
                                        &format!("SINTA {}", item.sinta),
                                        p.accent,
                                        p.accent_soft,
                                    );
                                } else if item.is_international {
                                    pill_badge(ui, "Intl OA", p.good, p.good_soft);
                                } else {
                                    pill_badge(ui, "Garuda", p.accent, p.accent_soft);
                                }
                            });
                        });

                        row.col(|ui| {
                            table_cell(ui, |ui| {
                                let (fg, bg) = if item.match_score >= 70.0 {
                                    (p.good, p.good_soft)
                                } else if item.match_score >= 45.0 {
                                    (p.warn, p.warn_soft)
                                } else {
                                    (p.muted, p.control_bg)
                                };
                                pill_badge(ui, &format!("{:.0}%", item.match_score), fg, bg);
                            });
                        });

                        row.col(|ui| {
                            table_cell(ui, |ui| {
                                let (fg, bg) = match item.apc_category.as_str() {
                                    "free" => (p.good, p.good_soft),
                                    "paid" => (p.warn, p.warn_soft),
                                    _ => (p.muted, p.control_bg),
                                };
                                pill_badge(ui, &item.apc_label, fg, bg);
                            });
                        });

                        row.col(|ui| {
                            table_cell(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 4.0;
                                    let act_lbl = activity_badge_label(item);
                                    let (fg, bg) = if item.is_dormant {
                                        (p.bad, p.bad_soft)
                                    } else if item.bot_protected && item.last_issue_year.is_none() {
                                        (p.warn, p.warn_soft)
                                    } else if item.activity_status.contains("Aktif") {
                                        (p.good, p.good_soft)
                                    } else {
                                        (p.warn, p.warn_soft)
                                    };
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        pill_badge(ui, &act_lbl, fg, bg);
                                        if !item.turnaround_label.trim().is_empty() {
                                            pill_badge(
                                                ui,
                                                &item.turnaround_label,
                                                p.accent,
                                                p.accent_soft,
                                            );
                                        }
                                    });
                                    if item.terbit_terdekat != "-" {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(format!(
                                                    "Next: {}",
                                                    item.terbit_terdekat
                                                ))
                                                .size(11.0)
                                                .color(p.muted),
                                            )
                                            .truncate(),
                                        );
                                    }
                                });
                            });
                        });

                        row.col(|ui| {
                            Frame::new()
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 6.0;
                                        ui.spacing_mut().button_padding = Vec2::new(7.0, 4.0);
                                        let starred = app_state.is_bookmarked(item);
                                        let (star_txt, star_fg, star_fill, star_border) =
                                            if starred {
                                                ("★", p.warn, p.warn_soft, p.warn)
                                            } else {
                                                ("☆", p.muted, p.control_bg, p.line)
                                            };
                                        let star_btn = egui::Button::new(
                                            RichText::new(star_txt).size(13.5).color(star_fg),
                                        )
                                        .fill(star_fill)
                                        .stroke(Stroke::new(1.0, star_border))
                                        .corner_radius(CornerRadius::same(7))
                                        .min_size(Vec2::new(26.0, 26.0));
                                        if ui
                                            .add(star_btn)
                                            .on_hover_text(if starred {
                                                "Hapus dari shortlist pilihan"
                                            } else {
                                                "Simpan ke shortlist pilihan"
                                            })
                                            .clicked()
                                        {
                                            app_state.toggle_bookmark(item);
                                        }
                                        if secondary_button_h(ui, p, "Detail", 26.0).clicked() {
                                            action = SearchPageAction::OpenDetail(real_idx);
                                        }
                                        if secondary_button_h(ui, p, "Web", 26.0).clicked() {
                                            let target = if item.url.starts_with("http") {
                                                &item.url
                                            } else {
                                                &item.garuda_url
                                            };
                                            open_url_with_feedback(target, sink);
                                        }
                                    });
                                });
                        });
                    });
                });
            });
        });
        ui.add_space(6.0);
        }
    });
    });

    action
}
