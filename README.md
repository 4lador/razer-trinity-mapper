# razer-trinity-mapper

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

> **Scope**: this tool is built specifically for the Razer Naga Trinity
> and supports that mouse only. For other devices, consider
> [input-remapper](https://github.com/sezanzeb/input-remapper).

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
- Clean dark theme with client-side decorations under Wayland (system
  decorations on X11).

## Installation

### Universal installer (any distro)

```bash
curl -fsSL https://raw.githubusercontent.com/4lador/razer-trinity-mapper/main/install.sh | bash
```

Downloads prebuilt binaries from GitHub Releases over HTTPS and installs
to `~/.local/bin` (add `--system` to install to `/usr/local/bin` instead).
The archive checksum is verified when the release publishes one; releases
are not GPG-signed. Prefer a package manager install below for tracked
upgrades and clean removal. Then follow the [requirements](#requirements)
steps.

### With udev rule + systemd service

```bash
curl -fsSL https://raw.githubusercontent.com/4lador/razer-trinity-mapper/main/install.sh \
  | bash -s -- --with-udev --with-service
```

### Other install methods

<details>
<summary>Fedora / openSUSE (OBS repository — from v0.3.0)</summary>

Fedora (adjust the version in the URL):

```bash
sudo dnf config-manager addrepo --from-repofile=\
https://download.opensuse.org/repositories/home:/4lador:/razer-trinity-mapper/Fedora_44/home:4lador:razer-trinity-mapper.repo
sudo dnf install razer-trinity-mapper
```

openSUSE Tumbleweed:

```bash
sudo zypper ar -f https://download.opensuse.org/repositories/home:/4lador:/razer-trinity-mapper/openSUSE_Tumbleweed/home:4lador:razer-trinity-mapper.repo
sudo zypper refresh && sudo zypper install razer-trinity-mapper
```

The udev rule and systemd user service are installed by the package.

</details>

<details>
<summary>Debian / Ubuntu (.deb)</summary>

Grab the `.deb` from the [releases page](https://github.com/4lador/razer-trinity-mapper/releases)
(built on Ubuntu 22.04, works on Debian 12+ and Ubuntu 22.04+), then:

```bash
sudo apt install ./razer-trinity-mapper_*_amd64.deb
```

</details>

<details>
<summary>cargo install (Rust users)</summary>

```bash
cargo install --git https://github.com/4lador/razer-trinity-mapper trinity-daemon trinity-gui
```

Installs all three binaries (`trinity-daemon` and `trinity-ctl` come from
the same crate). Requires `libxkbcommon` development headers and
`pkg-config` to build.

</details>

<details>
<summary>From source</summary>

```bash
git clone https://github.com/4lador/razer-trinity-mapper
cd razer-trinity-mapper
cargo build --release
install -Dm755 target/release/trinity-daemon ~/.local/bin/trinity-daemon
install -Dm755 target/release/trinity-gui ~/.local/bin/trinity-gui
install -Dm755 target/release/trinity-ctl ~/.local/bin/trinity-ctl
```

</details>

<details>
<summary>Arch Linux (AUR — when available)</summary>

```bash
yay -S razer-trinity-mapper
```

Arch users should prefer this package once available (tracked by pacman,
clean upgrades and removal).

</details>

### Requirements

- Linux with glibc ≥ 2.34 (Ubuntu 22.04+, Debian 12+, Fedora 38+, Arch,
  openSUSE) for the prebuilt binaries.
- `bash`, `curl` and `tar` for the install script.

After installing the binaries, set up the kernel module and permissions:

1. Load the `uinput` module (not loaded by default on some distros):

```bash
sudo modprobe uinput
echo uinput | sudo tee /etc/modules-load.d/uinput.conf   # at boot
```

2. Grant access to input devices and `/dev/uinput`:

```bash
sudo groupadd -r uinput 2>/dev/null || true
sudo usermod -aG input,uinput "$USER"
```

If you used `--with-udev`, the udev rule is already installed.
Otherwise:

```bash
sudo cp packaging/60-trinity-mapper.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

3. Log out and back in (group changes require a new session).

4. Start the engine and GUI:

```bash
trinity-daemon &
trinity-gui &
```

Or enable the systemd user service (if installed with `--with-service`):

```bash
systemctl --user enable --now trinity-mapper.service
```

### Troubleshooting

<details>
<summary>Mouse completely stops responding after enabling remapping</summary>

The daemon holds an exclusive grab. Kill it and the mouse returns to native mode:

```bash
pkill trinity-daemon
```

</details>

<details>
<summary>"No such device" or "Permission denied" when starting the daemon</summary>

The `uinput` module is not loaded or you lack permissions:

```bash
lsmod | grep uinput           # check module
sudo modprobe uinput           # load it
groups | grep -w input        # check group
groups | grep -w uinput       # check group
```

</details>

<details>
<summary>Keys are remapped to the wrong characters (AZERTY, QWERTZ…)</summary>

The GUI captures keys using your active keyboard layout (via xkbcommon).
Make sure your session exports the correct `XKB_DEFAULT_LAYOUT` or run the
recalibration from Settings → Diagnostics.

</details>

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

## Uninstall

### Via install.sh

Installed with the one-liner (no cloned repo)?

```bash
curl -fsSL https://raw.githubusercontent.com/4lador/razer-trinity-mapper/main/install.sh \
  | bash -s -- --uninstall          # remove binaries only
```

From a clone:

```bash
./install.sh --uninstall          # remove binaries only
./install.sh --uninstall --all    # remove everything (udev, systemd, config)
```

### Via pacman (AUR, when available)
```bash
sudo pacman -R razer-trinity-mapper
```

### Via dnf / zypper (OBS package)
```bash
sudo dnf remove razer-trinity-mapper      # Fedora
sudo zypper remove razer-trinity-mapper   # openSUSE
```

### Via apt (.deb)
```bash
sudo apt remove razer-trinity-mapper
```

User config (`~/.config/razer-trinity-mapper/`) is never removed by
pacman — delete it manually if you want a full cleanup.

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
