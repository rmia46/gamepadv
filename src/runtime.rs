use std::collections::HashSet;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use evdev::{Device, EventSummary, KeyCode, RelativeAxisCode};

use crate::engine::{GamepadState, InputMapper, MouseStickConfig, MouseStickEngine};
use crate::input::{probe_input_devices, InputRig};
use crate::profile::Profile;
use crate::virtual_pad::VirtualXbox360;

const SPIN_MARGIN: Duration = Duration::from_micros(750);

/// Read-only snapshot of the background 250Hz engine state for the GUI.
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    pub active: bool,
    pub latest_state: GamepadState,
    pub keyboard_count: usize,
    pub mouse_count: usize,
    pub status_message: String,
    pub permission_error: Option<String>,
}

#[derive(Debug)]
struct EngineShared {
    active: bool,
    requested_active: bool,
    latest_state: GamepadState,
    keyboard_count: usize,
    mouse_count: usize,
    status_message: String,
    permission_error: Option<String>,
    pending_profile: Option<Profile>,
    pending_mouse_config: Option<MouseStickConfig>,
}

/// Handle to the dedicated high-frequency `evdev` + `uinput` background thread.
pub struct EngineHandle {
    running: Arc<AtomicBool>,
    shared: Arc<Mutex<EngineShared>>,
    join_handle: Option<JoinHandle<()>>,
}

impl EngineHandle {
    /// Spawns the 250Hz+ input/uinput engine thread.
    ///
    /// If `/dev/input/event*` permissions are missing or `/dev/uinput` fails,
    /// the thread records the error in `permission_error` / `status_message`
    /// without panicking so the GUI can display actionable guidance.
    pub fn spawn(profile: Profile, mouse_config: MouseStickConfig, start_paused: bool) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let initial_active = !start_paused;
        let shared = Arc::new(Mutex::new(EngineShared {
            active: false,
            requested_active: initial_active,
            latest_state: GamepadState::default(),
            keyboard_count: 0,
            mouse_count: 0,
            status_message: "Initializing input engine...".to_string(),
            permission_error: None,
            pending_profile: None,
            pending_mouse_config: None,
        }));

        let r_flag = Arc::clone(&running);
        let s_ref = Arc::clone(&shared);

        let join_handle = thread::Builder::new()
            .name("gamepadv-engine".to_string())
            .spawn(move || run_engine_thread(r_flag, s_ref, profile, mouse_config))
            .ok();

        Self {
            running,
            shared,
            join_handle,
        }
    }

    /// Returns the latest snapshot of engine status and virtual gamepad state.
    pub fn snapshot(&self) -> EngineSnapshot {
        let guard = self.shared.lock().expect("engine mutex poisoned");
        EngineSnapshot {
            active: guard.active,
            latest_state: guard.latest_state,
            keyboard_count: guard.keyboard_count,
            mouse_count: guard.mouse_count,
            status_message: guard.status_message.clone(),
            permission_error: guard.permission_error.clone(),
        }
    }

    /// Requests the engine thread to activate or pause capture (`EVIOCGRAB`).
    pub fn set_active(&self, active: bool) {
        let mut guard = self.shared.lock().expect("engine mutex poisoned");
        guard.requested_active = active;
    }

    /// Sends an updated profile to the engine thread (applied on the next tick).
    pub fn update_profile(&self, profile: Profile) {
        let mut guard = self.shared.lock().expect("engine mutex poisoned");
        guard.pending_profile = Some(profile);
    }

    /// Sends an updated mouse DSP configuration to the engine thread.
    pub fn update_mouse_config(&self, config: MouseStickConfig) {
        let mut guard = self.shared.lock().expect("engine mutex poisoned");
        guard.pending_mouse_config = Some(config);
    }

    /// Signals the background thread to exit cleanly and waits for it to ungrab.
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn set_engine_error(shared: &Arc<Mutex<EngineShared>>, msg: String, is_perm: bool) {
    let mut guard = shared.lock().expect("engine mutex poisoned");
    guard.active = false;
    guard.requested_active = false;
    guard.status_message = msg.clone();
    if is_perm {
        guard.permission_error = Some(msg);
    }
}

fn run_engine_thread(
    running: Arc<AtomicBool>,
    shared: Arc<Mutex<EngineShared>>,
    profile: Profile,
    mouse_config: MouseStickConfig,
) {
    match probe_input_devices() {
        Ok(report)
            if report.event_nodes_readable == 0 && report.event_nodes_permission_denied > 0 =>
        {
            set_engine_error(
                &shared,
                format!(
                    "Permission denied reading /dev/input/event* ({} nodes denied). See docs/udev-setup.md to grant uaccess or input group permissions.",
                    report.event_nodes_permission_denied
                ),
                true,
            );
            return;
        }
        Err(err) => {
            set_engine_error(
                &shared,
                format!("Failed to probe /dev/input/event*: {err}"),
                false,
            );
            return;
        }
        Ok(_) => {}
    }

    let mut virtual_pad = match VirtualXbox360::new() {
        Ok(p) => p,
        Err(err) => {
            set_engine_error(
                &shared,
                format!("Failed to create virtual Xbox 360 pad on /dev/uinput: {err}"),
                true,
            );
            return;
        }
    };

    let mut rig = match InputRig::discover() {
        Ok(r) => r,
        Err(err) => {
            set_engine_error(&shared, format!("Failed to initialize input rig: {err}"), false);
            return;
        }
    };

    if rig.keyboard_count() == 0 && rig.mouse_count() == 0 {
        set_engine_error(
            &shared,
            "No readable keyboards or mice found under /dev/input/event*".to_string(),
            false,
        );
        return;
    }

    let mut mapper = InputMapper::new(profile, MouseStickEngine::new(mouse_config));
    let mut passthrough_pressed: HashSet<KeyCode> = HashSet::new();
    let mut active = false;

    let initial_requested = {
        let mut guard = shared.lock().expect("engine mutex poisoned");
        guard.keyboard_count = rig.keyboard_count();
        guard.mouse_count = rig.mouse_count();
        guard.status_message = if guard.requested_active {
            "ACTIVE (cursor grabbed, F8 to pause)".to_string()
        } else {
            "PAUSED (press F8 or click Activate)".to_string()
        };
        guard.requested_active
    };

    if initial_requested {
        match rig.set_grab(true) {
            Ok(()) => active = true,
            Err(err) => {
                set_engine_error(&shared, format!("Failed to grab input devices: {err}"), false);
                return;
            }
        }
        let mut guard = shared.lock().expect("engine mutex poisoned");
        guard.active = true;
    }

    let mut current_hz = mouse_config.hz.max(1.0);
    let mut tick_period = Duration::from_secs_f64(1.0 / current_hz);
    let mut last_tick = Instant::now();
    let mut next_tick = last_tick + tick_period;

    while running.load(Ordering::Relaxed) {
        // Apply any pending GUI updates without blocking.
        let requested_active: bool;
        {
            let mut guard = shared.lock().expect("engine mutex poisoned");
            if let Some(new_profile) = guard.pending_profile.take() {
                mapper.set_profile(new_profile);
            }
            if let Some(new_cfg) = guard.pending_mouse_config.take() {
                current_hz = new_cfg.hz.max(1.0);
                tick_period = Duration::from_secs_f64(1.0 / current_hz);
                mapper.set_mouse_config(new_cfg);
            }
            requested_active = guard.requested_active;
        }

        if requested_active != active {
            if let Err(err) = apply_active_transition(
                requested_active,
                &mut rig,
                &mut mapper,
                &mut virtual_pad,
                &mut passthrough_pressed,
            ) {
                set_engine_error(&shared, format!("Grab transition error: {err}"), false);
                break;
            }
            active = requested_active;
            let mut guard = shared.lock().expect("engine mutex poisoned");
            guard.active = active;
            guard.status_message = if active {
                "ACTIVE (cursor grabbed, F8 to pause)".to_string()
            } else {
                "PAUSED (press F8 or click Activate)".to_string()
            };
        }

        let now = Instant::now();
        let dt_secs = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;

        let toggle_key = mapper.profile().toggle_key;
        let mut toggle_pressed = false;
        if let Err(err) = drain_events(
            &mut rig,
            &mut mapper,
            toggle_key,
            active,
            &mut toggle_pressed,
            &mut passthrough_pressed,
        ) {
            set_engine_error(&shared, format!("Input loop error: {err}"), false);
            break;
        }

        if toggle_pressed {
            let next_active = !active;
            if let Err(err) = apply_active_transition(
                next_active,
                &mut rig,
                &mut mapper,
                &mut virtual_pad,
                &mut passthrough_pressed,
            ) {
                set_engine_error(&shared, format!("Grab toggle error: {err}"), false);
                break;
            }
            active = next_active;
            let mut guard = shared.lock().expect("engine mutex poisoned");
            guard.active = active;
            guard.requested_active = active;
            guard.status_message = if active {
                "ACTIVE (cursor grabbed, F8 to pause)".to_string()
            } else {
                "PAUSED (press F8 or click Activate)".to_string()
            };
        }

        let state = if active {
            let s = mapper.tick(dt_secs);
            if let Err(err) = virtual_pad.sync_state(&s) {
                set_engine_error(&shared, format!("Virtual pad sync error: {err}"), false);
                break;
            }
            s
        } else {
            GamepadState::default()
        };

        {
            let mut guard = shared.lock().expect("engine mutex poisoned");
            guard.latest_state = state;
        }

        let after_work = Instant::now();
        if next_tick > after_work {
            let remaining = next_tick - after_work;
            if remaining > SPIN_MARGIN {
                thread::sleep(remaining - SPIN_MARGIN);
            }
            while Instant::now() < next_tick {
                std::hint::spin_loop();
            }
        }
        next_tick += tick_period;
        if Instant::now() > next_tick + tick_period {
            next_tick = Instant::now() + tick_period;
        }
    }

    let _ = release_passthrough_keys(&mut rig, &mut passthrough_pressed);
    let _ = rig.set_grab(false);
    let _ = virtual_pad.reset_neutral();
}

fn apply_active_transition(
    next_active: bool,
    rig: &mut InputRig,
    mapper: &mut InputMapper,
    virtual_pad: &mut VirtualXbox360,
    passthrough_pressed: &mut HashSet<KeyCode>,
) -> std::io::Result<()> {
    if !next_active {
        release_passthrough_keys(rig, passthrough_pressed)?;
    }
    rig.set_grab(next_active)?;
    mapper.reset();
    virtual_pad.reset_neutral()?;
    Ok(())
}

fn drain_device_slice(
    devices: &mut [Device],
    mapper: &mut InputMapper,
    toggle_key: KeyCode,
    active: bool,
    toggle_pressed: &mut bool,
    passthrough_queue: &mut Vec<(KeyCode, i32)>,
) -> std::io::Result<()> {
    for dev in devices.iter_mut() {
        loop {
            match dev.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        match ev.destructure() {
                            EventSummary::Key(_, code, value) => {
                                if code == toggle_key {
                                    if value == 1 {
                                        *toggle_pressed = true;
                                    }
                                    continue;
                                }
                                if !active {
                                    continue;
                                }
                                if value == 0 || value == 1 {
                                    if !mapper.handle_key(code, value == 1) {
                                        passthrough_queue.push((code, value));
                                    }
                                } else if value == 2
                                    && !mapper.profile().bindings.contains_key(&code)
                                {
                                    passthrough_queue.push((code, 2));
                                }
                            }
                            EventSummary::RelativeAxis(_, axis, value) if active => match axis {
                                RelativeAxisCode::REL_X => {
                                    mapper.feed_mouse_delta(value as f64, 0.0);
                                }
                                RelativeAxisCode::REL_Y => {
                                    mapper.feed_mouse_delta(0.0, value as f64);
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Err(err),
            }
        }
    }
    Ok(())
}

fn release_passthrough_keys(
    rig: &mut InputRig,
    passthrough_pressed: &mut HashSet<KeyCode>,
) -> std::io::Result<()> {
    for code in passthrough_pressed.drain() {
        rig.forward_passthrough_key(code, 0)?;
    }
    Ok(())
}

fn drain_events(
    rig: &mut InputRig,
    mapper: &mut InputMapper,
    toggle_key: KeyCode,
    active: bool,
    toggle_pressed: &mut bool,
    passthrough_pressed: &mut HashSet<KeyCode>,
) -> std::io::Result<()> {
    let mut passthrough_queue: Vec<(KeyCode, i32)> = Vec::new();

    drain_device_slice(
        rig.keyboards_mut(),
        mapper,
        toggle_key,
        active,
        toggle_pressed,
        &mut passthrough_queue,
    )?;
    drain_device_slice(
        rig.mice_mut(),
        mapper,
        toggle_key,
        active,
        toggle_pressed,
        &mut passthrough_queue,
    )?;

    for (code, value) in passthrough_queue {
        if value == 1 {
            passthrough_pressed.insert(code);
        } else if value == 0 {
            passthrough_pressed.remove(&code);
        }
        rig.forward_passthrough_key(code, value)?;
    }

    Ok(())
}
