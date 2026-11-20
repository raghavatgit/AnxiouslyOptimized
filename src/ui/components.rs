use crate::ui::theme;
use egui::{Color32, CornerRadius, FontId, Margin, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};
use std::collections::VecDeque;

/// Renders the signature **Optimizer** wordmark paired with smaller **by Raghav** subtitle.
pub fn brand_wordmark(ui: &mut egui::Ui, hero: bool) {
    let title_size = if hero { 26.0 } else { 18.5 };
    let byline_size = if hero { 12.5 } else { 10.5 };
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.label(
            RichText::new("Optimizer")
                .size(title_size)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.label(
            RichText::new("by Raghav")
                .size(byline_size)
                .color(theme::ACCENT_GOLD),
        );
    });
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
}

pub fn section_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(
            RichText::new(title)
                .size(20.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.label(
            RichText::new(subtitle)
                .size(12.5)
                .color(theme::TEXT_SECONDARY),
        );
    });
    ui.add_space(6.0);
}

pub fn metric_card(
    ui: &mut egui::Ui,
    min_width: f32,
    label: &str,
    value: &str,
    sub_detail: &str,
    accent: Color32,
) {
    egui::Frame::new()
        .fill(theme::BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(min_width);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.horizontal(|ui| {
                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 3.5, accent);
                    ui.label(
                        RichText::new(label.to_ascii_uppercase())
                            .size(10.5)
                            .strong()
                            .color(theme::TEXT_MUTED),
                    );
                });
                ui.label(
                    RichText::new(value)
                        .size(16.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new(sub_detail)
                        .size(11.5)
                        .monospace()
                        .color(theme::TEXT_SECONDARY),
                );
            });
        });
}

pub fn status_badge(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, fg.linear_multiply(0.55)))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).strong().color(fg));
        });
}

pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).strong().color(Color32::WHITE))
            .fill(theme::ACCENT_COBALT)
            .stroke(Stroke::new(1.0_f32, theme::ACCENT_COBALT))
            .corner_radius(CornerRadius::same(6)),
    )
}

pub fn emerald_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).strong().color(Color32::from_rgb(6, 24, 18)))
            .fill(theme::ACCENT_EMERALD)
            .stroke(Stroke::new(1.0_f32, theme::ACCENT_EMERALD))
            .corner_radius(CornerRadius::same(6)),
    )
}

pub fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).strong().color(theme::ACCENT_ROSE))
            .fill(theme::ACCENT_ROSE_DIM)
            .stroke(Stroke::new(1.0_f32, theme::ACCENT_ROSE))
            .corner_radius(CornerRadius::same(6)),
    )
}

pub fn subtle_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(theme::TEXT_PRIMARY))
            .fill(theme::BG_ELEVATED)
            .stroke(Stroke::new(1.0_f32, theme::BORDER_STRONG))
            .corner_radius(CornerRadius::same(6)),
    )
}

pub fn kv_row(ui: &mut egui::Ui, key: &str, val: &str, mono: bool) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{key}:"))
                .size(12.5)
                .color(theme::TEXT_MUTED),
        );
        let rt = if mono {
            RichText::new(val).size(12.5).monospace().color(theme::TEXT_PRIMARY)
        } else {
            RichText::new(val).size(12.5).color(theme::TEXT_PRIMARY)
        };
        ui.label(rt);
    });
}

pub fn progress_bar(ui: &mut egui::Ui, fraction: f32, overlay_text: &str, accent: Color32) {
    let desired = Vec2::new(ui.available_width().max(120.0), 16.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(4), theme::BG_CANVAS);
    let clamped = fraction.clamp(0.0, 1.0);
    if clamped > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width() * clamped, rect.height()));
        p.rect_filled(fill_rect, CornerRadius::same(4), accent.linear_multiply(0.85));
    }
    p.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        overlay_text,
        FontId::new(11.0, egui::FontFamily::Monospace),
        theme::TEXT_PRIMARY,
    );
}

/// Real rolling 0..100% sparkline backed by live `sysinfo` sample buffer (`VecDeque<f32>`).
pub fn live_sparkline(
    ui: &mut egui::Ui,
    title: &str,
    current_value: &str,
    subtitle: &str,
    samples: &VecDeque<f32>,
    accent: Color32,
    chart_height: f32,
) {
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(13.0).strong().color(theme::TEXT_PRIMARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(current_value)
                        .size(16.0)
                        .strong()
                        .monospace()
                        .color(accent),
                );
            });
        });
        ui.label(RichText::new(subtitle).size(11.5).color(theme::TEXT_SECONDARY));
        ui.add_space(6.0);

        let width = ui.available_width().max(140.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, chart_height), egui::Sense::hover());
        let p = ui.painter();

        p.rect_filled(rect, CornerRadius::same(5), theme::BG_CANVAS);
        // Subtle 50% grid guide
        let mid_y = rect.center().y;
        p.line_segment(
            [Pos2::new(rect.left(), mid_y), Pos2::new(rect.right(), mid_y)],
            Stroke::new(1.0_f32, Color32::from_rgb(28, 35, 48)),
        );

        if samples.len() >= 2 {
            let n = samples.len();
            let dx = rect.width() / (n.saturating_sub(1).max(1) as f32);
            let inner_top = rect.top() + 5.0;
            let inner_h = (rect.height() - 10.0).max(8.0);

            let points: Vec<Pos2> = samples
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let norm = (v.clamp(0.0, 100.0)) / 100.0;
                    let x = rect.left() + (i as f32) * dx;
                    let y = inner_top + inner_h * (1.0 - norm);
                    Pos2::new(x, y)
                })
                .collect();

            // Subtle vertical gradient/segment fill ribbons under the curve
            let fill_color = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 36);
            for w in points.windows(2) {
                let mesh_poly = vec![
                    w[0],
                    w[1],
                    Pos2::new(w[1].x, rect.bottom() - 1.0),
                    Pos2::new(w[0].x, rect.bottom() - 1.0),
                ];
                p.add(egui::Shape::convex_polygon(mesh_poly, fill_color, Stroke::NONE));
            }
            p.add(egui::Shape::line(points.clone(), Stroke::new(2.0_f32, accent)));
            if let Some(last) = points.last() {
                p.circle_filled(*last, 3.5, accent);
            }
        } else {
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Collecting live samples...",
                FontId::new(11.0, egui::FontFamily::Proportional),
                theme::TEXT_MUTED,
            );
        }

        p.rect_stroke(
            rect,
            CornerRadius::same(5),
            Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
            StrokeKind::Inside,
        );
    });
}
