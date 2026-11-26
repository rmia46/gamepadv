use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gamepadv::engine::MouseStickConfig;
use gamepadv::gui::run_gui;
use gamepadv::input::{probe_input_devices, DeviceClass};
use gamepadv::profile::{default_3d_action, ProfileConfig};
use gamepadv::runtime::{run_daemon_socket, run_monitor, EngineHandle};
use gamepadv::virtual_pad::VirtualXbox360;

struct CliOptions {
    check_only: bool,
    monitor: bool,
    headless: bool,
    daemon: bool,
    socket: Option<PathBuf>,
    owner_uid: Option<u32>,
    owner_gid: Option<u32>,
    start_paused: Option<bool>,
    sensitivity: Option<f64>,
    anti_deadzone: Option<f64>,
    hz: Option<f64>,
    invert_y: bool,
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

fn parse_u32(flag: &str, raw: Option<String>) -> Result<u32, String> {
    let val = raw.ok_or_else(|| format!("{flag} requires an integer value"))?;
    val.parse::<u32>()
        .map_err(|_| format!("invalid {flag} value: {val}"))
}

fn parse_args() -> Result<CliOptions, String> {
    let mut opts = CliOptions {
        check_only: false,
        monitor: false,
        headless: false,
        daemon: false,
        socket: None,
        owner_uid: None,
        owner_gid: None,
        start_paused: None,
        sensitivity: None,
        anti_deadzone: None,
        hz: None,
        invert_y: false,
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--check" => opts.check_only = true,
            "--monitor" => opts.monitor = true,
            "--headless" => opts.headless = true,
            "--daemon" => opts.daemon = true,
            "--socket" => {
                let val = args
                    .next()
                    .ok_or_else(|| "--socket requires a path".to_string())?;
                opts.socket = Some(PathBuf::from(val));
            }
            "--owner-uid" => opts.owner_uid = Some(parse_u32("--owner-uid", args.next())?),
            "--owner-gid" => opts.owner_gid = Some(parse_u32("--owner-gid", args.next())?),
            "--start-paused" => opts.start_paused = Some(true),
            "--start-active" => opts.start_paused = Some(false),
            "--invert-y" => opts.invert_y = true,
            "--sensitivity" => {
                let val = parse_finite_f64("--sensitivity", args.next())?;
                if val <= 0.0 {
                    return Err("--sensitivity must be greater than 0.0".to_string());
                }
                opts.sensitivity = Some(val);
            }
            "--deadzone" => {
                let val = parse_finite_f64("--deadzone", args.next())?;
                if !(0.0..1.0).contains(&val) {
                    return Err("--deadzone must be in [0.0, 1.0)".to_string());
                }
                opts.anti_deadzone = Some(val);
            }
            "--hz" => {
                let hz = parse_finite_f64("--hz", args.next())?;
                if hz <= 0.0 {
                    return Err("--hz must be greater than 0.0".to_string());
                }
                opts.hz = Some(hz);
            }
            other => return Err(format!("unknown argument: {other} (try --help)")),
        }
    }
    Ok(opts)
}

fn print_help() {
    let defaults = MouseStickConfig::default();
    println!(
        "GamepadV v{} — Zero-config Linux virtual Xbox 360 gamepad (KBM -> uinput)

USAGE:
    gamepadv [OPTIONS]

OPTIONS:
    --headless            Run in terminal-only daemon mode (no GUI window)
    --monitor             Live CLI input check (prints key presses and mouse movement)
    --daemon              Run as Unix-socket IPC daemon (used with --socket / pkexec)
    --socket <PATH>       Unix socket path for --daemon mode
    --check               Probe /dev/uinput and /dev/input/event* permissions and exit
    --start-paused        Start paused (default in GUI mode; press F8 to enable capture)
    --start-active        Start active immediately (default in --headless mode)
    --sensitivity <FLOAT> Mouse-to-stick sensitivity (default: {})
    --deadzone <FLOAT>    Anti-deadzone floor [0.0, 1.0) (default: {})
    --hz <FLOAT>          Tick rate in Hz (default: {})
    --invert-y            Invert right-stick Y axis
    -h, --help            Print this help message",
        env!("CARGO_PKG_VERSION"),
        defaults.sensitivity,
        defaults.anti_deadzone,
        defaults.hz,
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
            "[WARN] Cannot read /dev/input/event* (permission denied). Use 'Unlock Input Engine (pkexec)' in the GUI or see docs/udev-setup.md."
        );
        return 2;
    }

    0
}

fn run_headless(
    profile: gamepadv::profile::Profile,
    mouse_config: MouseStickConfig,
    start_paused: bool,
) -> i32 {
    let running = Arc::new(AtomicBool::new(true));
    let r_flag = Arc::clone(&running);
    if let Err(err) = ctrlc::set_handler(move || {
        r_flag.store(false, Ordering::SeqCst);
    }) {
        eprintln!("Warning: failed to install Ctrl+C handler: {err}");
    }

    let mut engine = EngineHandle::spawn(profile, mouse_config, start_paused);
    std::thread::sleep(Duration::from_millis(120));
    let snap = engine.snapshot();
    if let Some(err) = snap.permission_error {
        eprintln!("Error: {err}");
        engine.shutdown();
        return 1;
    }

    println!(
        "[GamepadV] Headless ready — {} keyboard(s), {} mouse/mice | {} Hz | {} (F8 to toggle, Ctrl+C to quit)",
        snap.keyboard_count,
        snap.mouse_count,
        mouse_config.hz,
        if snap.active { "ACTIVE" } else { "PAUSED" }
    );

    let mut last_status = snap.status_message;
    while running.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
        let cur = engine.snapshot();
        if cur.status_message != last_status {
            println!("[GamepadV] {}", cur.status_message);
            last_status = cur.status_message;
        }
        if cur.permission_error.is_some() {
            break;
        }
    }

    engine.shutdown();
    println!("[GamepadV] Clean exit (devices ungrabbed).");
    0
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
    if opts.monitor {
        std::process::exit(run_monitor());
    }

    let (profile, mut mouse_config) = ProfileConfig::default_config_path()
        .filter(|p| p.exists())
        .and_then(|p| ProfileConfig::load_from_file(&p).ok())
        .map(ProfileConfig::into_parts)
        .unwrap_or_else(|| (default_3d_action(), MouseStickConfig::default()));

    if let Some(sens) = opts.sensitivity {
        mouse_config.sensitivity = sens;
    }
    if let Some(dz) = opts.anti_deadzone {
        mouse_config.anti_deadzone = dz;
    }
    if let Some(hz) = opts.hz {
        mouse_config.hz = hz;
    }
    if opts.invert_y {
        mouse_config.invert_y = true;
    }

    if opts.daemon {
        let Some(socket_path) = opts.socket else {
            eprintln!("Error: --daemon requires --socket <PATH>");
            std::process::exit(2);
        };
        let owner = match (opts.owner_uid, opts.owner_gid) {
            (Some(u), Some(g)) => Some((u, g)),
            _ => None,
        };
        let start_paused = opts.start_paused.unwrap_or(true);
        if let Err(err) =
            run_daemon_socket(&socket_path, owner, profile, mouse_config, start_paused)
        {
            eprintln!("Daemon error: {err}");
            std::process::exit(1);
        }
        return;
    }

    if opts.headless {
        let start_paused = opts.start_paused.unwrap_or(false);
        std::process::exit(run_headless(profile, mouse_config, start_paused));
    }

    // Default to paused in GUI mode so the user can configure bindings before locking cursor.
    let start_paused = opts.start_paused.unwrap_or(true);
    if let Err(err) = run_gui(profile, mouse_config, start_paused) {
        eprintln!("Error launching GUI: {err}");
        std::process::exit(1);
    }
}
