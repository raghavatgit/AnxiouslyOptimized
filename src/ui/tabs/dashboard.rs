use crate::engine::format_bytes;
use crate::engine::tweaks::preset_ids;
use crate::models::{PresetId, ValorantInstallStatus};
use crate::ui::components::{
    card_frame, emerald_button, live_sparkline, metric_card, primary_button, progress_bar,
    section_header, status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, TabId, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &UiState, actions: &mut Vec<AppAction>) {
    section_header(
        ui,
        "System Dashboard",
        "Live Windows hardware telemetry, optimization audit, and one-click workflows.",
    );

    let r = &state.system_report;

    // Top Hero Row: Privilege + Host + Power + Pending Reboot + GameMode
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if state.is_elevated {
                status_badge(ui, "ADMINISTRATOR", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
            } else {
                status_badge(
                    ui,
                    "STANDARD USER (LIMITED WRITES)",
                    theme::ACCENT_AMBER,
                    theme::ACCENT_AMBER_DIM,
                );
            }
            if r.platform.pending_reboot {
                status_badge(ui, "REBOOT PENDING", theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM);
            } else {
                status_badge(ui, "SYSTEM CLEAN", theme::ACCENT_COBALT, theme::ACCENT_COBALT_DIM);
            }
            if state.gamemode_status.daemon_running {
                status_badge(
                    ui,
                    &format!("GAME MODE: {}", state.gamemode_status.summary),
                    theme::ACCENT_EMERALD,
                    theme::ACCENT_EMERALD_DIM,
                );
            } else {
                status_badge(ui, "GAME MODE: STANDBY", theme::TEXT_SECONDARY, theme::BG_ELEVATED);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if subtle_button(ui, "Rescan Whole System").clicked() {
                    actions.push(AppAction::RescanSystem);
                }
                if subtle_button(ui, "View Full Specs").clicked() {
                    actions.push(AppAction::SwitchTab(TabId::Specs));
                }
            });
        });

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "{}  |  {} ({})  |  Build {}.{}",
                    r.platform.computer_name,
                    r.platform.os_name,
                    r.platform.display_version,
                    r.platform.build_number,
                    r.platform.ubr,
                ))
                .size(13.0)
                .color(theme::TEXT_PRIMARY),
            );
            ui.label(
                RichText::new(format!(
                    "   •   Power: {}   •   Timer: {:.2} ms",
                    r.power.active_scheme_name, state.gamemode_status.current_timer_ms
                ))
                .size(12.5)
                .monospace()
                .color(theme::TEXT_SECONDARY),
            );
        });
    });

    ui.add_space(10.0);

    // Hardware Summary Strip (4 Cards)
    let gpu_primary = r.gpus.first();
    let gpu_title = gpu_primary
        .map(|g| g.name.as_str())
        .unwrap_or("Scanning GPU...");
    let gpu_detail = gpu_primary
        .map(|g| {
            format!(
                "{} VRAM | {}",
                format_bytes(g.vram_bytes),
                if g.is_integrated { "Integrated" } else { "Discrete" }
            )
        })
        .unwrap_or_default();

    let applied_count = state.tweaks.iter().filter(|t| t.is_applied).count();
    let total_tweaks = state.tweaks.len();

    ui.columns(4, |cols| {
        metric_card(
            &mut cols[0],
            160.0,
            "Processor",
            if r.cpu.brand.is_empty() { "Detecting..." } else { &r.cpu.brand },
            &format!(
                "{}C / {}T | {}",
                r.cpu.physical_cores, r.cpu.logical_threads, r.cpu.topology_label
            ),
            theme::ACCENT_COBALT,
        );
        metric_card(
            &mut cols[1],
            160.0,
            "Primary GPU",
            gpu_title,
            &gpu_detail,
            theme::ACCENT_GOLD,
        );
        metric_card(
            &mut cols[2],
            160.0,
            "Memory",
            &format!(
                "{} {} @ {} MT/s",
                format_bytes(r.memory.total_phys_bytes),
                r.memory.memory_type,
                r.memory.speed_mts
            ),
            &format!(
                "{}/{} slots | {}",
                r.memory.used_slots, r.memory.total_slots, r.memory.channel_mode
            ),
            theme::ACCENT_EMERALD,
        );
        metric_card(
            &mut cols[3],
            160.0,
            "Optimization Audit",
            &format!("{applied_count} / {total_tweaks} Applied"),
            &format!(
                "{} active journal(s) | Timer {:.2} ms",
                state.journals.iter().filter(|j| !j.rolled_back).count(),
                state.gamemode_status.current_timer_ms
            ),
            if applied_count > 0 {
                theme::ACCENT_EMERALD
            } else {
                theme::ACCENT_AMBER
            },
        );
    });

    ui.add_space(10.0);

    // Real Live 1-Second Telemetry Sparklines
    ui.columns(2, |cols| {
        live_sparkline(
            &mut cols[0],
            "CPU Utilization (Live 60s)",
            &format!("{:.1}%", state.live_cpu_pct),
            &format!(
                "{} ({}C / {}T)",
                r.cpu.brand, r.cpu.physical_cores, r.cpu.logical_threads
            ),
            &state.cpu_history,
            theme::ACCENT_COBALT,
            92.0,
        );

        let ram_pct = if state.live_ram_total_bytes > 0 {
            (state.live_ram_used_bytes as f64 / state.live_ram_total_bytes as f64 * 100.0) as f32
        } else {
            r.memory.memory_load_pct as f32
        };
        live_sparkline(
            &mut cols[1],
            "Physical Memory Load (Live 60s)",
            &format!("{ram_pct:.1}%"),
            &format!(
                "{} used / {} total",
                format_bytes(state.live_ram_used_bytes),
                format_bytes(state.live_ram_total_bytes),
            ),
            &state.ram_history,
            theme::ACCENT_EMERALD,
            92.0,
        );
    });

    ui.add_space(10.0);

    // Quick Action Row + Storage Volumes + Gaming Readiness
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("One-Click Optimization & Maintenance")
                    .size(14.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.label(
                RichText::new("All changes write an atomic undo journal before touching Windows.")
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY),
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let g_count = preset_ids(PresetId::ExtremeWin11).len();
                if primary_button(ui, &format!("Apply Extreme Win11 & eSports ({g_count})")).clicked() {
                    actions.push(AppAction::ApplyPreset(PresetId::ExtremeWin11));
                }
                if emerald_button(ui, "Mem Reduct Clean Now").clicked() {
                    actions.push(AppAction::RunMemReductClean);
                }
                if subtle_button(ui, "Geek Uninstaller").clicked() {
                    actions.push(AppAction::SwitchTab(TabId::Uninstaller));
                }
                if subtle_button(ui, "Extreme Emulators").clicked() {
                    actions.push(AppAction::SwitchTab(TabId::Gaming));
                }
            });

            if let Some(purge) = &state.last_standby_purge {
                ui.add_space(8.0);
                let reclaimed = purge
                    .avail_after_bytes
                    .saturating_sub(purge.avail_before_bytes)
                    .max(purge.used_before_bytes.saturating_sub(purge.used_after_bytes));
                ui.label(
                    RichText::new(format!(
                        "Last Mem Reduct: Freed {} ({}) | Cache: {} -> {}",
                        format_bytes(reclaimed),
                        purge.regions_cleaned.join(" + "),
                        format_bytes(purge.cache_before_bytes),
                        format_bytes(purge.cache_after_bytes),
                    ))
                    .size(12.0)
                    .monospace()
                    .color(theme::ACCENT_EMERALD),
                );
            }

            ui.add_space(8.0);
            ui.label(
                RichText::new("Detected Gaming Ecosystem")
                    .size(13.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            let val_lbl = match state.valorant_topo.install_status {
                ValorantInstallStatus::Running => "VALORANT: Running (Vanguard Safe Mode)",
                ValorantInstallStatus::InstalledNotRunning => "VALORANT: Installed",
                ValorantInstallStatus::NotInstalled => "VALORANT: Not Detected",
            };
            ui.label(
                RichText::new(format!(
                    "{val_lbl}  |  Steam: {}  |  BlueStacks/MSI: {}",
                    if r.ecosystem.steam_installed { "Yes" } else { "No" },
                    if state.emulator_topo.bluestacks_installed || state.emulator_topo.msi_installed {
                        "Installed"
                    } else {
                        "None"
                    }
                ))
                .size(12.0)
                .color(theme::TEXT_SECONDARY),
            );
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Storage Volumes")
                        .size(14.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Deep Cleaner").clicked() {
                        actions.push(AppAction::SwitchTab(TabId::Health));
                    }
                });
            });
            ui.add_space(6.0);
            for vol in &r.volumes {
                let frac = (vol.used_pct / 100.0).clamp(0.0, 1.0);
                let accent = if vol.used_pct > 88.0 {
                    theme::ACCENT_ROSE
                } else if vol.used_pct > 75.0 {
                    theme::ACCENT_AMBER
                } else {
                    theme::ACCENT_COBALT
                };
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{} ({})", vol.mount, vol.label))
                            .size(12.5)
                            .strong()
                            .color(theme::TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} free / {}",
                                format_bytes(vol.free_bytes),
                                format_bytes(vol.total_bytes)
                            ))
                            .size(11.5)
                            .monospace()
                            .color(theme::TEXT_SECONDARY),
                        );
                    });
                });
                progress_bar(ui, frac, &format!("{:.1}% used [{}]", vol.used_pct, vol.filesystem), accent);
                ui.add_space(4.0);
            }
        });
    });
}
