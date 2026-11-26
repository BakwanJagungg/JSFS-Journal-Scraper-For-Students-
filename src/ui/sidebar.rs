//! Left navigation sidebar with JSFS brand identity, live engine indicator,
//! grouped colored icon tiles, active filter badges, and instant JSFS Dark / Daylight theme toggle.

use crate::ui::i18n::{t, AppLanguage};
use crate::ui::theme::{draw_vector_icon, Palette, ThemeMode, VectorIcon};
use egui::{
    Align, Color32, CornerRadius, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind,
    Ui, Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    Search,
    DetailWorkspace,
    DospenWorkspace,
    Filters,
    MatchEngine,
    History,
    Settings,
    Guide,
}

pub struct SidebarProps<'a> {
    pub current: NavPage,
    pub engine_label: &'a str,
    pub engine_ready: bool,
    pub is_searching: bool,
    pub result_count: usize,
    pub shortlist_count: usize,
    pub active_filter_count: usize,
    pub theme_mode: ThemeMode,
    pub lang: AppLanguage,
    pub collapsed: bool,
}

pub struct SidebarAction {
    pub selected: Option<NavPage>,
    pub toggle_theme: bool,
    pub toggle_collapse: bool,
}

fn tile_color(page: NavPage) -> Color32 {
    match page {
        NavPage::Search => Color32::from_rgb(0x0a, 0x84, 0xff),
        NavPage::DetailWorkspace => Color32::from_rgb(0x14, 0xb8, 0xa6),
        NavPage::DospenWorkspace => Color32::from_rgb(0xf5, 0x9e, 0x0b),
        NavPage::Filters => Color32::from_rgb(0x5e, 0x5c, 0xe6),
        NavPage::MatchEngine => Color32::from_rgb(0x30, 0xb9, 0x4d),
        NavPage::History => Color32::from_rgb(0xff, 0x9f, 0x0a),
        NavPage::Settings => Color32::from_rgb(0x64, 0x74, 0x8b),
        NavPage::Guide => Color32::from_rgb(0x32, 0xad, 0xe6),
    }
}

fn nav_icon(page: NavPage) -> VectorIcon {
    match page {
        NavPage::Search => VectorIcon::Search,
        NavPage::DetailWorkspace => VectorIcon::Document,
        NavPage::DospenWorkspace => VectorIcon::Star,
        NavPage::Filters => VectorIcon::Sliders,
        NavPage::MatchEngine => VectorIcon::Spark,
        NavPage::History => VectorIcon::Folder,
        NavPage::Settings => VectorIcon::Settings,
        NavPage::Guide => VectorIcon::Book,
    }
}

fn nav_label(page: NavPage, lang: AppLanguage) -> &'static str {
    let txt = t(lang);
    match page {
        NavPage::Search => txt.nav_search,
        NavPage::DetailWorkspace => txt.nav_detail,
        NavPage::DospenWorkspace => txt.nav_dospen,
        NavPage::Filters => txt.nav_filters,
        NavPage::MatchEngine => txt.nav_match,
        NavPage::History => txt.nav_history,
        NavPage::Settings => txt.nav_settings,
        NavPage::Guide => txt.nav_guide,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_nav_item(
    ui: &mut Ui,
    p: &Palette,
    page: NavPage,
    lang: AppLanguage,
    active: bool,
    collapsed: bool,
    badge: Option<String>,
    dot_color: Option<Color32>,
) -> bool {
    if collapsed {
        let row_w = ui.available_width();
        let (row_rect, _) = ui.allocate_exact_size(Vec2::new(row_w, 38.0), Sense::hover());
        let btn_rect = Rect::from_center_size(row_rect.center(), Vec2::splat(38.0));
        let resp = ui.interact(
            btn_rect,
            ui.scope_id().with(("nav_col", page as u8)),
            Sense::click(),
        );
        let hovered = resp.hovered();

        let bg = if active {
            p.accent_soft
        } else if hovered {
            p.nav_hover
        } else {
            Color32::TRANSPARENT
        };
        if bg != Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(btn_rect, CornerRadius::same(9), bg);
        }

        if active {
            let pill = Rect::from_center_size(
                egui::pos2(row_rect.left() + 1.5, row_rect.center().y),
                Vec2::new(3.0, 18.0),
            );
            ui.painter()
                .rect_filled(pill, CornerRadius::same(2), p.accent);
        }

        let tile_rect = Rect::from_center_size(btn_rect.center(), Vec2::splat(24.0));
        ui.painter()
            .rect_filled(tile_rect, CornerRadius::same(6), tile_color(page));
        ui.painter().rect_stroke(
            tile_rect,
            CornerRadius::same(6),
            Stroke::new(0.7, Color32::from_rgba_unmultiplied(255, 255, 255, 64)),
            StrokeKind::Inside,
        );
        draw_vector_icon(
            ui.painter(),
            tile_rect.shrink(4.0),
            nav_icon(page),
            Color32::WHITE,
        );

        if let Some(dot) = dot_color {
            let dot_c = if active { Color32::WHITE } else { dot };
            ui.painter()
                .circle_filled(tile_rect.right_top() + Vec2::new(1.0, 1.0), 3.2, dot_c);
        } else if badge.is_some() {
            ui.painter().circle_filled(
                tile_rect.right_top() + Vec2::new(1.0, 1.0),
                3.2,
                if active { Color32::WHITE } else { p.accent },
            );
        }
        let tip = match badge {
            Some(ref b) => format!("{} ({b})", nav_label(page, lang)),
            None => nav_label(page, lang).to_string(),
        };
        return resp.on_hover_text(tip).clicked();
    }

    let desired = Vec2::new(ui.available_width(), 34.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    let hovered = resp.hovered();

    let bg = if active {
        p.accent
    } else if hovered {
        p.nav_hover
    } else {
        Color32::TRANSPARENT
    };

    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(7), bg);
    }

    // Center X = rect.left() + 8.0 + 11.0 = rect.left() + 19.0 (aligned with header & footer)
    let tile_rect = Rect::from_min_size(
        egui::pos2(rect.left() + 8.0, rect.center().y - 11.0),
        Vec2::splat(22.0),
    );
    let tile_fill = if active {
        Color32::from_rgba_unmultiplied(255, 255, 255, 48)
    } else {
        tile_color(page)
    };
    ui.painter()
        .rect_filled(tile_rect, CornerRadius::same(6), tile_fill);
    ui.painter().rect_stroke(
        tile_rect,
        CornerRadius::same(6),
        Stroke::new(0.7, Color32::from_rgba_unmultiplied(255, 255, 255, 64)),
        StrokeKind::Inside,
    );
    draw_vector_icon(
        ui.painter(),
        tile_rect.shrink(3.5),
        nav_icon(page),
        Color32::WHITE,
    );

    let text_color = if active { Color32::WHITE } else { p.text };
    ui.painter().text(
        egui::pos2(tile_rect.right() + 9.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        nav_label(page, lang),
        egui::FontId::proportional(13.0),
        text_color,
    );

    if let Some(dot) = dot_color {
        let dot_c = if active { Color32::WHITE } else { dot };
        ui.painter()
            .circle_filled(egui::pos2(rect.right() - 16.0, rect.center().y), 3.6, dot_c);
    } else if let Some(b) = badge {
        let pill_bg = if active {
            Color32::from_rgba_unmultiplied(255, 255, 255, 52)
        } else {
            p.track_off
        };
        let pill_fg = if active { Color32::WHITE } else { p.muted };
        let pill_w = if b.len() >= 3 { 32.0 } else { 24.0 };
        let badge_rect = Rect::from_center_size(
            egui::pos2(rect.right() - (pill_w * 0.5 + 8.0), rect.center().y),
            Vec2::new(pill_w, 17.0),
        );
        ui.painter()
            .rect_filled(badge_rect, CornerRadius::same(8), pill_bg);
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            b,
            egui::FontId::proportional(10.5),
            pill_fg,
        );
    }

    resp.clicked()
}

fn draw_logo_box(ui: &mut Ui, p: &Palette) {
    let (logo_rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
    ui.painter()
        .rect_filled(logo_rect, CornerRadius::same(9), p.accent);
    ui.painter().rect_stroke(
        logo_rect,
        CornerRadius::same(9),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 64)),
        StrokeKind::Inside,
    );
    draw_vector_icon(
        ui.painter(),
        logo_rect.shrink(7.0),
        VectorIcon::BrandLogo,
        Color32::WHITE,
    );
    if crate::config::IS_PLUS_EDITION {
        let badge_pos = logo_rect.right_top() + Vec2::new(-2.0, 2.0);
        ui.painter().circle_filled(badge_pos, 4.5, p.sidebar_bg);
        ui.painter()
            .circle_filled(badge_pos, 3.0, Color32::from_rgb(0xf5, 0x9e, 0x0b));
    }
}

fn draw_sidebar_separator(ui: &mut Ui, p: &Palette) {
    let (sep_rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [sep_rect.left_center(), sep_rect.right_center()],
        Stroke::new(1.0, p.row_line),
    );
}

pub fn render_sidebar(ui: &mut Ui, p: &Palette, props: SidebarProps<'_>) -> SidebarAction {
    let mut action = SidebarAction {
        selected: None,
        toggle_theme: false,
        toggle_collapse: false,
    };

    ui.add_space(6.0);

    // Identity Brand Header
    if props.collapsed {
        ui.vertical_centered(|ui| {
            draw_logo_box(ui, p);
            ui.add_space(6.0);
            let (exp_rect, resp) = ui.allocate_exact_size(Vec2::new(40.0, 30.0), Sense::click());
            let hovered = resp.hovered();
            ui.painter().rect_filled(
                exp_rect,
                CornerRadius::same(8),
                if hovered { p.accent_soft } else { p.card_bg },
            );
            ui.painter().rect_stroke(
                exp_rect,
                CornerRadius::same(8),
                Stroke::new(1.0, if hovered { p.accent } else { p.line }),
                StrokeKind::Inside,
            );
            ui.painter().text(
                exp_rect.center(),
                egui::Align2::CENTER_CENTER,
                "»",
                egui::FontId::proportional(15.0),
                if hovered { p.accent } else { p.text },
            );
            if resp.on_hover_text("Perluas Sidebar").clicked() {
                action.toggle_collapse = true;
            }
        });
    } else {
        // Row 1 (36px): 36x36 icon + version title + vertically centered 28x28 collapse button
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 36.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                draw_logo_box(ui, p);
                ui.add_space(8.0);
                let brand_lbl = if crate::config::IS_PLUS_EDITION {
                    format!(
                        "JSFS+ v{}",
                        crate::config::APP_VERSION.trim_end_matches("-plus")
                    )
                } else {
                    format!("JSFS v{}", crate::config::APP_VERSION)
                };
                ui.label(RichText::new(brand_lbl).size(14.5).strong().color(p.text));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let col_btn = egui::Button::new(RichText::new("«").size(13.5).color(p.text))
                        .fill(p.control_bg)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius::same(7))
                        .min_size(Vec2::splat(28.0));
                    if ui.add(col_btn).on_hover_text("Ringkas Sidebar").clicked() {
                        action.toggle_collapse = true;
                    }
                });
            },
        );

        ui.add_space(6.0);

        // Row 2: Full-width compact status strip (no empty gap below logo)
        Frame::new()
            .fill(p.row_alt)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), 18.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        if crate::config::IS_PLUS_EDITION {
                            Frame::new()
                                .fill(p.accent_soft)
                                .corner_radius(CornerRadius::same(99))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("PLUS").size(10.5).strong().color(p.accent),
                                    );
                                });
                            ui.add_space(6.0);
                        }
                        let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                        let dot_c = if props.is_searching {
                            p.accent
                        } else if props.engine_ready {
                            p.good
                        } else {
                            p.warn
                        };
                        ui.painter().circle_filled(dot_r.center(), 3.6, dot_c);

                        let txt = t(props.lang);
                        let short_status = if props.is_searching {
                            txt.status_searching
                        } else {
                            props
                                .engine_label
                                .strip_prefix("Match Engine: ")
                                .unwrap_or(props.engine_label)
                        };
                        ui.add(
                            egui::Label::new(RichText::new(short_status).size(11.0).color(p.muted))
                                .wrap_mode(egui::TextWrapMode::Extend),
                        );
                    },
                );
            });
    }

    ui.add_space(8.0);
    draw_sidebar_separator(ui, p);
    ui.add_space(8.0);

    let items = [
        (
            NavPage::Search,
            (props.result_count > 0).then(|| props.result_count.to_string()),
            None,
        ),
        (NavPage::DetailWorkspace, None, None),
        (
            NavPage::DospenWorkspace,
            (props.shortlist_count > 0).then(|| props.shortlist_count.to_string()),
            None,
        ),
        (
            NavPage::Filters,
            (props.active_filter_count > 0).then(|| props.active_filter_count.to_string()),
            None,
        ),
        (
            NavPage::MatchEngine,
            None,
            Some(if props.engine_ready { p.good } else { p.warn }),
        ),
        (NavPage::History, None, None),
    ];

    let nav_gap = if props.collapsed { 6.0 } else { 3.0 };
    for (page, badge, dot) in items {
        if draw_nav_item(
            ui,
            p,
            page,
            props.lang,
            props.current == page,
            props.collapsed,
            badge,
            dot,
        ) {
            action.selected = Some(page);
        }
        ui.add_space(nav_gap);
    }

    // Subtle separator line above bottom section
    ui.add_space(6.0);
    let (sep_rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [
            egui::pos2(sep_rect.left() + 8.0, sep_rect.center().y),
            egui::pos2(sep_rect.right() - 8.0, sep_rect.center().y),
        ],
        Stroke::new(1.0, p.line),
    );
    ui.add_space(6.0);

    if draw_nav_item(
        ui,
        p,
        NavPage::Settings,
        props.lang,
        props.current == NavPage::Settings,
        props.collapsed,
        None,
        None,
    ) {
        action.selected = Some(NavPage::Settings);
    }
    ui.add_space(nav_gap);

    if draw_nav_item(
        ui,
        p,
        NavPage::Guide,
        props.lang,
        props.current == NavPage::Guide,
        props.collapsed,
        None,
        None,
    ) {
        action.selected = Some(NavPage::Guide);
    }

    // Pinned footer: Instant JSFS Dark / JSFS Daylight toggle
    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
        ui.add_space(8.0);
        if props.collapsed {
            ui.vertical_centered(|ui| {
                let (btn_rect, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                if resp.hovered() {
                    ui.painter()
                        .rect_filled(btn_rect, CornerRadius::same(8), p.nav_hover);
                }
                draw_vector_icon(
                    ui.painter(),
                    btn_rect.shrink(9.0),
                    VectorIcon::SunMoon,
                    p.accent,
                );
                if resp
                    .on_hover_text(format!(
                        "Tema: {} | Klik untuk ganti",
                        props.theme_mode.label()
                    ))
                    .clicked()
                {
                    action.toggle_theme = true;
                }
            });
            ui.add_space(6.0);
            let (sep_f, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
            ui.painter().line_segment(
                [
                    egui::pos2(sep_f.left() + 10.0, sep_f.center().y),
                    egui::pos2(sep_f.right() - 10.0, sep_f.center().y),
                ],
                Stroke::new(1.0, p.line),
            );
        } else {
            // Margin 7 + stroke 1 => content starts at inner_left + 8.0; 22 px slot centers icon at inner_left + 19.0
            Frame::new()
                .fill(p.card_bg)
                .stroke(Stroke::new(1.0, p.line))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(7, 7))
                .show(ui, |ui| {
                    let row_w = ui.available_width();
                    let resp = ui
                        .allocate_ui_with_layout(
                            Vec2::new(row_w, 22.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                let (slot_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                                draw_vector_icon(
                                    ui.painter(),
                                    slot_rect.shrink(3.0),
                                    VectorIcon::SunMoon,
                                    p.accent,
                                );
                                ui.label(
                                    RichText::new(props.theme_mode.label())
                                        .size(11.5)
                                        .strong()
                                        .color(p.text),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.add_space(2.0);
                                    ui.label(
                                        RichText::new(t(props.lang).theme_change)
                                            .size(10.5)
                                            .strong()
                                            .color(p.accent),
                                    );
                                });
                            },
                        )
                        .response
                        .interact(Sense::click());
                    if resp.clicked() {
                        action.toggle_theme = true;
                    }
                });
        }
    });

    action
}
