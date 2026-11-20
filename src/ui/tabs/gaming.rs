use crate::engine::format_bytes;
use crate::models::{ValorantInstallStatus, ValorantPresetKind};
use crate::ui::components::{
    card_frame, danger_button, emerald_button, kv_row, primary_button, section_header,
    status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, ConfirmRequest, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    section_header(
        ui,
        "Gaming & Extreme Android Emulator Lab",
        "Strict anti-cheat safety (zero Vanguard/EAC/BattlEye process handle probing), UTF-8 no-BOM INI patching, and Extreme BlueStacks/MSI/LDPlayer tuning.",
    );

    // ==================== 1. ACTIVE GAME MODE DAEMON ====================
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Active Game Mode Daemon (Vanguard / EAC / BattlEye Safe)")
                    .size(15.5)
                    .strong()
                    .color(theme::ACCENT_EMERALD),
            );
            if state.gamemode_status.daemon_running {
                status_badge(ui, "DAEMON RUNNING", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
            } else {
                status_badge(ui, "STOPPED", theme::TEXT_MUTED, theme::BG_ELEVATED);
            }
            status_badge(
                ui,
                &format!("Live Timer: {:.2} ms", state.gamemode_status.current_timer_ms),
                theme::ACCENT_COBALT,
                theme::BG_ELEVATED,
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if state.gamemode_status.daemon_running {
                    if danger_button(ui, "Stop Daemon & Revert").clicked() {
                        actions.push(AppAction::ToggleGameMode(false));
                    }
                } else if emerald_button(ui, "Start Game Mode Daemon").clicked() {
                    actions.push(AppAction::ToggleGameMode(true));
                }
            });
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("Status: {}", state.gamemode_status.summary))
                    .size(12.5)
                    .color(theme::TEXT_PRIMARY),
            );
            if let Some(mask_hex) = &state.gamemode_status.affinity_mask_hex {
                status_badge(ui, &format!("Affinity: {mask_hex}"), theme::ACCENT_GOLD, theme::BG_ELEVATED);
            }
            if state.gamemode_status.is_anticheat_protected {
                status_badge(ui, "ANTI-CHEAT PROTECTED (OS-ONLY)", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
            }
        });

        if !state.gamemode_status.interventions.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Active Interventions:").size(11.5).color(theme::TEXT_MUTED));
                for item in &state.gamemode_status.interventions {
                    status_badge(ui, item, theme::ACCENT_CYAN_SLATE, theme::BG_ELEVATED);
                }
            });
        }

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        let mut cfg_changed = false;
        ui.horizontal_wrapped(|ui| {
            cfg_changed |= ui
                .checkbox(&mut state.gamemode_cfg.enable_p_core_pinning, "Pin Standard Games to P-Cores / X3D CCD0")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut state.gamemode_cfg.enable_high_priority, "High CPU Priority (Standard Only)")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut state.gamemode_cfg.enable_power_scheme, "Auto Performance Power Plan")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut state.gamemode_cfg.enable_1ms_timer, "1.0 ms RAII Multimedia Timer")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut state.gamemode_cfg.enable_background_throttle, "Throttle Browsers/Launchers During Game")
                .changed();
        });
        if cfg_changed {
            actions.push(AppAction::SaveGameModeConfig);
        }

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Custom Game EXEs:").size(12.0).color(theme::TEXT_MUTED));
            ui.add_sized(
                [180.0, 24.0],
                egui::TextEdit::singleline(&mut state.gamemode_custom_input)
                    .hint_text("e.g. cyberpunk2077.exe"),
            );
            if subtle_button(ui, "Add Target").clicked() {
                let txt = state.gamemode_custom_input.trim().to_string();
                if !txt.is_empty() {
                    actions.push(AppAction::AddGameModeTarget(txt));
                    state.gamemode_custom_input.clear();
                }
            }
            for target in &state.gamemode_cfg.custom_targets {
                if ui
                    .small_button(format!("{target} ×"))
                    .on_hover_text("Remove custom target")
                    .clicked()
                {
                    actions.push(AppAction::RemoveGameModeTarget(target.clone()));
                }
            }
        });
    });

    ui.add_space(10.0);

    // ==================== 2. VALORANT + 3. EXTREME ANDROID EMULATORS SIDE-BY-SIDE ====================
    ui.columns(2, |cols| {
        // ---- VALORANT CARD ----
        card_frame().show(&mut cols[0], |ui| {
            let v = &state.valorant_topo;
            ui.horizontal(|ui| {
                ui.label(RichText::new("VALORANT Optimizer").size(15.5).strong().color(theme::ACCENT_ROSE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Refresh Audit").clicked() {
                        actions.push(AppAction::RefreshValorant);
                    }
                    let (lbl, fg, bg) = match v.install_status {
                        ValorantInstallStatus::Running => ("RUNNING", theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM),
                        ValorantInstallStatus::InstalledNotRunning => ("INSTALLED", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM),
                        ValorantInstallStatus::NotInstalled => ("NOT FOUND", theme::TEXT_MUTED, theme::BG_ELEVATED),
                    };
                    status_badge(ui, lbl, fg, bg);
                });
            });
            ui.add_space(4.0);
            kv_row(
                ui,
                "Install Path",
                v.install_path.as_deref().unwrap_or("Not detected"),
                true,
            );
            kv_row(
                ui,
                "Active INI",
                v.active_config_path.as_deref().unwrap_or("None found (run Valorant once)"),
                true,
            );
            kv_row(
                ui,
                "Render Quality / VSync",
                &format!("{}% | VSync {}", v.resolution_quality, if v.vsync_disabled { "OFF" } else { "ON" }),
                true,
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                bool_pill(ui, "FSO/DPI", v.fso_disabled && v.dpi_aware);
                bool_pill(ui, "QoS DSCP 46", v.qos_policy_exists);
                bool_pill(ui, "GameDVR Off", v.game_dvr_disabled);
                bool_pill(ui, "Nagle Off", v.nagle_disabled);
                bool_pill(ui, "MMCSS", v.mmcss_optimized);
                bool_pill(ui, "Raw Mouse", v.mouse_accel_disabled);
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if primary_button(ui, "Apply Vanguard-Safe Bundle").clicked() {
                    actions.push(AppAction::ApplyValorantBundle);
                }
                if subtle_button(ui, if v.fso_disabled { "Clear FSO/DPI" } else { "Set FSO/DPI" }).clicked() {
                    actions.push(AppAction::SetValorantFsoDpi(!v.fso_disabled));
                }
                if subtle_button(ui, if v.qos_policy_exists { "Remove QoS 46" } else { "Add QoS DSCP 46" }).clicked() {
                    actions.push(AppAction::SetValorantQos(!v.qos_policy_exists));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("INI Presets (No-BOM):").size(12.0).color(theme::TEXT_MUTED));
                if subtle_button(ui, "Competitive 75%").clicked() {
                    actions.push(AppAction::ApplyValorantPreset(ValorantPresetKind::CompetitivePotato));
                }
                if subtle_button(ui, "Tournament 240Hz").clicked() {
                    actions.push(AppAction::ApplyValorantPreset(ValorantPresetKind::Tournament240Hz));
                }
                if subtle_button(ui, "Laptop Balanced").clicked() {
                    actions.push(AppAction::ApplyValorantPreset(ValorantPresetKind::LaptopBalanced));
                }
            });
            ui.add_space(4.0);
            let val_cache = v.shader_cache_bytes + v.log_cache_bytes + v.crash_cache_bytes;
            ui.horizontal_wrapped(|ui| {
                if danger_button(
                    ui,
                    &format!("Purge Safe Caches ({})", format_bytes(val_cache)),
                )
                .clicked()
                {
                    actions.push(AppAction::PurgeValorantCaches);
                }
                if primary_button(ui, "Ping Riot Regions").clicked() {
                    actions.push(AppAction::PingRiotRegions);
                }
            });

            if !state.riot_pings.is_empty() {
                ui.add_space(6.0);
                egui::Grid::new("riot_ping_grid").striped(true).num_columns(3).show(ui, |ui| {
                    ui.label(RichText::new("Shard Region").size(11.5).strong().color(theme::TEXT_MUTED));
                    ui.label(RichText::new("RTT").size(11.5).strong().color(theme::TEXT_MUTED));
                    ui.label(RichText::new("Method").size(11.5).strong().color(theme::TEXT_MUTED));
                    ui.end_row();
                    for p in &state.riot_pings {
                        ui.label(RichText::new(&p.region).size(11.5));
                        let (txt, c) = match p.rtt_ms {
                            Some(ms) if ms <= 45 => (format!("{ms} ms"), theme::ACCENT_EMERALD),
                            Some(ms) if ms <= 100 => (format!("{ms} ms"), theme::ACCENT_AMBER),
                            Some(ms) => (format!("{ms} ms"), theme::ACCENT_ROSE),
                            None => ("Timeout".to_string(), theme::TEXT_MUTED),
                        };
                        ui.label(RichText::new(txt).monospace().size(11.5).color(c));
                        ui.label(RichText::new(&p.method).monospace().size(11.0).color(theme::TEXT_SECONDARY));
                        ui.end_row();
                    }
                });
            }
        });

        // ---- EXTREME ANDROID EMULATORS CARD ----
        card_frame().show(&mut cols[1], |ui| {
            let e = &state.emulator_topo;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Extreme Android Emulators Lab")
                        .size(15.5)
                        .strong()
                        .color(theme::ACCENT_GOLD),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Refresh").clicked() {
                        actions.push(AppAction::RefreshEmulators);
                    }
                    let count = usize::from(e.bluestacks_installed)
                        + usize::from(e.msi_installed)
                        + usize::from(e.ldplayer_installed);
                    status_badge(
                        ui,
                        &format!("{count} Detected"),
                        if count > 0 { theme::ACCENT_EMERALD } else { theme::TEXT_MUTED },
                        theme::BG_ELEVATED,
                    );
                });
            });
            ui.add_space(4.0);
            kv_row(
                ui,
                "BlueStacks 5",
                e.bluestacks_version.as_deref().unwrap_or(if e.bluestacks_installed { "Installed" } else { "Not detected" }),
                true,
            );
            kv_row(
                ui,
                "MSI App Player",
                e.msi_version.as_deref().unwrap_or(if e.msi_installed { "Installed" } else { "Not detected" }),
                true,
            );
            kv_row(
                ui,
                "LDPlayer 9",
                &if e.ldplayer_installed {
                    format!(
                        "{} ({} VM configs)",
                        e.ldplayer_exe.as_deref().unwrap_or("Installed"),
                        e.ldplayer_configs.len()
                    )
                } else {
                    "Not detected".to_string()
                },
                true,
            );
            kv_row(
                ui,
                "BS/MSI Instances",
                &if e.instances_found.is_empty() {
                    "None".to_string()
                } else {
                    e.instances_found.join(", ")
                },
                true,
            );
            kv_row(ui, "Virtualization", &e.hypervisor_status, true);

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                bool_pill(ui, "240 FPS", e.fps_240_unlocked);
                bool_pill(ui, "ROG ASUS_I005DA", e.rog_profile_active);
                bool_pill(ui, "ASTC + AdBlock", e.astc_and_debloat_active);
                bool_pill(ui, "IFEO High I/O+CPU", e.ifeo_priority_locked);
                bool_pill(ui, "dGPU Locked", e.gpu_locked);
                bool_pill(ui, "Raw Mouse", e.raw_mouse_active);
                bool_pill(ui, "Low-Lat Net", e.low_latency_net_active);
            });

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if emerald_button(ui, "Extreme Patch BS/MSI (240FPS + ROG + ASTC + 4GB+ + AdBlock)").clicked() {
                    actions.push(AppAction::PatchEmulatorConfigs {
                        fps_240: true,
                        rog_profile: true,
                    });
                }
                if !e.ldplayer_configs.is_empty() && primary_button(ui, "Extreme Patch LDPlayer 9 (240FPS + ROG)").clicked() {
                    actions.push(AppAction::PatchLdPlayerConfigs);
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if primary_button(
                    ui,
                    if e.ifeo_priority_locked {
                        "Unlock Kernel IFEO Priority"
                    } else {
                        "Lock Kernel IFEO High CPU & I/O Priority"
                    },
                )
                .clicked()
                {
                    actions.push(AppAction::SetEmulatorIfeoPriority(!e.ifeo_priority_locked));
                }
                if subtle_button(ui, "Lock dGPU (Existing EXEs)").clicked() {
                    actions.push(AppAction::LockEmulatorGpu);
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if subtle_button(ui, if e.raw_mouse_active { "Restore Default Mouse" } else { "Enable 1:1 Raw Aim" }).clicked() {
                    actions.push(AppAction::SetEmulatorRawMouse(!e.raw_mouse_active));
                }
                if subtle_button(ui, if e.low_latency_net_active { "Revert Low-Lat Net" } else { "Enable Low-Lat Net" }).clicked() {
                    actions.push(AppAction::SetEmulatorLowLatencyNet(!e.low_latency_net_active));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                let target_off = !e.hypervisor_off;
                let lbl = if target_off {
                    "Disable Hypervisor & HVCI (Raw VT-x Speed)"
                } else {
                    "Re-Enable Hypervisor & HVCI (BCD Auto)"
                };
                if danger_button(ui, lbl).clicked() {
                    actions.push(AppAction::RequestConfirm(ConfirmRequest {
                        title: if target_off {
                            "Disable Windows Hypervisor & Core Isolation (HVCI)?"
                        } else {
                            "Re-enable Windows Hypervisor & Core Isolation?"
                        }.into(),
                        message: if target_off {
                            "Sets `bcdedit /set hypervisorlaunchtype off` and disables HVCI/VBS so BlueStacks, MSI App Player, and LDPlayer run directly on raw hardware VT-x/AMD-V without Hyper-V emulation overhead.\n\nNote: WSL2 and Hyper-V VMs require Auto. Reboot required.".into()
                        } else {
                            "Restores `bcdedit /set hypervisorlaunchtype auto` and HVCI for WSL2, Hyper-V, and VBS. Requires reboot.".into()
                        },
                        confirm_label: "Apply Virtualization Change".into(),
                        action: Box::new(AppAction::SetHypervisorState { turn_off: target_off }),
                    }));
                }
                if subtle_button(ui, &format!("Clean Logs ({})", format_bytes(e.cache_bytes))).clicked() {
                    actions.push(AppAction::PurgeEmulatorLogs);
                }
            });
        });
    });
}

fn bool_pill(ui: &mut egui::Ui, label: &str, active: bool) {
    if active {
        status_badge(ui, &format!("{label}: ON"), theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
    } else {
        status_badge(ui, &format!("{label}: OFF"), theme::TEXT_MUTED, theme::BG_ELEVATED);
    }
}
