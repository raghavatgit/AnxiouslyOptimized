use crate::engine::format_bytes;
use crate::ui::components::{
    card_frame, danger_button, emerald_button, kv_row, primary_button, section_header,
    status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(
            ui,
            "Safety & Rollback Center",
            "Typed LIFO transaction rollback journals, on-demand Emergency .bat generator, and Volume Shadow Copy restore points.",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if primary_button(ui, "Refresh Safety Data").clicked() {
                actions.push(AppAction::RefreshSafety);
            }
        });
    });

    // VSS & System Restore Card
    card_frame().show(ui, |ui| {
        let v = &state.vss_health;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Windows System Protection & Restore Points")
                    .size(15.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            if v.service_exists && !v.is_disabled && v.sufficient_space && v.restore_enabled != Some(false) {
                status_badge(ui, "VSS HEALTHY", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
            } else {
                status_badge(ui, "VSS ATTENTION REQUIRED", theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if subtle_button(ui, "Open Windows Restore UI (rstrui.exe)").clicked() {
                    actions.push(AppAction::OpenSystemRestoreUi);
                }
            });
        });
        ui.add_space(4.0);
        kv_row(ui, "VSS Status", &v.summary, false);
        kv_row(
            ui,
            "System Drive Free",
            &format!("{} free on {}", format_bytes(v.system_drive_free_bytes), v.system_drive),
            true,
        );

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_sized(
                [300.0, 28.0],
                egui::TextEdit::singleline(&mut state.restore_desc_input)
                    .hint_text("Restore point description..."),
            );
            if emerald_button(ui, "Create Restore Point Now").clicked() {
                let desc = if state.restore_desc_input.trim().is_empty() {
                    "Optimizer by Raghav Manual Checkpoint".to_string()
                } else {
                    state.restore_desc_input.trim().to_string()
                };
                actions.push(AppAction::CreateRestorePoint(desc));
            }
        });

        if !state.restore_points.is_empty() {
            ui.add_space(6.0);
            egui::Grid::new("rp_grid").striped(true).num_columns(4).show(ui, |ui| {
                for h in ["Seq #", "Created At", "Type", "Description"] {
                    ui.label(RichText::new(h).size(11.5).strong().color(theme::TEXT_MUTED));
                }
                ui.end_row();
                for rp in state.restore_points.iter().take(8) {
                    ui.label(RichText::new(rp.sequence_number.to_string()).monospace().size(12.0));
                    ui.label(RichText::new(&rp.created_at).monospace().size(12.0));
                    ui.label(RichText::new(&rp.restore_type).size(12.0));
                    ui.label(RichText::new(&rp.description).size(12.0));
                    ui.end_row();
                }
            });
        }
    });

    ui.add_space(10.0);

    // Transaction Journals
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Transaction Undo Journals ({})", state.journals.len()))
                .size(15.5)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.label(
            RichText::new("  — Emergency .bat files are exported to Desktop ONLY when requested.")
                .size(12.0)
                .color(theme::TEXT_SECONDARY),
        );
    });
    ui.add_space(4.0);

    if state.journals.is_empty() {
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new("No transaction journals recorded yet. Every tweak, preset, DNS apply, and config patch creates one automatically.")
                    .size(13.0)
                    .color(theme::TEXT_SECONDARY),
            );
        });
        return;
    }

    for j in &state.journals {
        card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&j.title).size(14.0).strong().color(theme::TEXT_PRIMARY));
                        if j.rolled_back {
                            status_badge(ui, "ROLLED BACK", theme::TEXT_MUTED, theme::BG_ELEVATED);
                        } else {
                            status_badge(ui, "ACTIVE", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
                        }
                        status_badge(
                            ui,
                            &format!("{} step(s)", j.steps.len()),
                            theme::ACCENT_COBALT,
                            theme::BG_ELEVATED,
                        );
                    });
                    ui.label(
                        RichText::new(format!("{}  |  {}", j.journal_id, j.timestamp_iso))
                            .size(11.5)
                            .monospace()
                            .color(theme::TEXT_SECONDARY),
                    );
                    if let Some(err) = &j.last_error {
                        ui.label(RichText::new(err).size(11.5).color(theme::ACCENT_ROSE));
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Delete").clicked() {
                        actions.push(AppAction::DeleteJournal(j.journal_id.clone()));
                    }
                    if subtle_button(ui, "Export Emergency .bat").clicked() {
                        actions.push(AppAction::ExportJournalBat(j.journal_id.clone()));
                    }
                    if !j.rolled_back {
                        if danger_button(ui, "Rollback Now").clicked() {
                            actions.push(AppAction::RollbackJournal(j.journal_id.clone()));
                        }
                    }
                });
            });
        });
        ui.add_space(4.0);
    }
}
