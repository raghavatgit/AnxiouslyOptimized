use crate::engine::format_bytes;
use crate::models::{InstalledAppKind, LeftoverKind};
use crate::ui::components::{
    card_frame, danger_button, emerald_button, primary_button, section_header, status_badge,
    subtle_button,
};
use crate::ui::{theme, AppAction, ConfirmRequest, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(
            ui,
            "Geek Uninstaller & Leftover Force-Cleaner",
            "Virtualized 240 FPS program table across Win32 (64-bit/32-bit/HKCU) & Store apps with deep leftover sweep and Force Removal.",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if primary_button(ui, "Refresh Installed Programs").clicked() {
                actions.push(AppAction::ScanInstalledApps);
            }
            if ui
                .checkbox(&mut state.uninstaller_include_uwp, "Include Store / UWP Apps")
                .changed()
            {
                actions.push(AppAction::ScanInstalledApps);
            }
        });
    });

    // Leftover Deep-Clean Drawer when an app's leftovers are active
    if let Some(ref app) = state.active_leftover_app {
        let app_name = app.display_name.clone();
        let app_pub = app.publisher.clone();
        card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_badge(
                    ui,
                    "GEEK LEFTOVER & FORCE REMOVAL SCANNER",
                    theme::ACCENT_GOLD,
                    theme::ACCENT_AMBER_DIM,
                );
                ui.label(
                    RichText::new(format!(
                        "{} ({}) — {} leftover trace(s) found",
                        app_name,
                        app_pub,
                        state.leftover_items.len()
                    ))
                    .size(14.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Close Scanner").clicked() {
                        actions.push(AppAction::CloseLeftoversModal);
                    }
                    let sel_cnt = state.leftover_items.iter().filter(|i| i.is_selected).count();
                    if sel_cnt > 0 {
                        let total_sz: u64 = state
                            .leftover_items
                            .iter()
                            .filter(|i| i.is_selected)
                            .map(|i| i.size_bytes)
                            .sum();
                        if danger_button(
                            ui,
                            &format!(
                                "Force Delete Selected ({sel_cnt} items, {})",
                                format_bytes(total_sz)
                            ),
                        )
                        .clicked()
                        {
                            actions.push(AppAction::RequestConfirm(ConfirmRequest {
                                title: format!("Confirm Geek Force Removal: {app_name}"),
                                message: format!(
                                    "Permanently delete {sel_cnt} selected leftover files, folders, and registry keys for '{app_name}' ({})?",
                                    format_bytes(total_sz)
                                ),
                                confirm_label: "Delete All Selected Leftovers".into(),
                                action: Box::new(AppAction::DeleteSelectedLeftovers),
                            }));
                        }
                    }
                });
            });

            ui.add_space(6.0);
            if state.leftover_items.is_empty() {
                ui.label(
                    RichText::new("Zero leftover files, folders, or registry traces found for this application.")
                        .size(12.5)
                        .color(theme::ACCENT_EMERALD),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("geek_leftovers_scroll")
                    .max_height(210.0)
                    .show(ui, |ui| {
                        for item in state.leftover_items.iter_mut() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut item.is_selected, "");
                                let (fg, bg) = match item.kind {
                                    LeftoverKind::Directory => (theme::ACCENT_GOLD, theme::BG_ELEVATED),
                                    LeftoverKind::File => (theme::ACCENT_COBALT, theme::BG_ELEVATED),
                                    LeftoverKind::RegistryKey => (theme::ACCENT_ROSE, theme::ACCENT_ROSE_DIM),
                                    LeftoverKind::RegistryValue => (theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM),
                                };
                                status_badge(ui, item.kind.label(), fg, bg);
                                ui.label(
                                    RichText::new(&item.display_path)
                                        .size(12.0)
                                        .monospace()
                                        .color(theme::TEXT_PRIMARY),
                                );
                                if item.size_bytes > 0 {
                                    status_badge(
                                        ui,
                                        &format_bytes(item.size_bytes),
                                        theme::TEXT_SECONDARY,
                                        theme::BG_ELEVATED,
                                    );
                                }
                            });
                        }
                    });
            }
        });
        ui.add_space(10.0);
    }

    // Search + Filter + Sort Bar
    let mut sort_toggled = false;
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.add_sized(
                [250.0, 28.0],
                egui::TextEdit::singleline(&mut state.uninstaller_search)
                    .hint_text("Search programs by name, publisher, or version..."),
            );
            if !state.uninstaller_search.is_empty() && subtle_button(ui, "Clear").clicked() {
                state.uninstaller_search.clear();
            }

            ui.separator();

            if filter_pill(ui, "All", state.uninstaller_filter_kind.is_none()).clicked() {
                state.uninstaller_filter_kind = None;
            }
            for k in [
                InstalledAppKind::Win32x64,
                InstalledAppKind::Win32x86,
                InstalledAppKind::UserHkcu,
                InstalledAppKind::StoreUwp,
            ] {
                if filter_pill(ui, k.label(), state.uninstaller_filter_kind == Some(k)).clicked() {
                    state.uninstaller_filter_kind = Some(k);
                }
            }

            ui.separator();
            if subtle_button(
                ui,
                if state.uninstaller_sort_by_size {
                    "Sort: Size (Largest First)"
                } else {
                    "Sort: Name (A-Z)"
                },
            )
            .clicked()
            {
                state.uninstaller_sort_by_size = !state.uninstaller_sort_by_size;
                sort_toggled = true;
            }

            let total_size: u64 = state.installed_apps.iter().map(|a| a.estimated_size_bytes).sum();
            ui.label(
                RichText::new(format!(
                    "{} programs ({})",
                    state.installed_apps.len(),
                    format_bytes(total_size)
                ))
                .size(12.0)
                .monospace()
                .color(theme::TEXT_SECONDARY),
            );
        });
    });

    if sort_toggled {
        if state.uninstaller_sort_by_size {
            state
                .installed_apps
                .sort_by(|a, b| b.estimated_size_bytes.cmp(&a.estimated_size_bytes));
        } else {
            state.installed_apps.sort_by(|a, b| {
                a.display_name
                    .to_ascii_lowercase()
                    .cmp(&b.display_name.to_ascii_lowercase())
            });
        }
    }

    ui.add_space(8.0);

    if state.installed_apps.is_empty() {
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(if state.scanning_apps {
                    "Scanning registry hives and installed programs..."
                } else {
                    "Click 'Refresh Installed Programs' to load all Win32 & Store applications."
                })
                .size(13.0)
                .color(theme::TEXT_SECONDARY),
            );
        });
        return;
    }

    // Zero-clone index filter + virtualized row rendering (`show_rows`) for instant 240 FPS scrolling
    let q = state.uninstaller_search.trim().to_ascii_lowercase();
    let kind_filter = state.uninstaller_filter_kind;

    let matching_indices: Vec<usize> = state
        .installed_apps
        .iter()
        .enumerate()
        .filter_map(|(idx, app)| {
            let kind_ok = kind_filter.map(|k| app.kind == k).unwrap_or(true);
            let txt_ok = q.is_empty()
                || app.display_name.to_ascii_lowercase().contains(&q)
                || app.publisher.to_ascii_lowercase().contains(&q)
                || app.display_version.to_ascii_lowercase().contains(&q);
            if kind_ok && txt_ok {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    let row_height = 68.0;
    egui::ScrollArea::vertical()
        .id_salt("geek_virtual_table")
        .auto_shrink([false, false])
        .show_rows(ui, row_height, matching_indices.len(), |ui, row_range| {
            for row_i in row_range {
                let idx = matching_indices[row_i];
                let app = &state.installed_apps[idx];
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(&app.display_name)
                                        .size(14.0)
                                        .strong()
                                        .color(theme::TEXT_PRIMARY),
                                );
                                if !app.display_version.is_empty() {
                                    status_badge(
                                        ui,
                                        &format!("v{}", app.display_version),
                                        theme::ACCENT_COBALT,
                                        theme::BG_ELEVATED,
                                    );
                                }
                                status_badge(
                                    ui,
                                    app.kind.label(),
                                    theme::TEXT_SECONDARY,
                                    theme::BG_ELEVATED,
                                );
                                if app.estimated_size_bytes > 0 {
                                    status_badge(
                                        ui,
                                        &format_bytes(app.estimated_size_bytes),
                                        theme::ACCENT_GOLD,
                                        theme::BG_ELEVATED,
                                    );
                                }
                            });
                            ui.label(
                                RichText::new(format!(
                                    "Publisher: {}   |   Installed: {}   |   Location: {}",
                                    if app.publisher.is_empty() {
                                        "Unknown"
                                    } else {
                                        &app.publisher
                                    },
                                    if app.install_date.is_empty() {
                                        "N/A"
                                    } else {
                                        &app.install_date
                                    },
                                    app.install_location.as_deref().unwrap_or("Registry / System"),
                                ))
                                .size(11.5)
                                .color(theme::TEXT_SECONDARY),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if danger_button(ui, "Force Removal").clicked() {
                                actions.push(AppAction::ScanAppLeftovers(app.id.clone()));
                            }
                            if emerald_button(ui, "Uninstall").clicked() {
                                actions.push(AppAction::StandardUninstallApp(app.id.clone()));
                            }
                            if subtle_button(ui, "Scan Traces").clicked() {
                                actions.push(AppAction::ScanAppLeftovers(app.id.clone()));
                            }
                            if let Some(hive) = app.registry_hive {
                                if subtle_button(ui, "Regedit").clicked() {
                                    actions.push(AppAction::OpenRegeditKey {
                                        hive,
                                        subkey: app.registry_subkey.clone(),
                                    });
                                }
                            }
                            if let Some(ref loc) = app.install_location {
                                if subtle_button(ui, "Folder").clicked() {
                                    actions.push(AppAction::OpenFolder(loc.clone()));
                                }
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
}

fn filter_pill(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let (fill, fg, stroke) = if active {
        (theme::ACCENT_COBALT_DIM, theme::ACCENT_COBALT, theme::ACCENT_COBALT)
    } else {
        (theme::BG_ELEVATED, theme::TEXT_SECONDARY, theme::BORDER_SUBTLE)
    };
    ui.add(
        egui::Button::new(RichText::new(label).size(12.0).color(fg))
            .fill(fill)
            .stroke(egui::Stroke::new(1.0_f32, stroke))
            .corner_radius(egui::CornerRadius::same(12)),
    )
}
