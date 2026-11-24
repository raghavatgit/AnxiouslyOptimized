use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Visuals,
};
use std::sync::Arc;

// ==================== FUTURISTIC OBSIDIAN & ELECTRIC CYAN PALETTE ====================
// Intentional high-craft dark palette: deep space obsidian surfaces, hairline glass strokes,
// crisp crisp-white primary typography, and restrained semantic accents (zero purple-indigo slop).
pub const BG_VOID: Color32 = Color32::from_rgb(6, 8, 13);
pub const BG_CANVAS: Color32 = Color32::from_rgb(9, 12, 18);
pub const BG_RAIL: Color32 = Color32::from_rgb(11, 15, 23);
pub const BG_SURFACE: Color32 = Color32::from_rgb(14, 19, 29);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(20, 27, 40);
pub const BG_HOVER: Color32 = Color32::from_rgb(27, 36, 53);
pub const BG_ACTIVE: Color32 = Color32::from_rgb(16, 38, 56);

pub const BORDER_HAIRLINE: Color32 = Color32::from_rgb(28, 38, 54);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(36, 49, 70);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(56, 76, 108);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 247, 252);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(154, 168, 190);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(96, 111, 135);

// Primary Futuristic Accent (Electric Ice Cyan) + Semantic Accents
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
pub const ACCENT_CYAN_DIM: Color32 = Color32::from_rgb(14, 42, 62);
pub const ACCENT_COBALT: Color32 = Color32::from_rgb(59, 130, 246);
pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(16, 185, 129);
pub const ACCENT_EMERALD_DIM: Color32 = Color32::from_rgb(11, 48, 38);
pub const ACCENT_GOLD: Color32 = Color32::from_rgb(245, 158, 11);
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11);
pub const ACCENT_AMBER_DIM: Color32 = Color32::from_rgb(58, 39, 11);
pub const ACCENT_ROSE: Color32 = Color32::from_rgb(244, 63, 94);
pub const ACCENT_ROSE_DIM: Color32 = Color32::from_rgb(62, 18, 30);

/// Loads crisp native Windows typography (`Segoe UI Variable` / `Segoe UI` + `Cascadia Mono` / `Consolas`)
/// when available on disk, falling back cleanly to `egui` built-in fonts without increasing binary size.
pub fn install_system_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".to_string());
    let font_dir = std::path::Path::new(&win_dir).join("Fonts");

    let proportional_candidates = [
        "SegUIVar.ttf",
        "segoeui.ttf",
    ];
    for file in proportional_candidates {
        let p = font_dir.join(file);
        if let Ok(bytes) = std::fs::read(&p) {
            fonts.font_data.insert(
                "sys_sans".to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
                fam.insert(0, "sys_sans".to_owned());
            }
            break;
        }
    }

    let mono_candidates = [
        "CascadiaMono.ttf",
        "CascadiaCode.ttf",
        "consola.ttf",
    ];
    for file in mono_candidates {
        let p = font_dir.join(file);
        if let Ok(bytes) = std::fs::read(&p) {
            fonts.font_data.insert(
                "sys_mono".to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
                fam.insert(0, "sys_mono".to_owned());
            }
            break;
        }
    }

    ctx.set_fonts(fonts);
}

pub fn apply_theme(ctx: &egui::Context) {
    install_system_fonts(ctx);

    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = BG_CANVAS;
    visuals.window_fill = BG_SURFACE;
    visuals.extreme_bg_color = BG_VOID;
    visuals.faint_bg_color = BG_ELEVATED;
    visuals.code_bg_color = BG_VOID;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.hyperlink_color = ACCENT_CYAN;
    visuals.warn_fg_color = ACCENT_AMBER;
    visuals.error_fg_color = ACCENT_ROSE;

    visuals.selection.bg_fill = ACCENT_CYAN_DIM;
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT_CYAN);

    visuals.widgets.noninteractive.bg_fill = BG_SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = BG_RAIL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_HAIRLINE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    visuals.widgets.inactive.bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.weak_bg_fill = BG_SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_CYAN);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);

    visuals.widgets.active.bg_fill = BG_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT_CYAN);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);

    visuals.widgets.open.bg_fill = BG_ELEVATED;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, ACCENT_CYAN);
    visuals.widgets.open.corner_radius = CornerRadius::same(8);

    visuals.window_corner_radius = CornerRadius::same(14);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    visuals.window_shadow = egui::Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(160),
    };
    ctx.set_visuals(visuals);

    apply_scale_metrics(ctx, 1.0);
}

pub fn apply_scale_metrics(ctx: &egui::Context, ui_scale: f32) {
    ctx.set_zoom_factor(ui_scale.clamp(0.85, 1.35));

    let mut style = (*ctx.style()).clone();
    style.animation_time = 0.14;
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(13.0, 6.5);
    style.spacing.window_margin = Margin::same(16);
    style.spacing.scroll.bar_width = 5.0;
    style.spacing.scroll.bar_inner_margin = 3.0;
    style.spacing.scroll.floating = true;

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(13.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.2, FontFamily::Monospace),
    );
    ctx.set_style(style);
}
