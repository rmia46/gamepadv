mod discovery;
mod gamepad;

use std::env;
use std::fs::OpenOptions;
use std::process::ExitCode;

fn run_check() -> ExitCode {
    println!("=== GamepadV Phase 0 Kernel Spike (--check) ===");

    // 1. Direct /dev/uinput writeability check
    match OpenOptions::new().read(true).write(true).open("/dev/uinput") {
        Ok(_) => println!("[OK] /dev/uinput: open(O_RDWR) succeeded"),
        Err(err) => {
            eprintln!("[ERR] /dev/uinput: open(O_RDWR) failed: {err} ({:?})", err.kind());
            return ExitCode::FAILURE;
        }
    }

    // 2. Spike 0.1: Virtual Xbox 360 controller + neutral SYN_REPORT
    match gamepad::build_xbox360_controller() {
        Ok(mut dev) => {
            let syspath = dev
                .get_syspath()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("<err: {e}>"));
            match gamepad::emit_neutral_sync(&mut dev) {
                Ok(()) => println!(
                    "[OK] Spike 0.1: created '{}' at {} and emitted neutral SYN_REPORT",
                    gamepad::XBOX360_NAME,
                    syspath
                ),
                Err(err) => {
                    eprintln!("[ERR] Spike 0.1: emit_neutral_sync failed: {err}");
                    return ExitCode::FAILURE;
                }
            }
        }
        Err(err) => {
            eprintln!("[ERR] Spike 0.1: build_xbox360_controller failed: {err}");
            return ExitCode::FAILURE;
        }
    }

    // 3. Spike 0.2: Passthrough keyboard uinput creation
    let kbd_keys = discovery::standard_keyboard_keys();
    match discovery::build_passthrough_keyboard("GamepadV Passthrough Keyboard", &kbd_keys) {
        Ok(mut dev) => {
            let syspath = dev
                .get_syspath()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("<err: {e}>"));
            println!("[OK] Spike 0.2: passthrough uinput keyboard created at {syspath}");
        }
        Err(err) => {
            eprintln!("[ERR] Spike 0.2: passthrough keyboard creation failed: {err}");
            return ExitCode::FAILURE;
        }
    }

    // 4. Spike 0.2: Enumerate /dev/input/event* and report exact permission status
    match discovery::probe_input_devices() {
        Ok(report) => {
            println!(
                "[INFO] /dev/input/event*: total={}, readable={}, permission_denied={}, evdev::enumerate()={}",
                report.event_nodes_total,
                report.event_nodes_readable,
                report.event_nodes_permission_denied,
                report.enumerated_devices.len()
            );
            if let Some((path, kind, msg)) = report.sample_error {
                println!(
                    "[WARN] Sample /dev/input/event* open error: {} -> {:?} ({})",
                    path.display(),
                    kind,
                    msg
                );
            }
            for (path, name, class) in report.enumerated_devices {
                println!("  - {} | {:?} | {}", path.display(), class, name);
            }
        }
        Err(err) => {
            eprintln!("[ERR] probe_input_devices failed: {err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--check") || args.len() == 1 {
        return run_check();
    }

    eprintln!("Usage: {} [--check]", args[0]);
    ExitCode::FAILURE
}
