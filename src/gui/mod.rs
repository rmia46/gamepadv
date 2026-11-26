pub mod controller_view;
pub mod keymap;

use std::time::Duration;

use eframe::egui::{self, Color32, Rounding, Stroke};

use crate::engine::MouseStickConfig;
use crate::profile::{zelda_default, GamepadAction, Profile, ProfileConfig};
use crate::runtime::EngineHandle;

/// Main `eframe` application state for GamepadV.
pub struct GamepadVApp {
    engine: EngineHandle,
    profile: Profile,
    mouse_config: MouseStickConfig,
    listening_for: Option<GamepadAction>,
    just_started_listening: bool,
    prev_modifiers: egui::Modifiers,
    status_banner: Option<(String, bool)>,
}

impl GamepadVApp {
    /// Creates a new [`GamepadVApp`] and spawns the 250Hz background input engine.
    #[must_use]
    pub fn new(profile: Profile, mouse_config: MouseStickConfig, start_paused: bool) -> Self {
        let engine = EngineHandle::spawn(profile.clone(), mouse_config, start_paused);
        Self {
            engine,
            profile,
            mouse_config,
            listening_for: None,
            just_started_listening: false,
            prev_modifiers: egui::Modifiers::default(),
            status_banner: None,
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
        let snap = self.engine.snapshot();

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
                        "● ACTIVE  (F8)"
                    } else {
                        "❚❚ PAUSED  (F8)"
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

        if snap.active || snap.unlocking || self.listening_for.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
    }
}
