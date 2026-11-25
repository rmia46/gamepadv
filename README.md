# GamepadV

Zero-config Linux virtual Xbox 360 gamepad (`uinput`) driven by your keyboard and mouse, engineered for low-latency, jitter-free mouse-to-right-stick lookaround in emulators (Cemu, Dolphin, Yuzu/Ryujinx, RPCS3) and PC games. Works on both **X11 and Wayland**.

## Features
- **Virtual Xbox 360 Controller (`uinput`)**: Emulates `Microsoft X-Box 360 pad` (`0x045e:0x028e`) with full analog sticks, triggers, D-pad, and buttons.
- **Smooth Mouse → Right Stick Engine**: 250Hz+ hybrid sleep/spin loop with velocity EMA smoothing, exponential decay, radial anti-deadzone, power-curve response, and circular clamping.
- **Cursor Lock + Unmapped Key Passthrough**: Uses `EVIOCGRAB` when active so the cursor stays locked in-game while forwarding unmapped keys (e.g., `Alt+Tab`).
- **Instant Toggle (`F8`)**: Pause/resume capture at any time; `Ctrl+C` (`SIGINT`/`SIGTERM`) always ungrabs cleanly.
- **Built-in "3D Action / Zelda" Profile**:
  - `WASD` → Left Stick (`Left Alt` / `Left Ctrl` = Walk modifier)
  - `Mouse` → Right Stick
  - `Left Click` / `Right Click` → `X` (West / Attack) / `LT` (Shield / Target)
  - `Space` / `Left Shift` / `E` / `Q` → `Y` (Jump) / `B` (Sprint) / `A` (Interact) / `RT` (Bow)
  - `F` / `R` → `LB` / `RB`
  - `1`–`4` → D-Pad
  - `Tab` / `Enter` / `C` / `V` → `Select` / `Start` / `L3` / `R3`

## Quick Start

1. Ensure `/dev/uinput` and `/dev/input/event*` permissions are configured (see [docs/udev-setup.md](docs/udev-setup.md)).
2. Check environment:
   ```bash
   cargo run --release -- --check
   ```
3. Run GamepadV:
   ```bash
   cargo run --release
   ```
   Press `F8` to pause/resume capture, or `Ctrl+C` to exit cleanly.
