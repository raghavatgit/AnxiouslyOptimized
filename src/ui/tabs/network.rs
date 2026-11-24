use crate::models::PingState;
use crate::ui::components::{
    card_frame, danger_button, emerald_button, kv_row, primary_button, progress_bar,
    section_header, status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, ConfirmRequest, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(ui, "Network & DNS", "ICMP resolver benchmark & stack tools.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if primary_button(ui, "Benchmark DNS").clicked() {
                actions.push(AppAction::BenchmarkDns);
            }
            if emerald_button(ui, "Flush DNS").clicked() {
                actions.push(AppAction::FlushDnsCache);
            }
        });
    });

    let adapters = &state.system_report.network_adapters;
    if state.selected_adapter_idx >= adapters.len() {
        state.selected_adapter_idx = 0;
    }

    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("ACTIVE INTERFACE")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(6.0);
            if adapters.is_empty() {
                ui.label(RichText::new("No active adapters.").color(theme::ACCENT_AMBER));
            } else {
                egui::ComboBox::from_id_salt("adapter_combo")
                    .selected_text(&adapters[state.selected_adapter_idx].friendly_name)
                    .width(ui.available_width().min(280.0))
                    .show_ui(ui, |ui| {
                        for (idx, a) in adapters.iter().enumerate() {
                            ui.selectable_value(
                                &mut state.selected_adapter_idx,
                                idx,
                                format!("{} ({} Mbps)", a.friendly_name, a.link_speed_mbps),
                            );
                        }
                    });
                let ad = &adapters[state.selected_adapter_idx];
                ui.add_space(6.0);
                kv_row(ui, "IPv4", &ad.ipv4_addrs.join(", "), true);
                kv_row(
                    ui,
                    "DNS",
                    &if ad.dns_servers.is_empty() {
                        "Automatic (DHCP)".to_string()
                    } else {
                        ad.dns_servers.join(", ")
                    },
                    true,
                );
                ui.add_space(6.0);
                if subtle_button(ui, "Reset to DHCP").clicked() {
                    actions.push(AppAction::ResetPrimaryDnsDhcp);
                }
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.label(
                RichText::new("CUSTOM DNS & STACK")
                    .size(10.5)
                    .strong()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_sized(
                    [125.0, 28.0],
                    egui::TextEdit::singleline(&mut state.dns_custom_primary).hint_text("1.1.1.1"),
                );
                ui.add_sized(
                    [125.0, 28.0],
                    egui::TextEdit::singleline(&mut state.dns_custom_secondary).hint_text("1.0.0.1"),
                );
                if primary_button(ui, "Set").clicked() {
                    actions.push(AppAction::ApplyDnsToPrimary {
                        primary: state.dns_custom_primary.clone(),
                        secondary: state.dns_custom_secondary.clone(),
                        name: "Custom DNS".to_string(),
                    });
                }
            });
            ui.add_space(10.0);
            if danger_button(ui, "Reset Winsock & TCP/IP").clicked() {
                actions.push(AppAction::RequestConfirm(ConfirmRequest {
                    title: "Reset Network Stack?".into(),
                    message: "Runs netsh winsock reset & int ip reset (Reboot required).".into(),
                    confirm_label: "Reset Stack".into(),
                    action: Box::new(AppAction::ResetWinsockStack),
                }));
            }
        });
    });

    ui.add_space(12.0);

    // DNS Providers with Visual Latency Bars
    for p in &state.dns_providers {
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&p.name)
                                .size(13.8)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new(format!("{} • {}", p.primary_dns, p.secondary_dns))
                                .size(11.5)
                                .monospace()
                                .color(theme::TEXT_MUTED),
                        );
                        if p.is_fastest {
                            status_badge(
                                ui,
                                "FASTEST",
                                theme::ACCENT_EMERALD,
                                theme::ACCENT_EMERALD_DIM,
                            );
                        }
                    });

                    if let PingState::Measured { rtt_ms, loss_pct } = p.state {
                        let norm = (1.0 - (rtt_ms as f32 / 120.0)).clamp(0.08, 1.0);
                        let col = if rtt_ms <= 25 && loss_pct == 0 {
                            theme::ACCENT_EMERALD
                        } else if rtt_ms <= 60 {
                            theme::ACCENT_CYAN
                        } else {
                            theme::ACCENT_AMBER
                        };
                        progress_bar(ui, norm, "", col);
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "Apply").clicked() {
                        actions.push(AppAction::ApplyDnsToPrimary {
                            primary: p.primary_dns.clone(),
                            secondary: p.secondary_dns.clone(),
                            name: p.name.clone(),
                        });
                    }
                    match p.state {
                        PingState::NotTested => {}
                        PingState::Failed => {
                            status_badge(ui, "Timeout", theme::ACCENT_ROSE, theme::ACCENT_ROSE_DIM);
                        }
                        PingState::Measured { rtt_ms, .. } => {
                            ui.label(
                                RichText::new(format!("{rtt_ms} ms"))
                                    .size(13.0)
                                    .strong()
                                    .monospace()
                                    .color(theme::ACCENT_CYAN),
                            );
                        }
                    }
                });
            });
        });
        ui.add_space(5.0);
    }
}
