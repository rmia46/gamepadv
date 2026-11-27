pub mod controller_view;
pub mod keymap;
pub mod theme;

use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui::{self, Color32, Rounding, Stroke};

use crate::engine::MouseStickConfig;
use crate::profile::{builtin_presets, default_3d_action, GamepadAction, Profile, ProfileConfig};
use crate::runtime::{EngineHandle, EngineSnapshot};

/// Main `eframe` application state for GamepadV.
pub struct GamepadVApp {
    engine: EngineHandle,
    last_snapshot: EngineSnapshot,
    profile: Profile,
    mouse_config: MouseStickConfig,
    listening_for: Option<GamepadAction>,
    bind_mode: controller_view::BindMode,
    just_started_listening: bool,
    prev_modifiers: egui::Modifiers,
    status_banner: Option<(String, bool)>,
    show_tuning_panel: bool,
    show_save_as_modal: bool,
    save_as_name: String,
    saved_profiles: Vec<(String, PathBuf)>,
    loaded_file_path: Option<PathBuf>,
    confirmation_popup: Option<(String, String, bool)>,
    show_about_modal: bool,
    show_how_to_use_modal: bool,
    show_slider_guide_modal: bool,
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
            bind_mode: controller_view::BindMode::Add,
            just_started_listening: false,
            prev_modifiers: egui::Modifiers::default(),
            status_banner: None,
            show_tuning_panel: true,
            show_save_as_modal: false,
            save_as_name: String::new(),
            saved_profiles,
            loaded_file_path: None,
            confirmation_popup: None,
            show_about_modal: false,
            show_how_to_use_modal: false,
            show_slider_guide_modal: false,
        }
    }

    fn apply_loaded_profile(
        &mut self,
        profile: Profile,
        mouse_config: MouseStickConfig,
        loaded_path: Option<PathBuf>,
        popup_title: &str,
        popup_msg: String,
    ) {
        self.profile = profile;
        self.mouse_config = mouse_config;
        self.loaded_file_path = loaded_path;
        self.listening_for = None;
        self.engine.update_profile(self.profile.clone());
        self.engine.update_mouse_config(self.mouse_config);
        self.status_banner = Some((popup_msg.clone(), false));
        self.confirmation_popup = Some((popup_title.to_string(), popup_msg, false));
    }

    fn save_current_profile(&mut self) {
        if let Some(path) = self.loaded_file_path.clone() {
            let name = self.profile.name.clone();
            self.save_profile_to(path, &format!("profile '{name}'"));
            return;
        }

        // Never overwrite built-in presets; save modifications as the user's custom default.
        if Profile::is_builtin_preset_name(&self.profile.name) {
            self.profile.name = format!("{} (Custom)", self.profile.name);
            self.engine.update_profile(self.profile.clone());
        }

        match ProfileConfig::default_config_path() {
            Some(path) => {
                let name = self.profile.name.clone();
                self.save_profile_to(path, &format!("custom default '{name}'"));
            }
            None => {
                let msg = "Could not resolve config directory.".to_string();
                self.status_banner = Some((msg.clone(), true));
                self.confirmation_popup = Some(("Save Failed".to_string(), msg, true));
            }
        }
    }

    fn save_profile_to(&mut self, path: PathBuf, label: &str) {
        let cfg = ProfileConfig::from_parts(&self.profile, &self.mouse_config);
        match cfg.save_to_file(&path) {
            Ok(()) => {
                self.saved_profiles = ProfileConfig::list_saved_profiles();
                let msg = format!(
                    "Saved {label} to:\n{}\n\nBuilt-in presets remain untouched and can be restored anytime from the Profiles menu.",
                    path.display()
                );
                self.status_banner = Some((format!("Saved {label}"), false));
                self.confirmation_popup = Some(("Profile Saved".to_string(), msg, false));
            }
            Err(err) => {
                let msg = format!("Save failed: {err}");
                self.status_banner = Some((msg.clone(), true));
                self.confirmation_popup = Some(("Save Failed".to_string(), msg, true));
            }
        }
    }

    fn load_profile_from(&mut self, path: &Path) {
        match ProfileConfig::load_from_file(path) {
            Ok(cfg) => {
                let (p, m) = cfg.into_parts();
                let name = p.name.clone();
                let msg = format!("Loaded profile '{name}' from:\n{}", path.display());
                self.apply_loaded_profile(
                    p,
                    m,
                    Some(path.to_path_buf()),
                    "Profile Loaded",
                    msg,
                );
            }
            Err(err) => {
                let msg = format!("Failed to load profile: {err}");
                self.status_banner = Some((msg.clone(), true));
                self.confirmation_popup = Some(("Load Failed".to_string(), msg, true));
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
            theme::apply_modern_theme(&cc.egui_ctx);
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

        // App Menu Bar: File | Profiles | Settings | Help
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_CANVAS)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(12.0, 5.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Save Profile").clicked() {
                            self.save_current_profile();
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
                                    && ui.button("Custom Default (profile.json)").clicked()
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
                        if ui.button("Reset to Built-in Default").clicked() {
                            self.apply_loaded_profile(
                                default_3d_action(),
                                MouseStickConfig::default(),
                                None,
                                "Built-in Default Restored",
                                "Restored built-in '3D Action (Default)' profile.".to_string(),
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
                                    None,
                                    "Built-in Preset Loaded",
                                    format!("Loaded built-in preset '{label}'."),
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

                    ui.menu_button("Help", |ui| {
                        if ui.button("How to Use").clicked() {
                            self.show_how_to_use_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("Slider & Tuning Guide").clicked() {
                            self.show_slider_guide_modal = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        // Top Bar: Active/Paused Toggle, Device Status, and Permission Warning
        egui::TopBottomPanel::top("top_bar")
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0)),
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
                .default_width(308.0)
                .frame(
                    egui::Frame::none()
                        .fill(theme::BG_PANEL)
                        .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                        .inner_margin(egui::Margin::same(12.0)),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        theme::card_frame().show(ui, |ui| {
                            theme::section_header(ui, "Mouse Stick Tuning");

                            let mut mouse_changed = false;

                            ui.label("Sensitivity");
                            mouse_changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.mouse_config.sensitivity,
                                        0.0001..=0.0030,
                                    )
                                    .fixed_decimals(4),
                                )
                                .changed();

                            ui.label("Anti-Deadzone");
                            mouse_changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.mouse_config.anti_deadzone,
                                        0.0..=0.50,
                                    )
                                    .fixed_decimals(2),
                                )
                                .changed();

                            ui.label("Smoothing (EMA)");
                            mouse_changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.mouse_config.ema_alpha,
                                        0.05..=1.0,
                                    )
                                    .fixed_decimals(2),
                                )
                                .changed();

                            ui.label("Decay Half-Life (ms)");
                            mouse_changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.mouse_config.decay_half_life_ms,
                                        4.0..=60.0,
                                    )
                                    .fixed_decimals(1),
                                )
                                .changed();

                            mouse_changed |= ui
                                .checkbox(&mut self.mouse_config.invert_y, "Invert Y")
                                .changed();

                            if mouse_changed {
                                self.engine.update_mouse_config(self.mouse_config);
                            }
                        });

                        ui.add_space(10.0);

                        theme::card_frame().show(ui, |ui| {
                            theme::section_header(ui, "Key Stick Sweep");

                            let mut key_stick_changed = false;
                            ui.label("Sweep Time (ms, 0 = instant)");
                            key_stick_changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.profile.key_stick.sweep_ms,
                                        0.0..=200.0,
                                    )
                                    .fixed_decimals(0),
                                )
                                .changed();

                            ui.label("Release Decay (ms)");
                            key_stick_changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.profile.key_stick.release_ms,
                                        0.0..=100.0,
                                    )
                                    .fixed_decimals(0),
                                )
                                .changed();

                            if key_stick_changed {
                                self.engine.update_profile(self.profile.clone());
                            }
                        });

                        ui.add_space(10.0);

                        theme::card_frame().show(ui, |ui| {
                            theme::section_header(ui, "Active Profile");
                            ui.label(
                                egui::RichText::new(&self.profile.name)
                                    .strong()
                                    .color(theme::TEXT_PRIMARY),
                            );

                            ui.add_space(4.0);
                            let walk_keys: Vec<String> = self
                                .profile
                                .bindings_for_action(GamepadAction::WalkModifier)
                                .into_iter()
                                .map(keymap::format_keycode)
                                .collect();
                            let walk_label = if walk_keys.is_empty() {
                                "---".to_string()
                            } else {
                                walk_keys.join(" / ")
                            };
                            if ui
                                .button(format!("Walk Modifier: {walk_label}"))
                                .clicked()
                            {
                                self.listening_for = Some(GamepadAction::WalkModifier);
                                self.just_started_listening = true;
                            }

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    self.save_current_profile();
                                }
                                if ui.button("Save As...").clicked() {
                                    self.save_as_name = self.profile.name.clone();
                                    self.show_save_as_modal = true;
                                }
                                if ui.button("Reset Default").clicked() {
                                    self.apply_loaded_profile(
                                        default_3d_action(),
                                        MouseStickConfig::default(),
                                        None,
                                        "Built-in Default Restored",
                                        "Restored built-in '3D Action (Default)' profile."
                                            .to_string(),
                                    );
                                }
                            });
                            ui.add_space(4.0);
                            if ui.button("Clear All Bindings (Unmap All)").clicked() {
                                self.profile.bindings.clear();
                                self.listening_for = None;
                                self.engine.update_profile(self.profile.clone());
                                self.confirmation_popup = Some((
                                    "All Bindings Cleared".to_string(),
                                    "All controller buttons and stick keys have been unmapped."
                                        .to_string(),
                                    false,
                                ));
                            }
                        });

                        if let Some((msg, is_err)) = &self.status_banner {
                            ui.add_space(8.0);
                            let color = if *is_err {
                                Color32::from_rgb(245, 110, 110)
                            } else {
                                theme::ACCENT_GREEN
                            };
                            ui.colored_label(color, msg);
                        }
                    });
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
                                if let Some(path) = ProfileConfig::profile_path_for_name(&name) {
                                    self.profile.name = name.clone();
                                    self.loaded_file_path = Some(path.clone());
                                    self.engine.update_profile(self.profile.clone());
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

        // Profile Action Confirmation Popup
        if let Some((title, message, is_err)) = self.confirmation_popup.clone() {
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    theme::card_frame().show(ui, |ui| {
                        let color = if is_err {
                            theme::ACCENT_RED
                        } else {
                            theme::ACCENT_GREEN
                        };
                        ui.label(egui::RichText::new(&message).size(13.5).color(color));
                    });
                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("OK").clicked()
                            || ctx.input(|i| {
                                i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)
                            })
                        {
                            self.confirmation_popup = None;
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

        // Help: How to Use Modal
        if self.show_how_to_use_modal {
            egui::Window::new("How to Use GamepadV")
                .collapsible(false)
                .resizable(false)
                .default_width(510.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    theme::card_frame().show(ui, |ui| {
                        theme::section_header(ui, "Quick Start");
                        theme::guide_row(ui, "1. Unlock", "Click 'Unlock Input Engine (pkexec)' and enter your password.", theme::ACCENT_BLUE);
                        theme::guide_row(ui, "2. Launch Game", "Start your emulator or Steam/Proton game AFTER unlocking so it sees the virtual pad.", theme::ACCENT_BLUE);
                        theme::guide_row(ui, "3. Play (F8)", "Switch to your game and press F8 to lock your cursor and send Xbox 360 inputs.", theme::ACCENT_GREEN);
                        theme::guide_row(ui, "4. Pause (F8)", "Press F8 again anytime to release your cursor for menus or desktop use.", theme::ACCENT_AMBER);
                    });

                    ui.add_space(8.0);

                    theme::card_frame().show(ui, |ui| {
                        theme::section_header(ui, "Mapping & Profiles");
                        theme::guide_row(ui, "Rebind", "Click any button, trigger, D-Pad, or stick key pill, then press a key or mouse button (Esc cancels).", theme::ACCENT_BLUE);
                        theme::guide_row(ui, "Mouse Stick", "Use 'Mouse | Keys | Off' on Left or Right Stick to route raw mouse movement to that stick.", theme::ACCENT_BLUE);
                        theme::guide_row(ui, "Profiles", "Pick built-in presets in 'Profiles', or save custom ones via 'File -> Save Profile As...'.", theme::ACCENT_BLUE);
                    });

                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.show_how_to_use_modal = false;
                        }
                    });
                });
        }

        // Help: Slider & Tuning Guide Modal
        if self.show_slider_guide_modal {
            egui::Window::new("Slider & Tuning Guide")
                .collapsible(false)
                .resizable(false)
                .default_width(530.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    theme::card_frame().show(ui, |ui| {
                        theme::section_header(ui, "Mouse Stick Tuning");
                        theme::guide_row(ui, "Sensitivity", "Scales mouse speed to stick deflection. Higher = faster camera turns.", theme::ACCENT_BLUE);
                        theme::guide_row(ui, "Anti-Deadzone", "Minimum stick output for tiny moves so slow aiming bypasses game deadzones (0.14 - 0.20).", theme::ACCENT_BLUE);
                        theme::guide_row(ui, "Smoothing", "Blends mouse velocity. 1.00 = raw/instant; 0.30 - 0.50 reduces micro-jitter.", theme::ACCENT_BLUE);
                        theme::guide_row(ui, "Decay", "How fast the stick centers when mouse stops. Prevents 125Hz packet-gap stutter.", theme::ACCENT_BLUE);
                        theme::guide_row(ui, "Invert Y", "Flips vertical look direction.", theme::ACCENT_BLUE);
                    });

                    ui.add_space(8.0);

                    theme::card_frame().show(ui, |ui| {
                        theme::section_header(ui, "Key Stick Sweep (WASD)");
                        theme::guide_row(ui, "Sweep Time", "Time to sweep smoothly past deadzone to full tilt (0 ms = instant 2D snap; 70 - 110 ms = smooth 3D/racing).", theme::ACCENT_GREEN);
                        theme::guide_row(ui, "Release Decay", "How quickly the stick returns to center on key release (25 - 40 ms for crisp stops).", theme::ACCENT_GREEN);
                    });

                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.show_slider_guide_modal = false;
                        }
                    });
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
                let (mut profile_changed, banner_rect) = controller_view::draw_controller(
                    ui,
                    &mut self.profile,
                    &snap.latest_state,
                    &mut self.listening_for,
                    &mut self.just_started_listening,
                    &mut self.bind_mode,
                );

                profile_changed |= controller_view::handle_listening_input(
                    ui,
                    &mut self.profile,
                    &mut self.listening_for,
                    &mut self.just_started_listening,
                    &mut self.prev_modifiers,
                    self.bind_mode,
                    banner_rect,
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
