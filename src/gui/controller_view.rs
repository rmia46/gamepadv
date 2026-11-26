use eframe::egui::{
    self, Align2, Color32, FontId, Key, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2,
};
use evdev::KeyCode;

use crate::engine::GamepadState;
use crate::gui::keymap::{egui_key_to_evdev, format_keycode, pointer_button_to_evdev};
use crate::profile::{GamepadAction, Profile, StickMode};

/// Returns a readable label for a [`GamepadAction`].
#[must_use]
pub fn action_display_name(action: GamepadAction) -> &'static str {
    match action {
        GamepadAction::South => "A (South)",
        GamepadAction::East => "B (East)",
        GamepadAction::North => "Y (North)",
        GamepadAction::West => "X (West)",
        GamepadAction::LeftBumper => "LB",
        GamepadAction::RightBumper => "RB",
        GamepadAction::LeftTrigger => "LT",
        GamepadAction::RightTrigger => "RT",
        GamepadAction::Select => "Select",
        GamepadAction::Start => "Start",
        GamepadAction::Mode => "Xbox (Mode)",
        GamepadAction::LeftThumb => "L3 (Left Stick Click)",
        GamepadAction::RightThumb => "R3 (Right Stick Click)",
        GamepadAction::DpadUp => "D-Pad Up",
        GamepadAction::DpadDown => "D-Pad Down",
        GamepadAction::DpadLeft => "D-Pad Left",
        GamepadAction::DpadRight => "D-Pad Right",
        GamepadAction::LeftStickUp => "Left Stick Up",
        GamepadAction::LeftStickDown => "Left Stick Down",
        GamepadAction::LeftStickLeft => "Left Stick Left",
        GamepadAction::LeftStickRight => "Left Stick Right",
        GamepadAction::RightStickUp => "Right Stick Up",
        GamepadAction::RightStickDown => "Right Stick Down",
        GamepadAction::RightStickLeft => "Right Stick Left",
        GamepadAction::RightStickRight => "Right Stick Right",
        GamepadAction::WalkModifier => "Walk Modifier",
    }
}

fn binding_label(profile: &Profile, action: GamepadAction) -> String {
    let keys = profile.bindings_for_action(action);
    match keys.len() {
        0 => "---".to_string(),
        1 => format_keycode(keys[0]),
        n => {
            let first = format_keycode(keys[0]);
            if n == 2 {
                let second = format_keycode(keys[1]);
                let joined = format!("{first}/{second}");
                if joined.len() <= 11 {
                    return joined;
                }
            }
            format!("{first} +{}", n - 1)
        }
    }
}

/// Captures a key or mouse button press when `listening_for` is active.
/// Returns `true` if `profile` was modified.
pub fn handle_listening_input(
    ui: &Ui,
    profile: &mut Profile,
    listening_for: &mut Option<GamepadAction>,
    just_started_listening: &mut bool,
    prev_modifiers: &mut egui::Modifiers,
) -> bool {
    let current_mods = ui.input(|i| i.modifiers);

    if *just_started_listening {
        *just_started_listening = false;
        *prev_modifiers = current_mods;
        return false;
    }

    let Some(action) = *listening_for else {
        *prev_modifiers = current_mods;
        return false;
    };

    let mut captured: Option<KeyCode> = None;
    let mut cancelled = false;

    ui.input(|i| {
        for event in &i.events {
            match event {
                egui::Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => {
                    cancelled = true;
                    break;
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } => {
                    if let Some(code) = egui_key_to_evdev(*key) {
                        captured = Some(code);
                        break;
                    }
                }
                egui::Event::PointerButton {
                    button,
                    pressed: true,
                    ..
                } => {
                    if let Some(code) = pointer_button_to_evdev(*button) {
                        captured = Some(code);
                        break;
                    }
                }
                _ => {}
            }
        }
    });

    // Fallback for standalone modifier presses (Shift/Ctrl/Alt) that may not emit Event::Key.
    if !cancelled && captured.is_none() {
        if current_mods.shift && !prev_modifiers.shift {
            captured = Some(KeyCode::KEY_LEFTSHIFT);
        } else if current_mods.ctrl && !prev_modifiers.ctrl {
            captured = Some(KeyCode::KEY_LEFTCTRL);
        } else if current_mods.alt && !prev_modifiers.alt {
            captured = Some(KeyCode::KEY_LEFTALT);
        }
    }

    *prev_modifiers = current_mods;

    if cancelled {
        *listening_for = None;
        return false;
    }

    if let Some(code) = captured {
        profile.add_binding(action, code);
        *listening_for = None;
        return true;
    }

    false
}

/// Draws the interactive Xbox 360 virtual controller and returns `true` if `profile` changed.
pub fn draw_controller(
    ui: &mut Ui,
    profile: &mut Profile,
    state: &GamepadState,
    listening_for: &mut Option<GamepadAction>,
    just_started_listening: &mut bool,
) -> bool {
    let mut changed = false;

    if let Some(action) = *listening_for {
        let current_keys: Vec<String> = profile
            .bindings_for_action(action)
            .into_iter()
            .map(format_keycode)
            .collect();
        let current_str = if current_keys.is_empty() {
            "none".to_string()
        } else {
            current_keys.join(", ")
        };

        egui::Frame::none()
            .fill(Color32::from_rgb(56, 44, 18))
            .stroke(Stroke::new(1.5_f32, Color32::from_rgb(245, 180, 50)))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(255, 215, 90),
                        format!(
                            "Adding binding for {} [current: {current_str}] — press key/mouse (Esc cancels)",
                            action_display_name(action)
                        ),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            *listening_for = None;
                        }
                        if !current_keys.is_empty() && ui.button("Clear All").clicked() {
                            profile.clear_action_bindings(action);
                            *listening_for = None;
                            changed = true;
                        }
                    });
                });
            });
        ui.add_space(6.0);
    }

    let available = ui.available_size();
    let canvas_w = available.x.clamp(560.0, 740.0);
    let canvas_h = 460.0;

    ui.vertical_centered(|ui| {
        let (canvas_rect, _) =
            ui.allocate_exact_size(Vec2::new(canvas_w, canvas_h), Sense::hover());

        let painter = ui.painter_at(canvas_rect);

        // Controller chassis backdrop.
        let body_rect = canvas_rect.shrink2(Vec2::new(12.0, 48.0));
        painter.rect(
            body_rect,
            Rounding::same(48.0),
            Color32::from_rgb(26, 29, 36),
            Stroke::new(2.0_f32, Color32::from_rgb(58, 65, 78)),
        );

        let cx = canvas_rect.center().x;
        let top_y = canvas_rect.top() + 8.0;

        // Top Shoulders: LT, LB, RB, RT
        let shoulder_w = 104.0;
        let trigger_h = 34.0;
        let bumper_h = 30.0;

        let lt_rect = Rect::from_min_size(
            Pos2::new(body_rect.left() + 26.0, top_y),
            Vec2::new(shoulder_w, trigger_h),
        );
        let lb_rect = Rect::from_min_size(
            Pos2::new(body_rect.left() + 26.0, top_y + trigger_h + 6.0),
            Vec2::new(shoulder_w, bumper_h),
        );
        let rt_rect = Rect::from_min_size(
            Pos2::new(body_rect.right() - 26.0 - shoulder_w, top_y),
            Vec2::new(shoulder_w, trigger_h),
        );
        let rb_rect = Rect::from_min_size(
            Pos2::new(body_rect.right() - 26.0 - shoulder_w, top_y + trigger_h + 6.0),
            Vec2::new(shoulder_w, bumper_h),
        );

        draw_bind_rect_button(
            ui,
            lt_rect,
            "LT",
            &binding_label(profile, GamepadAction::LeftTrigger),
            state.lt > 0,
            *listening_for == Some(GamepadAction::LeftTrigger),
            Color32::from_rgb(70, 145, 255),
            || {
                *listening_for = Some(GamepadAction::LeftTrigger);
                *just_started_listening = true;
            },
        );
        draw_bind_rect_button(
            ui,
            lb_rect,
            "LB",
            &binding_label(profile, GamepadAction::LeftBumper),
            state.lb,
            *listening_for == Some(GamepadAction::LeftBumper),
            Color32::from_rgb(95, 165, 255),
            || {
                *listening_for = Some(GamepadAction::LeftBumper);
                *just_started_listening = true;
            },
        );
        draw_bind_rect_button(
            ui,
            rt_rect,
            "RT",
            &binding_label(profile, GamepadAction::RightTrigger),
            state.rt > 0,
            *listening_for == Some(GamepadAction::RightTrigger),
            Color32::from_rgb(70, 145, 255),
            || {
                *listening_for = Some(GamepadAction::RightTrigger);
                *just_started_listening = true;
            },
        );
        draw_bind_rect_button(
            ui,
            rb_rect,
            "RB",
            &binding_label(profile, GamepadAction::RightBumper),
            state.rb,
            *listening_for == Some(GamepadAction::RightBumper),
            Color32::from_rgb(95, 165, 255),
            || {
                *listening_for = Some(GamepadAction::RightBumper);
                *just_started_listening = true;
            },
        );

        // Center Row: Select, Xbox (Mode), Start
        let center_y = body_rect.top() + 72.0;
        let select_rect =
            Rect::from_center_size(Pos2::new(cx - 72.0, center_y), Vec2::new(68.0, 32.0));
        let start_rect =
            Rect::from_center_size(Pos2::new(cx + 72.0, center_y), Vec2::new(68.0, 32.0));
        let xbox_center = Pos2::new(cx, center_y - 4.0);

        draw_bind_rect_button(
            ui,
            select_rect,
            "Select",
            &binding_label(profile, GamepadAction::Select),
            state.select,
            *listening_for == Some(GamepadAction::Select),
            Color32::from_rgb(130, 195, 255),
            || {
                *listening_for = Some(GamepadAction::Select);
                *just_started_listening = true;
            },
        );
        draw_bind_rect_button(
            ui,
            start_rect,
            "Start",
            &binding_label(profile, GamepadAction::Start),
            state.start,
            *listening_for == Some(GamepadAction::Start),
            Color32::from_rgb(130, 195, 255),
            || {
                *listening_for = Some(GamepadAction::Start);
                *just_started_listening = true;
            },
        );
        draw_face_button(
            ui,
            xbox_center,
            24.0,
            "Xbox",
            &binding_label(profile, GamepadAction::Mode),
            Color32::from_rgb(76, 175, 80),
            state.mode,
            *listening_for == Some(GamepadAction::Mode),
            || {
                *listening_for = Some(GamepadAction::Mode);
                *just_started_listening = true;
            },
        );

        // Left Stick (top-left interior)
        let left_stick_center = Pos2::new(body_rect.left() + 125.0, body_rect.top() + 110.0);
        draw_stick_well(
            ui,
            left_stick_center,
            46.0,
            state.left_x,
            state.left_y,
            state.thumbl,
            "LS",
        );
        let l3_rect = Rect::from_center_size(
            Pos2::new(left_stick_center.x, left_stick_center.y + 62.0),
            Vec2::new(82.0, 24.0),
        );
        draw_bind_rect_button(
            ui,
            l3_rect,
            "L3",
            &binding_label(profile, GamepadAction::LeftThumb),
            state.thumbl,
            *listening_for == Some(GamepadAction::LeftThumb),
            Color32::from_rgb(120, 185, 255),
            || {
                *listening_for = Some(GamepadAction::LeftThumb);
                *just_started_listening = true;
            },
        );

        // Face Buttons (top-right interior): Y (top), X (left), B (right), A (bottom)
        let face_center = Pos2::new(body_rect.right() - 125.0, body_rect.top() + 115.0);
        let face_offset = 38.0;
        let face_r = 22.0;

        draw_face_button(
            ui,
            Pos2::new(face_center.x, face_center.y - face_offset),
            face_r,
            "Y",
            &binding_label(profile, GamepadAction::North),
            Color32::from_rgb(235, 190, 45),
            state.north,
            *listening_for == Some(GamepadAction::North),
            || {
                *listening_for = Some(GamepadAction::North);
                *just_started_listening = true;
            },
        );
        draw_face_button(
            ui,
            Pos2::new(face_center.x - face_offset, face_center.y),
            face_r,
            "X",
            &binding_label(profile, GamepadAction::West),
            Color32::from_rgb(65, 145, 245),
            state.west,
            *listening_for == Some(GamepadAction::West),
            || {
                *listening_for = Some(GamepadAction::West);
                *just_started_listening = true;
            },
        );
        draw_face_button(
            ui,
            Pos2::new(face_center.x + face_offset, face_center.y),
            face_r,
            "B",
            &binding_label(profile, GamepadAction::East),
            Color32::from_rgb(225, 70, 70),
            state.east,
            *listening_for == Some(GamepadAction::East),
            || {
                *listening_for = Some(GamepadAction::East);
                *just_started_listening = true;
            },
        );
        draw_face_button(
            ui,
            Pos2::new(face_center.x, face_center.y + face_offset),
            face_r,
            "A",
            &binding_label(profile, GamepadAction::South),
            Color32::from_rgb(70, 195, 95),
            state.south,
            *listening_for == Some(GamepadAction::South),
            || {
                *listening_for = Some(GamepadAction::South);
                *just_started_listening = true;
            },
        );

        // D-Pad (bottom-left interior)
        let dpad_center = Pos2::new(cx - 115.0, body_rect.bottom() - 96.0);
        let dpad_step = 34.0;
        let dpad_size = Vec2::new(38.0, 30.0);

        draw_bind_rect_button(
            ui,
            Rect::from_center_size(
                Pos2::new(dpad_center.x, dpad_center.y - dpad_step),
                dpad_size,
            ),
            "▲",
            &binding_label(profile, GamepadAction::DpadUp),
            state.hat_y < 0,
            *listening_for == Some(GamepadAction::DpadUp),
            Color32::from_rgb(130, 195, 255),
            || {
                *listening_for = Some(GamepadAction::DpadUp);
                *just_started_listening = true;
            },
        );
        draw_bind_rect_button(
            ui,
            Rect::from_center_size(
                Pos2::new(dpad_center.x, dpad_center.y + dpad_step),
                dpad_size,
            ),
            "▼",
            &binding_label(profile, GamepadAction::DpadDown),
            state.hat_y > 0,
            *listening_for == Some(GamepadAction::DpadDown),
            Color32::from_rgb(130, 195, 255),
            || {
                *listening_for = Some(GamepadAction::DpadDown);
                *just_started_listening = true;
            },
        );
        draw_bind_rect_button(
            ui,
            Rect::from_center_size(
                Pos2::new(dpad_center.x - dpad_step - 4.0, dpad_center.y),
                dpad_size,
            ),
            "◀",
            &binding_label(profile, GamepadAction::DpadLeft),
            state.hat_x < 0,
            *listening_for == Some(GamepadAction::DpadLeft),
            Color32::from_rgb(130, 195, 255),
            || {
                *listening_for = Some(GamepadAction::DpadLeft);
                *just_started_listening = true;
            },
        );
        draw_bind_rect_button(
            ui,
            Rect::from_center_size(
                Pos2::new(dpad_center.x + dpad_step + 4.0, dpad_center.y),
                dpad_size,
            ),
            "▶",
            &binding_label(profile, GamepadAction::DpadRight),
            state.hat_x > 0,
            *listening_for == Some(GamepadAction::DpadRight),
            Color32::from_rgb(130, 195, 255),
            || {
                *listening_for = Some(GamepadAction::DpadRight);
                *just_started_listening = true;
            },
        );

        // Right Stick (bottom-right interior)
        let right_stick_center = Pos2::new(cx + 115.0, body_rect.bottom() - 106.0);
        draw_stick_well(
            ui,
            right_stick_center,
            46.0,
            state.right_x,
            state.right_y,
            state.thumbr,
            "RS",
        );
        let r3_rect = Rect::from_center_size(
            Pos2::new(right_stick_center.x, right_stick_center.y + 62.0),
            Vec2::new(82.0, 24.0),
        );
        draw_bind_rect_button(
            ui,
            r3_rect,
            "R3",
            &binding_label(profile, GamepadAction::RightThumb),
            state.thumbr,
            *listening_for == Some(GamepadAction::RightThumb),
            Color32::from_rgb(120, 185, 255),
            || {
                *listening_for = Some(GamepadAction::RightThumb);
                *just_started_listening = true;
            },
        );
    });

    ui.add_space(6.0);

    // Stick Mode Selectors + Directional Key Pills below the controller
    ui.columns(2, |cols| {
        if draw_stick_config_card(
            &mut cols[0],
            "Left Stick Mode",
            true,
            profile,
            [
                ("Up", GamepadAction::LeftStickUp),
                ("Down", GamepadAction::LeftStickDown),
                ("Left", GamepadAction::LeftStickLeft),
                ("Right", GamepadAction::LeftStickRight),
            ],
            listening_for,
            just_started_listening,
        ) {
            changed = true;
        }

        if draw_stick_config_card(
            &mut cols[1],
            "Right Stick Mode",
            false,
            profile,
            [
                ("Up", GamepadAction::RightStickUp),
                ("Down", GamepadAction::RightStickDown),
                ("Left", GamepadAction::RightStickLeft),
                ("Right", GamepadAction::RightStickRight),
            ],
            listening_for,
            just_started_listening,
        ) {
            changed = true;
        }
    });

    changed
}

#[allow(clippy::too_many_arguments)]
fn draw_stick_config_card(
    ui: &mut Ui,
    title: &str,
    is_left: bool,
    profile: &mut Profile,
    dir_actions: [(&str, GamepadAction); 4],
    listening_for: &mut Option<GamepadAction>,
    just_started_listening: &mut bool,
) -> bool {
    let mut changed = false;
    let current_mode = if is_left {
        profile.left_stick_mode
    } else {
        profile.right_stick_mode
    };

    egui::Frame::none()
        .fill(Color32::from_rgb(24, 27, 34))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(50, 56, 68)))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(title);
                ui.add_space(8.0);
                for (label, target) in [
                    ("Mouse", StickMode::Mouse),
                    ("Keys", StickMode::Keys),
                    ("Off", StickMode::Disabled),
                ] {
                    if ui.selectable_label(current_mode == target, label).clicked()
                        && current_mode != target
                    {
                        if is_left {
                            profile.left_stick_mode = target;
                        } else {
                            profile.right_stick_mode = target;
                        }
                        changed = true;
                    }
                }
            });

            if current_mode == StickMode::Keys {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for (dir_label, action) in dir_actions {
                        let is_listening = *listening_for == Some(action);
                        let key_text = binding_label(profile, action);
                        let btn_text = if is_listening {
                            format!("{dir_label}: [...]")
                        } else {
                            format!("{dir_label}: {key_text}")
                        };
                        let btn = egui::Button::new(btn_text).fill(if is_listening {
                            Color32::from_rgb(110, 82, 24)
                        } else {
                            Color32::from_rgb(38, 44, 56)
                        });
                        if ui.add(btn).clicked() {
                            *listening_for = Some(action);
                            *just_started_listening = true;
                        }
                    }
                });
            } else if current_mode == StickMode::Mouse {
                ui.add_space(4.0);
                ui.colored_label(
                    Color32::from_rgb(135, 190, 255),
                    "Driven by raw mouse motion (DSP)",
                );
            } else {
                ui.add_space(4.0);
                ui.colored_label(Color32::from_rgb(140, 145, 155), "Stick centered (off)");
            }
        });

    changed
}

fn draw_stick_well(
    ui: &Ui,
    center: Pos2,
    radius: f32,
    raw_x: i16,
    raw_y: i16,
    clicked: bool,
    label: &str,
) {
    let painter = ui.painter();

    painter.circle(
        center,
        radius,
        Color32::from_rgb(18, 20, 26),
        Stroke::new(
            if clicked { 2.5_f32 } else { 1.5_f32 },
            if clicked {
                Color32::from_rgb(110, 195, 255)
            } else {
                Color32::from_rgb(70, 78, 92)
            },
        ),
    );

    // Subtle crosshair
    let cross_color = Color32::from_rgb(45, 50, 62);
    painter.line_segment(
        [
            Pos2::new(center.x - radius + 6.0, center.y),
            Pos2::new(center.x + radius - 6.0, center.y),
        ],
        Stroke::new(1.0_f32, cross_color),
    );
    painter.line_segment(
        [
            Pos2::new(center.x, center.y - radius + 6.0),
            Pos2::new(center.x, center.y + radius - 6.0),
        ],
        Stroke::new(1.0_f32, cross_color),
    );

    // Deflection dot
    let nx = (raw_x as f32 / 32767.0).clamp(-1.0, 1.0);
    let ny = (raw_y as f32 / 32767.0).clamp(-1.0, 1.0);
    let travel = radius - 12.0;
    let dot_pos = Pos2::new(center.x + nx * travel, center.y + ny * travel);
    let active = nx.abs() > 0.02 || ny.abs() > 0.02 || clicked;

    painter.circle(
        dot_pos,
        11.0,
        if active {
            Color32::from_rgb(75, 155, 255)
        } else {
            Color32::from_rgb(65, 72, 86)
        },
        Stroke::new(1.5_f32, Color32::from_rgb(200, 220, 255)),
    );
    painter.text(
        dot_pos,
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(10.0),
        Color32::WHITE,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_bind_rect_button(
    ui: &mut Ui,
    rect: Rect,
    title: &str,
    key_label: &str,
    pressed: bool,
    listening: bool,
    accent: Color32,
    on_click: impl FnOnce(),
) {
    let resp = ui.allocate_rect(rect, Sense::click());
    let hovered = resp.hovered();

    let fill = if listening {
        Color32::from_rgb(105, 78, 22)
    } else if pressed {
        accent.linear_multiply(0.55)
    } else if hovered {
        Color32::from_rgb(48, 54, 68)
    } else {
        Color32::from_rgb(34, 39, 49)
    };

    let border_color = if listening {
        Color32::from_rgb(255, 205, 65)
    } else if pressed {
        accent
    } else if hovered {
        Color32::from_rgb(130, 145, 170)
    } else {
        Color32::from_rgb(66, 74, 90)
    };

    let painter = ui.painter();
    painter.rect(
        rect,
        Rounding::same(6.0),
        fill,
        Stroke::new(
            if pressed || listening { 2.0_f32 } else { 1.0_f32 },
            border_color,
        ),
    );

    let sub = if listening { "..." } else { key_label };
    if rect.height() >= 28.0 {
        painter.text(
            Pos2::new(rect.center().x, rect.center().y - 6.0),
            Align2::CENTER_CENTER,
            title,
            FontId::proportional(11.0),
            Color32::from_rgb(200, 210, 225),
        );
        painter.text(
            Pos2::new(rect.center().x, rect.center().y + 7.0),
            Align2::CENTER_CENTER,
            sub,
            FontId::proportional(10.5),
            Color32::WHITE,
        );
    } else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("{title}: {sub}"),
            FontId::proportional(10.5),
            Color32::WHITE,
        );
    }

    if resp.clicked() {
        on_click();
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_face_button(
    ui: &mut Ui,
    center: Pos2,
    radius: f32,
    title: &str,
    key_label: &str,
    color: Color32,
    pressed: bool,
    listening: bool,
    on_click: impl FnOnce(),
) {
    let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
    let resp = ui.allocate_rect(rect, Sense::click());
    let hovered = resp.hovered();

    let fill = if listening {
        Color32::from_rgb(115, 85, 22)
    } else if pressed {
        color
    } else if hovered {
        color.linear_multiply(0.45)
    } else {
        color.linear_multiply(0.25)
    };

    let stroke_color = if listening {
        Color32::from_rgb(255, 210, 70)
    } else if pressed || hovered {
        color
    } else {
        color.linear_multiply(0.7)
    };

    let painter = ui.painter();
    painter.circle(
        center,
        radius,
        fill,
        Stroke::new(
            if pressed || listening { 2.5_f32 } else { 1.5_f32 },
            stroke_color,
        ),
    );

    let sub = if listening { "..." } else { key_label };
    painter.text(
        Pos2::new(center.x, center.y - 6.0),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(13.0),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(center.x, center.y + 8.0),
        Align2::CENTER_CENTER,
        sub,
        FontId::proportional(9.5),
        Color32::from_rgb(235, 240, 248),
    );

    if resp.clicked() {
        on_click();
    }
}
