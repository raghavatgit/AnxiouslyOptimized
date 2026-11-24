use crate::engine::format_bytes;
use crate::models::{PresetId, ValorantInstallStatus};
use crate::ui::components::{
    arc_gauge, bento_frame, card_frame, dot_pill, emerald_button, icon_button, live_sparkline,
    metric_card, primary_button, progress_bar, section_header, subtle_button, NavIcon,
};
use crate::ui::{theme, AppAction, TabId, UiState};
use egui::{RichText, Vec2};

pub fn render(ui: &mut egui::Ui, state: &UiState, actions: &mut Vec<AppAction>) {
    let r = &state.system_report;
    let avail_w = ui.available_width();

    // ==================== HEADER ROW ====================
    ui.horizontal(|ui| {
        section_header(
            ui,
            &if r.platform.computer_name.is_empty() {
                "Command Center".to_string()
            } else {
                r.platform.computer_name.clone()
            },
            &format!(
                "{} {}  •  Build {}.{}",
                r.platform.os_name, r.platform.display_version, r.platform.build_number, r.platform.ubr
            ),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, NavIcon::Refresh, "Rescan System (F5)", false).clicked() {
                actions.push(AppAction::RescanSystem);
            }
            if r.platform.pending_reboot {
                dot_pill(ui, "Reboot Needed", false);
            }
            dot_pill(
                ui,
                if state.gamemode_status.daemon_running {
                    "Game Mode Active"
                } else {
                    "Game Mode Idle"
                },
                state.gamemode_status.daemon_running,
            );
            dot_pill(
                ui,
                if state.is_elevated { "Admin" } else { "Limited" },
                state.is_elevated,
            );
        });
    });

    ui.add_space(4.0);

    // ==================== HERO BENTO ROW: 3 RADIAL ARC GAUGES + QUICK LAUNCH ====================
    let applied_count = state.tweaks.iter().filter(|t| t.is_applied).count();
    let total_tweaks = state.tweaks.len().max(1);
    let opt_frac = applied_count as f32 / total_tweaks as f32;

    let ram_pct = state.mem_stats.phys_load_pct as f32;
    let ram_accent = if ram_pct >= 85.0 {
        theme::ACCENT_ROSE
    } else if ram_pct >= 70.0 {
        theme::ACCENT_AMBER
    } else {
        theme::ACCENT_EMERALD
    };

    if avail_w >= 860.0 {
        ui.columns(2, |cols| {
            // Left Bento: Live Radial Telemetry Rings
            bento_frame(None).show(&mut cols[0], |ui| {
                ui.columns(3, |g| {
                    arc_gauge(
                        &mut g[0],
                        state.live_cpu_pct / 100.0,
                        &format!("{:.0}%", state.live_cpu_pct),
                        "CPU",
                        &format!("{}C / {}T", r.cpu.physical_cores, r.cpu.logical_threads),
                        theme::ACCENT_CYAN,
                        108.0,
                    );
                    arc_gauge(
                        &mut g[1],
                        ram_pct / 100.0,
                        &format!("{:.0}%", ram_pct),
                        "Memory",
                        &format!("{} free", format_bytes(state.mem_stats.phys_free_bytes)),
                        ram_accent,
                        108.0,
                    );
                    arc_gauge(
                        &mut g[2],
                        opt_frac,
                        &format!("{applied_count}/{total_tweaks}"),
                        "Tuned",
                        &format!("{:.2} ms", state.gamemode_status.current_timer_ms),
                        theme::ACCENT_COBALT,
                        108.0,
                    );
                });
            });

            // Right Bento: Minimalist One-Click Launchpad
            bento_frame(Some(theme::ACCENT_CYAN)).show(&mut cols[1], |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("INSTANT ACTIONS")
                                .size(10.5)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(&r.power.active_scheme_name)
                                    .size(11.0)
                                    .monospace()
                                    .color(theme::ACCENT_CYAN),
                            );
                        });
                    });
                    ui.add_space(8.0);

                    ui.columns(2, |b| {
                        let boost_btn = egui::Button::new(
                            RichText::new("⚡  Extreme Boost")
                                .size(13.2)
                                .strong()
                                .color(theme::BG_VOID),
                        )
                        .fill(theme::ACCENT_CYAN)
                        .min_size(Vec2::new(b[0].available_width(), 40.0));
                        if b[0].add(boost_btn).clicked() {
                            actions.push(AppAction::ApplyPreset(PresetId::ExtremeWin11));
                        }

                        let mem_btn = egui::Button::new(
                            RichText::new("✦  Purge RAM")
                                .size(13.2)
                                .strong()
                                .color(egui::Color32::from_rgb(4, 22, 16)),
                        )
                        .fill(theme::ACCENT_EMERALD)
                        .min_size(Vec2::new(b[1].available_width(), 40.0));
                        if b[1].add(mem_btn).clicked() {
                            actions.push(AppAction::RunMemReductClean);
                        }
                    });

                    ui.add_space(6.0);
                    ui.columns(3, |s| {
                        if s[0]
                            .add_sized(
                                [s[0].available_width(), 32.0],
                                egui::Button::new(
                                    RichText::new("PC Cleaner").size(12.2).color(theme::TEXT_PRIMARY),
                                )
                                .fill(theme::BG_ELEVATED),
                            )
                            .clicked()
                        {
                            actions.push(AppAction::SwitchTab(TabId::Health));
                        }
                        if s[1]
                            .add_sized(
                                [s[1].available_width(), 32.0],
                                egui::Button::new(
                                    RichText::new("Apps").size(12.2).color(theme::TEXT_PRIMARY),
                                )
                                .fill(theme::BG_ELEVATED),
                            )
                            .clicked()
                        {
                            actions.push(AppAction::SwitchTab(TabId::Uninstaller));
                        }
                        if s[2]
                            .add_sized(
                                [s[2].available_width(), 32.0],
                                egui::Button::new(
                                    RichText::new("Gaming").size(12.2).color(theme::TEXT_PRIMARY),
                                )
                                .fill(theme::BG_ELEVATED),
                            )
                            .clicked()
                        {
                            actions.push(AppAction::SwitchTab(TabId::Gaming));
                        }
                    });

                    if let Some(purge) = &state.last_standby_purge {
                        let freed = purge
                            .avail_after_bytes
                            .saturating_sub(purge.avail_before_bytes)
                            .max(purge.used_before_bytes.saturating_sub(purge.used_after_bytes));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!("Freed {} in last RAM purge", format_bytes(freed)))
                                .size(11.0)
                                .monospace()
                                .color(theme::ACCENT_EMERALD),
                        );
                    }
                });
            });
        });
    } else {
        // Compact stacked layout for narrow windows
        bento_frame(None).show(ui, |ui| {
            ui.columns(3, |g| {
                arc_gauge(
                    &mut g[0],
                    state.live_cpu_pct / 100.0,
                    &format!("{:.0}%", state.live_cpu_pct),
                    "CPU",
                    "",
                    theme::ACCENT_CYAN,
                    94.0,
                );
                arc_gauge(
                    &mut g[1],
                    ram_pct / 100.0,
                    &format!("{:.0}%", ram_pct),
                    "RAM",
                    "",
                    ram_accent,
                    94.0,
                );
                arc_gauge(
                    &mut g[2],
                    opt_frac,
                    &format!("{applied_count}/{total_tweaks}"),
                    "Tuned",
                    "",
                    theme::ACCENT_COBALT,
                    94.0,
                );
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if primary_button(ui, "⚡ Extreme Boost").clicked() {
                    actions.push(AppAction::ApplyPreset(PresetId::ExtremeWin11));
                }
                if emerald_button(ui, "Purge RAM").clicked() {
                    actions.push(AppAction::RunMemReductClean);
                }
                if subtle_button(ui, "Cleaner").clicked() {
                    actions.push(AppAction::SwitchTab(TabId::Health));
                }
            });
        });
    }

    ui.add_space(12.0);

    // ==================== HARDWARE IDENTITY STRIP ====================
    let gpu_primary = r.gpus.first();
    let gpu_name = gpu_primary.map(|g| g.name.as_str()).unwrap_or("Detecting...");
    let gpu_sub = gpu_primary
        .map(|g| format!("{} VRAM", format_bytes(g.vram_bytes)))
        .unwrap_or_default();

    let display_primary = r.displays.iter().find(|d| d.is_primary).or_else(|| r.displays.first());
    let disp_val = display_primary
        .map(|d| format!("{}×{} @ {}Hz", d.width, d.height, d.refresh_hz))
        .unwrap_or_else(|| "Display".to_string());
    let disp_sub = display_primary
        .map(|d| d.monitor_name.as_str())
        .unwrap_or("Standard");

    if avail_w >= 860.0 {
        ui.columns(4, |cols| {
            metric_card(
                &mut cols[0],
                120.0,
                "Processor",
                if r.cpu.brand.is_empty() { "Scanning..." } else { &r.cpu.brand },
                &r.cpu.topology_label,
                theme::ACCENT_CYAN,
            );
            metric_card(
                &mut cols[1],
                120.0,
                "Graphics",
                gpu_name,
                &gpu_sub,
                theme::ACCENT_GOLD,
            );
            metric_card(
                &mut cols[2],
                120.0,
                "Memory",
                &format!("{} {}", format_bytes(r.memory.total_phys_bytes), r.memory.memory_type),
                &format!("{} MT/s • {}", r.memory.speed_mts, r.memory.channel_mode),
                theme::ACCENT_EMERALD,
            );
            metric_card(
                &mut cols[3],
                120.0,
                "Display",
                &disp_val,
                disp_sub,
                theme::ACCENT_COBALT,
            );
        });
    } else {
        ui.columns(2, |cols| {
            metric_card(&mut cols[0], 120.0, "Processor", &r.cpu.brand, &r.cpu.topology_label, theme::ACCENT_CYAN);
            metric_card(&mut cols[1], 120.0, "Graphics", gpu_name, &gpu_sub, theme::ACCENT_GOLD);
        });
    }

    ui.add_space(12.0);

    // ==================== LIVE WAVEFORMS ====================
    ui.columns(2, |cols| {
        live_sparkline(
            &mut cols[0],
            "CPU Load",
            &format!("{:.1}%", state.live_cpu_pct),
            &format!("{} MHz max", r.cpu.max_mhz),
            &state.cpu_history,
            theme::ACCENT_CYAN,
            78.0,
        );

        live_sparkline(
            &mut cols[1],
            "Memory Load",
            &format!("{}%", state.mem_stats.phys_load_pct),
            &format!(
                "{} / {}",
                format_bytes(state.live_ram_used_bytes),
                format_bytes(state.live_ram_total_bytes)
            ),
            &state.ram_history,
            theme::ACCENT_EMERALD,
            78.0,
        );
    });

    ui.add_space(12.0);

    // ==================== STORAGE & ECOSYSTEM FOOTER BENTO ====================
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("STORAGE")
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Hardware →").clicked() {
                        actions.push(AppAction::SwitchTab(TabId::Specs));
                    }
                });
            });
            ui.add_space(4.0);
            for vol in &r.volumes {
                let frac = (vol.used_pct / 100.0).clamp(0.0, 1.0);
                let accent = if vol.used_pct > 88.0 {
                    theme::ACCENT_ROSE
                } else if vol.used_pct > 75.0 {
                    theme::ACCENT_AMBER
                } else {
                    theme::ACCENT_CYAN
                };
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{} {}", vol.mount, vol.label))
                            .size(12.5)
                            .strong()
                            .color(theme::TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} free",
                                format_bytes(vol.free_bytes)
                            ))
                            .size(11.2)
                            .monospace()
                            .color(theme::TEXT_SECONDARY),
                        );
                    });
                });
                progress_bar(ui, frac, "", accent);
                ui.add_space(6.0);
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("ECOSYSTEM")
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Safety →").clicked() {
                        actions.push(AppAction::SwitchTab(TabId::Safety));
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                dot_pill(
                    ui,
                    "VALORANT",
                    !matches!(state.valorant_topo.install_status, ValorantInstallStatus::NotInstalled),
                );
                dot_pill(ui, "Steam", r.ecosystem.steam_installed);
                dot_pill(
                    ui,
                    "Android VM",
                    state.emulator_topo.bluestacks_installed
                        || state.emulator_topo.msi_installed
                        || state.emulator_topo.ldplayer_installed,
                );
                dot_pill(ui, "HAGS", r.security.hags_enabled);
                dot_pill(ui, "VBS Off", !r.security.vbs_active);
                dot_pill(
                    ui,
                    &format!(
                        "{} Rollback Points",
                        state.journals.iter().filter(|j| !j.rolled_back).count()
                    ),
                    true,
                );
            });
        });
    });
}
