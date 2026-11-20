use crate::engine::tweaks::preset_ids;
use crate::models::PresetId;
use crate::ui::components::{
    card_frame, emerald_button, primary_button, section_header, status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, TabId, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &UiState, actions: &mut Vec<AppAction>) {
    section_header(
        ui,
        "Quick Boost & Extreme Windows 11 Presets",
        "Curated, OS-filtered optimization bundles backed by atomic undo journals.",
    );

    let presets = [
        (
            PresetId::EverydayBoost,
            "Everyday Responsiveness",
            "Balanced UI responsiveness, clean Windows 10/11 clutter, privacy hygiene, and NTFS last-access speedups without disabling balanced thermals.",
            theme::ACCENT_COBALT,
        ),
        (
            PresetId::GamingRig,
            "Gaming Rig Optimization",
            "Enables Game Mode, Hardware-Accelerated GPU Scheduling, 1:1 Raw Mouse Input, Performance Power Plan, disables Core Parking & GameDVR.",
            theme::ACCENT_EMERALD,
        ),
        (
            PresetId::CompetitiveLowLatency,
            "Competitive Low-Latency",
            "Full esports profile: Gaming Rig stack PLUS PCI GPU MSI-mode interrupts, MMCSS network un-throttling, Win32PrioritySeparation=0x26 & TCPNoDelay.",
            theme::ACCENT_GOLD,
        ),
        (
            PresetId::ExtremeWin11,
            "Extreme Windows 11 & eSports",
            "Maximum Windows 11 overhaul: disables Copilot/Recall, Widgets, VBS/HVCI, background UWP apps, power throttling, enables Windowed Flip-Model, Compact Explorer & SvcHost consolidation.",
            theme::ACCENT_ROSE,
        ),
    ];

    ui.columns(2, |cols| {
        for (i, (id, title, desc, accent)) in presets.into_iter().enumerate() {
            let ui = &mut cols[i % 2];
            let ids = preset_ids(id);
            let matching_tweaks: Vec<_> = state
                .tweaks
                .iter()
                .filter(|t| ids.contains(&t.id.as_str()))
                .collect();
            let applied = matching_tweaks.iter().filter(|t| t.is_applied).count();
            let total = matching_tweaks.len();

            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).size(15.5).strong().color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_badge(
                            ui,
                            &format!("{applied} / {total} Active"),
                            if applied == total && total > 0 {
                                theme::ACCENT_EMERALD
                            } else {
                                theme::TEXT_SECONDARY
                            },
                            theme::BG_ELEVATED,
                        );
                    });
                });
                ui.add_space(4.0);
                ui.label(RichText::new(desc).size(12.5).color(theme::TEXT_SECONDARY));
                ui.add_space(8.0);
                let btn = match id {
                    PresetId::EverydayBoost => primary_button(ui, "Apply Everyday Boost"),
                    PresetId::GamingRig => emerald_button(ui, "Apply Gaming Rig"),
                    PresetId::CompetitiveLowLatency => primary_button(ui, "Apply Competitive Profile"),
                    PresetId::ExtremeWin11 => emerald_button(ui, "Apply Extreme Win11 & eSports"),
                };
                if btn.clicked() {
                    actions.push(AppAction::ApplyPreset(id));
                }
            });
            ui.add_space(8.0);
        }
    });

    ui.add_space(6.0);

    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Direct Quick Actions").size(14.0).strong().color(theme::TEXT_PRIMARY));
                ui.label(
                    RichText::new(format!(
                        "Auto Restore Point before presets is currently: {}",
                        if state.settings.auto_restore_point_before_preset {
                            "ENABLED (recommended)"
                        } else {
                            "DISABLED"
                        }
                    ))
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if emerald_button(ui, "Mem Reduct Clean Now").clicked() {
                    actions.push(AppAction::RunMemReductClean);
                }
                if subtle_button(ui, "Open Geek Uninstaller").clicked() {
                    actions.push(AppAction::SwitchTab(TabId::Uninstaller));
                }
                if subtle_button(ui, "Inspect Safety Journals").clicked() {
                    actions.push(AppAction::SwitchTab(TabId::Safety));
                }
            });
        });
    });
}
