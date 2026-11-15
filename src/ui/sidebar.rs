//! Left navigation sidebar with JSFS brand identity, live engine indicator,
//! grouped colored icon tiles, active filter badges, and instant JSFS Dark / Daylight theme toggle.

use crate::ui::theme::{draw_vector_icon, Palette, ThemeMode, VectorIcon};
use egui::{
    Align, Color32, CornerRadius, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind,
    Ui, Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    Search,
    Filters,
    MatchEngine,
    History,
    Guide,
}

pub struct SidebarProps<'a> {
    pub current: NavPage,
    pub engine_label: &'a str,
    pub engine_ready: bool,
    pub is_searching: bool,
    pub result_count: usize,
    pub active_filter_count: usize,
    pub theme_mode: ThemeMode,
}

pub struct SidebarAction {
    pub selected: Option<NavPage>,
    pub toggle_theme: bool,
}

fn tile_color(page: NavPage) -> Color32 {
    match page {
        NavPage::Search => Color32::from_rgb(0x0a, 0x84, 0xff),
        NavPage::Filters => Color32::from_rgb(0x5e, 0x5c, 0xe6),
        NavPage::MatchEngine => Color32::from_rgb(0x30, 0xb9, 0x4d),
        NavPage::History => Color32::from_rgb(0xff, 0x9f, 0x0a),
        NavPage::Guide => Color32::from_rgb(0x32, 0xad, 0xe6),
    }
}

fn nav_icon(page: NavPage) -> VectorIcon {
    match page {
        NavPage::Search => VectorIcon::Search,
        NavPage::Filters => VectorIcon::Sliders,
        NavPage::MatchEngine => VectorIcon::Spark,
        NavPage::History => VectorIcon::Folder,
        NavPage::Guide => VectorIcon::Book,
    }
}

fn nav_label(page: NavPage) -> &'static str {
    match page {
        NavPage::Search => "Pencarian Jurnal",
        NavPage::Filters => "Filter Lanjutan",
        NavPage::MatchEngine => "Match Engine & AI",
        NavPage::History => "Riwayat & Ekspor",
        NavPage::Guide => "Panduan & Tentang",
    }
}

fn draw_nav_item(
    ui: &mut Ui,
    p: &Palette,
    page: NavPage,
    active: bool,
    badge: Option<String>,
    dot_color: Option<Color32>,
) -> bool {
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

    // 22x22 rounded tile icon for sidebar navigation
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
    draw_vector_icon(ui.painter(), tile_rect.shrink(3.5), nav_icon(page), Color32::WHITE);

    let text_color = if active { Color32::WHITE } else { p.text };
    ui.painter().text(
        egui::pos2(tile_rect.right() + 9.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        nav_label(page),
        egui::FontId::proportional(13.0),
        text_color,
    );

    if let Some(dot) = dot_color {
        let dot_c = if active { Color32::WHITE } else { dot };
        ui.painter()
            .circle_filled(egui::pos2(rect.right() - 14.0, rect.center().y), 3.5, dot_c);
    } else if let Some(b) = badge {
        let pill_bg = if active {
            Color32::from_rgba_unmultiplied(255, 255, 255, 52)
        } else {
            p.track_off
        };
        let pill_fg = if active { Color32::WHITE } else { p.muted };
        let badge_rect = Rect::from_center_size(
            egui::pos2(rect.right() - 18.0, rect.center().y),
            Vec2::new(24.0, 17.0),
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

pub fn render_sidebar(ui: &mut Ui, p: &Palette, props: SidebarProps<'_>) -> SidebarAction {
    let mut action = SidebarAction {
        selected: None,
        toggle_theme: false,
    };

    ui.add_space(10.0);

    // Identity Brand Header
    ui.horizontal(|ui| {
        let logo_size = Vec2::splat(44.0);
        let (logo_rect, _) = ui.allocate_exact_size(logo_size, Sense::hover());
        ui.painter()
            .rect_filled(logo_rect, CornerRadius::same(11), p.accent);
        ui.painter().rect_stroke(
            logo_rect,
            CornerRadius::same(11),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 64)),
            StrokeKind::Inside,
        );
        draw_vector_icon(
            ui.painter(),
            logo_rect.shrink(9.0),
            VectorIcon::BrandLogo,
            Color32::WHITE,
        );

        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(format!("JSFS v{}", crate::config::APP_VERSION))
                    .size(16.0)
                    .strong()
                    .color(p.text),
            );
            ui.horizontal(|ui| {
                let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                let dot_c = if props.is_searching {
                    p.accent
                } else if props.engine_ready {
                    p.good
                } else {
                    p.warn
                };
                ui.painter().circle_filled(dot_r.center(), 3.6, dot_c);

                let short_status = if props.is_searching {
                    "Mencari..."
                } else {
                    props
                        .engine_label
                        .strip_prefix("Match Engine: ")
                        .unwrap_or(props.engine_label)
                };
                ui.add(
                    egui::Label::new(RichText::new(short_status).size(11.5).color(p.muted))
                        .truncate(),
                );
            });
        });
    });

    ui.add_space(14.0);

    let items = [
        (
            NavPage::Search,
            (props.result_count > 0).then(|| props.result_count.to_string()),
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

    for (page, badge, dot) in items {
        if draw_nav_item(ui, p, page, props.current == page, badge, dot) {
            action.selected = Some(page);
        }
        ui.add_space(2.0);
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
        NavPage::Guide,
        props.current == NavPage::Guide,
        None,
        None,
    ) {
        action.selected = Some(NavPage::Guide);
    }

    // Pinned footer: Instant JSFS Dark / JSFS Daylight toggle card
    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
        ui.add_space(8.0);
        Frame::new()
            .fill(p.card_bg)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let resp = ui
                    .horizontal(|ui| {
                        let (ir, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                        draw_vector_icon(ui.painter(), ir, VectorIcon::SunMoon, p.accent);
                        ui.label(
                            RichText::new(props.theme_mode.label())
                                .size(12.0)
                                .strong()
                                .color(p.text),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new("Ubah").size(11.0).color(p.accent));
                        });
                    })
                    .response
                    .interact(Sense::click());
                if resp.clicked() {
                    action.toggle_theme = true;
                }
            });
    });

    action
}
