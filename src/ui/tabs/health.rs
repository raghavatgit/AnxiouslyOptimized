use crate::engine::format_bytes;
use crate::ui::components::{
    card_frame, danger_button, emerald_button, primary_button, progress_bar, section_header,
    status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, ConfirmRequest, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    section_header(
        ui,
        "Mem Reduct Engine, Extreme PC Cleaner & System Servicing",
        "6-region kernel memory reducer (Working Set, System Cache, Modified, Standby, Priority-0, Page Combine) + 14-category PC Cleaner.",
    );

    // ==================== 1. MEM REDUCT KERNEL MEMORY ENGINE ====================
    card_frame().show(ui, |ui| {
        let m = &state.mem_stats;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Mem Reduct — Multi-Region Kernel Memory Reducer")
                    .size(16.0)
                    .strong()
                    .color(theme::ACCENT_EMERALD),
            );
            if state.settings.mem_reduct.auto_reduct_enabled {
                status_badge(
                    ui,
                    &format!(
                        "AUTO-REDUCT: ON (>{}% / {}m)",
                        state.settings.mem_reduct.auto_reduct_threshold_pct,
                        state.settings.mem_reduct.auto_reduct_interval_mins
                    ),
                    theme::ACCENT_EMERALD,
                    theme::ACCENT_EMERALD_DIM,
                );
            } else {
                status_badge(ui, "AUTO-REDUCT: OFF", theme::TEXT_MUTED, theme::BG_ELEVATED);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if emerald_button(ui, "Clean Memory Now (Mem Reduct)").clicked() {
                    actions.push(AppAction::RunMemReductClean);
                }
            });
        });

        ui.add_space(6.0);

        // 3 Live Mem Reduct Gauges: Physical Memory, Page File / Commit Charge, Kernel System Cache
        ui.columns(3, |cols| {
            // Physical RAM
            cols[0].label(
                RichText::new(format!(
                    "Physical Memory: {}% ({} / {})",
                    m.phys_load_pct,
                    format_bytes(m.phys_used_bytes),
                    format_bytes(m.phys_total_bytes)
                ))
                .size(12.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
            );
            progress_bar(
                &mut cols[0],
                (m.phys_load_pct as f32 / 100.0).clamp(0.0, 1.0),
                &format!("{} Free", format_bytes(m.phys_free_bytes)),
                if m.phys_load_pct >= 85 {
                    theme::ACCENT_ROSE
                } else {
                    theme::ACCENT_EMERALD
                },
            );

            // Commit Charge / Pagefile
            cols[1].label(
                RichText::new(format!(
                    "Commit Charge (Pagefile): {}% ({} / {})",
                    m.commit_load_pct,
                    format_bytes(m.commit_total_bytes),
                    format_bytes(m.commit_limit_bytes)
                ))
                .size(12.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
            );
            progress_bar(
                &mut cols[1],
                (m.commit_load_pct as f32 / 100.0).clamp(0.0, 1.0),
                &format!("{} Headroom", format_bytes(m.commit_free_bytes)),
                theme::ACCENT_COBALT,
            );

            // System Working Set / Cache + Kernel Pools
            cols[2].label(
                RichText::new(format!(
                    "System Cache & Standby: {}",
                    format_bytes(m.system_cache_bytes)
                ))
                .size(12.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
            );
            cols[2].label(
                RichText::new(format!(
                    "Paged Pool: {}  |  Non-Paged: {}",
                    format_bytes(m.kernel_paged_bytes),
                    format_bytes(m.kernel_nonpaged_bytes)
                ))
                .size(11.5)
                .monospace()
                .color(theme::TEXT_SECONDARY),
            );
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        // 6 Mem Reduct Region Checkboxes
        let mut cfg_changed = false;
        let mr = &mut state.settings.mem_reduct;
        ui.label(RichText::new("Memory Regions to Clean:").size(12.0).strong().color(theme::TEXT_MUTED));
        ui.horizontal_wrapped(|ui| {
            cfg_changed |= ui
                .checkbox(&mut mr.clean_working_sets, "Working Set (Apps + System)")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut mr.clean_system_file_cache, "System File Cache")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut mr.clean_modified_page_list, "Modified Page List")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut mr.clean_standby_list, "Standby List (Full)")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut mr.clean_low_priority_standby, "Standby List (Priority-0)")
                .changed();
            cfg_changed |= ui
                .checkbox(&mut mr.clean_combine_memory_lists, "Combine Memory Lists (Deduplicate)")
                .changed();
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            cfg_changed |= ui
                .checkbox(
                    &mut mr.auto_reduct_enabled,
                    "Enable Auto-Reduct Background Cleaning",
                )
                .changed();
            ui.label(RichText::new("When RAM exceeds (%):").size(12.0).color(theme::TEXT_SECONDARY));
            cfg_changed |= ui
                .add(egui::Slider::new(&mut mr.auto_reduct_threshold_pct, 50..=95))
                .changed();
            ui.label(RichText::new("Or every (mins):").size(12.0).color(theme::TEXT_SECONDARY));
            cfg_changed |= ui
                .add(egui::Slider::new(&mut mr.auto_reduct_interval_mins, 1..=120))
                .changed();
        });

        if cfg_changed {
            actions.push(AppAction::SaveSettings);
        }

        if let Some(p) = &state.last_standby_purge {
            let gain = p.avail_after_bytes.saturating_sub(p.avail_before_bytes);
            let used_drop = p.used_before_bytes.saturating_sub(p.used_after_bytes);
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "Last Mem Reduct: Freed {} (Used RAM -{} | Cache {} -> {} | Combined {} pages | Regions: {} | NTSTATUS 0x{:08X})",
                    format_bytes(gain.max(used_drop)),
                    format_bytes(used_drop),
                    format_bytes(p.cache_before_bytes),
                    format_bytes(p.cache_after_bytes),
                    p.pages_combined,
                    p.regions_cleaned.join(", "),
                    p.nt_status as u32,
                ))
                .size(12.0)
                .monospace()
                .color(theme::ACCENT_EMERALD),
            );
        }
    });

    ui.add_space(10.0);

    // ==================== 2. HIBERNATION & EVENT LOGS STRIP ====================
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Windows Hibernation (hiberfil.sys)")
                        .size(14.5)
                        .strong()
                        .color(theme::ACCENT_COBALT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if state.hibernation.enabled {
                        if danger_button(ui, "Disable Hibernation").clicked() {
                            actions.push(AppAction::ToggleHibernation(false));
                        }
                    } else if primary_button(ui, "Enable Hibernation").clicked() {
                        actions.push(AppAction::ToggleHibernation(true));
                    }
                });
            });
            ui.label(
                RichText::new(if state.hibernation.enabled {
                    format!(
                        "ENABLED: {} occupies {}",
                        state.hibernation.file_path,
                        format_bytes(state.hibernation.size_bytes)
                    )
                } else {
                    format!("DISABLED ({} not present on system drive)", state.hibernation.file_path)
                })
                .size(12.5)
                .monospace()
                .color(theme::TEXT_PRIMARY),
            );
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Quick System Purge Utilities")
                        .size(14.5)
                        .strong()
                        .color(theme::ACCENT_GOLD),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Clear All Event Viewer Logs (.evtx)").clicked() {
                        actions.push(AppAction::ClearEventLogs);
                    }
                    if subtle_button(ui, "Empty Recycle Bin").clicked() {
                        actions.push(AppAction::EmptyRecycleBin);
                    }
                });
            });
            ui.label(
                RichText::new("One-click clearance of Windows Event Viewer channels (wevtutil) and all drive Recycle Bins.")
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY),
            );
        });
    });

    ui.add_space(10.0);

    // ==================== 3. EXTREME 14-CATEGORY PC CLEANER ====================
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Extreme PC Cleaner (14 Categories, Reparse-Safe)")
                        .size(15.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                let total_bytes: u64 = state
                    .cleaner_cats
                    .iter()
                    .filter(|c| c.is_selected)
                    .map(|c| c.scanned_bytes)
                    .sum();
                let total_files: usize = state
                    .cleaner_cats
                    .iter()
                    .filter(|c| c.is_selected)
                    .map(|c| c.file_count)
                    .sum();
                ui.label(
                    RichText::new(format!(
                        "Selected reclaimable: {} across {} file(s) | Temp cooldown: {}h",
                        format_bytes(total_bytes),
                        total_files,
                        state.settings.temp_cleaner_min_age_hours
                    ))
                    .size(12.0)
                    .monospace()
                    .color(theme::TEXT_SECONDARY),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if danger_button(ui, "Clean Selected Categories").clicked() {
                    actions.push(AppAction::PurgeCleaner);
                }
                if primary_button(ui, "Rescan All Categories").clicked() {
                    actions.push(AppAction::ScanCleaner);
                }
            });
        });

        ui.add_space(6.0);
        for cat in state.cleaner_cats.iter_mut() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut cat.is_selected, "");
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&cat.title).size(13.0).strong().color(theme::TEXT_PRIMARY));
                        status_badge(
                            ui,
                            &format!("{} ({} files)", format_bytes(cat.scanned_bytes), cat.file_count),
                            if cat.scanned_bytes > 0 {
                                theme::ACCENT_GOLD
                            } else {
                                theme::TEXT_MUTED
                            },
                            theme::BG_ELEVATED,
                        );
                        if let Some(reason) = &cat.skipped_reason {
                            status_badge(ui, reason, theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM);
                        }
                    });
                    ui.label(RichText::new(&cat.description).size(11.5).color(theme::TEXT_SECONDARY));
                    if let Some(warn) = &cat.warning_note {
                        ui.label(RichText::new(warn).size(11.0).color(theme::ACCENT_AMBER));
                    }
                });
            });
            ui.add_space(3.0);
        }
    });

    ui.add_space(10.0);

    // ==================== 4. LIVE STREAMING SFC & DISM ====================
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Windows System Integrity & Servicing (SFC / DISM)")
                    .size(15.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if state.stream_running {
                    if danger_button(ui, "Cancel Running Tool").clicked() {
                        actions.push(AppAction::CancelStreamTask);
                    }
                    status_badge(
                        ui,
                        &format!("RUNNING: {}", state.stream_label),
                        theme::ACCENT_AMBER,
                        theme::ACCENT_AMBER_DIM,
                    );
                } else {
                    if subtle_button(ui, "DISM ResetBase (Irreversible)").clicked() {
                        actions.push(AppAction::RequestConfirm(ConfirmRequest {
                            title: "Confirm DISM /StartComponentCleanup /ResetBase".into(),
                            message: "Running DISM /ResetBase permanently removes superseded Windows Update backups from WinSxS. Existing installed updates can NO LONGER be uninstalled afterward.\n\nProceed only if your system is completely stable.".into(),
                            confirm_label: "Run DISM /ResetBase".into(),
                            action: Box::new(AppAction::RunDismCleanup { reset_base: true }),
                        }));
                    }
                    if subtle_button(ui, "DISM Component Cleanup").clicked() {
                        actions.push(AppAction::RunDismCleanup { reset_base: false });
                    }
                    if primary_button(ui, "DISM /RestoreHealth").clicked() {
                        actions.push(AppAction::RunDismRestoreHealth);
                    }
                    if emerald_button(ui, "SFC /scannow").clicked() {
                        actions.push(AppAction::RunSfcScan);
                    }
                }
            });
        });

        ui.add_space(6.0);
        egui::Frame::new()
            .fill(theme::BG_CANVAS)
            .stroke(egui::Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("sfc_dism_stream")
                    .max_height(150.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if state.stream_lines.is_empty() {
                            ui.label(
                                RichText::new("Idle. Click SFC /scannow, DISM /RestoreHealth, or Install Runtimes to stream live UTF-16LE/UTF-8 console output here.")
                                    .size(11.5)
                                    .monospace()
                                    .color(theme::TEXT_MUTED),
                            );
                        } else {
                            for line in &state.stream_lines {
                                ui.label(RichText::new(line).size(11.5).monospace().color(theme::TEXT_PRIMARY));
                            }
                        }
                    });
            });
    });

    ui.add_space(10.0);

    // ==================== 5. ESSENTIAL RUNTIMES (WINGET) ====================
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Essential Runtimes & Gaming Tools (WinGet)")
                        .size(15.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new("Checks Windows Registry (64-bit & WOW6432Node), System32 + SysWOW64 DirectX DLLs, and .NET 8+ LTS.")
                        .size(12.0)
                        .color(theme::TEXT_SECONDARY),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let sel_rt = state.runtimes.iter().filter(|r| r.is_selected).count();
                if sel_rt > 0 && !state.stream_running {
                    if primary_button(ui, &format!("Install Selected ({sel_rt})")).clicked() {
                        actions.push(AppAction::InstallSelectedRuntimes);
                    }
                }
                if subtle_button(ui, "Refresh Detection").clicked() {
                    actions.push(AppAction::RefreshRuntimes);
                }
            });
        });

        ui.add_space(6.0);
        for item in state.runtimes.iter_mut() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut item.is_selected, "");
                ui.label(RichText::new(&item.name).size(13.0).strong().color(theme::TEXT_PRIMARY));
                if item.is_installed {
                    let lbl = item.detected_detail.as_deref().unwrap_or("Installed");
                    status_badge(
                        ui,
                        &format!("INSTALLED ({lbl})"),
                        theme::ACCENT_EMERALD,
                        theme::ACCENT_EMERALD_DIM,
                    );
                } else {
                    status_badge(
                        ui,
                        item.detected_detail.as_deref().unwrap_or("Missing"),
                        theme::ACCENT_AMBER,
                        theme::ACCENT_AMBER_DIM,
                    );
                }
                status_badge(ui, &item.id, theme::TEXT_MUTED, theme::BG_ELEVATED);
            });
            ui.label(RichText::new(format!("   {}", item.description)).size(11.5).color(theme::TEXT_SECONDARY));
        }
    });
}
