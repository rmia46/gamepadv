pub mod controller_view;
pub mod keymap;

use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui::{self, Color32, Rounding, Stroke};

use crate::engine::MouseStickConfig;
use crate::profile::{builtin_presets, zelda_default, GamepadAction, Profile, ProfileConfig};
use crate::runtime::{EngineHandle, EngineSnapshot};

/// Main `eframe` application state for GamepadV.
pub struct GamepadVApp {
    engine: EngineHandle,
    last_snapshot: EngineSnapshot,
    profile: Profile,
    mouse_config: MouseStickConfig,
    listening_for: Option<GamepadAction>,
    just_started_listening: bool,
    prev_modifiers: egui::Modifiers,
    status_banner: Option<(String, bool)>,
    show_tuning_panel: bool,
    show_save_as_modal: bool,
    save_as_name: String,
    saved_profiles: Vec<(String, PathBuf)>,
    show_about_modal: bool,
}

impl GamepadVApp {
    /// Creates a new [`GamepadVApp`] and spawns the 250Hz background input engine.
    #[must_use]
    pub fn new(profile: Profile, mouse_config: MouseStickConfig, start_paused: bool) -> Self {
        let engine = EngineHandle::spawn(profile.clone(), mouse_config, start_paused);
        let last_snapshot = engine.snapshot();
        let saved_profiles = ProfileConfig::list_saved_profiles();
        Self {
            engine,
            last_snapshot,
            profile,
            mouse_config,
            listening_for: None,
            just_started_listening: false,
            prev_modifiers: egui::Modifiers::default(),
            status_banner: None,
            show_tuning_panel: true,
            show_save_as_modal: false,
            save_as_name: String::new(),
            saved_profiles,
            show_about_modal: false,
        }
    }

    fn apply_loaded_profile(
        &mut self,
        profile: Profile,
        mouse_config: MouseStickConfig,
        banner: String,
    ) {
        self.profile = profile;
        self.mouse_config = mouse_config;
        self.listening_for = None;
        self.engine.update_profile(self.profile.clone());
        self.engine.update_mouse_config(self.mouse_config);
        self.status_banner = Some((banner, false));
    }

    fn save_profile_to(&mut self, path: PathBuf, label: &str) {
        let cfg = ProfileConfig::from_parts(&self.profile, &self.mouse_config);
        match cfg.save_to_file(&path) {
            Ok(()) => {
                self.saved_profiles = ProfileConfig::list_saved_profiles();
                self.status_banner = Some((format!("Saved {label}: {}", path.display()), false));
            }
            Err(err) => {
                self.status_banner = Some((format!("Save failed: {err}"), true));
            }
        }
    }

    fn load_profile_from(&mut self, path: &Path) {
        match ProfileConfig::load_from_file(path) {
            Ok(cfg) => {
                let (p, m) = cfg.into_parts();
                let name = p.name.clone();
                self.apply_loaded_profile(p, m, format!("Loaded profile: {name}"));
            }
            Err(err) => {
                self.status_banner = Some((format!("Failed to load profile: {err}"), true));
            }
        }
    }
}

/// Launches the interactive GamepadV GUI window.
pub fn run_gui(
    profile: Profile,
    mouse_config: MouseStickConfig,
    start_paused: bool,
) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        vsync: false,
        viewport: egui::ViewportBuilder::default()
            .with_title("GamepadV — Virtual Xbox 360 Controller")
            .with_inner_size([1080.0, 720.0])
            .with_min_inner_size([860.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "GamepadV",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(GamepadVApp::new(
                profile,
                mouse_config,
                start_paused,
            )))
        }),
    )
}

impl eframe::App for GamepadVApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(fresh) = self.engine.try_snapshot() {
            self.last_snapshot = fresh;
        }
        let snap = self.last_snapshot.clone();

        // App Menu Bar: File | Profiles | Settings
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(15, 17, 22))
                    .inner_margin(egui::Margin::symmetric(10.0, 4.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Save Default Profile").clicked() {
                            if let Some(path) = ProfileConfig::default_config_path() {
                                self.save_profile_to(path, "default profile");
                            }
                            ui.close_menu();
                        }
                        if ui.button("Save Profile As...").clicked() {
                            self.save_as_name = self.profile.name.clone();
                            self.show_save_as_modal = true;
                            ui.close_menu();
                        }
                        ui.menu_button("Load Saved Profile", |ui| {
                            if let Some(def_path) = ProfileConfig::default_config_path() {
                                if def_path.exists()
                                    && ui.button("Default (profile.json)").clicked()
                                {
                                    self.load_profile_from(&def_path);
                                    ui.close_menu();
                                }
                            }
                            let saved = self.saved_profiles.clone();
                            if saved.is_empty() {
                                ui.label(
                                    egui::RichText::new("No named profiles saved yet")
                                        .small()
                                        .color(Color32::GRAY),
                                );
                            } else {
                                for (name, path) in saved {
                                    if ui.button(&name).clicked() {
                                        self.load_profile_from(&path);
                                        ui.close_menu();
                                    }
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("Reset to Zelda Default").clicked() {
                            self.apply_loaded_profile(
                                zelda_default(),
                                MouseStickConfig::default(),
                                "Reset to Zelda Default".to_string(),
                            );
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.menu_button("Profiles", |ui| {
                        for (label, preset_profile, preset_mouse) in builtin_presets() {
                            let selected = self.profile.name == preset_profile.name;
                            if ui.selectable_label(selected, label).clicked() {
                                self.apply_loaded_profile(
                                    preset_profile,
                                    preset_mouse,
                                    format!("Loaded preset: {label}"),
                                );
                                ui.close_menu();
                            }
                        }
                    });

                    ui.menu_button("Settings", |ui| {
                        ui.checkbox(&mut self.show_tuning_panel, "Show Tuning Panel");
                        if ui
                            .checkbox(&mut self.mouse_config.invert_y, "Invert Y Axis")
                            .changed()
                        {
                            self.engine.update_mouse_config(self.mouse_config);
                        }
                        ui.menu_button("Engine Tick Rate (Hz)", |ui| {
                            for hz in [125.0_f64, 250.0, 500.0] {
                                let selected = (self.mouse_config.hz - hz).abs() < 0.5;
                                if ui
                                    .selectable_label(selected, format!("{hz:.0} Hz"))
                                    .clicked()
                                {
                                    self.mouse_config.hz = hz;
                                    self.engine.update_mouse_config(self.mouse_config);
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("About / Permissions").clicked() {
                            self.show_about_modal = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        // Top Bar: Active/Paused Toggle, Device Status, and Permission Warning
        egui::TopBottomPanel::top("top_bar")
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 23, 29))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let toggle_label = if snap.active {
                        "> ACTIVE  (F8)"
                    } else {
                        "|| PAUSED  (F8)"
                    };
                    let toggle_fill = if snap.active {
                        Color32::from_rgb(38, 130, 68)
                    } else {
                        Color32::from_rgb(95, 72, 28)
                    };
                    let toggle_btn = egui::Button::new(
                        egui::RichText::new(toggle_label)
                            .strong()
                            .size(15.0)
                            .color(Color32::WHITE),
                    )
                    .fill(toggle_fill)
                    .min_size(egui::vec2(170.0, 34.0));

                    if ui
                        .add_enabled(
                            snap.permission_error.is_none() && !snap.unlocking,
                            toggle_btn,
                        )
                        .clicked()
                    {
                        self.engine.set_active(!snap.active);
                    }

                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} keyboards, {} mice",
                                snap.keyboard_count, snap.mouse_count
                            ))
                            .strong(),
                        );
                        ui.label(
                            egui::RichText::new(&snap.status_message)
                                .small()
                                .color(Color32::from_rgb(160, 170, 185)),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let key_text = snap.last_key.as_deref().unwrap_or("—");
                        let (dx, dy) = snap.mouse_delta;
                        egui::Frame::none()
                            .fill(Color32::from_rgb(30, 34, 44))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 68, 84)))
                            .rounding(Rounding::same(5.0))
                            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Input Check | Key: {key_text} | Mouse: ({dx:+}, {dy:+})"
                                    ))
                                    .size(14.5)
                                    .strong()
                                    .color(Color32::from_rgb(200, 225, 250)),
                                );
                            });
                    });
                });

                if let Some(err) = &snap.permission_error {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(Color32::from_rgb(72, 24, 24))
                        .stroke(Stroke::new(1.5_f32, Color32::from_rgb(220, 75, 75)))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(
                                    Color32::from_rgb(255, 190, 190),
                                    format!("Permission error: {err}"),
                                );
                                ui.add_space(10.0);
                                let unlock_label = if snap.unlocking {
                                    "Authorizing..."
                                } else {
                                    "Unlock Input Engine (pkexec)"
                                };
                                let unlock_btn = egui::Button::new(
                                    egui::RichText::new(unlock_label)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(46, 115, 185));
                                if ui.add_enabled(!snap.unlocking, unlock_btn).clicked() {
                                    self.engine
                                        .unlock_with_pkexec(self.profile.clone(), self.mouse_config);
                                }
                            });
                        });
                }
            });

        // Right Panel: Mouse DSP Tuning & Profile Management
        if self.show_tuning_panel {
            egui::SidePanel::right("tuning_panel")
            .resizable(false)
            .default_width(290.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 23, 29))
                    .inner_margin(egui::Margin::same(14.0)),
            )
            .show(ctx, |ui| {
                ui.heading("Mouse Stick Tuning");
                ui.add_space(6.0);

                let mut mouse_changed = false;

                ui.label("Sensitivity");
                mouse_changed |= ui
                    .add(
                        egui::Slider::new(&mut self.mouse_config.sensitivity, 0.0001..=0.0030)
                            .fixed_decimals(4),
                    )
                    .changed();

                ui.add_space(4.0);
                ui.label("Anti-Deadzone");
                mouse_changed |= ui
                    .add(
                        egui::Slider::new(&mut self.mouse_config.anti_deadzone, 0.0..=0.50)
                            .fixed_decimals(2),
                    )
                    .changed();

                ui.add_space(4.0);
                ui.label("Smoothing (EMA)");
                mouse_changed |= ui
                    .add(
                        egui::Slider::new(&mut self.mouse_config.ema_alpha, 0.05..=1.0)
                            .fixed_decimals(2),
                    )
                    .changed();

                ui.add_space(4.0);
                ui.label("Decay Half-Life (ms)");
                mouse_changed |= ui
                    .add(
                        egui::Slider::new(&mut self.mouse_config.decay_half_life_ms, 4.0..=60.0)
                            .fixed_decimals(1),
                    )
                    .changed();

                ui.add_space(6.0);
                mouse_changed |= ui
                    .checkbox(&mut self.mouse_config.invert_y, "Invert Y")
                    .changed();

                if mouse_changed {
                    self.engine.update_mouse_config(self.mouse_config);
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                ui.heading("Profile");
                ui.add_space(4.0);
                ui.label(format!("Active: {}", self.profile.name));

                ui.add_space(6.0);
                let walk_key = self
                    .profile
                    .binding_for_action(GamepadAction::WalkModifier)
                    .map(keymap::format_keycode)
                    .unwrap_or_else(|| "---".to_string());
                if ui
                    .button(format!("Walk Modifier: {walk_key}"))
                    .clicked()
                {
                    self.listening_for = Some(GamepadAction::WalkModifier);
                    self.just_started_listening = true;
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Save Profile").clicked() {
                        match ProfileConfig::default_config_path() {
                            Some(path) => {
                                let cfg = ProfileConfig::from_parts(
                                    &self.profile,
                                    &self.mouse_config,
                                );
                                match cfg.save_to_file(&path) {
                                    Ok(()) => {
                                        self.status_banner = Some((
                                            format!("Saved to {}", path.display()),
                                            false,
                                        ));
                                    }
                                    Err(err) => {
                                        self.status_banner =
                                            Some((format!("Save failed: {err}"), true));
                                    }
                                }
                            }
                            None => {
                                self.status_banner = Some((
                                    "Could not resolve config path".to_string(),
                                    true,
                                ));
                            }
                        }
                    }

                    if ui.button("Reset Zelda Default").clicked() {
                        self.profile = zelda_default();
                        self.mouse_config = MouseStickConfig::default();
                        self.listening_for = None;
                        self.engine.update_profile(self.profile.clone());
                        self.engine.update_mouse_config(self.mouse_config);
                        self.status_banner =
                            Some(("Restored zelda_default profile".to_string(), false));
                    }
                });

                if let Some((msg, is_err)) = &self.status_banner {
                    ui.add_space(8.0);
                    let color = if *is_err {
                        Color32::from_rgb(245, 110, 110)
                    } else {
                        Color32::from_rgb(120, 215, 145)
                    };
                    ui.colored_label(color, msg);
                }
            });
        }

        // Save Profile As Modal
        if self.show_save_as_modal {
            egui::Window::new("Save Profile As")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Profile Name:");
                    ui.text_edit_singleline(&mut self.save_as_name);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let name = self.save_as_name.trim().to_string();
                            if !name.is_empty() {
                                self.profile.name = name.clone();
                                if let Some(path) = ProfileConfig::profile_path_for_name(&name) {
                                    self.save_profile_to(path, &format!("profile '{name}'"));
                                }
                                self.show_save_as_modal = false;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_save_as_modal = false;
                        }
                    });
                });
        }

        // About / Permissions Modal
        if self.show_about_modal {
            egui::Window::new("About GamepadV")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading(format!("GamepadV v{}", env!("CARGO_PKG_VERSION")));
                    ui.label("Zero-config Linux virtual Xbox 360 gamepad (KBM -> uinput).");
                    ui.add_space(6.0);
                    ui.label(format!(
                        "Detected: {} keyboard(s), {} mouse/mice",
                        snap.keyboard_count, snap.mouse_count
                    ));
                    ui.label(format!("Tick Rate: {:.0} Hz", self.mouse_config.hz));
                    ui.add_space(6.0);
                    ui.label("Permissions: Use 'Unlock Input Engine (pkexec)' or see docs/udev-setup.md.");
                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        self.show_about_modal = false;
                    }
                });
        }

        // Central Panel: Interactive Xbox 360 Controller
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(15, 17, 22))
                    .inner_margin(egui::Margin::same(14.0)),
            )
            .show(ctx, |ui| {
                let mut profile_changed = controller_view::handle_listening_input(
                    ui,
                    &mut self.profile,
                    &mut self.listening_for,
                    &mut self.just_started_listening,
                    &mut self.prev_modifiers,
                );

                profile_changed |= controller_view::draw_controller(
                    ui,
                    &mut self.profile,
                    &snap.latest_state,
                    &mut self.listening_for,
                    &mut self.just_started_listening,
                );

                if profile_changed {
                    self.engine.update_profile(self.profile.clone());
                }
            });

        if snap.active || snap.unlocking || snap.permission_error.is_none() || self.listening_for.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}
