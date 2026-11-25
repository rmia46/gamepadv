# Input Device Permissions Setup (`udev` / `input` group)

GamepadV needs:
1. Write access to `/dev/uinput` (to create the virtual Xbox 360 controller).
2. Read/grab access to `/dev/input/event*` (to capture keyboard and mouse events on X11 and Wayland).

On many Linux distributions, `/dev/uinput` has a `uaccess` ACL for the active desktop user, while `/dev/input/event*` is restricted to the `input` group (`crw-rw---- root:input`).

## Option A: `udev` Rule (Recommended — No Logout Required)

Create `/etc/udev/rules.d/99-gamepadv-input.rules`:

```udev
KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess"
SUBSYSTEM=="input", KERNEL=="event*", TAG+="uaccess"
```

Then reload and trigger `udev`:

```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
```

## Option B: Add Your User to the `input` Group

```bash
sudo usermod -aG input "$USER"
```

Log out and log back in (or reboot) for the new group membership to apply.
