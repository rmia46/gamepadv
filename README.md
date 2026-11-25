# GamepadV

Zero-config Linux virtual Xbox 360 gamepad (`uinput`) driven by your keyboard and mouse, engineered for low-latency, jitter-free mouse-to-right-stick lookaround in emulators (Cemu, Dolphin, Yuzu/Ryujinx, RPCS3) and PC games. Works on both **X11 and Wayland**.

## Features
- **Interactive Visual Xbox 360 GUI (`eframe` / `egui`)**:
  - Click any button, trigger, D-pad, or stick direction to bind a keyboard key or mouse button.
  - One-click **`Mouse` | `Keys` | `Off`** mode selector on both **Left Stick** and **Right Stick** so you can route raw mouse movement to either stick instantly.
  - Live stick deflection & button highlight preview, plus real-time sliders for sensitivity, anti-deadzone, smoothing, and decay.
  - Saves/loads custom profiles to `~/.config/gamepadv/profile.json`.
- **Virtual Xbox 360 Controller (`uinput`)**: Emulates `Microsoft X-Box 360 pad` (`0x045e:0x028e`) on a dedicated 250Hz+ background thread decoupled from UI rendering.
- **Cursor Lock + Unmapped Key Passthrough**: Uses `EVIOCGRAB` when active so the cursor stays locked in-game while forwarding unmapped keys (e.g., `Alt+Tab`).
- **Instant Toggle (`F8`)**: Pause/resume capture at any time; closing the window or pressing `Ctrl+C` always ungrabs cleanly.
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
3. Launch the GUI:
   ```bash
   cargo run --release
   ```
   *(Or run terminal-only with `cargo run --release -- --headless`.)*
   Press `F8` (or click **ACTIVE / PAUSED**) to toggle capture.
