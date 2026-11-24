use crate::engine::format_bytes;
use crate::models::{ValorantInstallStatus, ValorantPresetKind};
use crate::ui::components::{
    action_switch, bento_frame, card_frame, danger_button, dot_pill, emerald_button, kv_row,
    primary_button, section_header, segmented_tabs, status_badge, subtle_button, toggle_switch,
};
use crate::ui::{theme, AppAction, ConfirmRequest, GamingSubView, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(ui, "Gaming Lab", "Anti-cheat safe scheduler, VALORANT & Android VMs.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            segmented_tabs(
                ui,
                &mut state.gaming_subview,
                &[
                    (GamingSubView::Engine, "Game Mode"),
                    (GamingSubView::Valorant, "VALORANT"),
                    (GamingSubView::Emulators, "Emulators"),
                ],
            );
        });
    });

    ui.add_space(4.0);

    match state.gaming_subview {
        GamingSubView::Engine => render_gamemode(ui, state, actions),
        GamingSubView::Valorant => render_valorant(ui, state, actions),
        GamingSubView::Emulators => render_emulators(ui, state, actions),
    }
}

fn render_gamemode(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    let running = state.gamemode_status.daemon_running;

    bento_frame(if running { Some(theme::ACCENT_EMERALD) } else { None }).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Active Game Mode Daemon")
                            .size(16.5)
                            .strong()
                            .color(theme::TEXT_PRIMARY),
                    );
                    dot_pill(ui, if running { "ACTIVE" } else { "STANDBY" }, running);
                });
                ui.label(
                    RichText::new(&state.gamemode_status.summary)
                        .size(12.2)
                        .color(theme::TEXT_SECONDARY),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action_switch(ui, "gm_master_switch", running).clicked() {
                    actions.push(AppAction::ToggleGameMode(!running));
                }
                status_badge(
                    ui,
                    &format!("{:.2} ms", state.gamemode_status.current_timer_ms),
                    theme::ACCENT_CYAN,
                    theme::BG_VOID,
                );
            });
        });
    });

    ui.add_space(10.0);

    let mut cfg_changed = false;
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("SCHEDULER & TIMER")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(6.0);
            let opts: [(&str, &mut bool); 5] = [
                ("Pin to P-Cores / X3D CCD0", &mut state.gamemode_cfg.enable_p_core_pinning),
                ("High CPU Priority", &mut state.gamemode_cfg.enable_high_priority),
                ("Performance Power Plan", &mut state.gamemode_cfg.enable_power_scheme),
                ("1.0 ms Multimedia Timer", &mut state.gamemode_cfg.enable_1ms_timer),
                ("Throttle Background Apps", &mut state.gamemode_cfg.enable_background_throttle),
            ];
            for (lbl, flag) in opts {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(lbl).size(12.8).color(theme::TEXT_PRIMARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        cfg_changed |= toggle_switch(ui, flag).changed();
                    });
                });
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.label(
                RichText::new("CUSTOM EXECUTABLES")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_sized(
                    [190.0, 28.0],
                    egui::TextEdit::singleline(&mut state.gamemode_custom_input)
                        .hint_text("cs2.exe, cyberpunk2077.exe"),
                );
                if primary_button(ui, "Add").clicked() {
                    let txt = state.gamemode_custom_input.trim().to_string();
                    if !txt.is_empty() {
                        actions.push(AppAction::AddGameModeTarget(txt));
                        state.gamemode_custom_input.clear();
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for target in &state.gamemode_cfg.custom_targets {
                    if subtle_button(ui, &format!("{target}  ×")).clicked() {
                        actions.push(AppAction::RemoveGameModeTarget(target.clone()));
                    }
                }
            });
        });
    });

    if cfg_changed {
        actions.push(AppAction::SaveGameModeConfig);
    }
}

fn render_valorant(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    let v = &state.valorant_topo;
    let detected = !matches!(v.install_status, ValorantInstallStatus::NotInstalled);

    bento_frame(Some(theme::ACCENT_ROSE)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("VALORANT")
                    .size(17.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            dot_pill(
                ui,
                match v.install_status {
                    ValorantInstallStatus::Running => "Running",
                    ValorantInstallStatus::InstalledNotRunning => "Installed",
                    ValorantInstallStatus::NotInstalled => "Not Installed",
                },
                detected,
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, "Apply Vanguard-Safe Bundle").clicked() {
                    actions.push(AppAction::ApplyValorantBundle);
                }
                if subtle_button(ui, "Refresh").clicked() {
                    actions.push(AppAction::RefreshValorant);
                }
            });
        });

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            dot_pill(ui, "FSO/DPI", v.fso_disabled && v.dpi_aware);
            dot_pill(ui, "QoS DSCP 46", v.qos_policy_exists);
            dot_pill(ui, "No GameDVR", v.game_dvr_disabled);
            dot_pill(ui, "TCP NoDelay", v.nagle_disabled);
            dot_pill(ui, "Raw Mouse", v.mouse_accel_disabled);
        });
    });

    ui.add_space(10.0);

    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("RENDER PRESETS & FLAGS")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(6.0);
            kv_row(
                ui,
                "Quality",
                &format!("{}%  •  VSync {}", v.resolution_quality, if v.vsync_disabled { "Off" } else { "On" }),
                true,
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if subtle_button(ui, "Competitive 75%").clicked() {
                    actions.push(AppAction::ApplyValorantPreset(ValorantPresetKind::CompetitivePotato));
                }
                if subtle_button(ui, "Tournament 240Hz").clicked() {
                    actions.push(AppAction::ApplyValorantPreset(ValorantPresetKind::Tournament240Hz));
                }
                if subtle_button(ui, "Balanced").clicked() {
                    actions.push(AppAction::ApplyValorantPreset(ValorantPresetKind::LaptopBalanced));
                }
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if subtle_button(ui, if v.fso_disabled { "Reset FSO" } else { "Disable FSO" }).clicked() {
                    actions.push(AppAction::SetValorantFsoDpi(!v.fso_disabled));
                }
                if subtle_button(ui, if v.qos_policy_exists { "Remove QoS" } else { "Apply QoS 46" }).clicked() {
                    actions.push(AppAction::SetValorantQos(!v.qos_policy_exists));
                }
                let val_cache = v.shader_cache_bytes + v.log_cache_bytes + v.crash_cache_bytes;
                if danger_button(ui, &format!("Purge Cache ({})", format_bytes(val_cache))).clicked() {
                    actions.push(AppAction::PurgeValorantCaches);
                }
            });
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("RIOT SHARD LATENCY")
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button(ui, "Ping Shards").clicked() {
                        actions.push(AppAction::PingRiotRegions);
                    }
                });
            });
            ui.add_space(6.0);
            if state.riot_pings.is_empty() {
                ui.label(
                    RichText::new("Click 'Ping Shards' to measure regional latency.")
                        .size(12.0)
                        .color(theme::TEXT_SECONDARY),
                );
            } else {
                for p in &state.riot_pings {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&p.region).size(12.2).color(theme::TEXT_PRIMARY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (txt, c) = match p.rtt_ms {
                                Some(ms) if ms <= 45 => (format!("{ms} ms"), theme::ACCENT_EMERALD),
                                Some(ms) if ms <= 100 => (format!("{ms} ms"), theme::ACCENT_AMBER),
                                Some(ms) => (format!("{ms} ms"), theme::ACCENT_ROSE),
                                None => ("Timeout".to_string(), theme::TEXT_MUTED),
                            };
                            ui.label(RichText::new(txt).monospace().size(12.0).color(c));
                        });
                    });
                }
            }
        });
    });
}

fn render_emulators(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    let e = &state.emulator_topo;
    let count = usize::from(e.bluestacks_installed)
        + usize::from(e.msi_installed)
        + usize::from(e.ldplayer_installed);

    bento_frame(Some(theme::ACCENT_GOLD)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Android Emulator Lab")
                    .size(16.5)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            dot_pill(ui, &format!("{count} Detected"), count > 0);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if emerald_button(ui, "⚡ Extreme Patch (240 FPS + ROG)").clicked() {
                    actions.push(AppAction::PatchEmulatorConfigs {
                        fps_240: true,
                        rog_profile: true,
                    });
                }
                if !e.ldplayer_configs.is_empty() && primary_button(ui, "Patch LDPlayer 9").clicked() {
                    actions.push(AppAction::PatchLdPlayerConfigs);
                }
                if subtle_button(ui, "Refresh").clicked() {
                    actions.push(AppAction::RefreshEmulators);
                }
            });
        });

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            dot_pill(ui, "240 FPS", e.fps_240_unlocked);
            dot_pill(ui, "ROG Profile", e.rog_profile_active);
            dot_pill(ui, "ASTC + No Ads", e.astc_and_debloat_active);
            dot_pill(ui, "IFEO High CPU", e.ifeo_priority_locked);
            dot_pill(ui, "dGPU Locked", e.gpu_locked);
            dot_pill(ui, "Raw Aim", e.raw_mouse_active);
        });
    });

    ui.add_space(10.0);

    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("DETECTED ENGINES")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(6.0);
            kv_row(
                ui,
                "BlueStacks 5",
                e.bluestacks_version.as_deref().unwrap_or("Not installed"),
                true,
            );
            kv_row(
                ui,
                "MSI Player",
                e.msi_version.as_deref().unwrap_or("Not installed"),
                true,
            );
            kv_row(
                ui,
                "LDPlayer 9",
                if e.ldplayer_installed { "Installed" } else { "Not installed" },
                true,
            );
            kv_row(ui, "Hypervisor", &e.hypervisor_status, true);
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.label(
                RichText::new("KERNEL & HARDWARE CONTROLS")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("IFEO High CPU Priority").size(12.8));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if action_switch(ui, "emu_ifeo", e.ifeo_priority_locked).clicked() {
                        actions.push(AppAction::SetEmulatorIfeoPriority(!e.ifeo_priority_locked));
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("1:1 Raw Mouse Aim").size(12.8));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if action_switch(ui, "emu_mouse", e.raw_mouse_active).clicked() {
                        actions.push(AppAction::SetEmulatorRawMouse(!e.raw_mouse_active));
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Low-Latency Socket").size(12.8));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if action_switch(ui, "emu_net", e.low_latency_net_active).clicked() {
                        actions.push(AppAction::SetEmulatorLowLatencyNet(!e.low_latency_net_active));
                    }
                });
            });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if subtle_button(ui, "Lock Discrete GPU").clicked() {
                    actions.push(AppAction::LockEmulatorGpu);
                }
                if subtle_button(ui, &format!("Purge Logs ({})", format_bytes(e.cache_bytes))).clicked() {
                    actions.push(AppAction::PurgeEmulatorLogs);
                }
                let target_off = !e.hypervisor_off;
                if danger_button(
                    ui,
                    if target_off { "Raw VT-x (HVCI Off)" } else { "Restore Hyper-V" },
                )
                .clicked()
                {
                    actions.push(AppAction::RequestConfirm(ConfirmRequest {
                        title: "Toggle Windows Hypervisor?".into(),
                        message: "Updates BCD hypervisorlaunchtype and HVCI state (Reboot required).".into(),
                        confirm_label: "Apply".into(),
                        action: Box::new(AppAction::SetHypervisorState { turn_off: target_off }),
                    }));
                }
            });
        });
    });
}
