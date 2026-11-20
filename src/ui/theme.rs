use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};

// Refined Obsidian & High-Contrast Slate Surface Hierarchy (Zero purple-cyan slop)
pub const BG_CANVAS: Color32 = Color32::from_rgb(9, 13, 20);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(13, 18, 28);
pub const BG_SURFACE: Color32 = Color32::from_rgb(18, 25, 38);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(25, 34, 51);
pub const BG_HOVER: Color32 = Color32::from_rgb(33, 45, 67);

pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(36, 48, 69);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(56, 74, 105);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(243, 247, 252);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(165, 180, 202);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(112, 128, 154);

// Semantic Accents
pub const ACCENT_COBALT: Color32 = Color32::from_rgb(59, 130, 246);
pub const ACCENT_COBALT_DIM: Color32 = Color32::from_rgb(26, 52, 98);
pub const ACCENT_GOLD: Color32 = Color32::from_rgb(245, 158, 11);
pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(16, 185, 129);
pub const ACCENT_EMERALD_DIM: Color32 = Color32::from_rgb(14, 62, 48);
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11);
pub const ACCENT_AMBER_DIM: Color32 = Color32::from_rgb(74, 49, 12);
pub const ACCENT_ROSE: Color32 = Color32::from_rgb(244, 63, 94);
pub const ACCENT_ROSE_DIM: Color32 = Color32::from_rgb(78, 20, 35);
pub const ACCENT_CYAN_SLATE: Color32 = Color32::from_rgb(56, 189, 248);

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = BG_CANVAS;
    visuals.window_fill = BG_SURFACE;
    visuals.extreme_bg_color = BG_CANVAS;
    visuals.faint_bg_color = BG_ELEVATED;
    visuals.code_bg_color = BG_SIDEBAR;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.hyperlink_color = ACCENT_CYAN_SLATE;
    visuals.warn_fg_color = ACCENT_AMBER;
    visuals.error_fg_color = ACCENT_ROSE;

    visuals.selection.bg_fill = ACCENT_COBALT_DIM;
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT_COBALT);

    visuals.widgets.noninteractive.bg_fill = BG_SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = BG_SIDEBAR;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(7);

    visuals.widgets.inactive.bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.weak_bg_fill = BG_SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(7);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_COBALT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(7);

    visuals.widgets.active.bg_fill = ACCENT_COBALT_DIM;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT_COBALT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(7);

    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    // Ultra-responsive 110ms micro-transitions & sleek scrollbar geometry
    style.animation_time = 0.11;
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(11.0, 5.5);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(18.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(13.2, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(12.8, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.2, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.2, FontFamily::Monospace),
    );
    ctx.set_style(style);
}
