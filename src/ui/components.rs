use crate::ui::theme;
use egui::{
    Color32, CornerRadius, FontFamily, FontId, Margin, Pos2, Rect, Response, RichText, Sense,
    Stroke, StrokeKind, Vec2,
};
use std::collections::VecDeque;
use std::f32::consts::TAU;

// ==================== VECTOR ICON SYSTEM (ZERO EMOJI / ZERO FONT DEPENDENCY) ====================
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavIcon {
    Home,
    Boost,
    Tweaks,
    Clean,
    Apps,
    Gaming,
    Network,
    Safety,
    Specs,
    Settings,
    Search,
    Refresh,
    Update,
    Terminal,
    SidebarToggle,
}

pub fn paint_icon(painter: &egui::Painter, center: Pos2, size: f32, icon: NavIcon, color: Color32) {
    let s = size * 0.5;
    let stroke = Stroke::new(1.65_f32, color);

    match icon {
        NavIcon::Home => {
            // 2x2 Bento grid glyph
            let gap = 1.6;
            let cell = s - gap;
            let tl = Rect::from_min_size(Pos2::new(center.x - s, center.y - s), Vec2::splat(cell));
            let tr = Rect::from_min_size(Pos2::new(center.x + gap, center.y - s), Vec2::splat(cell));
            let bl = Rect::from_min_size(Pos2::new(center.x - s, center.y + gap), Vec2::splat(cell));
            let br = Rect::from_min_size(Pos2::new(center.x + gap, center.y + gap), Vec2::splat(cell));
            for r in [tl, tr, bl, br] {
                painter.rect_stroke(r, CornerRadius::same(2), stroke, StrokeKind::Inside);
            }
        }
        NavIcon::Boost => {
            // Sharp lightning bolt
            let pts = vec![
                Pos2::new(center.x + s * 0.15, center.y - s),
                Pos2::new(center.x - s * 0.65, center.y + s * 0.10),
                Pos2::new(center.x - s * 0.05, center.y + s * 0.10),
                Pos2::new(center.x - s * 0.15, center.y + s),
                Pos2::new(center.x + s * 0.65, center.y - s * 0.10),
                Pos2::new(center.x + s * 0.05, center.y - s * 0.10),
                Pos2::new(center.x + s * 0.15, center.y - s),
            ];
            painter.add(egui::Shape::line(pts, stroke));
        }
        NavIcon::Tweaks => {
            // 3 horizontal sliders with circular knobs
            for (i, knob_x) in [-0.35_f32, 0.40, -0.15].iter().enumerate() {
                let y = center.y + (i as f32 - 1.0) * (s * 0.68);
                painter.line_segment(
                    [Pos2::new(center.x - s, y), Pos2::new(center.x + s, y)],
                    stroke,
                );
                let kx = center.x + knob_x * s;
                painter.circle_filled(Pos2::new(kx, y), 2.3, theme::BG_SURFACE);
                painter.circle_stroke(Pos2::new(kx, y), 2.3, stroke);
            }
        }
        NavIcon::Clean => {
            // Memory / gauge arc + needle
            let r = s * 0.92;
            let mut arc = Vec::with_capacity(17);
            let start = std::f32::consts::PI * 0.80;
            let sweep = std::f32::consts::PI * 1.40;
            for i in 0..=16 {
                let a = start + sweep * (i as f32 / 16.0);
                arc.push(Pos2::new(center.x + a.cos() * r, center.y + a.sin() * r));
            }
            painter.add(egui::Shape::line(arc, stroke));
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y + s * 0.15),
                    Pos2::new(center.x + s * 0.48, center.y - s * 0.42),
                ],
                stroke,
            );
            painter.circle_filled(Pos2::new(center.x, center.y + s * 0.15), 2.0, color);
        }
        NavIcon::Apps => {
            // Isometric layered cube / package box
            let top = Pos2::new(center.x, center.y - s);
            let right = Pos2::new(center.x + s * 0.92, center.y - s * 0.42);
            let mid = Pos2::new(center.x, center.y + s * 0.08);
            let left = Pos2::new(center.x - s * 0.92, center.y - s * 0.42);
            let bot = Pos2::new(center.x, center.y + s);
            let br = Pos2::new(center.x + s * 0.92, center.y + s * 0.48);
            let bl = Pos2::new(center.x - s * 0.92, center.y + s * 0.48);
            painter.add(egui::Shape::closed_line(vec![top, right, mid, left], stroke));
            painter.line_segment([left, bl], stroke);
            painter.line_segment([right, br], stroke);
            painter.line_segment([mid, bot], stroke);
            painter.line_segment([bl, bot], stroke);
            painter.line_segment([br, bot], stroke);
        }
        NavIcon::Gaming => {
            // Futuristic crosshair reticle
            let r = s * 0.72;
            painter.circle_stroke(center, r, stroke);
            painter.line_segment(
                [Pos2::new(center.x, center.y - s), Pos2::new(center.x, center.y - r * 0.55)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(center.x, center.y + r * 0.55), Pos2::new(center.x, center.y + s)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(center.x - s, center.y), Pos2::new(center.x - r * 0.55, center.y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(center.x + r * 0.55, center.y), Pos2::new(center.x + s, center.y)],
                stroke,
            );
            painter.circle_filled(center, 1.6, color);
        }
        NavIcon::Network => {
            // Connected signal pulse / globe nodes
            let p1 = Pos2::new(center.x - s * 0.85, center.y + s * 0.35);
            let p2 = Pos2::new(center.x - s * 0.25, center.y - s * 0.55);
            let p3 = Pos2::new(center.x + s * 0.30, center.y + s * 0.45);
            let p4 = Pos2::new(center.x + s * 0.88, center.y - s * 0.35);
            painter.add(egui::Shape::line(vec![p1, p2, p3, p4], stroke));
            for p in [p1, p2, p3, p4] {
                painter.circle_filled(p, 2.2, color);
            }
        }
        NavIcon::Safety => {
            // Clean geometric shield
            let pts = vec![
                Pos2::new(center.x, center.y - s),
                Pos2::new(center.x + s * 0.82, center.y - s * 0.58),
                Pos2::new(center.x + s * 0.72, center.y + s * 0.22),
                Pos2::new(center.x, center.y + s),
                Pos2::new(center.x - s * 0.72, center.y + s * 0.22),
                Pos2::new(center.x - s * 0.82, center.y - s * 0.58),
            ];
            painter.add(egui::Shape::closed_line(pts, stroke));
            // Checkmark inside
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(center.x - s * 0.30, center.y + s * 0.02),
                    Pos2::new(center.x - s * 0.05, center.y + s * 0.28),
                    Pos2::new(center.x + s * 0.36, center.y - s * 0.22),
                ],
                stroke,
            ));
        }
        NavIcon::Specs => {
            // Silicon die / CPU chip with pins
            let body = Rect::from_center_size(center, Vec2::splat(s * 1.25));
            painter.rect_stroke(body, CornerRadius::same(2), stroke, StrokeKind::Inside);
            let core = Rect::from_center_size(center, Vec2::splat(s * 0.55));
            painter.rect_filled(core, CornerRadius::same(1), color);
            for off in [-0.35_f32, 0.35] {
                let d = off * s;
                painter.line_segment(
                    [Pos2::new(center.x + d, body.top() - 2.5), Pos2::new(center.x + d, body.top())],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(center.x + d, body.bottom()), Pos2::new(center.x + d, body.bottom() + 2.5)],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(body.left() - 2.5, center.y + d), Pos2::new(body.left(), center.y + d)],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(body.right(), center.y + d), Pos2::new(body.right() + 2.5, center.y + d)],
                    stroke,
                );
            }
        }
        NavIcon::Settings => {
            // Precision hex-nut / gear ring
            let r_out = s * 0.88;
            let mut hex = Vec::with_capacity(6);
            for i in 0..6 {
                let a = (i as f32 / 6.0) * TAU - std::f32::consts::FRAC_PI_6;
                hex.push(Pos2::new(center.x + a.cos() * r_out, center.y + a.sin() * r_out));
            }
            painter.add(egui::Shape::closed_line(hex, stroke));
            painter.circle_stroke(center, s * 0.34, stroke);
        }
        NavIcon::Search => {
            let c = Pos2::new(center.x - s * 0.15, center.y - s * 0.15);
            let r = s * 0.62;
            painter.circle_stroke(c, r, stroke);
            painter.line_segment(
                [
                    Pos2::new(c.x + r * 0.72, c.y + r * 0.72),
                    Pos2::new(center.x + s * 0.85, center.y + s * 0.85),
                ],
                stroke,
            );
        }
        NavIcon::Refresh => {
            let r = s * 0.78;
            let mut arc = Vec::with_capacity(14);
            for i in 0..=13 {
                let a = -0.6 + (i as f32 / 13.0) * (TAU * 0.78);
                arc.push(Pos2::new(center.x + a.cos() * r, center.y + a.sin() * r));
            }
            painter.add(egui::Shape::line(arc, stroke));
            let tip = Pos2::new(center.x + (-0.6_f32).cos() * r, center.y + (-0.6_f32).sin() * r);
            painter.line_segment([tip, Pos2::new(tip.x - 4.0, tip.y - 1.0)], stroke);
            painter.line_segment([tip, Pos2::new(tip.x - 0.5, tip.y + 4.0)], stroke);
        }
        NavIcon::Update => {
            painter.line_segment(
                [Pos2::new(center.x, center.y + s * 0.55), Pos2::new(center.x, center.y - s * 0.75)],
                stroke,
            );
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(center.x - s * 0.50, center.y - s * 0.25),
                    Pos2::new(center.x, center.y - s * 0.78),
                    Pos2::new(center.x + s * 0.50, center.y - s * 0.25),
                ],
                stroke,
            ));
            painter.line_segment(
                [
                    Pos2::new(center.x - s * 0.72, center.y + s * 0.85),
                    Pos2::new(center.x + s * 0.72, center.y + s * 0.85),
                ],
                stroke,
            );
        }
        NavIcon::Terminal => {
            let box_r = Rect::from_center_size(center, Vec2::new(s * 1.85, s * 1.45));
            painter.rect_stroke(box_r, CornerRadius::same(3), stroke, StrokeKind::Inside);
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(center.x - s * 0.52, center.y - s * 0.28),
                    Pos2::new(center.x - s * 0.16, center.y + s * 0.04),
                    Pos2::new(center.x - s * 0.52, center.y + s * 0.36),
                ],
                stroke,
            ));
            painter.line_segment(
                [
                    Pos2::new(center.x + s * 0.05, center.y + s * 0.36),
                    Pos2::new(center.x + s * 0.52, center.y + s * 0.36),
                ],
                stroke,
            );
        }
        NavIcon::SidebarToggle => {
            let box_r = Rect::from_center_size(center, Vec2::new(s * 1.80, s * 1.45));
            painter.rect_stroke(box_r, CornerRadius::same(3), stroke, StrokeKind::Inside);
            let split_x = box_r.left() + box_r.width() * 0.36;
            painter.line_segment(
                [Pos2::new(split_x, box_r.top()), Pos2::new(split_x, box_r.bottom())],
                stroke,
            );
        }
    }
}

// ==================== BRAND WORDMARK ====================
/// Renders the signature **Optimizer** wordmark paired with smaller **by Raghav** subtitle.
pub fn brand_wordmark(ui: &mut egui::Ui, hero: bool) {
    let title_size = if hero { 25.0 } else { 16.5 };
    let byline_size = if hero { 12.0 } else { 10.0 };
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(
            RichText::new("Optimizer")
                .size(title_size)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.label(
            RichText::new("by Raghav")
                .size(byline_size)
                .color(theme::ACCENT_CYAN),
        );
    });
}

// ==================== SURFACES & CONTAINERS ====================
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_HAIRLINE))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(16))
}

pub fn bento_frame(accent_border: Option<Color32>) -> egui::Frame {
    let stroke_col = accent_border
        .map(|c| c.linear_multiply(0.45))
        .unwrap_or(theme::BORDER_HAIRLINE);
    egui::Frame::new()
        .fill(theme::BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, stroke_col))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(18))
}

pub fn sub_surface_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::BG_VOID)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_HAIRLINE))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 11))
}

/// Minimalist page header — crisp title + optional whisper-quiet 3-6 word caption.
pub fn section_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(
            RichText::new(title)
                .size(22.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        if !subtitle.is_empty() {
            ui.label(
                RichText::new(subtitle)
                    .size(12.2)
                    .color(theme::TEXT_MUTED),
            );
        }
    });
    ui.add_space(6.0);
}

// ==================== MODERN SEGMENTED PILLS & SWITCHES ====================
pub fn segmented_tabs<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(theme::BG_VOID)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_HAIRLINE))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                for &(val, label) in options {
                    let active = *current == val;
                    let fill = if active {
                        theme::BG_ELEVATED
                    } else {
                        Color32::TRANSPARENT
                    };
                    let stroke = if active {
                        Stroke::new(1.0_f32, theme::ACCENT_CYAN.linear_multiply(0.65))
                    } else {
                        Stroke::NONE
                    };
                    let fg = if active {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    };
                    let btn = egui::Button::new(RichText::new(label).size(12.5).strong().color(fg))
                        .fill(fill)
                        .stroke(stroke)
                        .corner_radius(CornerRadius::same(7))
                        .min_size(Vec2::new(72.0, 28.0));
                    if ui.add(btn).clicked() && !active {
                        *current = val;
                        changed = true;
                    }
                }
            });
        });
    changed
}

pub fn filter_chip(ui: &mut egui::Ui, label: &str, active: bool) -> Response {
    let (fill, fg, stroke) = if active {
        (
            theme::ACCENT_CYAN_DIM,
            theme::ACCENT_CYAN,
            Stroke::new(1.0_f32, theme::ACCENT_CYAN.linear_multiply(0.8)),
        )
    } else {
        (
            theme::BG_VOID,
            theme::TEXT_SECONDARY,
            Stroke::new(1.0_f32, theme::BORDER_HAIRLINE),
        )
    };
    ui.add(
        egui::Button::new(RichText::new(label).size(12.0).strong().color(fg))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(16))
            .min_size(Vec2::new(0.0, 27.0)),
    )
}

/// Animated iOS/Avionics style pill toggle switch (`40x22`) replacing native square checkboxes.
pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> Response {
    let desired_size = Vec2::new(38.0, 21.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
    let painter = ui.painter();

    let off_bg = theme::BG_VOID;
    let on_bg = theme::ACCENT_CYAN;
    let bg = lerp_color(off_bg, on_bg, how_on);
    let border = if response.hovered() {
        theme::TEXT_PRIMARY
    } else if *on {
        theme::ACCENT_CYAN
    } else {
        theme::BORDER_STRONG
    };

    let radius = (rect.height() * 0.5) as u8;
    painter.rect_filled(rect, CornerRadius::same(radius), bg);
    painter.rect_stroke(
        rect,
        CornerRadius::same(radius),
        Stroke::new(1.0_f32, border),
        StrokeKind::Inside,
    );

    let pad = 3.0;
    let knob_r = rect.height() * 0.5 - pad;
    let x_off = rect.left() + pad + knob_r;
    let x_on = rect.right() - pad - knob_r;
    let cx = egui::lerp(x_off..=x_on, how_on);
    let knob_col = if *on {
        theme::BG_VOID
    } else {
        theme::TEXT_SECONDARY
    };
    painter.circle_filled(Pos2::new(cx, rect.center().y), knob_r, knob_col);

    response
}

/// Interactive toggle switch for stateless actions (renders `is_on` and returns `true` when clicked).
pub fn action_switch(ui: &mut egui::Ui, id_salt: impl std::hash::Hash, is_on: bool) -> Response {
    let desired_size = Vec2::new(40.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
    let anim_id = ui.make_persistent_id(id_salt);
    let how_on = ui.ctx().animate_bool_responsive(anim_id, is_on);

    let bg = lerp_color(theme::BG_VOID, theme::ACCENT_EMERALD, how_on);
    let border = if response.hovered() {
        theme::ACCENT_CYAN
    } else if is_on {
        theme::ACCENT_EMERALD
    } else {
        theme::BORDER_STRONG
    };

    let p = ui.painter();
    let r = (rect.height() * 0.5) as u8;
    p.rect_filled(rect, CornerRadius::same(r), bg);
    p.rect_stroke(
        rect,
        CornerRadius::same(r),
        Stroke::new(1.0_f32, border),
        StrokeKind::Inside,
    );

    let pad = 3.0;
    let knob_r = rect.height() * 0.5 - pad;
    let cx = egui::lerp((rect.left() + pad + knob_r)..=(rect.right() - pad - knob_r), how_on);
    let knob_col = if is_on {
        theme::BG_VOID
    } else {
        theme::TEXT_SECONDARY
    };
    p.circle_filled(Pos2::new(cx, rect.center().y), knob_r, knob_col);

    response
}

/// Sleek search input bar with vector magnifying glass and inline clear button.
pub fn search_bar(ui: &mut egui::Ui, query: &mut String, hint: &str, width: f32) {
    egui::Frame::new()
        .fill(theme::BG_VOID)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.set_width(width.min(ui.available_width()));
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), Sense::hover());
                paint_icon(
                    ui.painter(),
                    icon_rect.center(),
                    13.0,
                    NavIcon::Search,
                    theme::TEXT_MUTED,
                );
                let edit_w = (ui.available_width() - if query.is_empty() { 4.0 } else { 24.0 }).max(60.0);
                ui.add_sized(
                    [edit_w, 20.0],
                    egui::TextEdit::singleline(query)
                        .frame(false)
                        .hint_text(RichText::new(hint).color(theme::TEXT_MUTED)),
                );
                if !query.is_empty()
                    && ui
                        .add(
                            egui::Button::new(RichText::new("×").size(14.0).color(theme::TEXT_MUTED))
                                .frame(false),
                        )
                        .clicked()
                {
                    query.clear();
                }
            });
        });
}

// ==================== RADIAL ARC RING GAUGE ====================
/// Futuristic radial arc gauge (`240°` sweep) with glowing active arc, big numeric readout, and label.
pub fn arc_gauge(
    ui: &mut egui::Ui,
    fraction: f32,
    center_value: &str,
    title: &str,
    sub_value: &str,
    accent: Color32,
    diameter: f32,
) {
    let clamped = fraction.clamp(0.0, 1.0);
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
        let p = ui.painter();
        let center = rect.center();
        let radius = diameter * 0.5 - 7.0;

        let start_angle = std::f32::consts::PI * 0.75; // 135 deg
        let total_sweep = std::f32::consts::PI * 1.50; // 270 deg
        let steps = 40usize;

        // Track arc
        let track_pts: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = start_angle + total_sweep * (i as f32 / steps as f32);
                Pos2::new(center.x + a.cos() * radius, center.y + a.sin() * radius)
            })
            .collect();
        p.add(egui::Shape::line(
            track_pts,
            Stroke::new(6.5_f32, theme::BG_ELEVATED),
        ));

        // Active glowing arc
        if clamped > 0.005 {
            let active_steps = ((steps as f32 * clamped).ceil() as usize).max(2);
            let active_pts: Vec<Pos2> = (0..=active_steps)
                .map(|i| {
                    let t = (i as f32 / active_steps as f32) * clamped;
                    let a = start_angle + total_sweep * t;
                    Pos2::new(center.x + a.cos() * radius, center.y + a.sin() * radius)
                })
                .collect();

            // Soft outer halo
            p.add(egui::Shape::line(
                active_pts.clone(),
                Stroke::new(
                    11.0_f32,
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 28),
                ),
            ));
            p.add(egui::Shape::line(active_pts, Stroke::new(6.5_f32, accent)));
        }

        // Center metric readout
        p.text(
            Pos2::new(center.x, center.y - 5.0),
            egui::Align2::CENTER_CENTER,
            center_value,
            FontId::new(diameter * 0.22, FontFamily::Monospace),
            theme::TEXT_PRIMARY,
        );
        p.text(
            Pos2::new(center.x, center.y + diameter * 0.16),
            egui::Align2::CENTER_CENTER,
            title.to_ascii_uppercase(),
            FontId::new(10.0, FontFamily::Proportional),
            theme::TEXT_MUTED,
        );

        if !sub_value.is_empty() {
            ui.add_space(2.0);
            ui.label(
                RichText::new(sub_value)
                    .size(11.2)
                    .monospace()
                    .color(theme::TEXT_SECONDARY),
            );
        }
    });
}

// ==================== METRIC CARDS, BADGES & BUTTONS ====================
pub fn metric_card(
    ui: &mut egui::Ui,
    min_width: f32,
    label: &str,
    value: &str,
    sub_detail: &str,
    accent: Color32,
) {
    card_frame().show(ui, |ui| {
        ui.set_min_width(min_width);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.horizontal(|ui| {
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 3.2, accent);
                ui.label(
                    RichText::new(label.to_ascii_uppercase())
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
            });
            ui.add(
                egui::Label::new(
                    RichText::new(value)
                        .size(15.5)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                )
                .truncate(),
            );
            if !sub_detail.is_empty() {
                ui.add(
                    egui::Label::new(
                        RichText::new(sub_detail)
                            .size(11.2)
                            .monospace()
                            .color(theme::TEXT_SECONDARY),
                    )
                    .truncate(),
                );
            }
        });
    });
}

pub fn status_badge(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, fg.linear_multiply(0.40)))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.5).strong().color(fg));
        });
}

pub fn dot_pill(ui: &mut egui::Ui, label: &str, active: bool) {
    let (fg, bg) = if active {
        (theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM)
    } else {
        (theme::TEXT_MUTED, theme::BG_VOID)
    };
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, fg.linear_multiply(0.35)))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (r, _) = ui.allocate_exact_size(Vec2::splat(6.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 2.8, fg);
                ui.label(RichText::new(label).size(11.0).strong().color(fg));
            });
        });
}

pub fn primary_button(ui: &mut egui::Ui, label: &str) -> Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(12.8)
                .strong()
                .color(theme::BG_VOID),
        )
        .fill(theme::ACCENT_CYAN)
        .stroke(Stroke::new(1.0_f32, theme::ACCENT_CYAN))
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(0.0, 31.0)),
    )
}

pub fn emerald_button(ui: &mut egui::Ui, label: &str) -> Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(12.8)
                .strong()
                .color(Color32::from_rgb(4, 22, 16)),
        )
        .fill(theme::ACCENT_EMERALD)
        .stroke(Stroke::new(1.0_f32, theme::ACCENT_EMERALD))
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(0.0, 31.0)),
    )
}

pub fn danger_button(ui: &mut egui::Ui, label: &str) -> Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(12.5)
                .strong()
                .color(theme::ACCENT_ROSE),
        )
        .fill(theme::ACCENT_ROSE_DIM)
        .stroke(Stroke::new(1.0_f32, theme::ACCENT_ROSE.linear_multiply(0.75)))
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(0.0, 30.0)),
    )
}

pub fn subtle_button(ui: &mut egui::Ui, label: &str) -> Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(12.5)
                .color(theme::TEXT_SECONDARY),
        )
        .fill(theme::BG_ELEVATED)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(0.0, 30.0)),
    )
}

pub fn icon_button(
    ui: &mut egui::Ui,
    icon: NavIcon,
    tooltip: &str,
    active: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    let fill = if active {
        theme::ACCENT_CYAN_DIM
    } else if resp.hovered() {
        theme::BG_HOVER
    } else {
        Color32::TRANSPARENT
    };
    let fg = if active || resp.hovered() {
        theme::ACCENT_CYAN
    } else {
        theme::TEXT_SECONDARY
    };
    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(7), fill);
    }
    paint_icon(ui.painter(), rect.center(), 15.0, icon, fg);
    resp.on_hover_text(tooltip)
}

pub fn kv_row(ui: &mut egui::Ui, key: &str, val: &str, mono: bool) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [115.0, 18.0],
            egui::Label::new(RichText::new(key).size(12.0).color(theme::TEXT_MUTED)),
        );
        let rt = if mono {
            RichText::new(val)
                .size(12.2)
                .monospace()
                .color(theme::TEXT_PRIMARY)
        } else {
            RichText::new(val).size(12.2).color(theme::TEXT_PRIMARY)
        };
        ui.add(egui::Label::new(rt).truncate());
    });
}

/// Sleek hairline progress bar (`8px` bar + crisp top-right readout when overlay text provided).
pub fn progress_bar(ui: &mut egui::Ui, fraction: f32, overlay_text: &str, accent: Color32) {
    let clamped = fraction.clamp(0.0, 1.0);
    if !overlay_text.is_empty() {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(overlay_text)
                        .size(11.0)
                        .monospace()
                        .color(theme::TEXT_SECONDARY),
                );
            });
        });
    }
    let desired = Vec2::new(ui.available_width().max(80.0), 7.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(4), theme::BG_VOID);
    if clamped > 0.0 {
        let fill_rect =
            Rect::from_min_size(rect.min, Vec2::new((rect.width() * clamped).max(5.0), rect.height()));
        p.rect_filled(fill_rect, CornerRadius::same(4), accent);
    }
}

/// Minimalist real-time telemetry waveform card.
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
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(
                    RichText::new(title.to_ascii_uppercase())
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                if !subtitle.is_empty() {
                    ui.add(
                        egui::Label::new(
                            RichText::new(subtitle)
                                .size(11.5)
                                .color(theme::TEXT_SECONDARY),
                        )
                        .truncate(),
                    );
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(current_value)
                        .size(20.0)
                        .strong()
                        .monospace()
                        .color(accent),
                );
            });
        });
        ui.add_space(6.0);

        let width = ui.available_width().max(120.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, chart_height), Sense::hover());
        let p = ui.painter();

        p.rect_filled(rect, CornerRadius::same(8), theme::BG_VOID);
        let mid_y = rect.center().y;
        p.line_segment(
            [Pos2::new(rect.left() + 4.0, mid_y), Pos2::new(rect.right() - 4.0, mid_y)],
            Stroke::new(1.0_f32, theme::BORDER_HAIRLINE),
        );

        if samples.len() >= 2 {
            let n = samples.len();
            let dx = rect.width() / (n.saturating_sub(1).max(1) as f32);
            let inner_top = rect.top() + 6.0;
            let inner_h = (rect.height() - 12.0).max(8.0);

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

            let fill_color = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 28);
            for w in points.windows(2) {
                let poly = vec![
                    w[0],
                    w[1],
                    Pos2::new(w[1].x, rect.bottom() - 1.0),
                    Pos2::new(w[0].x, rect.bottom() - 1.0),
                ];
                p.add(egui::Shape::convex_polygon(poly, fill_color, Stroke::NONE));
            }
            p.add(egui::Shape::line(points.clone(), Stroke::new(1.8_f32, accent)));
            if let Some(&last) = points.last() {
                p.circle_filled(
                    last,
                    5.5,
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 55),
                );
                p.circle_filled(last, 2.8, accent);
            }
        }
    });
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let r = (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8;
    let g = (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8;
    let bl = (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8;
    Color32::from_rgb(r, g, bl)
}
