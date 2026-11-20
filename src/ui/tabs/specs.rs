use crate::engine::format_bytes;
use crate::ui::components::{
    card_frame, kv_row, primary_button, progress_bar, section_header, status_badge, subtle_button,
};
use crate::ui::{theme, AppAction, UiState};
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &UiState, actions: &mut Vec<AppAction>) {
    ui.horizontal(|ui| {
        section_header(
            ui,
            "Whole-System Specifications",
            "Complete native Win32, DXGI, SMBIOS & WMI hardware topology report.",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if primary_button(ui, "Rescan Hardware").clicked() {
                actions.push(AppAction::RescanSystem);
            }
            if subtle_button(ui, "Export Markdown (.md)").clicked() {
                actions.push(AppAction::ExportSpecsMarkdown);
            }
            if subtle_button(ui, "Copy Summary").clicked() {
                actions.push(AppAction::CopyText {
                    text: state.system_report.to_markdown_summary(),
                    label: "System hardware summary copied to clipboard".to_string(),
                });
            }
        });
    });

    let r = &state.system_report;

    // CPU + Platform Row
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(RichText::new("Processor (CPU)").size(14.0).strong().color(theme::ACCENT_COBALT));
            ui.add_space(4.0);
            kv_row(ui, "Model", &r.cpu.brand, false);
            kv_row(ui, "Vendor / Arch", &format!("{} | {}", r.cpu.vendor, r.cpu.architecture), true);
            kv_row(ui, "Socket", &r.cpu.socket, false);
            kv_row(
                ui,
                "Topology",
                &format!("{} Physical / {} Logical ({})", r.cpu.physical_cores, r.cpu.logical_threads, r.cpu.topology_label),
                true,
            );
            if r.cpu.p_core_mask != 0 {
                kv_row(ui, "P-Core Mask", &format!("0x{:X}", r.cpu.p_core_mask), true);
            }
            kv_row(
                ui,
                "Clocks (Base / Max)",
                &format!("{} MHz / {} MHz", r.cpu.base_mhz, r.cpu.max_mhz),
                true,
            );
            kv_row(
                ui,
                "Cache",
                &format!("L1 {} KB | L2 {} KB | L3 {} KB", r.cpu.l1_kb, r.cpu.l2_kb, r.cpu.l3_kb),
                true,
            );
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.label(RichText::new("Platform, BIOS & Security").size(14.0).strong().color(theme::ACCENT_EMERALD));
            ui.add_space(4.0);
            kv_row(
                ui,
                "Operating System",
                &format!("{} {} (Build {}.{})", r.platform.os_name, r.platform.display_version, r.platform.build_number, r.platform.ubr),
                false,
            );
            kv_row(ui, "Host / User", &format!("{} / {}", r.platform.computer_name, r.platform.user_name), true);
            kv_row(ui, "Motherboard", &r.platform.motherboard, false);
            kv_row(ui, "BIOS", &format!("{} | {} ({})", r.platform.bios_vendor, r.platform.bios_version, r.platform.bios_date), true);
            kv_row(ui, "Chassis / Battery", &format!(
                "{} | {}",
                r.platform.chassis_type,
                match r.platform.battery_pct {
                    Some(p) => format!("{p}% ({})", if r.platform.ac_online == Some(true) { "AC" } else { "Battery" }),
                    None => "AC Desktop".to_string(),
                }
            ), false);
            kv_row(
                ui,
                "Security",
                &format!(
                    "SecureBoot: {} | TPM: {} {} | VBS/HVCI: {} | HAGS: {}",
                    match r.security.secure_boot {
                        Some(true) => "On",
                        Some(false) => "Off",
                        None => "Unknown",
                    },
                    if r.security.tpm_enabled { "Enabled" } else if r.security.tpm_present { "Present" } else { "Unknown/Admin" },
                    r.security.tpm_spec_version,
                    if r.security.vbs_active { "Active" } else { "Off" },
                    if r.security.hags_enabled { "Enabled" } else { "Disabled" },
                ),
                true,
            );
            if r.platform.pending_reboot {
                kv_row(ui, "Pending Reboot", &r.platform.pending_reboot_reasons.join(", "), false);
            }
        });
    });

    ui.add_space(8.0);

    // GPUs & Displays
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(RichText::new("Graphics Adapters (DXGI)").size(14.0).strong().color(theme::ACCENT_GOLD));
            ui.add_space(4.0);
            for (i, gpu) in r.gpus.iter().enumerate() {
                if i > 0 {
                    ui.separator();
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&gpu.name).strong().color(theme::TEXT_PRIMARY));
                    status_badge(
                        ui,
                        if gpu.is_integrated { "iGPU" } else { "Discrete" },
                        if gpu.is_integrated { theme::TEXT_SECONDARY } else { theme::ACCENT_GOLD },
                        theme::BG_ELEVATED,
                    );
                });
                kv_row(
                    ui,
                    "VRAM / Shared",
                    &format!("{} dedicated / {} shared", format_bytes(gpu.vram_bytes), format_bytes(gpu.shared_sys_mem_bytes)),
                    true,
                );
                kv_row(ui, "Driver", &format!("{} ({})", gpu.driver_version, gpu.driver_date), true);
                kv_row(
                    ui,
                    "MSI Mode",
                    match gpu.msi_supported {
                        Some(true) => "Enabled (MSISupported=1)",
                        Some(false) => "Disabled",
                        None => "Default / Standard",
                    },
                    true,
                );
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.label(RichText::new("Connected Displays").size(14.0).strong().color(theme::ACCENT_CYAN_SLATE));
            ui.add_space(4.0);
            if r.displays.is_empty() {
                ui.label(RichText::new("No active displays reported.").color(theme::TEXT_MUTED));
            }
            for d in &r.displays {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&d.monitor_name).strong().color(theme::TEXT_PRIMARY));
                    if d.is_primary {
                        status_badge(ui, "PRIMARY", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
                    }
                });
                kv_row(
                    ui,
                    "Mode",
                    &format!("{}x{} @ {} Hz ({}-bit) [{}]", d.width, d.height, d.refresh_hz, d.bits_per_pixel, d.device_name),
                    true,
                );
            }
        });
    });

    ui.add_space(8.0);

    // Memory & DIMM Table
    card_frame().show(ui, |ui| {
        ui.label(RichText::new("Memory & DIMM Slots").size(14.0).strong().color(theme::ACCENT_EMERALD));
        ui.add_space(4.0);
        kv_row(
            ui,
            "Summary",
            &format!(
                "{} Total ({} Free / {} Used, {}% Load) | {}/{} Slots | {} @ {} MT/s ({})",
                format_bytes(r.memory.total_phys_bytes),
                format_bytes(r.memory.avail_phys_bytes),
                format_bytes(r.memory.used_phys_bytes),
                r.memory.memory_load_pct,
                r.memory.used_slots,
                r.memory.total_slots,
                r.memory.memory_type,
                r.memory.speed_mts,
                r.memory.channel_mode,
            ),
            true,
        );
        ui.add_space(4.0);
        egui::Grid::new("dimm_grid").striped(true).num_columns(6).show(ui, |ui| {
            for h in ["Slot", "Capacity", "Type", "Configured", "Form Factor", "Vendor / Part"] {
                ui.label(RichText::new(h).size(11.5).strong().color(theme::TEXT_MUTED));
            }
            ui.end_row();
            for d in &r.memory.dimms {
                ui.label(RichText::new(&d.slot).monospace().size(12.0));
                ui.label(RichText::new(format_bytes(d.capacity_bytes)).monospace().size(12.0));
                ui.label(RichText::new(&d.memory_type).monospace().size(12.0));
                ui.label(RichText::new(format!("{} MT/s (rated {})", d.configured_mts, d.rated_mts)).monospace().size(12.0));
                ui.label(RichText::new(&d.form_factor).size(12.0));
                ui.label(RichText::new(format!("{} | {}", d.manufacturer, d.part_number)).size(12.0));
                ui.end_row();
            }
        });
    });

    ui.add_space(8.0);

    // Storage + Network Adapters + Ecosystem
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.label(RichText::new("Physical Disks & Volumes").size(14.0).strong().color(theme::ACCENT_COBALT));
            ui.add_space(4.0);
            for disk in &r.physical_disks {
                kv_row(
                    ui,
                    &disk.friendly_name,
                    &format!("{} | {} | {} | Health: {}", format_bytes(disk.size_bytes), disk.media_type, disk.bus_type, disk.health_status),
                    true,
                );
            }
            ui.add_space(6.0);
            for vol in &r.volumes {
                progress_bar(
                    ui,
                    (vol.used_pct / 100.0).clamp(0.0, 1.0),
                    &format!(
                        "{} ({}) [{}] - {} free / {}",
                        vol.mount,
                        vol.label,
                        vol.filesystem,
                        format_bytes(vol.free_bytes),
                        format_bytes(vol.total_bytes)
                    ),
                    theme::ACCENT_COBALT,
                );
                ui.add_space(3.0);
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.label(RichText::new("Network Interfaces & Ecosystem").size(14.0).strong().color(theme::ACCENT_GOLD));
            ui.add_space(4.0);
            for nic in &r.network_adapters {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&nic.friendly_name).strong().color(theme::TEXT_PRIMARY));
                    if nic.is_physical_primary {
                        status_badge(ui, "PRIMARY", theme::ACCENT_EMERALD, theme::ACCENT_EMERALD_DIM);
                    }
                });
                kv_row(
                    ui,
                    "Link / IPv4",
                    &format!("{} ({} Mbps) | {}", nic.adapter_kind, nic.link_speed_mbps, nic.ipv4_addrs.join(", ")),
                    true,
                );
                kv_row(ui, "DNS / Gateway", &format!("DNS: {} | GW: {}", nic.dns_servers.join(", "), nic.gateways.join(", ")), true);
            }
            ui.add_space(6.0);
            kv_row(
                ui,
                "Power & Hibernation",
                &format!(
                    "{} | Timer {:.2} ms | Hiberfil: {}",
                    r.power.active_scheme_name,
                    r.power.timer_res_current_ms,
                    if r.power.hibernation_enabled {
                        format_bytes(r.power.hiberfil_bytes)
                    } else {
                        "Disabled".into()
                    }
                ),
                true,
            );
            kv_row(
                ui,
                "WinGet CLI",
                r.ecosystem.winget_version.as_deref().unwrap_or(if r.ecosystem.winget_available { "Installed" } else { "Not detected" }),
                true,
            );
        });
    });

    if !r.diagnostics.is_empty() {
        ui.add_space(8.0);
        card_frame().show(ui, |ui| {
            ui.label(RichText::new("Collector Diagnostics").size(13.0).strong().color(theme::ACCENT_AMBER));
            for d in &r.diagnostics {
                ui.label(RichText::new(d).size(11.5).monospace().color(theme::TEXT_SECONDARY));
            }
        });
    }
}
