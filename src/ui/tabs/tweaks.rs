use crate::models::{TweakCategory, TweakOsTarget, TweakRisk};
use crate::ui::components::{
    card_frame, danger_button, emerald_button, primary_button, section_header, status_badge,
    subtle_button,
};
use crate::ui::{theme, AppAction, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(
            ui,
            "System Tweaks",
            "Live-audited registry, power, scheduler, and input tweaks with single-step and batch rollback.",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let sel_cnt = state.tweaks.iter().filter(|t| t.is_selected).count();
            if sel_cnt > 0 {
                if emerald_button(ui, &format!("Apply Selected ({sel_cnt})")).clicked() {
                    actions.push(AppAction::ApplySelectedTweaks);
                }
            }
            if subtle_button(ui, "Refresh Audit").clicked() {
                actions.push(AppAction::AuditTweaks);
            }
            if state.explorer_restart_needed {
                if danger_button(ui, "Restart Explorer.exe").clicked() {
                    actions.push(AppAction::RestartExplorer);
                }
            } else if subtle_button(ui, "Restart Explorer").clicked() {
                actions.push(AppAction::RestartExplorer);
            }
        });
    });

    // Search + Category Filters
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.add_sized(
                [260.0, 28.0],
                egui::TextEdit::singleline(&mut state.tweaks_search)
                    .hint_text("Search tweaks by name or description..."),
            );
            if !state.tweaks_search.is_empty() && subtle_button(ui, "Clear").clicked() {
                state.tweaks_search.clear();
            }

            ui.separator();

            let is_all = state.tweaks_category.is_none();
            if pill_btn(ui, "All", is_all).clicked() {
                state.tweaks_category = None;
            }
            for cat in [
                TweakCategory::WindowsUi,
                TweakCategory::Privacy,
                TweakCategory::Gaming,
                TweakCategory::Speed,
                TweakCategory::Network,
            ] {
                let active = state.tweaks_category == Some(cat);
                if pill_btn(ui, cat.label(), active).clicked() {
                    state.tweaks_category = Some(cat);
                }
            }
        });
    });

    ui.add_space(6.0);

    let search_q = state.tweaks_search.trim().to_ascii_lowercase();
    let cat_filter = state.tweaks_category;

    ui.horizontal(|ui| {
        if subtle_button(ui, "Select Visible Unapplied").clicked() {
            for t in state.tweaks.iter_mut() {
                let cat_ok = cat_filter.map(|c| t.category == c).unwrap_or(true);
                let txt_ok = search_q.is_empty()
                    || t.title.to_ascii_lowercase().contains(&search_q)
                    || t.description.to_ascii_lowercase().contains(&search_q);
                if cat_ok && txt_ok && !t.is_applied {
                    t.is_selected = true;
                }
            }
        }
        if subtle_button(ui, "Clear Selection").clicked() {
            for t in state.tweaks.iter_mut() {
                t.is_selected = false;
            }
        }
        let applied_cnt = state.tweaks.iter().filter(|t| t.is_applied).count();
        ui.label(
            RichText::new(format!("{applied_cnt} / {} currently applied", state.tweaks.len()))
                .size(12.0)
                .monospace()
                .color(theme::TEXT_SECONDARY),
        );
    });

    ui.add_space(6.0);

    for t in state.tweaks.iter_mut() {
        let cat_ok = cat_filter.map(|c| t.category == c).unwrap_or(true);
        let txt_ok = search_q.is_empty()
            || t.title.to_ascii_lowercase().contains(&search_q)
            || t.description.to_ascii_lowercase().contains(&search_q);
        if !cat_ok || !txt_ok {
            continue;
        }

        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut t.is_selected, "");
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&t.title).size(14.0).strong().color(theme::TEXT_PRIMARY));
                        if t.is_applied {
                            status_badge(ui, "APPLIED", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
                        } else {
                            status_badge(ui, "STOCK", theme::TEXT_MUTED, theme::BG_ELEVATED);
                        }
                        let (r_fg, r_bg) = match t.risk {
                            TweakRisk::Safe => (theme::ACCENT_COBALT, theme::ACCENT_COBALT_DIM),
                            TweakRisk::Moderate => (theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM),
                            TweakRisk::Advanced => (theme::ACCENT_ROSE, theme::ACCENT_ROSE_DIM),
                        };
                        status_badge(ui, t.risk.label(), r_fg, r_bg);
                        status_badge(ui, t.category.label(), theme::TEXT_SECONDARY, theme::BG_ELEVATED);
                        if matches!(t.os_target, TweakOsTarget::Win11Only) {
                            status_badge(ui, "Win 11", theme::ACCENT_CYAN_SLATE, theme::BG_ELEVATED);
                        }
                        if t.requires_restart {
                            status_badge(ui, "Reboot Required", theme::ACCENT_AMBER, theme::ACCENT_AMBER_DIM);
                        } else if t.requires_explorer_restart {
                            status_badge(ui, "Explorer Restart", theme::TEXT_SECONDARY, theme::BG_ELEVATED);
                        }
                    });
                    ui.label(RichText::new(&t.description).size(12.5).color(theme::TEXT_SECONDARY));
                    if let Some(note) = &t.note {
                        ui.label(RichText::new(note).size(11.5).color(theme::TEXT_MUTED));
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if t.is_applied {
                        if danger_button(ui, "Revert").clicked() {
                            actions.push(AppAction::RevertSingleTweak(t.id.clone()));
                        }
                    } else if primary_button(ui, "Apply").clicked() {
                        actions.push(AppAction::ApplySingleTweak(t.id.clone()));
                    }
                });
            });
        });
        ui.add_space(4.0);
    }
}

fn pill_btn(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
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
