use crate::engine::format_bytes;
use crate::ui::components::{
    arc_gauge, bento_frame, card_frame, danger_button, emerald_button, primary_button,
    section_header, segmented_tabs, status_badge, sub_surface_frame, subtle_button, toggle_switch,
};
use crate::ui::{theme, AppAction, CleanSubView, ConfirmRequest, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(ui, "Clean & Repair", "Kernel memory, deep storage & servicing.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            segmented_tabs(
                ui,
                &mut state.clean_subview,
                &[
                    (CleanSubView::Memory, "Memory"),
                    (CleanSubView::Storage, "Storage"),
                    (CleanSubView::Repair, "Repair"),
                ],
            );
        });
    });

    ui.add_space(4.0);

    match state.clean_subview {
        CleanSubView::Memory => render_memory_view(ui, state, actions),
        CleanSubView::Storage => render_storage_view(ui, state, actions),
        CleanSubView::Repair => render_repair_view(ui, state, actions),
    }
}

fn render_memory_view(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    let m = &state.mem_stats;

    bento_frame(Some(theme::ACCENT_EMERALD)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("MEM REDUCT ENGINE")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if emerald_button(ui, "✦  Clean Memory Now").clicked() {
                    actions.push(AppAction::RunMemReductClean);
                }
            });
        });

        ui.add_space(8.0);
        let cache_frac = if m.phys_total_bytes > 0 {
            (m.system_cache_bytes as f32 / m.phys_total_bytes as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        ui.columns(3, |cols| {
            arc_gauge(
                &mut cols[0],
                m.phys_load_pct as f32 / 100.0,
                &format!("{}%", m.phys_load_pct),
                "Physical",
                &format!("{} free", format_bytes(m.phys_free_bytes)),
                if m.phys_load_pct >= 85 {
                    theme::ACCENT_ROSE
                } else {
                    theme::ACCENT_EMERALD
                },
                116.0,
            );
            arc_gauge(
                &mut cols[1],
                m.commit_load_pct as f32 / 100.0,
                &format!("{}%", m.commit_load_pct),
                "Commit",
                &format!("{} / {}", format_bytes(m.commit_total_bytes), format_bytes(m.commit_limit_bytes)),
                theme::ACCENT_CYAN,
                116.0,
            );
            arc_gauge(
                &mut cols[2],
                cache_frac,
                &format_bytes(m.system_cache_bytes),
                "Cache",
                &format!("Paged {}", format_bytes(m.kernel_paged_bytes)),
                theme::ACCENT_COBALT,
                116.0,
            );
        });

        if let Some(p) = &state.last_standby_purge {
            let gain = p.avail_after_bytes.saturating_sub(p.avail_before_bytes);
            let used_drop = p.used_before_bytes.saturating_sub(p.used_after_bytes);
            ui.add_space(8.0);
            sub_surface_frame().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    status_badge(
                        ui,
                        &format!("FREED {}", format_bytes(gain.max(used_drop))),
                        theme::ACCENT_EMERALD,
                        theme::ACCENT_EMERALD_DIM,
                    );
                    ui.label(
                        RichText::new(format!(
                            "Cache {} → {}  •  {} pages combined",
                            format_bytes(p.cache_before_bytes),
                            format_bytes(p.cache_after_bytes),
                            p.pages_combined
                        ))
                        .size(11.8)
                        .monospace()
                        .color(theme::TEXT_SECONDARY),
                    );
                });
            });
        }
    });

    ui.add_space(10.0);

    // Region Switches + Auto-Reduct
    let mut cfg_changed = false;
    let mr = &mut state.settings.mem_reduct;

    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("KERNEL REGIONS")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(6.0);
            let regions: [(&str, &mut bool); 6] = [
                ("Working Sets", &mut mr.clean_working_sets),
                ("System File Cache", &mut mr.clean_system_file_cache),
                ("Modified Page List", &mut mr.clean_modified_page_list),
                ("Standby List (Full)", &mut mr.clean_standby_list),
                ("Priority-0 Standby", &mut mr.clean_low_priority_standby),
                ("Combine Memory Lists", &mut mr.clean_combine_memory_lists),
            ];
            for (lbl, flag) in regions {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(lbl).size(12.8).color(theme::TEXT_PRIMARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        cfg_changed |= toggle_switch(ui, flag).changed();
                    });
                });
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("AUTO-REDUCT")
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    cfg_changed |= toggle_switch(ui, &mut mr.auto_reduct_enabled).changed();
                });
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Threshold (%)").size(12.0).color(theme::TEXT_SECONDARY));
            cfg_changed |= ui
                .add(egui::Slider::new(&mut mr.auto_reduct_threshold_pct, 50..=95))
                .changed();
            ui.add_space(6.0);
            ui.label(RichText::new("Interval (minutes)").size(12.0).color(theme::TEXT_SECONDARY));
            cfg_changed |= ui
                .add(egui::Slider::new(&mut mr.auto_reduct_interval_mins, 1..=120))
                .changed();
        });
    });

    if cfg_changed {
        actions.push(AppAction::SaveSettings);
    }
}

fn render_storage_view(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
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

    // Hero Reclaimable Banner
    bento_frame(Some(theme::ACCENT_CYAN)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(
                    RichText::new("RECLAIMABLE SPACE")
                        .size(10.5)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                ui.label(
                    RichText::new(format_bytes(total_bytes))
                        .size(26.0)
                        .strong()
                        .monospace()
                        .color(theme::ACCENT_CYAN),
                );
                ui.label(
                    RichText::new(format!("{total_files} files selected"))
                        .size(11.8)
                        .color(theme::TEXT_SECONDARY),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if danger_button(ui, "Purge Selected").clicked() {
                    actions.push(AppAction::PurgeCleaner);
                }
                if primary_button(ui, "Scan").clicked() {
                    actions.push(AppAction::ScanCleaner);
                }
                if subtle_button(ui, "Empty Trash").clicked() {
                    actions.push(AppAction::EmptyRecycleBin);
                }
                if subtle_button(ui, "Clear Event Logs").clicked() {
                    actions.push(AppAction::ClearEventLogs);
                }
            });
        });
    });

    ui.add_space(10.0);

    // Hibernation Compact Strip
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Hibernation File (hiberfil.sys)")
                        .size(13.2)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new(if state.hibernation.enabled {
                        format!("Occupies {}", format_bytes(state.hibernation.size_bytes))
                    } else {
                        "Disabled (0 B)".to_string()
                    })
                    .size(11.5)
                    .monospace()
                    .color(theme::TEXT_SECONDARY),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut hib_on = state.hibernation.enabled;
                if toggle_switch(ui, &mut hib_on).clicked() {
                    actions.push(AppAction::ToggleHibernation(hib_on));
                }
            });
        });
    });

    ui.add_space(10.0);

    // 14 Cleaner Categories as Clean Rows
    for cat in state.cleaner_cats.iter_mut() {
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                toggle_switch(ui, &mut cat.is_selected);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(&cat.title)
                                .size(13.2)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        if let Some(reason) = &cat.skipped_reason {
                            status_badge(ui, reason, theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM);
                        }
                    });
                    ui.label(
                        RichText::new(&cat.description)
                            .size(11.5)
                            .color(theme::TEXT_SECONDARY),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_badge(
                        ui,
                        &format_bytes(cat.scanned_bytes),
                        if cat.scanned_bytes > 0 {
                            theme::ACCENT_CYAN
                        } else {
                            theme::TEXT_MUTED
                        },
                        theme::BG_VOID,
                    );
                });
            });
        });
        ui.add_space(4.0);
    }
}

fn render_repair_view(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    // SFC & DISM Live Terminal Card
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("OS Integrity (SFC & DISM)")
                    .size(14.5)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if state.stream_running {
                    if danger_button(ui, "Stop").clicked() {
                        actions.push(AppAction::CancelStreamTask);
                    }
                    status_badge(
                        ui,
                        &state.stream_label,
                        theme::ACCENT_AMBER,
                        theme::ACCENT_AMBER_DIM,
                    );
                } else {
                    if subtle_button(ui, "ResetBase").clicked() {
                        actions.push(AppAction::RequestConfirm(ConfirmRequest {
                            title: "DISM /ResetBase".into(),
                            message: "Permanently removes superseded Windows Update backups from WinSxS.".into(),
                            confirm_label: "Run ResetBase".into(),
                            action: Box::new(AppAction::RunDismCleanup { reset_base: true }),
                        }));
                    }
                    if subtle_button(ui, "Cleanup").clicked() {
                        actions.push(AppAction::RunDismCleanup { reset_base: false });
                    }
                    if primary_button(ui, "DISM Repair").clicked() {
                        actions.push(AppAction::RunDismRestoreHealth);
                    }
                    if emerald_button(ui, "SFC Scan").clicked() {
                        actions.push(AppAction::RunSfcScan);
                    }
                }
            });
        });

        ui.add_space(8.0);
        sub_surface_frame().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("sfc_dism_stream")
                .max_height(160.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if state.stream_lines.is_empty() {
                        ui.label(
                            RichText::new("Ready — select SFC or DISM above.")
                                .size(11.5)
                                .monospace()
                                .color(theme::TEXT_MUTED),
                        );
                    } else {
                        for line in &state.stream_lines {
                            ui.label(
                                RichText::new(line)
                                    .size(11.5)
                                    .monospace()
                                    .color(theme::TEXT_PRIMARY),
                            );
                        }
                    }
                });
        });
    });

    ui.add_space(10.0);

    // Essential Runtimes
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("System Runtimes (WinGet)")
                    .size(14.5)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let sel_rt = state.runtimes.iter().filter(|r| r.is_selected).count();
                if sel_rt > 0 && !state.stream_running {
                    if primary_button(ui, &format!("Install ({sel_rt})")).clicked() {
                        actions.push(AppAction::InstallSelectedRuntimes);
                    }
                }
                if subtle_button(ui, "Refresh").clicked() {
                    actions.push(AppAction::RefreshRuntimes);
                }
            });
        });

        ui.add_space(8.0);
        for item in state.runtimes.iter_mut() {
            ui.horizontal(|ui| {
                toggle_switch(ui, &mut item.is_selected);
                ui.label(
                    RichText::new(&item.name)
                        .size(13.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if item.is_installed {
                        status_badge(
                            ui,
                            item.detected_detail.as_deref().unwrap_or("Installed"),
                            theme::ACCENT_EMERALD,
                            theme::ACCENT_EMERALD_DIM,
                        );
                    } else {
                        status_badge(ui, "Missing", theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM);
                    }
                });
            });
            ui.add_space(3.0);
        }
    });
}
