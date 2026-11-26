use std::collections::HashSet;
use std::io::{self, BufReader, ErrorKind};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use evdev::{Device, EventSummary, KeyCode, RelativeAxisCode};

use crate::engine::{GamepadState, InputMapper, MouseStickConfig, MouseStickEngine};
use crate::input::{probe_input_devices, InputRig};
use crate::ipc::{
    bind_restricted_socket, current_uid_gid, default_socket_path, read_json_line,
    write_json_line, IpcCommand, IpcSnapshot,
};
use crate::profile::{key_display_name, Profile, ProfileConfig};
use crate::virtual_pad::VirtualXbox360;

const SPIN_MARGIN: Duration = Duration::from_micros(750);

/// Read-only snapshot of the background 250Hz engine state for the GUI.
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    pub active: bool,
    pub unlocking: bool,
    pub latest_state: GamepadState,
    pub keyboard_count: usize,
    pub mouse_count: usize,
    pub status_message: String,
    pub permission_error: Option<String>,
    pub last_key: Option<String>,
    pub mouse_delta: (i32, i32),
}

#[derive(Debug)]
struct EngineShared {
    active: bool,
    unlocking: bool,
    requested_active: bool,
    latest_state: GamepadState,
    keyboard_count: usize,
    mouse_count: usize,
    status_message: String,
    permission_error: Option<String>,
    last_key: Option<String>,
    mouse_delta: (i32, i32),
    pending_profile: Option<Profile>,
    pending_mouse_config: Option<MouseStickConfig>,
    latest_profile: Profile,
    latest_mouse_config: MouseStickConfig,
}

/// Handle to the dedicated high-frequency `evdev` + `uinput` engine (either in-process
/// or connected via Unix-socket IPC to a `pkexec`-elevated `--daemon` child).
pub struct EngineHandle {
    running: Arc<AtomicBool>,
    shared: Arc<Mutex<EngineShared>>,
    join_handle: Option<JoinHandle<()>>,
    ipc_writer: Arc<Mutex<Option<UnixStream>>>,
    pkexec_child: Arc<Mutex<Option<Child>>>,
}

impl EngineHandle {
    /// Spawns the 250Hz+ input/uinput engine thread in-process.
    pub fn spawn(profile: Profile, mouse_config: MouseStickConfig, start_paused: bool) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let initial_active = !start_paused;
        let shared = Arc::new(Mutex::new(EngineShared {
            active: false,
            unlocking: false,
            requested_active: initial_active,
            latest_state: GamepadState::default(),
            keyboard_count: 0,
            mouse_count: 0,
            status_message: "Initializing input engine...".to_string(),
            permission_error: None,
            last_key: None,
            mouse_delta: (0, 0),
            pending_profile: None,
            pending_mouse_config: None,
            latest_profile: profile.clone(),
            latest_mouse_config: mouse_config,
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
            ipc_writer: Arc::new(Mutex::new(None)),
            pkexec_child: Arc::new(Mutex::new(None)),
        }
    }

    /// Launches a privileged `gamepadv --daemon` via `pkexec` and connects over a
    /// user-restricted (`0600`) Unix socket so the GUI stays unprivileged.
    pub fn unlock_with_pkexec(&mut self, profile: Profile, mouse_config: MouseStickConfig) {
        self.shutdown();

        self.running = Arc::new(AtomicBool::new(true));
        {
            let mut guard = self.shared.lock().expect("engine mutex poisoned");
            guard.active = false;
            guard.requested_active = false;
            guard.unlocking = true;
            guard.permission_error = None;
            guard.status_message = "Waiting for pkexec authorization...".to_string();
            guard.latest_profile = profile.clone();
            guard.latest_mouse_config = mouse_config;
        }

        let r_flag = Arc::clone(&self.running);
        let s_ref = Arc::clone(&self.shared);
        let writer_ref = Arc::clone(&self.ipc_writer);
        let child_ref = Arc::clone(&self.pkexec_child);

        self.join_handle = thread::Builder::new()
            .name("gamepadv-ipc-client".to_string())
            .spawn(move || {
                run_pkexec_ipc_client(r_flag, s_ref, writer_ref, child_ref, profile, mouse_config)
            })
            .ok();
    }

    /// Returns the latest snapshot of engine status and virtual gamepad state.
    pub fn snapshot(&self) -> EngineSnapshot {
        let guard = self.shared.lock().expect("engine mutex poisoned");
        Self::snapshot_from_guard(&guard)
    }

    /// Non-blocking snapshot read for the GUI thread (returns `None` if contended).
    pub fn try_snapshot(&self) -> Option<EngineSnapshot> {
        self.shared
            .try_lock()
            .ok()
            .map(|guard| Self::snapshot_from_guard(&guard))
    }

    fn snapshot_from_guard(guard: &EngineShared) -> EngineSnapshot {
        EngineSnapshot {
            active: guard.active,
            unlocking: guard.unlocking,
            latest_state: guard.latest_state,
            keyboard_count: guard.keyboard_count,
            mouse_count: guard.mouse_count,
            status_message: guard.status_message.clone(),
            permission_error: guard.permission_error.clone(),
            last_key: guard.last_key.clone(),
            mouse_delta: guard.mouse_delta,
        }
    }

    /// Requests the engine to activate or pause capture (`EVIOCGRAB`).
    pub fn set_active(&self, active: bool) {
        if let Some(stream) = self.ipc_writer.lock().expect("ipc mutex poisoned").as_mut() {
            let _ = write_json_line(stream, &IpcCommand::SetActive(active));
            return;
        }
        let mut guard = self.shared.lock().expect("engine mutex poisoned");
        guard.requested_active = active;
    }

    /// Sends an updated profile to the engine.
    pub fn update_profile(&self, profile: Profile) {
        let cfg = {
            let mut guard = self.shared.lock().expect("engine mutex poisoned");
            guard.latest_profile = profile.clone();
            guard.pending_profile = Some(profile.clone());
            ProfileConfig::from_parts(&profile, &guard.latest_mouse_config)
        };
        if let Some(stream) = self.ipc_writer.lock().expect("ipc mutex poisoned").as_mut() {
            let _ = write_json_line(stream, &IpcCommand::UpdateProfile(cfg));
        }
    }

    /// Sends an updated mouse DSP configuration to the engine.
    pub fn update_mouse_config(&self, config: MouseStickConfig) {
        let cfg = {
            let mut guard = self.shared.lock().expect("engine mutex poisoned");
            guard.latest_mouse_config = config;
            guard.pending_mouse_config = Some(config);
            ProfileConfig::from_parts(&guard.latest_profile, &config)
        };
        if let Some(stream) = self.ipc_writer.lock().expect("ipc mutex poisoned").as_mut() {
            let _ = write_json_line(stream, &IpcCommand::UpdateProfile(cfg));
        }
    }

    /// Signals the engine (or remote daemon) to exit cleanly and ungrab all devices.
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(stream) = self
            .ipc_writer
            .lock()
            .expect("ipc mutex poisoned")
            .take()
            .as_mut()
        {
            let _ = write_json_line(stream, &IpcCommand::Shutdown);
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
        if let Some(mut child) = self.pkexec_child.lock().expect("child mutex poisoned").take() {
            let _ = child.wait();
        }
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_pkexec_ipc_client(
    running: Arc<AtomicBool>,
    shared: Arc<Mutex<EngineShared>>,
    ipc_writer: Arc<Mutex<Option<UnixStream>>>,
    pkexec_child: Arc<Mutex<Option<Child>>>,
    profile: Profile,
    mouse_config: MouseStickConfig,
) {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(err) => {
            set_unlock_failed(&shared, format!("Failed to locate gamepadv binary: {err}"));
            return;
        }
    };
    let (uid, gid) = current_uid_gid().unwrap_or((1000, 1000));
    let sock_path = default_socket_path();

    let child = match Command::new("pkexec")
        .arg(&exe)
        .arg("--daemon")
        .arg("--socket")
        .arg(&sock_path)
        .arg("--owner-uid")
        .arg(uid.to_string())
        .arg("--owner-gid")
        .arg(gid.to_string())
        .arg("--start-paused")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(err) => {
            set_unlock_failed(&shared, format!("Failed to launch pkexec: {err}"));
            return;
        }
    };

    *pkexec_child.lock().expect("child mutex poisoned") = Some(child);

    // Wait up to 45s for the user to enter their password and the daemon to bind the socket.
    let deadline = Instant::now() + Duration::from_secs(45);
    let stream = loop {
        if !running.load(Ordering::Relaxed) {
            return;
        }
        match UnixStream::connect(&sock_path) {
            Ok(s) => break s,
            Err(_) => {
                let exited = {
                    let mut guard = pkexec_child.lock().expect("child mutex poisoned");
                    guard
                        .as_mut()
                        .and_then(|c| c.try_wait().ok().flatten())
                };
                if let Some(status) = exited {
                    set_unlock_failed(
                        &shared,
                        format!("pkexec authorization cancelled or failed (exit: {status})"),
                    );
                    return;
                }
                if Instant::now() >= deadline {
                    set_unlock_failed(
                        &shared,
                        "Timed out waiting for pkexec daemon socket.".to_string(),
                    );
                    return;
                }
                thread::sleep(Duration::from_millis(60));
            }
        }
    };

    let mut writer = match stream.try_clone() {
        Ok(w) => w,
        Err(err) => {
            set_unlock_failed(&shared, format!("IPC stream clone failed: {err}"));
            return;
        }
    };

    let initial_cfg = ProfileConfig::from_parts(&profile, &mouse_config);
    let _ = write_json_line(&mut writer, &IpcCommand::UpdateProfile(initial_cfg));
    *ipc_writer.lock().expect("ipc mutex poisoned") = Some(writer);

    {
        let mut guard = shared.lock().expect("engine mutex poisoned");
        guard.unlocking = false;
        guard.permission_error = None;
        guard.status_message = "Connected to privileged input daemon (PAUSED)".to_string();
    }

    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let mut reader = BufReader::new(stream);
    while running.load(Ordering::Relaxed) {
        match read_json_line::<IpcSnapshot>(&mut reader) {
            Ok(Some(snap)) => {
                let mut guard = shared.lock().expect("engine mutex poisoned");
                guard.active = snap.active;
                guard.latest_state = snap.latest_state;
                guard.keyboard_count = snap.keyboard_count;
                guard.mouse_count = snap.mouse_count;
                guard.status_message = snap.status_message;
                guard.permission_error = snap.permission_error;
                if snap.last_key.is_some() {
                    guard.last_key = snap.last_key;
                }
                guard.mouse_delta = snap.mouse_delta;
            }
            Ok(None) => break,
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => break,
        }
    }

    *ipc_writer.lock().expect("ipc mutex poisoned") = None;
}

fn set_unlock_failed(shared: &Arc<Mutex<EngineShared>>, msg: String) {
    let mut guard = shared.lock().expect("engine mutex poisoned");
    guard.unlocking = false;
    guard.active = false;
    guard.status_message = msg.clone();
    guard.permission_error = Some(msg);
}

/// Runs the privileged `--daemon` server bound to `socket_path` for GUI IPC.
pub fn run_daemon_socket(
    socket_path: &Path,
    owner: Option<(u32, u32)>,
    profile: Profile,
    mouse_config: MouseStickConfig,
    start_paused: bool,
) -> io::Result<()> {
    let (listener, _socket_guard) = bind_restricted_socket(socket_path, owner)?;
    let (stream, _) = listener.accept()?;

    let engine = Arc::new(Mutex::new(EngineHandle::spawn(
        profile,
        mouse_config,
        start_paused,
    )));
    let daemon_running = Arc::new(AtomicBool::new(true));
    let r_flag = Arc::clone(&daemon_running);
    let _ = ctrlc::set_handler(move || {
        r_flag.store(false, Ordering::SeqCst);
    });

    let mut writer = stream.try_clone()?;
    let cmd_running = Arc::clone(&daemon_running);
    let cmd_engine = Arc::clone(&engine);

    let cmd_reader_handle = thread::spawn(move || {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
        let mut reader = BufReader::new(stream);
        while cmd_running.load(Ordering::Relaxed) {
            match read_json_line::<IpcCommand>(&mut reader) {
                Ok(Some(cmd)) => {
                    let eng = cmd_engine.lock().expect("daemon engine mutex poisoned");
                    match cmd {
                        IpcCommand::SetActive(val) => eng.set_active(val),
                        IpcCommand::UpdateProfile(cfg) => {
                            let (p, m) = cfg.into_parts();
                            eng.update_profile(p);
                            eng.update_mouse_config(m);
                        }
                        IpcCommand::Shutdown => {
                            cmd_running.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                }
                Ok(None) => {
                    cmd_running.store(false, Ordering::SeqCst);
                    break;
                }
                Err(err)
                    if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
                {
                    continue;
                }
                Err(_) => {
                    cmd_running.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }
    });

    // Wait briefly for initial device discovery, then stream snapshots to the GUI.
    thread::sleep(Duration::from_millis(80));
    while daemon_running.load(Ordering::Relaxed) {
        let snap = {
            let eng = engine.lock().expect("daemon engine mutex poisoned");
            eng.snapshot()
        };
        let ipc_snap = IpcSnapshot {
            active: snap.active,
            latest_state: snap.latest_state,
            keyboard_count: snap.keyboard_count,
            mouse_count: snap.mouse_count,
            status_message: snap.status_message,
            permission_error: snap.permission_error,
            last_key: snap.last_key,
            mouse_delta: snap.mouse_delta,
        };
        if write_json_line(&mut writer, &ipc_snap).is_err() {
            daemon_running.store(false, Ordering::SeqCst);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    {
        let mut eng = engine.lock().expect("daemon engine mutex poisoned");
        eng.shutdown();
    }
    let _ = cmd_reader_handle.join();
    Ok(())
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
                    "Permission denied reading /dev/input/event* ({} nodes denied).",
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
        let mut last_key_event: Option<String> = None;
        let mut tick_mouse_delta: (i32, i32) = (0, 0);
        if let Err(err) = drain_events(
            &mut rig,
            &mut mapper,
            toggle_key,
            active,
            &mut toggle_pressed,
            &mut passthrough_pressed,
            &mut last_key_event,
            &mut tick_mouse_delta,
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

        // Tick the mapper even when paused so the GUI shows live stick/button preview,
        // but only emit to VirtualXbox360 when active.
        let state = mapper.tick(dt_secs);
        if active {
            if let Err(err) = virtual_pad.sync_state(&state) {
                set_engine_error(&shared, format!("Virtual pad sync error: {err}"), false);
                break;
            }
        }

        {
            let mut guard = shared.lock().expect("engine mutex poisoned");
            guard.latest_state = state;
            if let Some(k) = last_key_event {
                guard.last_key = Some(k);
            }
            if tick_mouse_delta != (0, 0) {
                guard.mouse_delta = tick_mouse_delta;
            }
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

#[allow(clippy::too_many_arguments)]
fn drain_device_slice(
    devices: &mut [Device],
    mapper: &mut InputMapper,
    toggle_key: KeyCode,
    active: bool,
    toggle_pressed: &mut bool,
    passthrough_queue: &mut Vec<(KeyCode, i32)>,
    last_key_event: &mut Option<String>,
    tick_mouse_delta: &mut (i32, i32),
) -> std::io::Result<()> {
    for dev in devices.iter_mut() {
        loop {
            match dev.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        match ev.destructure() {
                            EventSummary::Key(_, code, value) => {
                                if value == 0 || value == 1 {
                                    *last_key_event = Some(format!(
                                        "{} ({})",
                                        key_display_name(code),
                                        if value == 1 { "down" } else { "up" }
                                    ));
                                }
                                if code == toggle_key {
                                    if value == 1 {
                                        *toggle_pressed = true;
                                    }
                                    continue;
                                }
                                if value == 0 || value == 1 {
                                    let handled = mapper.handle_key(code, value == 1);
                                    if active && !handled {
                                        passthrough_queue.push((code, value));
                                    }
                                } else if active
                                    && value == 2
                                    && !mapper.profile().bindings.contains_key(&code)
                                {
                                    passthrough_queue.push((code, 2));
                                }
                            }
                            EventSummary::RelativeAxis(_, axis, value) => match axis {
                                RelativeAxisCode::REL_X => {
                                    tick_mouse_delta.0 += value;
                                    mapper.feed_mouse_delta(value as f64, 0.0);
                                }
                                RelativeAxisCode::REL_Y => {
                                    tick_mouse_delta.1 += value;
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

#[allow(clippy::too_many_arguments)]
fn drain_events(
    rig: &mut InputRig,
    mapper: &mut InputMapper,
    toggle_key: KeyCode,
    active: bool,
    toggle_pressed: &mut bool,
    passthrough_pressed: &mut HashSet<KeyCode>,
    last_key_event: &mut Option<String>,
    tick_mouse_delta: &mut (i32, i32),
) -> std::io::Result<()> {
    let mut passthrough_queue: Vec<(KeyCode, i32)> = Vec::new();

    drain_device_slice(
        rig.keyboards_mut(),
        mapper,
        toggle_key,
        active,
        toggle_pressed,
        &mut passthrough_queue,
        last_key_event,
        tick_mouse_delta,
    )?;
    drain_device_slice(
        rig.mice_mut(),
        mapper,
        toggle_key,
        active,
        toggle_pressed,
        &mut passthrough_queue,
        last_key_event,
        tick_mouse_delta,
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

/// Passive CLI live input monitor (`--monitor`): prints raw key presses and mouse
/// movements without `EVIOCGRAB` or `uinput` output.
pub fn run_monitor() -> i32 {
    let report = match probe_input_devices() {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Error probing /dev/input/event*: {err}");
            return 1;
        }
    };

    let mut rig = match InputRig::discover() {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Error discovering input devices: {err}");
            return 1;
        }
    };

    if rig.keyboard_count() == 0 && rig.mouse_count() == 0 {
        if report.event_nodes_permission_denied > 0 {
            eprintln!(
                "Error: Permission denied reading /dev/input/event* ({} nodes denied).\n\
                 Run with `sudo` (e.g. `sudo ./target/release/gamepadv --monitor`) or see docs/udev-setup.md.",
                report.event_nodes_permission_denied
            );
        } else {
            eprintln!("Error: no readable keyboards or mice found under /dev/input/event*");
        }
        return 1;
    }

    let running = Arc::new(AtomicBool::new(true));
    let r_flag = Arc::clone(&running);
    let _ = ctrlc::set_handler(move || {
        r_flag.store(false, Ordering::SeqCst);
    });

    println!(
        "[GamepadV Monitor] Listening passively on {} keyboard(s), {} mouse/mice (no grab). Move mouse or press keys (Ctrl+C to quit)...",
        rig.keyboard_count(),
        rig.mouse_count()
    );

    while running.load(Ordering::Relaxed) {
        let mut dx = 0i32;
        let mut dy = 0i32;
        if let Err(err) = poll_monitor_slice(rig.keyboards_mut(), &mut dx, &mut dy) {
            eprintln!("Read error: {err}");
            return 1;
        }
        if let Err(err) = poll_monitor_slice(rig.mice_mut(), &mut dx, &mut dy) {
            eprintln!("Read error: {err}");
            return 1;
        }
        if dx != 0 || dy != 0 {
            println!("[MOUSE] dx={dx:+}, dy={dy:+}");
        }
        thread::sleep(Duration::from_millis(16));
    }

    println!("[GamepadV Monitor] Exited.");
    0
}

fn poll_monitor_slice(devices: &mut [Device], dx: &mut i32, dy: &mut i32) -> io::Result<()> {
    for dev in devices.iter_mut() {
        loop {
            match dev.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        match ev.destructure() {
                            EventSummary::Key(_, code, value) if value == 0 || value == 1 => {
                                println!(
                                    "[KEY] {:<14} {}",
                                    key_display_name(code),
                                    if value == 1 { "DOWN" } else { "UP" }
                                );
                            }
                            EventSummary::RelativeAxis(_, axis, value) => match axis {
                                RelativeAxisCode::REL_X => *dx += value,
                                RelativeAxisCode::REL_Y => *dy += value,
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
