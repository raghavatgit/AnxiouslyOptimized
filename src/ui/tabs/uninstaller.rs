use crate::engine::format_bytes;
use crate::models::{InstalledAppKind, LeftoverKind};
use crate::ui::components::{
    bento_frame, card_frame, danger_button, filter_chip, icon_button, search_bar, section_header,
    segmented_tabs, status_badge, subtle_button, toggle_switch, NavIcon,
};
use crate::ui::tabs::debloat;
use crate::ui::{theme, AppAction, AppsSubView, ConfirmRequest, UiState};
use egui::{RichText, Vec2};

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        let total_size: u64 = state
            .installed_apps
            .iter()
            .map(|a| a.estimated_size_bytes)
            .sum();
        section_header(
            ui,
            "Applications",
            &format!("{} installed  •  {}", state.installed_apps.len(), format_bytes(total_size)),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            segmented_tabs(
                ui,
                &mut state.apps_subview,
                &[
                    (AppsSubView::Installed, "Installed"),
                    (AppsSubView::Debloat, "OS Debloat"),
                ],
            );
        });
    });

    if state.apps_subview == AppsSubView::Debloat {
        debloat::render(ui, state, actions);
        return;
    }

    // ==================== LEFTOVER DEEP-SCAN PANEL ====================
    if let Some(ref app) = state.active_leftover_app {
        let app_name = app.display_name.clone();
        bento_frame(Some(theme::ACCENT_GOLD)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_badge(
                    ui,
                    &format!("{} TRACES", state.leftover_items.len()),
                    theme::ACCENT_GOLD,
                    theme::ACCENT_AMBER_DIM,
                );
                ui.label(
                    RichText::new(&app_name)
                        .size(14.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Close").clicked() {
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
                            &format!("Force Delete ({sel_cnt} • {})", format_bytes(total_sz)),
                        )
                        .clicked()
                        {
                            actions.push(AppAction::RequestConfirm(ConfirmRequest {
                                title: format!("Force Delete Traces — {app_name}"),
                                message: format!(
                                    "Permanently delete {sel_cnt} leftover files and registry keys ({})?",
                                    format_bytes(total_sz)
                                ),
                                confirm_label: "Delete Traces".into(),
                                action: Box::new(AppAction::DeleteSelectedLeftovers),
                            }));
                        }
                    }
                });
            });

            ui.add_space(6.0);
            if state.leftover_items.is_empty() {
                ui.label(
                    RichText::new("Clean — zero leftover traces detected.")
                        .size(12.2)
                        .color(theme::ACCENT_EMERALD),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("geek_leftovers_scroll")
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for item in state.leftover_items.iter_mut() {
                            ui.horizontal(|ui| {
                                toggle_switch(ui, &mut item.is_selected);
                                let fg = match item.kind {
                                    LeftoverKind::Directory => theme::ACCENT_GOLD,
                                    LeftoverKind::File => theme::ACCENT_CYAN,
                                    LeftoverKind::RegistryKey | LeftoverKind::RegistryValue => {
                                        theme::ACCENT_ROSE
                                    }
                                };
                                status_badge(ui, item.kind.label(), fg, theme::BG_VOID);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&item.display_path)
                                            .size(11.8)
                                            .monospace()
                                            .color(theme::TEXT_PRIMARY),
                                    )
                                    .truncate(),
                                );
                            });
                        }
                    });
            }
        });
        ui.add_space(10.0);
    }

    // ==================== COMPACT TOOLBAR ====================
    let mut sort_toggled = false;
    ui.horizontal_wrapped(|ui| {
        search_bar(ui, &mut state.uninstaller_search, "Search apps...", 230.0);

        if filter_chip(ui, "All", state.uninstaller_filter_kind.is_none()).clicked() {
            state.uninstaller_filter_kind = None;
        }
        for (k, short) in [
            (InstalledAppKind::Win32x64, "x64"),
            (InstalledAppKind::Win32x86, "x86"),
            (InstalledAppKind::UserHkcu, "User"),
            (InstalledAppKind::StoreUwp, "Store"),
        ] {
            if filter_chip(ui, short, state.uninstaller_filter_kind == Some(k)).clicked() {
                state.uninstaller_filter_kind = Some(k);
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, NavIcon::Refresh, "Rescan Apps", false).clicked() {
                actions.push(AppAction::ScanInstalledApps);
            }
            if subtle_button(
                ui,
                if state.uninstaller_sort_by_size {
                    "Sort: Size"
                } else {
                    "Sort: A–Z"
                },
            )
            .clicked()
            {
                state.uninstaller_sort_by_size = !state.uninstaller_sort_by_size;
                sort_toggled = true;
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("UWP").size(11.5).color(theme::TEXT_MUTED));
                if toggle_switch(ui, &mut state.uninstaller_include_uwp).changed() {
                    actions.push(AppAction::ScanInstalledApps);
                }
            });
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
                    "Indexing installed applications..."
                } else {
                    "No applications indexed yet."
                })
                .size(12.8)
                .color(theme::TEXT_SECONDARY),
            );
        });
        return;
    }

    // ==================== VIRTUALIZED SLEEK ROW LIST ====================
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
                || app.publisher.to_ascii_lowercase().contains(&q);
            if kind_ok && txt_ok {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    let row_height = 60.0;
    egui::ScrollArea::vertical()
        .id_salt("geek_virtual_table")
        .auto_shrink([false, false])
        .show_rows(ui, row_height, matching_indices.len(), |ui, row_range| {
            for row_i in row_range {
                let idx = matching_indices[row_i];
                let app = &state.installed_apps[idx];

                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Left: Name + clean Publisher/Version subtitle
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&app.display_name)
                                            .size(13.8)
                                            .strong()
                                            .color(theme::TEXT_PRIMARY),
                                    )
                                    .truncate(),
                                );
                                if !app.display_version.is_empty() {
                                    ui.label(
                                        RichText::new(format!("v{}", app.display_version))
                                            .size(11.2)
                                            .monospace()
                                            .color(theme::TEXT_MUTED),
                                    );
                                }
                            });
                            ui.label(
                                RichText::new(if app.publisher.is_empty() {
                                    app.kind.label()
                                } else {
                                    &app.publisher
                                })
                                .size(11.5)
                                .color(theme::TEXT_SECONDARY),
                            );
                        });

                        // Right: Size + Clean Uninstall + Compact "•••" Context Menu
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.menu_button(
                                RichText::new("•••").size(13.0).strong().color(theme::TEXT_SECONDARY),
                                |ui| {
                                    ui.set_min_width(155.0);
                                    if ui
                                        .button(
                                            RichText::new("Force Remove & Traces")
                                                .color(theme::ACCENT_ROSE),
                                        )
                                        .clicked()
                                    {
                                        actions.push(AppAction::ScanAppLeftovers(app.id.clone()));
                                        ui.close_menu();
                                    }
                                    if ui.button("Scan Leftovers").clicked() {
                                        actions.push(AppAction::ScanAppLeftovers(app.id.clone()));
                                        ui.close_menu();
                                    }
                                    if let Some(ref loc) = app.install_location {
                                        if ui.button("Open Install Folder").clicked() {
                                            actions.push(AppAction::OpenFolder(loc.clone()));
                                            ui.close_menu();
                                        }
                                    }
                                    if let Some(hive) = app.registry_hive {
                                        if ui.button("Open Registry Key").clicked() {
                                            actions.push(AppAction::OpenRegeditKey {
                                                hive,
                                                subkey: app.registry_subkey.clone(),
                                            });
                                            ui.close_menu();
                                        }
                                    }
                                },
                            );

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Uninstall")
                                            .size(12.2)
                                            .strong()
                                            .color(theme::ACCENT_CYAN),
                                    )
                                    .fill(theme::ACCENT_CYAN_DIM)
                                    .stroke(egui::Stroke::new(
                                        1.0_f32,
                                        theme::ACCENT_CYAN.linear_multiply(0.5),
                                    ))
                                    .min_size(Vec2::new(82.0, 28.0)),
                                )
                                .clicked()
                            {
                                actions.push(AppAction::StandardUninstallApp(app.id.clone()));
                            }

                            if app.estimated_size_bytes > 0 {
                                ui.label(
                                    RichText::new(format_bytes(app.estimated_size_bytes))
                                        .size(12.0)
                                        .monospace()
                                        .color(theme::TEXT_SECONDARY),
                                );
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
}
