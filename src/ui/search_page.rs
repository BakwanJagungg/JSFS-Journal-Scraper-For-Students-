//! Primary search workspace: hero keyword bar, manuscript uploader, 4 student presets,
//! dual live progress bars, 4 KPI cards, and high-speed virtualized results table.

use crate::engine::file_ops::open_url_with_feedback;
use crate::engine::manuscript::ManuscriptSummary;
use crate::engine::EventSink;
use crate::models::UiJournalItem;
use crate::ui::state::{
    activity_badge_label, filtered_sorted_indices, format_duration, ApcFilter, FilterSettings,
    QuickPreset, SearchProgressView, SortColumn,
};
use crate::ui::theme::{
    accent_button_h, card, danger_button_h, page_header, pill_badge, secondary_button,
    secondary_button_h, Palette,
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
    let resp = Frame::new()
        .fill(fill)
        .stroke(Stroke::new(if active { 1.4 } else { 1.0 }, border))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(11, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
                ui.painter().circle_filled(dot_r.center(), 4.0, accent);
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).size(12.5).strong().color(p.text));
                    ui.label(RichText::new(desc).size(11.0).color(p.muted));
                });
            });
        })
        .response
        .interact(Sense::click());
    resp.clicked()
}

fn kpi_card(ui: &mut Ui, p: &Palette, title: &str, value: usize, color: egui::Color32) {
    Frame::new()
        .fill(p.card_bg)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(title).size(11.0).color(p.muted));
            ui.label(RichText::new(value.to_string()).size(20.0).strong().color(color));
        });
}

#[allow(clippy::too_many_arguments)]
pub fn render_search_page(
    ui: &mut Ui,
    p: &Palette,
    keyword: &mut String,
    target_text: &mut String,
    filters: &mut FilterSettings,
    manuscript: Option<&ManuscriptSummary>,
    is_searching: bool,
    progress: &SearchProgressView,
    results: &[UiJournalItem],
    table_filter: &mut String,
    apc_filter: &mut ApcFilter,
    sort_col: &mut SortColumn,
    sort_desc: &mut bool,
    show_logs: &mut bool,
    logs: &[String],
    sink: &EventSink,
) -> SearchPageAction {
    let mut action = SearchPageAction::None;

    page_header(
        ui,
        p,
        "Pencarian Jurnal SINTA & Garuda",
        "Zero-trap verifikasi APC, audit keaktifan terbit, dan pencocokan naskah otomatis.",
    );

    // ── Card 1: Hero Search & Quick Presets ─────────────────────────────────
    card(ui, p, |ui| {
        let mut enter_pressed = false;
        ui.allocate_ui(Vec2::new(ui.available_width(), 36.0), |ui| {
            ui.horizontal_centered(|ui| {
                let input_w = (ui.available_width() - 480.0).max(160.0);
                let resp = ui.add_sized(
                    [input_w, 36.0],
                    egui::TextEdit::singleline(keyword)
                        .align(Align2::LEFT_CENTER)
                        .margin(Margin::symmetric(10, 7))
                        .hint_text("Ketik topik skripsi (mis. sistem informasi, akuntansi, UMKM)..."),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    enter_pressed = true;
                }

                ui.label(RichText::new("Target:").size(13.0).color(p.muted));
                let tgt_resp = ui.add_sized(
                    [54.0, 36.0],
                    egui::TextEdit::singleline(target_text)
                        .align(Align2::CENTER_CENTER)
                        .margin(Margin::symmetric(6, 7))
                        .hint_text("40"),
                );
                if tgt_resp.changed() {
                    if let Ok(n) = target_text.trim().parse::<usize>() {
                        filters.clamp_target(n);
                        filters.active_preset = QuickPreset::Custom;
                    }
                }

                if secondary_button_h(ui, p, "Unggah Naskah", 36.0).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Dokumen Naskah", &["docx", "pdf"])
                        .pick_file()
                    {
                        action = SearchPageAction::PickManuscript(path);
                    }
                }

                if is_searching {
                    if danger_button_h(ui, p, "Hentikan", 36.0).clicked() {
                        action = SearchPageAction::StopSearch;
                    }
                } else if accent_button_h(ui, p, "Mulai Cari Jurnal", 36.0).clicked() || enter_pressed {
                    if let Ok(n) = target_text.trim().parse::<usize>() {
                        filters.clamp_target(n);
                    }
                    action = SearchPageAction::StartSearch;
                }

                if secondary_button_h(ui, p, "Filter", 36.0).clicked() {
                    action = SearchPageAction::NavigateFilters;
                }
            });
        });

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if let Some(ms) = manuscript {
                pill_badge(
                    ui,
                    &format!("Naskah: {} • {} kar.", ms.name, ms.char_count),
                    p.good,
                    p.good_soft,
                );
                if secondary_button(ui, p, "Hapus Naskah").clicked() {
                    action = SearchPageAction::ClearManuscript;
                }
            } else {
                ui.label(
                    RichText::new("Opsional: klik Unggah Naskah (.docx / .pdf) agar Match Score dihitung otomatis dari abstrak & isi.")
                        .size(11.5)
                        .color(p.muted),
                );
            }
        });

        ui.add_space(10.0);
        ui.columns(4, |cols| {
            let presets = [
                (
                    QuickPreset::SkripsiS1,
                    "Target Skripsi S1",
                    "SINTA 3–5 • <= Rp 1,5 jt",
                    p.accent,
                ),
                (
                    QuickPreset::JurnalGratis,
                    "Jurnal Gratis (Rp 0)",
                    "Bebas Biaya Publikasi",
                    p.good,
                ),
                (
                    QuickPreset::TerbitTerdekat,
                    "Terbit Terdekat",
                    "Edisi 1–3 Bln Mendatang",
                    p.warn,
                ),
                (
                    QuickPreset::ReputasiTinggi,
                    "Reputasi Tinggi",
                    "SINTA 1 & 2 / Nasional A",
                    egui::Color32::from_rgb(0xbf, 0x5a, 0xf2),
                ),
            ];
            for (idx, (preset, title, desc, dot_c)) in presets.into_iter().enumerate() {
                if preset_chip(
                    &mut cols[idx],
                    p,
                    filters.active_preset == preset,
                    title,
                    desc,
                    dot_c,
                ) {
                    let msg = filters.apply_preset(preset);
                    *target_text = filters.target_count.to_string();
                    sink.send(crate::engine::EngineEvent::LogLine(msg.to_string()));
                    sink.send(crate::engine::EngineEvent::Toast(msg.to_string()));
                }
            }
        });
    });

    ui.add_space(10.0);

    // ── Row 2: 4 KPI Cards ──────────────────────────────────────────────────
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

    ui.columns(4, |cols| {
        kpi_card(&mut cols[0], p, "Total Jurnal", kpi_total, p.text);
        kpi_card(&mut cols[1], p, "Gratis Rp 0", kpi_free, p.good);
        kpi_card(&mut cols[2], p, "Berbayar", kpi_paid, p.warn);
        kpi_card(&mut cols[3], p, "Terbit 1–3 Bln", kpi_fast, p.accent);
    });

    ui.add_space(10.0);

    // ── Row 3: Dual Progress Strip + Log Drawer Toggle ──────────────────────
    card(ui, p, |ui| {
        let phase1_frac = match progress.phase.as_str() {
            "fase1_portal" => 0.18,
            "fase2_deep" if progress.total > 0 => {
                (progress.current as f32 / progress.total as f32).clamp(0.0, 1.0)
            }
            "fase3_ai" | "fase4_export" => 1.0,
            _ if !results.is_empty() => 1.0,
            _ => 0.0,
        };
        let phase2_frac = match progress.phase.as_str() {
            "fase3_ai" if progress.total > 0 => {
                (progress.current as f32 / progress.total as f32).clamp(0.1, 0.95)
            }
            "fase4_export" => 1.0,
            _ if !is_searching && !results.is_empty() => 1.0,
            _ => 0.0,
        };

        ui.columns(2, |cols| {
            cols[0].horizontal(|ui| {
                ui.label(RichText::new("Portal & Deep OJS:").size(11.5).color(p.muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} / {}", progress.current, progress.total))
                            .size(11.5)
                            .color(p.text),
                    );
                });
            });
            cols[0].add(ProgressBar::new(phase1_frac).fill(p.accent).desired_height(6.0));

            cols[1].horizontal(|ui| {
                ui.label(RichText::new("Match Engine & Validasi:").size(11.5).color(p.muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(&progress.status_text).size(11.0).color(p.muted));
                });
            });
            cols[1].add(ProgressBar::new(phase2_frac).fill(p.good).desired_height(6.0));
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(&progress.current_item)
                        .size(11.5)
                        .color(p.muted),
                )
                .truncate(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let log_lbl = if *show_logs {
                    "Tutup Log"
                } else {
                    "Log Teknis"
                };
                if secondary_button(ui, p, log_lbl).clicked() {
                    *show_logs = !*show_logs;
                }
                let rate_txt = format!(
                    "{}  |  ~{:.1}/dtk",
                    format_duration(progress.elapsed_seconds),
                    progress.rate_per_sec
                );
                pill_badge(ui, &rate_txt, p.muted, p.control_bg);
                if let Some(eta) = progress.eta_seconds {
                    pill_badge(ui, &format!("ETA {eta}s"), p.accent, p.accent_soft);
                }
            });
        });

        if *show_logs {
            ui.add_space(8.0);
            Frame::new()
                .fill(p.content_bg)
                .stroke(Stroke::new(1.0, p.line))
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ScrollArea::vertical()
                        .max_height(110.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if logs.is_empty() {
                                ui.label(RichText::new("Belum ada log aktivitas.").size(11.0).color(p.muted));
                            } else {
                                for line in logs.iter().rev().take(80).rev() {
                                    ui.label(
                                        RichText::new(line)
                                            .monospace()
                                            .size(11.0)
                                            .color(p.text),
                                    );
                                }
                            }
                        });
                });
        }
    });

    ui.add_space(10.0);

    // ── Row 4: Virtualized Results Table or Empty State ─────────────────────
    card(ui, p, |ui| {
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
            return;
        }

        let filtered = filtered_sorted_indices(results, table_filter, *apc_filter, *sort_col, *sort_desc);

        // Table Toolbar
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Hasil Pencarian").size(14.5).strong().color(p.text));
            pill_badge(
                ui,
                &format!("{} / {} jurnal", filtered.len(), results.len()),
                p.accent,
                p.accent_soft,
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let dir_lbl = if *sort_desc { "Turun" } else { "Naik" };
                if secondary_button(ui, p, dir_lbl).clicked() {
                    *sort_desc = !*sort_desc;
                }

                egui::ComboBox::from_id_salt("sort_col_combo")
                    .selected_text(match *sort_col {
                        SortColumn::MatchScore => "Urut: Match",
                        SortColumn::Cost => "Urut: Biaya",
                        SortColumn::Sinta => "Urut: SINTA",
                        SortColumn::Activity => "Urut: Keaktifan",
                        SortColumn::Title => "Urut: Nama",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(sort_col, SortColumn::MatchScore, "Match Score");
                        ui.selectable_value(sort_col, SortColumn::Cost, "Biaya (APC)");
                        ui.selectable_value(sort_col, SortColumn::Sinta, "Peringkat SINTA");
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
                    [200.0, 26.0],
                    egui::TextEdit::singleline(table_filter)
                        .hint_text("Filter nama, penerbit, email..."),
                );
            });
        });

        ui.add_space(8.0);

        let avail_h = (ui.available_height() - 52.0).max(220.0);
        ScrollArea::horizontal().show(ui, |ui| {
            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .max_scroll_height(avail_h)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::initial(285.0).at_least(210.0).clip(true))
                .column(Column::exact(96.0))
                .column(Column::exact(82.0))
                .column(Column::initial(156.0).at_least(125.0).clip(true))
                .column(Column::initial(148.0).at_least(125.0).clip(true))
                .column(Column::exact(136.0));

            table
                .header(28.0, |mut header| {
                    for title in [
                        "Jurnal & Penerbit",
                        "SINTA",
                        "Match",
                        "Biaya APC",
                        "Keaktifan & Terbit",
                        "Aksi",
                    ] {
                        header.col(|ui| {
                            table_cell(ui, |ui| {
                                ui.label(RichText::new(title).size(11.5).strong().color(p.muted));
                            });
                        });
                    }
                })
                .body(|body| {
                    body.rows(52.0, filtered.len(), |mut row| {
                        let real_idx = filtered[row.index()];
                        let item = &results[real_idx];

                        row.col(|ui| {
                            table_cell(ui, |ui| {
                                ui.vertical(|ui| {
                                    let link_resp = ui
                                        .add(
                                            egui::Label::new(
                                                RichText::new(&item.nama)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(p.text),
                                            )
                                            .truncate()
                                            .sense(Sense::click()),
                                        );
                                    if link_resp.clicked() {
                                        action = SearchPageAction::OpenDetail(real_idx);
                                    }
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(format!(
                                                "{} • {}",
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
                                let sinta_txt = if item.sinta.is_empty() {
                                    "Garuda".to_string()
                                } else {
                                    format!("SINTA {}", item.sinta)
                                };
                                pill_badge(ui, &sinta_txt, p.accent, p.accent_soft);
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
                                    let act_lbl = activity_badge_label(item);
                                    let (fg, bg) = if item.is_dormant {
                                        (p.bad, p.bad_soft)
                                    } else if item.activity_status.contains("Aktif") {
                                        (p.good, p.good_soft)
                                    } else {
                                        (p.warn, p.warn_soft)
                                    };
                                    pill_badge(ui, &act_lbl, fg, bg);
                                    if item.terbit_terdekat != "-" {
                                        ui.label(
                                            RichText::new(format!("Next: {}", item.terbit_terdekat))
                                                .size(11.0)
                                                .color(p.muted),
                                        );
                                    }
                                });
                            });
                        });

                        row.col(|ui| {
                            table_cell(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if secondary_button(ui, p, "Detail").clicked() {
                                        action = SearchPageAction::OpenDetail(real_idx);
                                    }
                                    if secondary_button(ui, p, "Web").clicked() {
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
        ui.add_space(6.0);
    });

    action
}
