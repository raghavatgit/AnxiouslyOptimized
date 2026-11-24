use crate::models::{TweakCategory, TweakOsTarget, TweakRisk};
use crate::ui::components::{
    action_switch, card_frame, danger_button, filter_chip, icon_button, search_bar,
    section_header, status_badge, subtle_button, NavIcon,
};
use crate::ui::{theme, AppAction, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    let applied_cnt = state.tweaks.iter().filter(|t| t.is_applied).count();
    let total_cnt = state.tweaks.len();

    ui.horizontal(|ui| {
        section_header(
            ui,
            "System Tweaks",
            &format!("{applied_cnt} of {total_cnt} active"),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, NavIcon::Refresh, "Re-audit tweaks", false).clicked() {
                actions.push(AppAction::AuditTweaks);
            }
            if state.explorer_restart_needed {
                if danger_button(ui, "Restart Explorer").clicked() {
                    actions.push(AppAction::RestartExplorer);
                }
            } else if subtle_button(ui, "Restart Explorer").clicked() {
                actions.push(AppAction::RestartExplorer);
            }
        });
    });

    // ==================== FILTER & SEARCH BAR ====================
    ui.horizontal_wrapped(|ui| {
        search_bar(ui, &mut state.tweaks_search, "Filter tweaks...", 240.0);
        ui.add_space(4.0);

        if filter_chip(ui, "All", state.tweaks_category.is_none()).clicked() {
            state.tweaks_category = None;
        }
        for cat in [
            TweakCategory::Gaming,
            TweakCategory::Speed,
            TweakCategory::WindowsUi,
            TweakCategory::Privacy,
            TweakCategory::Network,
        ] {
            if filter_chip(ui, cat.label(), state.tweaks_category == Some(cat)).clicked() {
                state.tweaks_category = Some(cat);
            }
        }
    });

    ui.add_space(10.0);

    // ==================== CLEAN SINGLE-SWITCH TWEAK CARDS ====================
    let search_q = state.tweaks_search.trim().to_ascii_lowercase();
    let cat_filter = state.tweaks_category;

    for t in state.tweaks.iter() {
        let cat_ok = cat_filter.map(|c| t.category == c).unwrap_or(true);
        let txt_ok = search_q.is_empty()
            || t.title.to_ascii_lowercase().contains(&search_q)
            || t.description.to_ascii_lowercase().contains(&search_q);
        if !cat_ok || !txt_ok {
            continue;
        }

        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(&t.title)
                                .size(13.8)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );

                        // Minimalist risk badge only when Moderate/Advanced, or Win11 tag
                        match t.risk {
                            TweakRisk::Safe => {}
                            TweakRisk::Moderate => {
                                status_badge(
                                    ui,
                                    "MODERATE",
                                    theme::ACCENT_AMBER,
                                    theme::ACCENT_AMBER_DIM,
                                );
                            }
                            TweakRisk::Advanced => {
                                status_badge(
                                    ui,
                                    "ADVANCED",
                                    theme::ACCENT_ROSE,
                                    theme::ACCENT_ROSE_DIM,
                                );
                            }
                        }
                        if matches!(t.os_target, TweakOsTarget::Win11Only) {
                            status_badge(ui, "Win11", theme::ACCENT_CYAN, theme::BG_VOID);
                        }
                        if t.requires_restart {
                            status_badge(ui, "Reboot", theme::TEXT_MUTED, theme::BG_VOID);
                        }
                    });

                    let resp = ui.label(
                        RichText::new(&t.description)
                            .size(12.0)
                            .color(theme::TEXT_SECONDARY),
                    );
                    if let Some(note) = &t.note {
                        resp.on_hover_text(note);
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let sw = action_switch(ui, &t.id, t.is_applied);
                    if sw.clicked() {
                        if t.is_applied {
                            actions.push(AppAction::RevertSingleTweak(t.id.clone()));
                        } else {
                            actions.push(AppAction::ApplySingleTweak(t.id.clone()));
                        }
                    }
                });
            });
        });
        ui.add_space(5.0);
    }
}
