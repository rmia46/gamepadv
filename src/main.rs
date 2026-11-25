use std::collections::HashSet;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use evdev::{Device, EventSummary, KeyCode, RelativeAxisCode};

use gamepadv::engine::{InputMapper, MouseStickConfig, MouseStickEngine};
use gamepadv::input::{probe_input_devices, DeviceClass, InputRig};
use gamepadv::profile::zelda_default;
use gamepadv::virtual_pad::VirtualXbox360;

const SPIN_MARGIN: Duration = Duration::from_micros(750);

struct CliOptions {
    check_only: bool,
    start_paused: bool,
    config: MouseStickConfig,
}

fn parse_finite_f64(flag: &str, raw: Option<String>) -> Result<f64, String> {
    let val = raw.ok_or_else(|| format!("{flag} requires a numeric value"))?;
    let parsed = val
        .parse::<f64>()
        .map_err(|_| format!("invalid {flag} value: {val}"))?;
    if !parsed.is_finite() {
        return Err(format!("{flag} must be a finite number"));
    }
    Ok(parsed)
}

fn parse_args() -> Result<CliOptions, String> {
    let mut opts = CliOptions {
        check_only: false,
        start_paused: false,
        config: MouseStickConfig::default(),
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--check" => opts.check_only = true,
            "--start-paused" => opts.start_paused = true,
            "--invert-y" => opts.config.invert_y = true,
            "--sensitivity" => {
                let val = parse_finite_f64("--sensitivity", args.next())?;
                if val <= 0.0 {
                    return Err("--sensitivity must be greater than 0.0".to_string());
                }
                opts.config.sensitivity = val;
            }
            "--deadzone" => {
                let val = parse_finite_f64("--deadzone", args.next())?;
                if !(0.0..1.0).contains(&val) {
                    return Err("--deadzone must be in [0.0, 1.0)".to_string());
                }
                opts.config.anti_deadzone = val;
            }
            "--hz" => {
                let hz = parse_finite_f64("--hz", args.next())?;
                if hz <= 0.0 {
                    return Err("--hz must be greater than 0.0".to_string());
                }
                opts.config.hz = hz;
            }
            other => return Err(format!("unknown argument: {other} (try --help)")),
        }
    }
    Ok(opts)
}

fn print_help() {
    println!(
        "GamepadV v{} — Zero-config Linux virtual Xbox 360 gamepad (KBM -> uinput)

USAGE:
    gamepadv [OPTIONS]

OPTIONS:
    --check               Probe /dev/uinput and /dev/input/event* permissions and exit
    --start-paused        Start paused (press F8 to enable capture)
    --sensitivity <FLOAT> Mouse-to-stick sensitivity (default: {})
    --deadzone <FLOAT>    Anti-deadzone floor [0.0, 1.0) (default: {})
    --hz <FLOAT>          Tick rate in Hz (default: {})
    --invert-y            Invert right-stick Y axis
    -h, --help            Print this help message",
        env!("CARGO_PKG_VERSION"),
        MouseStickConfig::default().sensitivity,
        MouseStickConfig::default().anti_deadzone,
        MouseStickConfig::default().hz,
    );
}

fn run_check() -> i32 {
    println!("=== GamepadV Environment Check ===");
    match VirtualXbox360::new() {
        Ok(mut pad) => {
            let _ = pad.reset_neutral();
            println!("[OK] /dev/uinput: virtual Xbox 360 pad created successfully");
        }
        Err(err) => {
            eprintln!("[ERR] /dev/uinput: failed to create virtual pad: {err}");
            return 1;
        }
    }

    let report = match probe_input_devices() {
        Ok(r) => r,
        Err(err) => {
            eprintln!("[ERR] Failed to probe /dev/input/event*: {err}");
            return 1;
        }
    };

    let kbd_count = report
        .enumerated_devices
        .iter()
        .filter(|(_, _, class)| *class == DeviceClass::Keyboard)
        .count();
    let mouse_count = report
        .enumerated_devices
        .iter()
        .filter(|(_, _, class)| *class == DeviceClass::Mouse)
        .count();

    println!(
        "[INFO] /dev/input/event*: total={}, readable={}, permission_denied={}, keyboards={}, mice={}",
        report.event_nodes_total,
        report.event_nodes_readable,
        report.event_nodes_permission_denied,
        kbd_count,
        mouse_count
    );

    if report.event_nodes_readable == 0 && report.event_nodes_permission_denied > 0 {
        eprintln!(
            "[WARN] Cannot read /dev/input/event* (permission denied). See docs/udev-setup.md to grant input access."
        );
        return 2;
    }

    0
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
    active: &mut bool,
    virtual_pad: &mut VirtualXbox360,
    passthrough_pressed: &mut HashSet<KeyCode>,
) -> std::io::Result<()> {
    let mut toggle_pressed = false;
    let mut passthrough_queue: Vec<(KeyCode, i32)> = Vec::new();
    let is_active = *active;

    drain_device_slice(
        rig.keyboards_mut(),
        mapper,
        toggle_key,
        is_active,
        &mut toggle_pressed,
        &mut passthrough_queue,
    )?;
    drain_device_slice(
        rig.mice_mut(),
        mapper,
        toggle_key,
        is_active,
        &mut toggle_pressed,
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

    if toggle_pressed {
        let next_active = !*active;
        if !next_active {
            release_passthrough_keys(rig, passthrough_pressed)?;
        }
        rig.set_grab(next_active)?;
        *active = next_active;
        mapper.reset();
        virtual_pad.reset_neutral()?;
        println!(
            "[GamepadV] {}",
            if *active {
                "ACTIVE (cursor grabbed, F8 to pause)"
            } else {
                "PAUSED (cursor released, F8 to resume)"
            }
        );
    }

    Ok(())
}

fn main() {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("Error: {msg}");
            std::process::exit(2);
        }
    };

    if opts.check_only {
        std::process::exit(run_check());
    }

    let report = match probe_input_devices() {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Error: failed to probe /dev/input/event*: {err}");
            std::process::exit(1);
        }
    };

    if report.event_nodes_readable == 0 && report.event_nodes_permission_denied > 0 {
        eprintln!(
            "Error: Permission denied reading /dev/input/event* ({} nodes denied).\n\
             See docs/udev-setup.md to grant uaccess or input group permissions.",
            report.event_nodes_permission_denied
        );
        std::process::exit(1);
    }

    let mut virtual_pad = match VirtualXbox360::new() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Error: failed to create virtual Xbox 360 pad on /dev/uinput: {err}");
            std::process::exit(1);
        }
    };

    let mut rig = match InputRig::discover() {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Error: failed to initialize input rig: {err}");
            std::process::exit(1);
        }
    };

    if rig.keyboard_count() == 0 && rig.mouse_count() == 0 {
        eprintln!("Error: no readable keyboards or mice found under /dev/input/event*");
        std::process::exit(1);
    }

    let running = Arc::new(AtomicBool::new(true));
    let r_flag = Arc::clone(&running);
    if let Err(err) = ctrlc::set_handler(move || {
        r_flag.store(false, Ordering::SeqCst);
    }) {
        eprintln!("Warning: failed to install Ctrl+C handler: {err}");
    }

    let profile = zelda_default();
    let toggle_key = profile.toggle_key;
    let mut mapper = InputMapper::new(profile, MouseStickEngine::new(opts.config));
    let mut passthrough_pressed: HashSet<KeyCode> = HashSet::new();

    let mut active = !opts.start_paused;
    if active {
        if let Err(err) = rig.set_grab(true) {
            eprintln!("Error: failed to grab input devices: {err}");
            std::process::exit(1);
        }
    }

    println!(
        "[GamepadV] Ready — {} keyboard(s), {} mouse/mice | {} Hz | {} (F8 to toggle, Ctrl+C to quit)",
        rig.keyboard_count(),
        rig.mouse_count(),
        opts.config.hz,
        if active { "ACTIVE" } else { "PAUSED" }
    );

    let tick_period = Duration::from_secs_f64(1.0 / opts.config.hz);
    let mut last_tick = Instant::now();
    let mut next_tick = last_tick + tick_period;

    while running.load(Ordering::Relaxed) {
        let now = Instant::now();
        let dt_secs = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;

        if let Err(err) = drain_events(
            &mut rig,
            &mut mapper,
            toggle_key,
            &mut active,
            &mut virtual_pad,
            &mut passthrough_pressed,
        ) {
            eprintln!("Error in input loop: {err}");
            break;
        }

        if active {
            let state = mapper.tick(dt_secs);
            if let Err(err) = virtual_pad.sync_state(&state) {
                eprintln!("Error emitting virtual pad state: {err}");
                break;
            }
        }

        let after_work = Instant::now();
        if next_tick > after_work {
            let remaining = next_tick - after_work;
            if remaining > SPIN_MARGIN {
                std::thread::sleep(remaining - SPIN_MARGIN);
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
    println!("[GamepadV] Clean exit (devices ungrabbed).");
}
