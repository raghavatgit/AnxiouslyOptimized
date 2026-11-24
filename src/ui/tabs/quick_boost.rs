use crate::engine::tweaks::preset_ids;
use crate::models::PresetId;
use crate::ui::components::{
    bento_frame, dot_pill, emerald_button, primary_button, progress_bar, section_header,
    status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, TabId, UiState};
use egui::{RichText, Vec2};

pub fn render(ui: &mut egui::Ui, state: &UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(ui, "Boost Profiles", "One-click atomic optimization bundles.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            dot_pill(
                ui,
                if state.settings.auto_restore_point_before_preset {
                    "Auto Restore Point"
                } else {
                    "Restore Point Off"
                },
                state.settings.auto_restore_point_before_preset,
            );
            if subtle_button(ui, "Rollbacks").clicked() {
                actions.push(AppAction::SwitchTab(TabId::Safety));
            }
        });
    });

    let presets = [
        (
            PresetId::EverydayBoost,
            "Everyday",
            "Snappy UI, zero telemetry clutter & fast NTFS.",
            &["Snappy UI", "Privacy", "NTFS"],
            theme::ACCENT_COBALT,
        ),
        (
            PresetId::GamingRig,
            "Gaming",
            "HAGS, 1:1 raw mouse, unparked cores & GameDVR off.",
            &["HAGS", "Raw Input", "No GameDVR"],
            theme::ACCENT_EMERALD,
        ),
        (
            PresetId::CompetitiveLowLatency,
            "Competitive",
            "PCI MSI interrupts, Win32 quantum 0x26 & TCP NoDelay.",
            &["GPU MSI", "0x26 Quantum", "Zero Throttle"],
            theme::ACCENT_GOLD,
        ),
        (
            PresetId::ExtremeWin11,
            "Extreme Win11",
            "Disables VBS/HVCI, Recall, Widgets & enables Flip-Model VRR.",
            &["No VBS", "Flip-Model", "SvcHost Merge"],
            theme::ACCENT_CYAN,
        ),
    ];

    let wide = ui.available_width() >= 760.0;
    let cols_count = if wide { 2 } else { 1 };

    ui.columns(cols_count, |cols| {
        for (i, (id, title, tagline, pills, accent)) in presets.into_iter().enumerate() {
            let col_ui = &mut cols[i % cols_count];
            let ids = preset_ids(id);
            let matching: Vec<_> = state
                .tweaks
                .iter()
                .filter(|t| ids.contains(&t.id.as_str()))
                .collect();
            let applied = matching.iter().filter(|t| t.is_applied).count();
            let total = matching.len().max(1);
            let frac = applied as f32 / total as f32;
            let all_active = applied == matching.len() && !matching.is_empty();

            bento_frame(if all_active { Some(accent) } else { None }).show(col_ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).size(18.0).strong().color(theme::TEXT_PRIMARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_badge(
                            ui,
                            &format!("{applied}/{total}"),
                            if all_active { theme::ACCENT_EMERALD } else { accent },
                            theme::BG_VOID,
                        );
                    });
                });

                ui.add_space(2.0);
                ui.label(RichText::new(tagline).size(12.2).color(theme::TEXT_SECONDARY));
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    for p in pills {
                        status_badge(ui, p, theme::TEXT_SECONDARY, theme::BG_VOID);
                    }
                });

                ui.add_space(10.0);
                progress_bar(ui, frac, "", accent);
                ui.add_space(10.0);

                let btn_label = if all_active {
                    format!("Re-Apply {title}")
                } else {
                    format!("Activate {title}")
                };
                let btn = if matches!(id, PresetId::ExtremeWin11) {
                    primary_button(ui, &btn_label)
                } else if matches!(id, PresetId::GamingRig) {
                    emerald_button(ui, &btn_label)
                } else {
                    ui.add(
                        egui::Button::new(
                            RichText::new(&btn_label)
                                .size(12.8)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        )
                        .fill(theme::BG_ELEVATED)
                        .stroke(egui::Stroke::new(1.0_f32, accent.linear_multiply(0.65)))
                        .min_size(Vec2::new(140.0, 32.0)),
                    )
                };
                if btn.clicked() {
                    actions.push(AppAction::ApplyPreset(id));
                }
            });
            col_ui.add_space(10.0);
        }
    });
}
