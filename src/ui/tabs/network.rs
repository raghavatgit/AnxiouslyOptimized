use crate::models::PingState;
use crate::ui::components::{
    card_frame, danger_button, emerald_button, kv_row, primary_button, section_header,
    status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, ConfirmRequest, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(
            ui,
            "Network & DNS Benchmark",
            "Concurrent native Win32 ICMP latency benchmark (including <1 ms / 0 ms fiber) and TCP/IP repair tools.",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if primary_button(ui, "Run DNS Benchmark").clicked() {
                actions.push(AppAction::BenchmarkDns);
            }
            if emerald_button(ui, "Flush DNS Cache").clicked() {
                actions.push(AppAction::FlushDnsCache);
            }
        });
    });

    // Adapter Selector & Current DNS + Custom/Repair Strip
    let adapters = &state.system_report.network_adapters;
    if state.selected_adapter_idx >= adapters.len() {
        state.selected_adapter_idx = 0;
    }

    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(RichText::new("Target Network Adapter").size(14.0).strong().color(theme::ACCENT_COBALT));
            ui.add_space(4.0);
            if adapters.is_empty() {
                ui.label(RichText::new("No active adapters detected. Click Rescan System.").color(theme::ACCENT_AMBER));
            } else {
                egui::ComboBox::from_id_salt("adapter_combo")
                    .selected_text(&adapters[state.selected_adapter_idx].friendly_name)
                    .width(ui.available_width().min(320.0))
                    .show_ui(ui, |ui| {
                        for (idx, a) in adapters.iter().enumerate() {
                            let label = format!(
                                "{} ({} Mbps){}",
                                a.friendly_name,
                                a.link_speed_mbps,
                                if a.is_physical_primary { " [PRIMARY]" } else { "" }
                            );
                            ui.selectable_value(&mut state.selected_adapter_idx, idx, label);
                        }
                    });
                let ad = &adapters[state.selected_adapter_idx];
                ui.add_space(4.0);
                kv_row(ui, "Description", &ad.description, false);
                kv_row(ui, "Link Speed", &format!("{} Mbps ({})", ad.link_speed_mbps, ad.adapter_kind), true);
                kv_row(ui, "IPv4 / Gateway", &format!("{} | {}", ad.ipv4_addrs.join(", "), ad.gateways.join(", ")), true);
                kv_row(
                    ui,
                    "Current DNS",
                    &if ad.dns_servers.is_empty() {
                        "DHCP / Automatic".to_string()
                    } else {
                        ad.dns_servers.join(", ")
                    },
                    true,
                );
                ui.add_space(6.0);
                if subtle_button(ui, "Reset Selected Adapter DNS to DHCP").clicked() {
                    actions.push(AppAction::ResetPrimaryDnsDhcp);
                }
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.label(RichText::new("Custom DNS & Stack Repairs").size(14.0).strong().color(theme::ACCENT_GOLD));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Primary:");
                ui.add_sized(
                    [130.0, 26.0],
                    egui::TextEdit::singleline(&mut state.dns_custom_primary).hint_text("1.1.1.1"),
                );
                ui.label("Secondary:");
                ui.add_sized(
                    [130.0, 26.0],
                    egui::TextEdit::singleline(&mut state.dns_custom_secondary).hint_text("1.0.0.1"),
                );
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if primary_button(ui, "Apply Custom DNS").clicked() {
                    actions.push(AppAction::ApplyDnsToPrimary {
                        primary: state.dns_custom_primary.clone(),
                        secondary: state.dns_custom_secondary.clone(),
                        name: "Custom DNS".to_string(),
                    });
                }
                if danger_button(ui, "Reset Winsock & TCP/IP Stack").clicked() {
                    actions.push(AppAction::RequestConfirm(ConfirmRequest {
                        title: "Confirm Winsock & TCP/IP Stack Reset".into(),
                        message: "Runs `netsh winsock reset` and `netsh int ip reset`. This repairs corrupted LSP/socket hooks and resets TCP/IP configuration (requires PC reboot afterward).".into(),
                        confirm_label: "Reset Network Stack".into(),
                        action: Box::new(AppAction::ResetWinsockStack),
                    }));
                }
            });
        });
    });

    ui.add_space(10.0);

    // DNS Benchmark Providers
    for p in &state.dns_providers {
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&p.name).size(14.5).strong().color(theme::TEXT_PRIMARY));
                        if p.is_fastest {
                            status_badge(ui, "FASTEST", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
                        }
                        match p.state {
                            PingState::NotTested => {
                                status_badge(ui, "NOT TESTED", theme::TEXT_MUTED, theme::BG_ELEVATED);
                            }
                            PingState::Failed => {
                                status_badge(ui, "TIMEOUT / 100% LOSS", theme::ACCENT_ROSE, theme::ACCENT_ROSE_DIM);
                            }
                            PingState::Measured { rtt_ms, loss_pct } => {
                                let fg = if rtt_ms <= 25 && loss_pct == 0 {
                                    theme::ACCENT_EMERALD
                                } else if rtt_ms <= 60 {
                                    theme::ACCENT_AMBER
                                } else {
                                    theme::ACCENT_ROSE
                                };
                                status_badge(
                                    ui,
                                    &format!("{rtt_ms} ms | {loss_pct}% loss"),
                                    fg,
                                    theme::BG_ELEVATED,
                                );
                            }
                        }
                    });
                    ui.label(
                        RichText::new(format!("{} / {}  —  {}", p.primary_dns, p.secondary_dns, p.description))
                            .size(12.0)
                            .color(theme::TEXT_SECONDARY),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button(ui, "Apply DNS").clicked() {
                        actions.push(AppAction::ApplyDnsToPrimary {
                            primary: p.primary_dns.clone(),
                            secondary: p.secondary_dns.clone(),
                            name: p.name.clone(),
                        });
                    }
                });
            });
        });
        ui.add_space(4.0);
    }
}
