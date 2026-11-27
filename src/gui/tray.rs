use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use eframe::egui;
use ksni::blocking::TrayMethods;

/// Actions triggered from the desktop system tray icon/menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ToggleShowHide,
    ToggleActive,
    Quit,
}

/// D-Bus `StatusNotifierItem` tray integration for GamepadV.
pub struct GamepadVTray {
    tx: mpsc::Sender<TrayCommand>,
    egui_ctx: egui::Context,
    active: Arc<AtomicBool>,
    visible: Arc<AtomicBool>,
}

impl GamepadVTray {
    /// Spawns the system tray service in a background thread.
    pub fn spawn(
        egui_ctx: egui::Context,
        active: Arc<AtomicBool>,
        visible: Arc<AtomicBool>,
    ) -> Option<(ksni::blocking::Handle<Self>, mpsc::Receiver<TrayCommand>)> {
        let (tx, rx) = mpsc::channel();
        let tray = Self {
            tx,
            egui_ctx,
            active,
            visible,
        };
        match tray.spawn() {
            Ok(handle) => Some((handle, rx)),
            Err(err) => {
                eprintln!("Warning: failed to initialize system tray: {err}");
                None
            }
        }
    }

    fn send(&self, cmd: TrayCommand) {
        let _ = self.tx.send(cmd);
        self.egui_ctx.request_repaint();
    }
}

impl ksni::Tray for GamepadVTray {
    fn id(&self) -> String {
        "gamepadv".to_string()
    }

    fn title(&self) -> String {
        format!("GamepadV v{}", env!("CARGO_PKG_VERSION"))
    }

    fn icon_name(&self) -> String {
        "input-gaming".to_string()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![build_tray_icon(self.active.load(Ordering::Relaxed))]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(TrayCommand::ToggleShowHide);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{MenuItem, StandardItem};

        let is_visible = self.visible.load(Ordering::Relaxed);
        let is_active = self.active.load(Ordering::Relaxed);

        vec![
            StandardItem {
                label: if is_visible {
                    "Hide Window".to_string()
                } else {
                    "Show Window".to_string()
                },
                activate: Box::new(|this: &mut Self| this.send(TrayCommand::ToggleShowHide)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if is_active {
                    "Pause Capture (F8)".to_string()
                } else {
                    "Activate Capture (F8)".to_string()
                },
                activate: Box::new(|this: &mut Self| this.send(TrayCommand::ToggleActive)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit GamepadV".to_string(),
                activate: Box::new(|this: &mut Self| this.send(TrayCommand::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Generates a clean 24x24 ARGB32 rounded gamepad badge icon (green when active, blue when paused).
fn build_tray_icon(active: bool) -> ksni::Icon {
    const SIZE: i32 = 24;
    let mut data = vec![0u8; (SIZE * SIZE * 4) as usize];
    let (r, g, b) = if active {
        (34u8, 197u8, 94u8)
    } else {
        (59u8, 130u8, 246u8)
    };

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - 11.5;
            let dy = y as f32 - 11.5;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * SIZE + x) * 4) as usize;
            if dist <= 10.5 {
                // Inner white "G" / controller dot accent
                let inner_dot = ((dx + 3.0).hypot(dy) <= 2.2) || ((dx - 3.0).hypot(dy) <= 2.2);
                if inner_dot {
                    data[idx] = 255;
                    data[idx + 1] = 255;
                    data[idx + 2] = 255;
                    data[idx + 3] = 255;
                } else {
                    data[idx] = 255;
                    data[idx + 1] = r;
                    data[idx + 2] = g;
                    data[idx + 3] = b;
                }
            }
        }
    }

    ksni::Icon {
        width: SIZE,
        height: SIZE,
        data,
    }
}
