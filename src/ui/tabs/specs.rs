use crate::engine::format_bytes;
use crate::ui::components::{
    card_frame, dot_pill, icon_button, kv_row, primary_button, progress_bar, section_header,
    status_badge, subtle_button, NavIcon,
};
use crate::ui::{theme, AppAction, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(ui, "Hardware", "DXGI, SMBIOS & Win32 topology.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, NavIcon::Refresh, "Rescan Hardware", false).clicked() {
                actions.push(AppAction::RescanSystem);
            }
            if primary_button(ui, "Export .md").clicked() {
                actions.push(AppAction::ExportSpecsMarkdown);
            }
            if subtle_button(ui, "Copy").clicked() {
                actions.push(AppAction::CopyText {
                    text: state.system_report.to_markdown_summary(),
                    label: "Hardware summary copied".to_string(),
                });
            }
        });
    });

    let r = &state.system_report;

    ui.columns(2, |cols| {
        // CPU
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("PROCESSOR")
                    .size(10.5)
                    .strong()
                    .color(theme::ACCENT_CYAN),
            );
            ui.add_space(6.0);
            kv_row(ui, "Model", &r.cpu.brand, false);
            kv_row(
                ui,
                "Cores",
                &format!("{}C / {}T ({})", r.cpu.physical_cores, r.cpu.logical_threads, r.cpu.topology_label),
                true,
            );
            kv_row(ui, "Clocks", &format!("{} / {} MHz", r.cpu.base_mhz, r.cpu.max_mhz), true);
            kv_row(
                ui,
                "Cache",
                &format!("L1 {}K • L2 {}K • L3 {}K", r.cpu.l1_kb, r.cpu.l2_kb, r.cpu.l3_kb),
                true,
            );
            kv_row(ui, "Socket", &r.cpu.socket, false);
        });

        // Platform & Security
        card_frame().show(&mut cols[1], |ui| {
            ui.label(
                RichText::new("PLATFORM & SECURITY")
                    .size(10.5)
                    .strong()
                    .color(theme::ACCENT_EMERALD),
            );
            ui.add_space(6.0);
            kv_row(
                ui,
                "OS",
                &format!("{} {} ({}.{})", r.platform.os_name, r.platform.display_version, r.platform.build_number, r.platform.ubr),
                false,
            );
            kv_row(ui, "Board", &r.platform.motherboard, false);
            kv_row(ui, "BIOS", &format!("{} ({})", r.platform.bios_version, r.platform.bios_date), true);
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                dot_pill(ui, "Secure Boot", r.security.secure_boot == Some(true));
                dot_pill(ui, "TPM", r.security.tpm_enabled || r.security.tpm_present);
                dot_pill(ui, "HAGS", r.security.hags_enabled);
                dot_pill(ui, "VBS Off", !r.security.vbs_active);
            });
        });
    });

    ui.add_space(10.0);

    ui.columns(2, |cols| {
        // GPUs
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("GRAPHICS")
                    .size(10.5)
                    .strong()
                    .color(theme::ACCENT_GOLD),
            );
            ui.add_space(6.0);
            for (i, gpu) in r.gpus.iter().enumerate() {
                if i > 0 {
                    ui.add_space(6.0);
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&gpu.name).size(13.0).strong().color(theme::TEXT_PRIMARY));
                    status_badge(
                        ui,
                        if gpu.is_integrated { "iGPU" } else { "dGPU" },
                        if gpu.is_integrated { theme::TEXT_SECONDARY } else { theme::ACCENT_GOLD },
                        theme::BG_VOID,
                    );
                });
                kv_row(ui, "VRAM", &format_bytes(gpu.vram_bytes), true);
                kv_row(ui, "Driver", &gpu.driver_version, true);
            }
        });

        // Displays
        card_frame().show(&mut cols[1], |ui| {
            ui.label(
                RichText::new("DISPLAYS")
                    .size(10.5)
                    .strong()
                    .color(theme::ACCENT_COBALT),
            );
            ui.add_space(6.0);
            for d in &r.displays {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&d.monitor_name).size(13.0).strong());
                    if d.is_primary {
                        status_badge(ui, "PRIMARY", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
                    }
                });
                kv_row(ui, "Mode", &format!("{}×{} @ {} Hz", d.width, d.height, d.refresh_hz), true);
            }
        });
    });

    ui.add_space(10.0);

    // Memory & Storage
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(
                RichText::new("MEMORY DIMMS")
                    .size(10.5)
                    .strong()
                    .color(theme::ACCENT_EMERALD),
            );
            ui.add_space(6.0);
            kv_row(
                ui,
                "Total",
                &format!(
                    "{} {} @ {} MT/s ({}/{})",
                    format_bytes(r.memory.total_phys_bytes),
                    r.memory.memory_type,
                    r.memory.speed_mts,
                    r.memory.used_slots,
                    r.memory.total_slots
                ),
                true,
            );
            ui.add_space(4.0);
            for d in &r.memory.dimms {
                ui.label(
                    RichText::new(format!(
                        "{}  •  {} {} @ {} MT/s ({})",
                        d.slot,
                        format_bytes(d.capacity_bytes),
                        d.memory_type,
                        d.configured_mts,
                        d.manufacturer
                    ))
                    .size(11.5)
                    .monospace()
                    .color(theme::TEXT_SECONDARY),
                );
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.label(
                RichText::new("DRIVES")
                    .size(10.5)
                    .strong()
                    .color(theme::ACCENT_CYAN),
            );
            ui.add_space(6.0);
            for disk in &r.physical_disks {
                ui.label(
                    RichText::new(format!(
                        "{}  •  {} {} ({})",
                        disk.friendly_name,
                        format_bytes(disk.size_bytes),
                        disk.bus_type,
                        disk.health_status
                    ))
                    .size(11.8)
                    .monospace()
                    .color(theme::TEXT_PRIMARY),
                );
            }
            ui.add_space(6.0);
            for vol in &r.volumes {
                progress_bar(
                    ui,
                    (vol.used_pct / 100.0).clamp(0.0, 1.0),
                    &format!("{} {} free", vol.mount, format_bytes(vol.free_bytes)),
                    theme::ACCENT_CYAN,
                );
                ui.add_space(4.0);
            }
        });
    });
}
