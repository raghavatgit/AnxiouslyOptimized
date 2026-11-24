use crate::engine::installer::default_install_dir;
use crate::engine::updater::APP_VERSION;
use crate::engine::{app_data_dir, journals_dir};
use crate::models::UpdateStatus;
use crate::ui::components::{
    bento_frame, brand_wordmark, card_frame, danger_button, dot_pill, emerald_button, filter_chip,
    kv_row, primary_button, section_header, subtle_button, toggle_switch,
};
use crate::ui::{theme, AppAction, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    section_header(ui, "Settings", "Scale, installation, updates & preferences.");

    // ==================== INTERFACE SCALING & APPEARANCE ====================
    bento_frame(Some(theme::ACCENT_CYAN)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(
                    RichText::new("Interface Scale")
                        .size(14.5)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new("Scale typography & cards smoothly (Ctrl + / Ctrl -)")
                        .size(11.8)
                        .color(theme::TEXT_SECONDARY),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for (scale, lbl) in [(1.25_f32, "125%"), (1.10, "110%"), (1.00, "100%"), (0.90, "90%")] {
                    let active = (state.settings.ui_scale - scale).abs() < 0.03;
                    if filter_chip(ui, lbl, active).clicked() {
                        actions.push(AppAction::SetUiScale(scale));
                    }
                }
            });
        });
    });

    ui.add_space(10.0);

    // ==================== INSTALLATION & UPDATES ====================
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("SYSTEM INSTALL")
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    dot_pill(
                        ui,
                        if state.is_installed_on_pc { "Installed" } else { "Portable" },
                        state.is_installed_on_pc,
                    );
                });
            });
            ui.add_space(6.0);
            let target_dir = default_install_dir().to_string_lossy().to_string();
            kv_row(ui, "Path", &target_dir, true);
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if emerald_button(
                    ui,
                    if state.is_installed_on_pc {
                        "Reinstall"
                    } else {
                        "Install to PC"
                    },
                )
                .clicked()
                {
                    actions.push(AppAction::InstallAppToSystem);
                }
                if state.is_installed_on_pc && danger_button(ui, "Uninstall").clicked() {
                    actions.push(AppAction::UninstallAppFromSystem);
                }
            });
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("UPDATES")
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    dot_pill(ui, &format!("v{APP_VERSION}"), true);
                });
            });
            ui.add_space(6.0);

            match &state.update_status {
                UpdateStatus::Idle => {
                    ui.label(RichText::new("GitHub release channel ready.").size(12.0).color(theme::TEXT_SECONDARY));
                }
                UpdateStatus::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Checking GitHub...").size(12.0).color(theme::ACCENT_CYAN));
                    });
                }
                UpdateStatus::UpToDate { version } => {
                    ui.label(RichText::new(format!("Up to date (v{version})")).size(12.2).color(theme::ACCENT_EMERALD));
                }
                UpdateStatus::Available(info) => {
                    ui.label(
                        RichText::new(format!("v{} available", info.latest_version))
                            .size(12.8)
                            .strong()
                            .color(theme::ACCENT_GOLD),
                    );
                }
                UpdateStatus::Error(err) => {
                    ui.label(RichText::new(err).size(11.8).color(theme::ACCENT_ROSE));
                }
            }

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if primary_button(ui, "Check Updates").clicked() {
                    actions.push(AppAction::CheckForUpdates);
                }
                if let UpdateStatus::Available(info) = &state.update_status {
                    if emerald_button(ui, &format!("Install v{}", info.latest_version)).clicked() {
                        actions.push(AppAction::InstallUpdateNow(info.setup_download_url.clone()));
                    }
                } else if subtle_button(ui, "GitHub").clicked() {
                    actions.push(AppAction::OpenUrl(
                        "https://github.com/raghavatgit/AnxiouslyOptimized/releases".to_string(),
                    ));
                }
            });
        });
    });

    ui.add_space(10.0);

    // ==================== PREFERENCES & ABOUT ====================
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("PREFERENCES")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(8.0);

            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Auto Restore Point before Boost").size(12.8));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    changed |= toggle_switch(ui, &mut state.settings.auto_restore_point_before_preset).changed();
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Compact Navigation Rail").size(12.8));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    changed |= toggle_switch(ui, &mut state.settings.sidebar_collapsed).changed();
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Show Activity Log Drawer").size(12.8));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    changed |= toggle_switch(ui, &mut state.settings.show_console_drawer).changed();
                });
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Temp Cooldown (h)").size(12.2).color(theme::TEXT_SECONDARY));
                changed |= ui
                    .add(egui::Slider::new(
                        &mut state.settings.temp_cleaner_min_age_hours,
                        0..=168,
                    ))
                    .changed();
            });

            if changed {
                actions.push(AppAction::SaveSettings);
            }

            ui.add_space(8.0);
            let app_dir = app_data_dir().to_string_lossy().to_string();
            let j_dir = journals_dir().to_string_lossy().to_string();
            ui.horizontal_wrapped(|ui| {
                if subtle_button(ui, "AppData").clicked() {
                    actions.push(AppAction::OpenFolder(app_dir));
                }
                if subtle_button(ui, "Journals").clicked() {
                    actions.push(AppAction::OpenFolder(j_dir));
                }
            });
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                if let Some(tex) = &state.logo_tex {
                    ui.add(
                        egui::Image::new(tex)
                            .fit_to_exact_size(egui::vec2(46.0, 46.0))
                            .corner_radius(egui::CornerRadius::same(10)),
                    );
                    ui.add_space(6.0);
                }
                brand_wordmark(ui, true);
            });
            ui.add_space(10.0);
            kv_row(ui, "Build", &format!("v{APP_VERSION} • Rust + Glow"), true);
            kv_row(ui, "Command", "Ctrl+K", true);
            kv_row(ui, "Purge RAM", "Ctrl+R", true);
            kv_row(ui, "Zoom", "Ctrl + / -", true);
        });
    });
}
