# razer-trinity-mapper

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

Fast, native remapper for the **12 side buttons** of the Razer Naga Trinity
on Linux.

A purpose-built alternative to generic tools like input-remapper: one
window, a 3×4 grid mirroring the mouse, click a button, press a key — done.
The headless engine works at the **kernel level** (evdev grab + uinput), so
remapping is transparent to Wayland, X11 and games.

```
┌─────────────────────────────┐
│ Trinity Mapper      [active]│
│                             │
│  ┌────┐ ┌────┐ ┌────┐       │
│  │ 1  │ │ 2  │ │ 3  │      │  click a cell,
│  │ctrl│ │ a  │ │ f  │      │  press a key,
│  │ +1 │ │    │ │    │      │  it's mapped.
│  └────┘ └────┘ └────┘       │
│  … (full 3×4 grid)          │
│                             │
│  [profile: mmo ▾] [recalibrate]│
└─────────────────────────────┘
```

## Features

- **3×4 button grid** — click a cell, press a key (modifiers supported:
  `Ctrl+1`, `Shift+A`, …).
- **Kernel-level remapping** (evdev grab + uinput injection) — works on
  Wayland, X11 and in games; no proprietary driver, no reverse engineering.
- **Guided calibration** — press the 12 buttons once, the tool learns what
  the firmware actually emits (it is quirkier than you'd think).
- **Multiple profiles** (TOML, per profile), instant toggle.
- **Headless daemon + GUI** — start the engine at login, the GUI just
  pilots it over a local socket.
- **Layout-aware key capture** (via xkbcommon): AZERTY, QWERTZ, BEPO…
  your `A` is the real `A`.
- **6 languages**: English, French, German, Spanish, Italian,
  Portuguese (Brazil).
- Clean dark theme with client-side decorations under Wayland.

## Installation

### Requirements

- Rust (stable) and Linux with a recent kernel.
- The `uinput` kernel module (not loaded by default on some distros):

```bash
sudo modprobe uinput
echo uinput | sudo tee /etc/modules-load.d/uinput.conf   # at boot
```

- Read access to the input nodes (usually granted to local sessions via
  udev ACLs; otherwise add yourself to the `input` group).
- Write access to `/dev/uinput`:

```bash
sudo groupadd -r uinput 2>/dev/null || true
sudo usermod -aG input,uinput "$USER"
sudo cp packaging/60-trinity-mapper.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

Then log out and back in.

### Build

```bash
git clone https://github.com/4lador/razer-trinity-mapper
cd razer-trinity-mapper
cargo build --release
install -Dm755 target/release/trinity-daemon ~/.local/bin/trinity-daemon
install -Dm755 target/release/trinity-gui ~/.local/bin/trinity-gui
```

### Optional: start the engine at login

```bash
mkdir -p ~/.config/systemd/user
cp packaging/trinity-mapper.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now trinity-mapper.service
```

## Usage

1. Start the engine: `trinity-daemon` (or the systemd unit above).
2. Start the GUI: `trinity-gui`.
3. **First run**: launch the guided calibration and press the side buttons
   1 → 12 once.
4. Click a grid cell and press the key to assign (hold modifiers to build
   combos like `Ctrl+1`). Right-click a cell to clear it.
5. The **active** toggle enables/disables remapping on the fly.

Profiles live in `~/.config/razer-trinity-mapper/` (one TOML file per
profile + `calibration.toml`).

## Keyboard shortcuts

Switch profiles from a key combination by binding `trinity-ctl` in your
desktop's shortcut settings:

| Desktop | Where to configure |
|---|---|
| KDE Plasma | System Settings → Shortcuts → Add Command |
| [GNOME](https://help.gnome.org/users/gnome-help/stable/keyboard.html) | Settings → Keyboard → Custom Shortcuts |
| [sway](https://man.archlinux.org/man/sway.5) | `bindsym $mod+F1 exec trinity-ctl profile mmo` in config |
| [i3](https://i3wm.org/docs/userguide.html#keybindings) | Same syntax as sway |
| Hyprland | `bind = $mod, F1, exec, trinity-ctl profile mmo` in hyprland.conf |
| [Xfce](https://docs.xfce.org/xfce/xfce4-settings/keyboard) | Settings → Keyboard → Application Shortcuts |
| Any X11 WM | [xbindkeys](https://man.archlinux.org/man/xbindkeys.1) |

## CLI

The daemon can also be driven from the CLI:

```bash
trinity-ctl status              # daemon state
trinity-ctl profiles            # list profiles
trinity-ctl profile mmo         # switch to a profile
```

Or directly over the Unix socket (JSON lines):

```bash
echo '{"type":"get_status"}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/trinity-mapper.sock
echo '{"type":"set_enabled","enabled":false}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/trinity-mapper.sock
```

## Architecture

Strict hexagonal architecture — a pure domain, adapters around it:

```
trinity-gui ──► trinity-app ◄── trinity-infra
trinity-daemon ─►    │
                    ▼
              trinity-core
```

| Crate | Role |
|---|---|
| `trinity-core` | Pure domain: buttons, key combos, profiles, calibration — zero I/O |
| `trinity-app` | Ports (traits), use cases, shared IPC protocol |
| `trinity-infra` | evdev, uinput, TOML and by-id adapters |
| `trinity-gui` | Iced GUI (composition root) |
| `trinity-daemon` | Headless engine + IPC (composition root) |

## Development

```bash
cargo test                              # unit tests
just test-integration                   # uinput tests (input/uinput groups)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
scripts/check-architecture.sh           # hexagonal dependency rule
just check                              # all of the above
```

## Contributing

Bug reports, translations (one TOML file per language in
`crates/trinity-gui/locales/`) and pull requests are welcome.

## Credits

- Icons: [Material Design Icons](https://pictogrammers.com/library/mdi/)
  (Apache-2.0)
- Font: [Inter](https://rsms.me/inter/) (SIL OFL 1.1)
- GUI: [Iced](https://iced.rs) (MIT)

## License

Copyright (C) 2026 4lador

This program is free software: you can redistribute it and/or modify it
under the terms of the GNU General Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your option)
any later version. See [LICENSE](LICENSE).
