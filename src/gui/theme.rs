use eframe::egui::{self, Color32, Margin, Rounding, Shadow, Stroke};

pub const BG_CANVAS: Color32 = Color32::from_rgb(12, 15, 22);
pub const BG_PANEL: Color32 = Color32::from_rgb(17, 21, 30);
pub const BG_CARD: Color32 = Color32::from_rgb(23, 28, 40);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(32, 39, 55);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(44, 53, 74);
pub const BORDER_ACCENT: Color32 = Color32::from_rgb(59, 130, 246);

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(34, 197, 94);
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(217, 119, 6);
pub const ACCENT_RED: Color32 = Color32::from_rgb(220, 68, 68);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(236, 241, 248);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

/// Applies a modern dark-slate peripheral-utility theme across all `egui` widgets.
pub fn apply_modern_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = Margin::same(16.0);
    style.spacing.slider_width = 165.0;

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_CANVAS;
    visuals.window_fill = BG_CARD;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.window_rounding = Rounding::same(12.0);
    visuals.window_shadow = Shadow {
        offset: egui::vec2(0.0, 10.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_black_alpha(140),
    };
    visuals.menu_rounding = Rounding::same(8.0);
    visuals.popup_shadow = visuals.window_shadow;
    visuals.selection.bg_fill = ACCENT_BLUE.linear_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT_BLUE);

    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(7.0);

    visuals.widgets.inactive.bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.weak_bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(7.0);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(43, 52, 74);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(43, 52, 74);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_BLUE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(7.0);

    visuals.widgets.active.bg_fill = ACCENT_BLUE;
    visuals.widgets.active.weak_bg_fill = ACCENT_BLUE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, Color32::from_rgb(147, 197, 253));
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(7.0);

    style.visuals = visuals;
    ctx.set_style(style);
}

/// Standard elevated card frame for side panels and modal sections.
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_CARD)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(12.0))
}

/// Renders a subtle section header inside a card.
pub fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title.to_uppercase())
            .size(11.0)
            .strong()
            .color(TEXT_MUTED),
    );
    ui.add_space(4.0);
}

/// Renders a styled key/parameter badge + description row for Help dialogs.
pub fn guide_row(ui: &mut egui::Ui, badge: &str, description: &str, accent: Color32) {
    ui.horizontal_wrapped(|ui| {
        egui::Frame::none()
            .fill(accent.linear_multiply(0.18))
            .stroke(Stroke::new(1.0_f32, accent.linear_multiply(0.6)))
            .rounding(Rounding::same(5.0))
            .inner_margin(Margin::symmetric(7.0, 2.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(badge)
                        .size(12.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
            });
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(description)
                .size(13.0)
                .color(Color32::from_rgb(203, 213, 225)),
        );
    });
    ui.add_space(4.0);
}
