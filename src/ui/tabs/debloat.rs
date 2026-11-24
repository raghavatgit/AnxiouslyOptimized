use crate::engine::debloat::PROTECTED_WHITELIST;
use crate::ui::components::{
    card_frame, danger_button, dot_pill, primary_button, search_bar, status_badge, subtle_button,
    toggle_switch,
};
use crate::ui::{theme, AppAction, ConfirmRequest, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal_wrapped(|ui| {
        search_bar(ui, &mut state.bloat_search, "Filter UWP packages...", 240.0);
        dot_pill(
            ui,
            &format!("{} Core Apps Protected", PROTECTED_WHITELIST.len()),
            true,
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let sel = state.bloat_items.iter().filter(|i| i.is_selected).count();
            if sel > 0 && danger_button(ui, &format!("Remove ({sel})")).clicked() {
                let names: Vec<String> = state
                    .bloat_items
                    .iter()
                    .filter(|i| i.is_selected)
                    .map(|i| format!("• {}", i.display_name))
                    .collect();
                actions.push(AppAction::RequestConfirm(ConfirmRequest {
                    title: format!("Remove {sel} UWP App(s)?"),
                    message: names.join("\n"),
                    confirm_label: "Uninstall".into(),
                    action: Box::new(AppAction::RemoveSelectedBloat),
                }));
            }
            if primary_button(ui, "Scan UWP Bloat").clicked() {
                actions.push(AppAction::ScanBloat);
            }
            if !state.bloat_items.is_empty() && subtle_button(ui, "Select All").clicked() {
                for b in state.bloat_items.iter_mut() {
                    b.is_selected = true;
                }
            }
        });
    });

    ui.add_space(10.0);

    if state.bloat_items.is_empty() {
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new("Click 'Scan UWP Bloat' to detect preinstalled removable packages.")
                    .size(12.5)
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
        {
            continue;
        }
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                toggle_switch(ui, &mut item.is_selected);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(&item.display_name)
                                .size(13.5)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        status_badge(ui, &item.category, theme::ACCENT_CYAN, theme::BG_VOID);
                    });
                    ui.label(
                        RichText::new(&item.description)
                            .size(11.8)
                            .color(theme::TEXT_SECONDARY),
                    );
                });
            });
        });
        ui.add_space(4.0);
    }
}
