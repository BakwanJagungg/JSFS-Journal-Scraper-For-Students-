//! JSFS System-Settings inspired theme, color tokens, native Segoe UI typography, and reusable widgets.

use egui::{
    Align, Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame,
    Layout, Margin, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2,
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn toggled(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "JSFS Dark",
            Self::Light => "JSFS Daylight",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub mode: ThemeMode,
    pub sidebar_bg: Color32,
    pub content_bg: Color32,
    pub card_bg: Color32,
    pub control_bg: Color32,
    pub control_hover: Color32,
    pub nav_hover: Color32,
    pub row_alt: Color32,
    pub line: Color32,
    pub row_line: Color32,
    pub track_off: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub good: Color32,
    pub good_soft: Color32,
    pub warn: Color32,
    pub warn_soft: Color32,
    pub bad: Color32,
    pub bad_soft: Color32,
}

impl Palette {
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                mode,
                sidebar_bg: Color32::from_rgb(0x23, 0x23, 0x26),
                content_bg: Color32::from_rgb(0x18, 0x18, 0x1b),
                card_bg: Color32::from_rgb(0x27, 0x27, 0x2a),
                control_bg: Color32::from_rgb(0x34, 0x34, 0x38),
                control_hover: Color32::from_rgb(0x3f, 0x3f, 0x45),
                nav_hover: Color32::from_rgba_unmultiplied(255, 255, 255, 16),
                row_alt: Color32::from_rgb(0x21, 0x21, 0x25),
                line: Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                row_line: Color32::from_rgba_unmultiplied(255, 255, 255, 14),
                track_off: Color32::from_rgba_unmultiplied(255, 255, 255, 42),
                text: Color32::from_rgb(0xf5, 0xf5, 0xf7),
                muted: Color32::from_rgb(0xa1, 0xa1, 0xaa),
                faint: Color32::from_rgb(0x71, 0x71, 0x7a),
                accent: Color32::from_rgb(0x0a, 0x84, 0xff),
                accent_soft: Color32::from_rgba_unmultiplied(0x0a, 0x84, 0xff, 64),
                good: Color32::from_rgb(0x32, 0xd7, 0x4b),
                good_soft: Color32::from_rgba_unmultiplied(0x32, 0xd7, 0x4b, 34),
                warn: Color32::from_rgb(0xff, 0xb3, 0x40),
                warn_soft: Color32::from_rgba_unmultiplied(0xff, 0xb3, 0x40, 36),
                bad: Color32::from_rgb(0xff, 0x45, 0x3a),
                bad_soft: Color32::from_rgba_unmultiplied(0xff, 0x45, 0x3a, 34),
            },
            ThemeMode::Light => Self {
                mode,
                sidebar_bg: Color32::from_rgb(248, 250, 252), // #f8fafc
                content_bg: Color32::from_rgb(238, 242, 246), // #eef2f6
                card_bg: Color32::from_rgb(255, 255, 255),    // #ffffff
                control_bg: Color32::from_rgb(241, 245, 249), // #f1f5f9
                control_hover: Color32::from_rgb(226, 232, 240), // #e2e8f0
                nav_hover: Color32::from_rgba_unmultiplied(15, 23, 42, 16),
                row_alt: Color32::from_rgb(248, 250, 252),
                line: Color32::from_rgb(203, 213, 225), // #cbd5e1
                row_line: Color32::from_rgb(226, 232, 240),
                track_off: Color32::from_rgb(203, 213, 225),
                text: Color32::from_rgb(15, 23, 42),   // #0f172a
                muted: Color32::from_rgb(71, 85, 105), // #475569
                faint: Color32::from_rgb(100, 116, 139), // #64748b
                accent: Color32::from_rgb(29, 78, 216), // #1d4ed8
                accent_soft: Color32::from_rgb(219, 234, 254), // #dbeafe
                good: Color32::from_rgb(21, 128, 61),  // #15803d
                good_soft: Color32::from_rgb(220, 252, 231), // #dcfce7
                warn: Color32::from_rgb(146, 64, 14),  // #92400e
                warn_soft: Color32::from_rgb(254, 243, 199), // #fef3c7
                bad: Color32::from_rgb(185, 28, 28),   // #b91c1c
                bad_soft: Color32::from_rgb(254, 226, 226), // #fee2e2
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VectorIcon {
    Search,
    Document,
    Star,
    Sliders,
    Spark,
    Folder,
    Book,
    Settings,
    SunMoon,
    BrandLogo,
}

/// Loads Windows system Segoe UI font when available for crisp desktop legibility.
pub fn load_crisp_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let candidate_paths = [
        r"C:\Windows\Fonts\segoeui.ttf",
        r"/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        r"/System/Library/Fonts/SFNS.ttf",
    ];
    for p in candidate_paths {
        if let Ok(bytes) = std::fs::read(p) {
            fonts.font_data.insert(
                "system_sans".to_string(),
                Arc::new(FontData::from_owned(bytes)),
            );
            if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
                fam.insert(0, "system_sans".to_string());
            }
            break;
        }
    }
    if let Ok(sym_bytes) = std::fs::read(r"C:\Windows\Fonts\seguisym.ttf") {
        fonts.font_data.insert(
            "system_symbol".to_string(),
            Arc::new(FontData::from_owned(sym_bytes)),
        );
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
            let pos = fam
                .iter()
                .position(|n| n == "system_sans")
                .map_or(0, |i| i + 1);
            fam.insert(pos, "system_symbol".to_string());
        }
    }
    ctx.set_fonts(fonts);
}

pub fn apply_theme(ctx: &Context, p: &Palette) {
    let mut visuals = match p.mode {
        ThemeMode::Dark => egui::Visuals::dark(),
        ThemeMode::Light => egui::Visuals::light(),
    };

    visuals.panel_fill = p.content_bg;
    visuals.window_fill = p.card_bg;
    visuals.extreme_bg_color = p.control_bg;
    visuals.faint_bg_color = p.row_alt;
    visuals.code_bg_color = p.control_bg;
    visuals.override_text_color = Some(p.text);
    visuals.window_corner_radius = CornerRadius::same(12);
    visuals.window_stroke = Stroke::new(1.0, p.line);

    visuals.selection.bg_fill = p.accent;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

    let cr = CornerRadius::same(6);
    visuals.widgets.noninteractive.bg_fill = p.card_bg;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    visuals.widgets.noninteractive.corner_radius = cr;

    visuals.widgets.inactive.bg_fill = p.control_bg;
    visuals.widgets.inactive.weak_bg_fill = p.control_bg;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.line);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    visuals.widgets.inactive.corner_radius = cr;

    visuals.widgets.hovered.bg_fill = p.control_hover;
    visuals.widgets.hovered.weak_bg_fill = p.control_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    visuals.widgets.hovered.corner_radius = cr;

    visuals.widgets.active.bg_fill = p.accent_soft;
    visuals.widgets.active.weak_bg_fill = p.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.2, p.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.2, p.text);
    visuals.widgets.active.corner_radius = cr;

    let target_theme = match p.mode {
        ThemeMode::Dark => egui::Theme::Dark,
        ThemeMode::Light => egui::Theme::Light,
    };
    ctx.set_theme(target_theme);
    ctx.style_mut_of(target_theme, |style| {
        style.visuals = visuals;
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(21.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Name("Subheading".into()),
            FontId::new(15.0, FontFamily::Proportional),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(13.5, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        );

        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 6.0);
        style.spacing.interact_size.y = 32.0;
    });
}

pub fn card_frame(p: &Palette) -> Frame {
    Frame::new()
        .fill(p.card_bg)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
}

pub fn card(ui: &mut Ui, p: &Palette, add_contents: impl FnOnce(&mut Ui)) {
    card_frame(p).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        add_contents(ui);
    });
}

pub fn card_padded(ui: &mut Ui, p: &Palette, pad: i8, add_contents: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(p.card_bg)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(pad))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui);
        });
}

/// Color pair (foreground, background) for international Q1–Q4 accreditation pills.
pub fn quartile_pill_colors(p: &Palette, q: &str) -> (Color32, Color32) {
    match q.to_ascii_uppercase().as_str() {
        "Q1" => (
            Color32::from_rgb(0xd9, 0x77, 0x06),
            Color32::from_rgba_unmultiplied(0xf5, 0x9e, 0x0b, 38),
        ),
        "Q2" => (
            Color32::from_rgb(0x93, 0x33, 0xea),
            Color32::from_rgba_unmultiplied(0xa8, 0x55, 0xf7, 36),
        ),
        "Q3" => (p.accent, p.accent_soft),
        _ => (p.good, p.good_soft),
    }
}

pub fn card_divider(ui: &mut Ui, p: &Palette) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, p.row_line),
    );
    ui.add_space(4.0);
}

pub fn page_header(ui: &mut Ui, p: &Palette, title: &str, subtitle: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).size(21.0).strong().color(p.text));
    if !subtitle.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(13.5).color(p.muted));
    }
    ui.add_space(10.0);
}

fn styled_button(
    ui: &mut Ui,
    label: &str,
    fg: Color32,
    fill: Color32,
    stroke: Color32,
    strong: bool,
    height: f32,
) -> Response {
    let mut rt = RichText::new(label).size(13.0).color(fg);
    if strong {
        rt = rt.strong();
    }
    let btn = egui::Button::new(rt)
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(CornerRadius::same(7))
        .min_size(Vec2::new(0.0, height));
    ui.add(btn)
}

pub fn accent_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    accent_button_h(ui, p, label, 30.0)
}

pub fn accent_button_h(ui: &mut Ui, p: &Palette, label: &str, height: f32) -> Response {
    styled_button(ui, label, Color32::WHITE, p.accent, p.accent, true, height)
}

pub fn secondary_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    secondary_button_h(ui, p, label, 28.0)
}

pub fn secondary_button_h(ui: &mut Ui, p: &Palette, label: &str, height: f32) -> Response {
    styled_button(ui, label, p.text, p.control_bg, p.line, false, height)
}

pub fn danger_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    danger_button_h(ui, p, label, 30.0)
}

pub fn danger_button_h(ui: &mut Ui, p: &Palette, label: &str, height: f32) -> Response {
    styled_button(ui, label, Color32::WHITE, p.bad, p.bad, true, height)
}

pub fn pill_badge(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).strong().color(fg));
        });
}

/// System-Settings style 38x22 smooth sliding toggle switch.
pub fn toggle_switch(ui: &mut Ui, p: &Palette, value: &mut bool) -> Response {
    let desired_size = Vec2::new(38.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *value);
        let radius = rect.height() * 0.5;
        let bg = Color32::from_rgba_unmultiplied(
            ((p.track_off.r() as f32) * (1.0 - how_on) + (p.accent.r() as f32) * how_on) as u8,
            ((p.track_off.g() as f32) * (1.0 - how_on) + (p.accent.g() as f32) * how_on) as u8,
            ((p.track_off.b() as f32) * (1.0 - how_on) + (p.accent.b() as f32) * how_on) as u8,
            ((p.track_off.a() as f32) * (1.0 - how_on) + (p.accent.a() as f32) * how_on) as u8,
        );
        ui.painter().rect_filled(rect, radius, bg);
        let knob_r = rect.height() * 0.5 - 2.0;
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        ui.painter()
            .circle_filled(Pos2::new(x, rect.center().y), knob_r, Color32::WHITE);
    }
    response
}

/// Full-width settings row with title, hint caption, and trailing pill switch.
pub fn toggle_row(ui: &mut Ui, p: &Palette, value: &mut bool, label: &str, hint: &str) -> Response {
    let mut changed_resp = None;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 56.0).max(120.0));
            ui.label(RichText::new(label).size(13.0).color(p.text));
            if !hint.is_empty() {
                ui.label(RichText::new(hint).size(11.5).color(p.muted));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            changed_resp = Some(toggle_switch(ui, p, value));
        });
    });
    changed_resp.expect("toggle_switch rendered")
}

/// Crisp DPI-independent vector icons rendered directly via GPU primitives.
pub fn draw_vector_icon(painter: &egui::Painter, rect: Rect, icon: VectorIcon, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let stroke = Stroke::new((s * 0.11).max(1.4), color);

    match icon {
        VectorIcon::Search => {
            let lens_c = Pos2::new(c.x - s * 0.08, c.y - s * 0.08);
            painter.circle_stroke(lens_c, s * 0.25, stroke);
            painter.line_segment(
                [
                    Pos2::new(lens_c.x + s * 0.18, lens_c.y + s * 0.18),
                    Pos2::new(c.x + s * 0.34, c.y + s * 0.34),
                ],
                stroke,
            );
        }
        VectorIcon::Document => {
            let doc_r = Rect::from_center_size(c, Vec2::new(s * 0.54, s * 0.68));
            painter.rect_stroke(doc_r, 2.0, stroke, StrokeKind::Middle);
            for dy in [-0.14_f32, 0.0, 0.14] {
                painter.line_segment(
                    [
                        Pos2::new(c.x - s * 0.15, c.y + s * dy),
                        Pos2::new(c.x + s * 0.15, c.y + s * dy),
                    ],
                    stroke,
                );
            }
        }
        VectorIcon::Star => {
            let mut pts = Vec::with_capacity(10);
            for i in 0..10 {
                let angle = -std::f32::consts::FRAC_PI_2 + (i as f32) * std::f32::consts::PI / 5.0;
                let rad = if i % 2 == 0 { s * 0.36 } else { s * 0.16 };
                let (sin_a, cos_a) = angle.sin_cos();
                pts.push(Pos2::new(c.x + cos_a * rad, c.y + sin_a * rad));
            }
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        VectorIcon::Sliders => {
            for (idx, knob_x) in [-0.14_f32, 0.16, -0.02].iter().enumerate() {
                let y = c.y + (idx as f32 - 1.0) * s * 0.26;
                painter.line_segment(
                    [Pos2::new(c.x - s * 0.34, y), Pos2::new(c.x + s * 0.34, y)],
                    stroke,
                );
                painter.circle_filled(Pos2::new(c.x + knob_x * s, y), s * 0.09, color);
            }
        }
        VectorIcon::Spark => {
            let pts = [
                Pos2::new(c.x, c.y - s * 0.36),
                Pos2::new(c.x + s * 0.11, c.y - s * 0.11),
                Pos2::new(c.x + s * 0.36, c.y),
                Pos2::new(c.x + s * 0.11, c.y + s * 0.11),
                Pos2::new(c.x, c.y + s * 0.36),
                Pos2::new(c.x - s * 0.11, c.y + s * 0.11),
                Pos2::new(c.x - s * 0.36, c.y),
                Pos2::new(c.x - s * 0.11, c.y - s * 0.11),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts.to_vec(),
                color,
                Stroke::NONE,
            ));
        }
        VectorIcon::Folder => {
            let folder_r = Rect::from_center_size(
                Pos2::new(c.x, c.y + s * 0.04),
                Vec2::new(s * 0.68, s * 0.48),
            );
            painter.rect_stroke(folder_r, 2.5, stroke, StrokeKind::Middle);
            painter.line_segment(
                [
                    Pos2::new(folder_r.left() + s * 0.04, folder_r.top() - s * 0.08),
                    Pos2::new(folder_r.left() + s * 0.30, folder_r.top() - s * 0.08),
                ],
                stroke,
            );
        }
        VectorIcon::Book => {
            let r = Rect::from_center_size(c, Vec2::new(s * 0.60, s * 0.68));
            painter.rect_stroke(r, 2.5, stroke, StrokeKind::Middle);
            painter.line_segment(
                [
                    Pos2::new(r.left() + s * 0.15, r.top()),
                    Pos2::new(r.left() + s * 0.15, r.bottom()),
                ],
                stroke,
            );
        }
        VectorIcon::Settings => {
            painter.circle_stroke(c, s * 0.22, stroke);
            for step in 0..6 {
                let ang = (step as f32) * std::f32::consts::TAU / 6.0;
                let (sin_a, cos_a) = ang.sin_cos();
                painter.line_segment(
                    [
                        Pos2::new(c.x + cos_a * s * 0.25, c.y + sin_a * s * 0.25),
                        Pos2::new(c.x + cos_a * s * 0.36, c.y + sin_a * s * 0.36),
                    ],
                    stroke,
                );
            }
        }
        VectorIcon::SunMoon => {
            painter.circle_stroke(c, s * 0.24, stroke);
            painter.circle_filled(Pos2::new(c.x - s * 0.05, c.y), s * 0.12, color);
        }
        VectorIcon::BrandLogo => {
            let page = Rect::from_center_size(
                Pos2::new(c.x - s * 0.03, c.y),
                Vec2::new(s * 0.56, s * 0.62),
            );
            painter.rect_stroke(page, 4.0, Stroke::new(2.0, color), StrokeKind::Middle);
            painter.circle_filled(
                Pos2::new(page.center().x + s * 0.08, page.center().y - s * 0.04),
                s * 0.12,
                color,
            );
            painter.line_segment(
                [
                    Pos2::new(page.left() + s * 0.12, page.top() + s * 0.16),
                    Pos2::new(page.right() - s * 0.14, page.top() + s * 0.16),
                ],
                Stroke::new(2.0, color),
            );
        }
    }
}
