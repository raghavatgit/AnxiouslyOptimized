use crate::engine::installer::default_install_dir;
use crate::engine::updater::APP_VERSION;
use crate::engine::{app_data_dir, journals_dir};
use crate::models::UpdateStatus;
use crate::ui::components::{
    brand_wordmark, card_frame, danger_button, emerald_button, kv_row, primary_button,
    section_header, status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    section_header(
        ui,
        "Settings, Desktop Installation & Updates",
        "Manage system installation (Program Files & Shortcuts), check for GitHub releases, and configure app behavior.",
    );

    // Top Row: Desktop Installation + GitHub Update Checker
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Windows Desktop Installation")
                        .size(15.0)
                        .strong()
                        .color(theme::ACCENT_EMERALD),
                );
                if state.is_installed_on_pc {
                    status_badge(
                        ui,
                        "INSTALLED IN PROGRAM FILES",
                        theme::ACCENT_EMERALD,
                        theme::ACCENT_EMERALD_DIM,
                    );
                } else {
                    status_badge(
                        ui,
                        "PORTABLE MODE",
                        theme::ACCENT_GOLD,
                        theme::ACCENT_AMBER_DIM,
                    );
                }
            });
            ui.add_space(4.0);
            let target_dir = default_install_dir().to_string_lossy().to_string();
            kv_row(ui, "Install Path", &target_dir, true);
            ui.label(
                RichText::new(
                    "Installs Optimizer by Raghav into Program Files, adds Desktop & Start Menu shortcuts, and registers Windows Installed Apps (Add/Remove Programs).",
                )
                .size(12.0)
                .color(theme::TEXT_SECONDARY),
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if emerald_button(
                    ui,
                    if state.is_installed_on_pc {
                        "Reinstall / Update Installed App"
                    } else {
                        "Install App to PC (Program Files + Shortcuts)"
                    },
                )
                .clicked()
                {
                    actions.push(AppAction::InstallAppToSystem);
                }
                if state.is_installed_on_pc && danger_button(ui, "Uninstall from PC").clicked() {
                    actions.push(AppAction::UninstallAppFromSystem);
                }
            });
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Software Updates (GitHub Releases)")
                        .size(15.0)
                        .strong()
                        .color(theme::ACCENT_COBALT),
                );
                status_badge(
                    ui,
                    &format!("CURRENT: v{APP_VERSION}"),
                    theme::ACCENT_COBALT,
                    theme::ACCENT_COBALT_DIM,
                );
            });
            ui.add_space(4.0);

            match &state.update_status {
                UpdateStatus::Idle => {
                    ui.label(
                        RichText::new("Click 'Check for Updates' to query GitHub Releases for newer versions.")
                            .size(12.5)
                            .color(theme::TEXT_SECONDARY),
                    );
                }
                UpdateStatus::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            RichText::new("Querying GitHub Releases API...")
                                .size(12.5)
                                .color(theme::ACCENT_GOLD),
                        );
                    });
                }
                UpdateStatus::UpToDate { version } => {
                    ui.label(
                        RichText::new(format!(
                            "You are running the latest version (v{version})."
                        ))
                        .size(12.5)
                        .strong()
                        .color(theme::ACCENT_EMERALD),
                    );
                }
                UpdateStatus::Available(info) => {
                    ui.label(
                        RichText::new(format!(
                            "NEW RELEASE AVAILABLE: v{} (Current: v{})",
                            info.latest_version, info.current_version
                        ))
                        .size(13.0)
                        .strong()
                        .color(theme::ACCENT_GOLD),
                    );
                    ui.label(
                        RichText::new(&info.release_notes)
                            .size(11.5)
                            .color(theme::TEXT_SECONDARY),
                    );
                }
                UpdateStatus::Error(err) => {
                    ui.label(
                        RichText::new(format!("Update check error: {err}"))
                            .size(12.0)
                            .color(theme::ACCENT_ROSE),
                    );
                }
            }

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if primary_button(ui, "Check for Updates Now").clicked() {
                    actions.push(AppAction::CheckForUpdates);
                }
                if let UpdateStatus::Available(info) = &state.update_status {
                    if emerald_button(
                        ui,
                        &format!("Download & Install v{}", info.latest_version),
                    )
                    .clicked()
                    {
                        actions.push(AppAction::InstallUpdateNow(
                            info.setup_download_url.clone(),
                        ));
                    }
                    if subtle_button(ui, "View Release Notes").clicked() {
                        actions.push(AppAction::OpenUrl(info.release_url.clone()));
                    }
                } else if subtle_button(ui, "Open GitHub Releases").clicked() {
                    actions.push(AppAction::OpenUrl(
                        "https://github.com/raghavatgit/AnxiouslyOptimized/releases".to_string(),
                    ));
                }
            });
        });
    });

    ui.add_space(10.0);

    ui.columns(2, |cols| {
        // Settings Form
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("Behavior & Safety Preferences")
                    .size(15.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.add_space(6.0);

            let mut changed = false;
            changed |= ui
                .checkbox(
                    &mut state.settings.auto_restore_point_before_preset,
                    "Create System Restore Point before Quick Boost Presets",
                )
                .changed();

            changed |= ui
                .checkbox(
                    &mut state.settings.show_console_drawer,
                    "Show Activity Console Drawer at bottom of window",
                )
                .changed();

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Temp Cleaner Min File Age (hours):")
                        .color(theme::TEXT_SECONDARY),
                );
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

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(
                RichText::new("Local Storage & Privilege Status")
                    .size(14.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Process Privilege:")
                        .size(12.5)
                        .color(theme::TEXT_MUTED),
                );
                if state.is_elevated {
                    status_badge(
                        ui,
                        "ELEVATED ADMINISTRATOR",
                        theme::ACCENT_EMERALD,
                        theme::ACCENT_EMERALD_DIM,
                    );
                } else {
                    status_badge(
                        ui,
                        "STANDARD USER",
                        theme::ACCENT_AMBER,
                        theme::ACCENT_AMBER_DIM,
                    );
                }
            });

            let app_dir = app_data_dir().to_string_lossy().to_string();
            let j_dir = journals_dir().to_string_lossy().to_string();
            kv_row(ui, "AppData Dir", &app_dir, true);
            kv_row(ui, "Journals Dir", &j_dir, true);
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if subtle_button(ui, "Open AppData Folder").clicked() {
                    actions.push(AppAction::OpenFolder(app_dir));
                }
                if subtle_button(ui, "Open Journals Folder").clicked() {
                    actions.push(AppAction::OpenFolder(j_dir));
                }
                if primary_button(ui, "Save Settings").clicked() {
                    actions.push(AppAction::SaveSettings);
                }
            });
        });

        // Brand / About Card
        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                if let Some(tex) = &state.logo_tex {
                    ui.add(
                        egui::Image::new(tex)
                            .fit_to_exact_size(egui::vec2(52.0, 52.0))
                            .corner_radius(egui::CornerRadius::same(10)),
                    );
                }
                ui.add_space(8.0);
                brand_wordmark(ui, true);
            });
            ui.add_space(8.0);
            kv_row(
                ui,
                "Version",
                &format!("{APP_VERSION} (Pure Rust Native Edition)"),
                true,
            );
            kv_row(ui, "Author", "Raghav Goyal", false);
            kv_row(
                ui,
                "Renderer",
                "eframe / egui GPU (glow) + Win32 / DXGI / WMI",
                false,
            );
            kv_row(
                ui,
                "Hotkeys",
                "Ctrl+1..9 Tabs  |  Ctrl+R Mem Reduct  |  F5 Refresh",
                true,
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Built from scratch in Rust with 6-region Mem Reduct kernel memory cleaning, \
                     240 FPS virtualized Geek.exe Uninstaller & Force-Cleaner, 14-category Extreme PC Cleaner, \
                     Extreme Windows 11 & Android Emulator optimizations, typed LIFO rollback journals, \
                     and strict anti-cheat isolation.",
                )
                .size(12.2)
                .color(theme::TEXT_SECONDARY),
            );
        });
    });
}
