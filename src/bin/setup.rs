#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, CornerRadius, Margin, Pos2, RichText, Sense, Stroke, StrokeKind, Vec2};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, KEY_WRITE};
use winreg::RegKey;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_ICON_PNG: &[u8] = include_bytes!("../../assets/app_icon.png");
const PAYLOAD_EXE: &[u8] = include_bytes!("../../dist/OptimizerByRaghav.exe");
const UNINSTALL_REG_PATH: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\OptimizerByRaghav";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const BG_VOID: Color32 = Color32::from_rgb(6, 8, 13);
const BG_CANVAS: Color32 = Color32::from_rgb(9, 12, 18);
const BG_SURFACE: Color32 = Color32::from_rgb(14, 19, 29);
const BORDER_HAIRLINE: Color32 = Color32::from_rgb(28, 38, 54);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 247, 252);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(154, 168, 190);
const TEXT_MUTED: Color32 = Color32::from_rgb(96, 111, 135);
const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
const ACCENT_EMERALD: Color32 = Color32::from_rgb(16, 185, 129);
const ACCENT_ROSE: Color32 = Color32::from_rgb(244, 63, 94);

fn default_install_dir() -> PathBuf {
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    Path::new(&pf).join("Optimizer by Raghav")
}

fn is_installed() -> bool {
    let exe_ok = default_install_dir().join("OptimizerByRaghav.exe").is_file();
    let reg_ok = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(UNINSTALL_REG_PATH, KEY_READ | KEY_WOW64_64KEY)
        .is_ok();
    exe_ok && reg_ok
}

fn perform_install(
    install_dir: &Path,
    desktop_shortcut: bool,
    start_menu_shortcut: bool,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(install_dir).map_err(|e| {
        format!(
            "Failed creating {}: {e} (Run Setup as Administrator)",
            install_dir.display()
        )
    })?;

    let target_exe = install_dir.join("OptimizerByRaghav.exe");
    if target_exe.exists() && std::fs::write(&target_exe, PAYLOAD_EXE).is_err() {
        let _ = Command::new("taskkill.exe")
            .args(["/F", "/IM", "OptimizerByRaghav.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        std::thread::sleep(std::time::Duration::from_millis(350));
        let old_bak = install_dir.join(".OptimizerByRaghav.old.exe");
        let _ = std::fs::remove_file(&old_bak);
        let _ = std::fs::rename(&target_exe, &old_bak);
    }

    std::fs::write(&target_exe, PAYLOAD_EXE)
        .map_err(|e| format!("Failed writing {}: {e}", target_exe.display()))?;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (uninst_key, _) = hklm
        .create_subkey_with_flags(UNINSTALL_REG_PATH, KEY_WRITE | KEY_WOW64_64KEY)
        .map_err(|e| format!("Failed writing Uninstall registry key: {e}"))?;

    let exe_str = target_exe.to_string_lossy().to_string();
    let dir_str = install_dir.to_string_lossy().to_string();
    let icon_str = format!("{exe_str},0");
    let uninst_cmd = format!("\"{exe_str}\" --uninstall");
    let quiet_uninst_cmd = format!("\"{exe_str}\" --uninstall --quiet");
    let size_kb = ((PAYLOAD_EXE.len() as u64) / 1024).max(1) as u32;

    let _ = uninst_key.set_value("DisplayName", &"Optimizer by Raghav");
    let _ = uninst_key.set_value("DisplayVersion", &APP_VERSION);
    let _ = uninst_key.set_value("Publisher", &"Raghav Goyal");
    let _ = uninst_key.set_value("DisplayIcon", &icon_str);
    let _ = uninst_key.set_value("InstallLocation", &dir_str);
    let _ = uninst_key.set_value("UninstallString", &uninst_cmd);
    let _ = uninst_key.set_value("QuietUninstallString", &quiet_uninst_cmd);
    let _ = uninst_key.set_value("EstimatedSize", &size_kb);
    let _ = uninst_key.set_value("NoModify", &1u32);
    let _ = uninst_key.set_value("NoRepair", &1u32);

    let mut shortcut_targets = Vec::new();
    if start_menu_shortcut {
        let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        shortcut_targets.push(
            Path::new(&pd)
                .join(r"Microsoft\Windows\Start Menu\Programs\Optimizer by Raghav.lnk")
                .to_string_lossy()
                .to_string(),
        );
    }
    if desktop_shortcut {
        if let Ok(up) = std::env::var("USERPROFILE") {
            shortcut_targets.push(
                Path::new(&up)
                    .join(r"Desktop\Optimizer by Raghav.lnk")
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }

    if !shortcut_targets.is_empty() {
        let exe_escaped = exe_str.replace('\'', "''");
        let dir_escaped = dir_str.replace('\'', "''");
        let mut ps = String::from("$ws = New-Object -ComObject WScript.Shell; ");
        for lnk in &shortcut_targets {
            let lnk_escaped = lnk.replace('\'', "''");
            ps.push_str(&format!(
                "$s = $ws.CreateShortcut('{0}'); \
                 $s.TargetPath = '{1}'; \
                 $s.WorkingDirectory = '{2}'; \
                 $s.IconLocation = '{1},0'; \
                 $s.Description = 'Optimizer by Raghav - Windows System Identifier & Performance Optimizer'; \
                 $s.Save(); ",
                lnk_escaped, exe_escaped, dir_escaped
            ));
        }
        let _ = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }

    Ok(target_exe)
}

fn perform_uninstall() -> Result<String, String> {
    let _ = Command::new("taskkill.exe")
        .args(["/F", "/IM", "OptimizerByRaghav.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    std::thread::sleep(std::time::Duration::from_millis(300));

    let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    let _ = std::fs::remove_file(
        Path::new(&pd).join(r"Microsoft\Windows\Start Menu\Programs\Optimizer by Raghav.lnk"),
    );
    if let Ok(up) = std::env::var("USERPROFILE") {
        let _ = std::fs::remove_file(Path::new(&up).join(r"Desktop\Optimizer by Raghav.lnk"));
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(parent) = hklm.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        KEY_WRITE | KEY_WOW64_64KEY,
    ) {
        let _ = parent.delete_subkey_all("OptimizerByRaghav");
    }

    let dir = default_install_dir();
    if dir.exists() {
        let _ = std::fs::remove_dir_all(&dir);
    }

    Ok("Uninstalled cleanly.".to_string())
}

fn pill_toggle(ui: &mut egui::Ui, on: &mut bool) {
    let desired = Vec2::new(36.0, 20.0);
    let (rect, mut resp) = ui.allocate_exact_size(desired, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let p = ui.painter();
    let bg = Color32::from_rgb(
        (BG_VOID.r() as f32 + (ACCENT_CYAN.r() as f32 - BG_VOID.r() as f32) * t) as u8,
        (BG_VOID.g() as f32 + (ACCENT_CYAN.g() as f32 - BG_VOID.g() as f32) * t) as u8,
        (BG_VOID.b() as f32 + (ACCENT_CYAN.b() as f32 - BG_VOID.b() as f32) * t) as u8,
    );
    p.rect_filled(rect, CornerRadius::same(10), bg);
    p.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0_f32, if *on { ACCENT_CYAN } else { BORDER_HAIRLINE }),
        StrokeKind::Inside,
    );
    let r = rect.height() * 0.5 - 3.0;
    let cx = egui::lerp((rect.left() + 3.0 + r)..=(rect.right() - 3.0 - r), t);
    p.circle_filled(
        Pos2::new(cx, rect.center().y),
        r,
        if *on { BG_VOID } else { TEXT_SECONDARY },
    );
}

struct SetupWizardApp {
    install_dir: String,
    desktop_shortcut: bool,
    start_menu_shortcut: bool,
    launch_after_install: bool,
    already_installed: bool,
    status_msg: Option<(bool, String)>,
    logo_tex: Option<egui::TextureHandle>,
}

impl SetupWizardApp {
    fn new(cc: &eframe::CreationContext<'_>, logo_rgba: Option<(Vec<u8>, u32, u32)>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG_CANVAS;
        visuals.window_fill = BG_SURFACE;
        visuals.extreme_bg_color = BG_VOID;
        visuals.override_text_color = Some(TEXT_PRIMARY);
        cc.egui_ctx.set_visuals(visuals);

        let logo_tex = logo_rgba.map(|(rgba, w, h)| {
            let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            cc.egui_ctx
                .load_texture("setup_logo", img, egui::TextureOptions::LINEAR)
        });

        Self {
            install_dir: default_install_dir().to_string_lossy().to_string(),
            desktop_shortcut: true,
            start_menu_shortcut: true,
            launch_after_install: true,
            already_installed: is_installed(),
            status_msg: None,
            logo_tex,
        }
    }
}

impl eframe::App for SetupWizardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Minimalist Borderless Drag Bar
        egui::TopBottomPanel::top("setup_bar")
            .exact_height(36.0)
            .frame(
                egui::Frame::new()
                    .fill(BG_VOID)
                    .stroke(Stroke::new(1.0_f32, BORDER_HAIRLINE))
                    .inner_margin(Margin::symmetric(14, 4)),
            )
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new(format!("SETUP  •  v{APP_VERSION}"))
                            .size(10.5)
                            .strong()
                            .color(TEXT_MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("✕").size(12.5).color(TEXT_SECONDARY))
                                    .frame(false),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
                let drag = ui.interact(rect, egui::Id::new("setup_drag"), Sense::drag());
                if drag.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG_CANVAS)
                    .inner_margin(Margin::symmetric(24, 20)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(tex) = &self.logo_tex {
                        ui.add(
                            egui::Image::new(tex)
                                .fit_to_exact_size(egui::vec2(46.0, 46.0))
                                .corner_radius(CornerRadius::same(10)),
                        );
                        ui.add_space(6.0);
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new("Optimizer").size(24.0).strong().color(TEXT_PRIMARY));
                        ui.label(RichText::new("by Raghav").size(11.5).color(ACCENT_CYAN));
                    });
                });

                ui.add_space(16.0);

                egui::Frame::new()
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER_HAIRLINE))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(16))
                    .show(ui, |ui| {
                        ui.label(RichText::new("LOCATION").size(10.0).strong().color(TEXT_MUTED));
                        ui.add_space(4.0);
                        ui.add_sized(
                            [ui.available_width(), 28.0],
                            egui::TextEdit::singleline(&mut self.install_dir),
                        );

                        ui.add_space(12.0);
                        for (lbl, flag) in [
                            ("Desktop shortcut", &mut self.desktop_shortcut),
                            ("Start Menu shortcut", &mut self.start_menu_shortcut),
                            ("Launch when ready", &mut self.launch_after_install),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lbl).size(12.8).color(TEXT_PRIMARY));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        pill_toggle(ui, flag);
                                    },
                                );
                            });
                            ui.add_space(2.0);
                        }
                    });

                ui.add_space(14.0);

                if let Some((ok, msg)) = &self.status_msg {
                    ui.label(
                        RichText::new(msg)
                            .size(12.0)
                            .color(if *ok { ACCENT_EMERALD } else { ACCENT_ROSE }),
                    );
                    ui.add_space(6.0);
                }

                ui.horizontal(|ui| {
                    let lbl = if self.already_installed {
                        format!("Update v{APP_VERSION}")
                    } else {
                        format!("Install v{APP_VERSION}")
                    };
                    let btn = egui::Button::new(
                        RichText::new(lbl).size(13.5).strong().color(BG_VOID),
                    )
                    .min_size(Vec2::new(
                        if self.already_installed { 320.0 } else { ui.available_width() },
                        38.0,
                    ))
                    .fill(ACCENT_CYAN)
                    .corner_radius(CornerRadius::same(9));

                    if ui.add(btn).clicked() {
                        match perform_install(
                            Path::new(&self.install_dir),
                            self.desktop_shortcut,
                            self.start_menu_shortcut,
                        ) {
                            Ok(exe_path) => {
                                self.already_installed = true;
                                self.status_msg = Some((true, "Installed cleanly.".into()));
                                if self.launch_after_install {
                                    let _ = Command::new(&exe_path).spawn();
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            }
                            Err(e) => self.status_msg = Some((false, e)),
                        }
                    }

                    if self.already_installed {
                        let uninst = egui::Button::new(
                            RichText::new("Uninstall").size(12.5).strong().color(ACCENT_ROSE),
                        )
                        .min_size(Vec2::new(110.0, 38.0))
                        .fill(Color32::from_rgb(52, 16, 26))
                        .stroke(Stroke::new(1.0_f32, ACCENT_ROSE.linear_multiply(0.65)))
                        .corner_radius(CornerRadius::same(9));

                        if ui.add(uninst).clicked() {
                            match perform_uninstall() {
                                Ok(msg) => {
                                    self.already_installed = false;
                                    self.status_msg = Some((true, msg));
                                }
                                Err(e) => self.status_msg = Some((false, e)),
                            }
                        }
                    }
                });
            });
    }
}

fn load_icon() -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(APP_ICON_PNG).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--silent" || a == "/S" || a == "--silent-launch") {
        let launch = args.iter().any(|a| a == "--silent-launch");
        if let Ok(exe) = perform_install(&default_install_dir(), true, true) {
            if launch {
                let _ = Command::new(exe).spawn();
            }
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    let icon_rgba = load_icon();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!("Optimizer by Raghav v{APP_VERSION} Setup"))
        .with_decorations(false)
        .with_inner_size([490.0, 360.0])
        .with_resizable(false);

    if let Some((ref rgba, w, h)) = icon_rgba {
        viewport = viewport.with_icon(std::sync::Arc::new(egui::IconData {
            rgba: rgba.clone(),
            width: w,
            height: h,
        }));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Optimizer by Raghav Setup",
        native_options,
        Box::new(move |cc| Ok(Box::new(SetupWizardApp::new(cc, icon_rgba)))),
    );
}
