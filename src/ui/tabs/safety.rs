use crate::engine::format_bytes;
use crate::ui::components::{
    bento_frame, card_frame, danger_button, dot_pill, emerald_button, icon_button, section_header,
    status_badge, subtle_button, NavIcon,
};
use crate::ui::{theme, AppAction, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(ui, "Safety & Rollback", "Atomic undo timeline & System Restore.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, NavIcon::Refresh, "Refresh Safety", false).clicked() {
                actions.push(AppAction::RefreshSafety);
            }
            if subtle_button(ui, "System Restore UI").clicked() {
                actions.push(AppAction::OpenSystemRestoreUi);
            }
        });
    });

    // ==================== SYSTEM RESTORE CHECKPOINT CARD ====================
    let v = &state.vss_health;
    let vss_ok = v.service_exists && !v.is_disabled && v.sufficient_space && v.restore_enabled != Some(false);

    bento_frame(None).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Restore Checkpoint")
                    .size(15.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            dot_pill(ui, if vss_ok { "VSS Ready" } else { "VSS Disabled" }, vss_ok);
            ui.label(
                RichText::new(format!(
                    "{} free on {}",
                    format_bytes(v.system_drive_free_bytes),
                    v.system_drive
                ))
                .size(11.5)
                .monospace()
                .color(theme::TEXT_MUTED),
            );
        });

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_sized(
                [260.0, 30.0],
                egui::TextEdit::singleline(&mut state.restore_desc_input)
                    .hint_text("Checkpoint label..."),
            );
            if emerald_button(ui, "Create Checkpoint").clicked() {
                let desc = if state.restore_desc_input.trim().is_empty() {
                    "Optimizer Checkpoint".to_string()
                } else {
                    state.restore_desc_input.trim().to_string()
                };
                actions.push(AppAction::CreateRestorePoint(desc));
            }
        });

        if !state.restore_points.is_empty() {
            ui.add_space(8.0);
            for rp in state.restore_points.iter().take(4) {
                ui.horizontal(|ui| {
                    status_badge(
                        ui,
                        &format!("#{}", rp.sequence_number),
                        theme::ACCENT_CYAN,
                        theme::BG_VOID,
                    );
                    ui.label(
                        RichText::new(&rp.created_at)
                            .size(11.5)
                            .monospace()
                            .color(theme::TEXT_MUTED),
                    );
                    ui.label(RichText::new(&rp.description).size(12.2).color(theme::TEXT_PRIMARY));
                });
            }
        }
    });

    ui.add_space(12.0);

    // ==================== UNDO TIMELINE ====================
    ui.label(
        RichText::new(format!("UNDO TIMELINE ({})", state.journals.len()))
            .size(10.5)
            .strong()
            .color(theme::TEXT_MUTED),
    );
    ui.add_space(6.0);

    if state.journals.is_empty() {
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new("No changes recorded yet — every tweak & preset automatically saves an undo point here.")
                    .size(12.2)
                    .color(theme::TEXT_SECONDARY),
            );
        });
        return;
    }

    for j in &state.journals {
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&j.title)
                                .size(13.8)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        status_badge(
                            ui,
                            &format!("{} steps", j.steps.len()),
                            if j.rolled_back {
                                theme::TEXT_MUTED
                            } else {
                                theme::ACCENT_EMERALD
                            },
                            theme::BG_VOID,
                        );
                    });
                    ui.label(
                        RichText::new(&j.timestamp_iso)
                            .size(11.2)
                            .monospace()
                            .color(theme::TEXT_MUTED),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button(RichText::new("•••").strong(), |ui| {
                        if ui.button("Export Emergency .bat").clicked() {
                            actions.push(AppAction::ExportJournalBat(j.journal_id.clone()));
                            ui.close_menu();
                        }
                        if ui
                            .button(RichText::new("Delete Entry").color(theme::ACCENT_ROSE))
                            .clicked()
                        {
                            actions.push(AppAction::DeleteJournal(j.journal_id.clone()));
                            ui.close_menu();
                        }
                    });

                    if j.rolled_back {
                        status_badge(ui, "Reverted", theme::TEXT_MUTED, theme::BG_VOID);
                    } else if danger_button(ui, "Undo").clicked() {
                        actions.push(AppAction::RollbackJournal(j.journal_id.clone()));
                    }
                });
            });
        });
        ui.add_space(5.0);
    }
}
