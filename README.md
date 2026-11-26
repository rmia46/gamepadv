# GamepadV

Zero-config Linux virtual Xbox 360 gamepad (`uinput`) driven by your keyboard and mouse, engineered for low-latency, jitter-free mouse-to-right-stick lookaround in emulators (Cemu, Dolphin, Yuzu/Ryujinx, RPCS3) and PC/Proton games. Works on both **X11 and Wayland**.

---

## Quick Install (Linux x86_64)

### One-Line Install (Any Distro — Arch, SteamOS, Fedora, Ubuntu)
```bash
curl -fsSL https://raw.githubusercontent.com/rmia46/gamepadv/feature/gamepadv-core/install.sh | bash
```

### Arch Linux (`.pkg.tar.zst`)
```bash
curl -fsSLO https://github.com/rmia46/gamepadv/releases/latest/download/gamepadv-0.1.0-1-x86_64.pkg.tar.zst && sudo pacman -U --noconfirm ./gamepadv-0.1.0-1-x86_64.pkg.tar.zst
```

### Debian / Ubuntu / Linux Mint (`.deb`)
```bash
curl -fsSLO https://github.com/rmia46/gamepadv/releases/latest/download/gamepadv_0.1.0_amd64.deb && sudo apt install -y ./gamepadv_0.1.0_amd64.deb
```

### Fedora / RHEL / openSUSE (`.rpm`)
```bash
sudo dnf install -y https://github.com/rmia46/gamepadv/releases/latest/download/gamepadv-0.1.0-1.x86_64.rpm
```

### Build from Source
```bash
git clone https://github.com/rmia46/gamepadv.git && cd gamepadv && cargo build --release
```

---

## Usage
1. Launch **`gamepadv`** (from your app menu or terminal).
2. Click **`Unlock Input Engine (pkexec)`** and enter your password *(keeps `/dev/input/event*` isolated from your normal user session; see [docs/udev-setup.md](docs/udev-setup.md) for permanent `udev` setup)*.
3. **Start your game/emulator after unlocking** so it detects the virtual Xbox 360 controller (`Microsoft X-Box 360 pad`).
4. Press **`F8`** (or click **`|| PAUSED (F8)`**) to toggle capture:
   - **`|| PAUSED (F8)`** — Cursor is free for menus/desktop; live GUI preview still works.
   - **`> ACTIVE (F8)`** — Cursor is locked (`EVIOCGRAB`) and inputs drive the virtual Xbox 360 pad.

---

## Features
- **Interactive Visual Xbox 360 GUI (`eframe` / `egui`)**:
  - Click any button, trigger, D-pad, or stick direction to bind a keyboard key or mouse button.
  - One-click **`Mouse` | `Keys` | `Off`** mode selector on both **Left Stick** and **Right Stick**.
  - Live stick deflection, button highlights, and **Input Check** monitor.
  - Real-time sliders for **Mouse Stick Tuning** (Sensitivity, Anti-Deadzone, Smoothing EMA, Decay Half-Life, Invert Y) and **Key Stick Sweep** (anti-deadzone-aware ease-out + arc-blended turns).
  - Built-in presets (**3D Action**, **FPS / Shooter**, **Racing / Driving**, **2D Platformer**) and custom profile save/load (`~/.config/gamepadv/profiles/`).
- **Virtual Xbox 360 Controller (`uinput`)**: Emulates `Microsoft X-Box 360 pad` (`0x045e:0x028e`) on a dedicated 250Hz+ thread decoupled from UI rendering.
- **Cursor Lock + Unmapped Key Passthrough**: Uses `EVIOCGRAB` when active while forwarding unmapped keys (e.g., `Alt+Tab`).
- **Built-in "3D Action (Default)" Profile**:
  - `WASD` → Left Stick (`Left Alt` / `Left Ctrl` = Walk modifier)
  - `Mouse` → Right Stick
  - `Left Click` / `Right Click` → `X` (Attack) / `LT` (Aim / Shield)
  - `Space` / `Left Shift` / `E` / `Q` → `Y` (Jump) / `B` (Back / Sprint) / `A` (Confirm / Interact) / `RT`
  - `F` / `R` → `LB` / `RB`
  - `1`–`4` → D-Pad
  - `Tab` / `Enter` / `C` / `V` → `Select` / `Start` / `L3` / `R3`
