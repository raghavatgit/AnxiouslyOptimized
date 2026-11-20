use crate::engine::debloat::PROTECTED_WHITELIST;
use crate::ui::components::{
    card_frame, danger_button, primary_button, section_header, status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, ConfirmRequest, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(
            ui,
            "Windows App Debloater",
            "Exact-match removal for preinstalled UWP bloat. Critical Store, Defender, Calculator, Media Player & VCLibs packages are hard-protected.",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let sel = state.bloat_items.iter().filter(|i| i.is_selected).count();
            if sel > 0 {
                if danger_button(ui, &format!("Remove Selected ({sel})")).clicked() {
                    let names: Vec<String> = state
                        .bloat_items
                        .iter()
                        .filter(|i| i.is_selected)
                        .map(|i| format!("• {} ({})", i.display_name, i.package_name))
                        .collect();
                    actions.push(AppAction::RequestConfirm(ConfirmRequest {
                        title: format!("Confirm Removal of {sel} Windows App(s)"),
                        message: format!(
                            "The following packages will be uninstalled for all users and deprovisioned:\n\n{}\n\nProtected core Windows apps remain whitelisted.",
                            names.join("\n")
                        ),
                        confirm_label: "Uninstall Selected Apps".into(),
                        action: Box::new(AppAction::RemoveSelectedBloat),
                    }));
                }
            }
            if primary_button(ui, "Scan Installed Apps").clicked() {
                actions.push(AppAction::ScanBloat);
            }
        });
    });

    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            status_badge(
                ui,
                &format!("{} PROTECTED PACKAGES LOCKED", PROTECTED_WHITELIST.len()),
                theme::ACCENT_EMERALD,
                theme::ACCENT_EMERALD_DIM,
            );
            ui.label(
                RichText::new("Zero items are ever pre-selected by default. Exact Package Name equality only — never wildcard `-like`.")
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY),
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [260.0, 28.0],
                egui::TextEdit::singleline(&mut state.bloat_search)
                    .hint_text("Filter detected bloatware apps..."),
            );
            if subtle_button(ui, "Clear Checks").clicked() {
                for b in state.bloat_items.iter_mut() {
                    b.is_selected = false;
                }
            }
            if subtle_button(ui, "Select All Detected").clicked() {
                let q = state.bloat_search.trim().to_ascii_lowercase();
                for b in state.bloat_items.iter_mut() {
                    if q.is_empty()
                        || b.display_name.to_ascii_lowercase().contains(&q)
                        || b.package_name.to_ascii_lowercase().contains(&q)
                    {
                        b.is_selected = true;
                    }
                }
            }
        });
    });

    ui.add_space(8.0);

    if state.bloat_items.is_empty() {
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new("No removable catalog bloat apps currently scanned or detected. Click 'Scan Installed Apps' to query live AppxPackages.")
                    .size(13.0)
                    .color(theme::TEXT_SECONDARY),
            );
        });
        return;
    }

    let q = state.bloat_search.trim().to_ascii_lowercase();
    for item in state.bloat_items.iter_mut() {
        if !q.is_empty()
            && !item.display_name.to_ascii_lowercase().contains(&q)
            && !item.package_name.to_ascii_lowercase().contains(&q)
            && !item.category.to_ascii_lowercase().contains(&q)
        {
            continue;
        }
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut item.is_selected, "");
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&item.display_name).size(14.0).strong().color(theme::TEXT_PRIMARY));
                        status_badge(ui, &item.category, theme::ACCENT_GOLD, theme::BG_ELEVATED);
                        ui.label(RichText::new(&item.package_name).size(11.5).monospace().color(theme::TEXT_MUTED));
                    });
                    ui.label(RichText::new(&item.description).size(12.0).color(theme::TEXT_SECONDARY));
                });
            });
        });
        ui.add_space(4.0);
    }
}
