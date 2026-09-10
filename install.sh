#!/usr/bin/env bash
# razer-trinity-mapper installer — downloads the latest GitHub release.
#
# Usage:
#   ./install.sh                          # user install (~/.local/bin)
#   ./install.sh --system                 # system install (/usr/local/bin, sudo)
#   ./install.sh --with-udev              # also install the udev rule
#   ./install.sh --with-service           # also install the systemd user service
#   ./install.sh --uninstall              # remove binaries
#   ./install.sh --uninstall --all        # remove binaries + udev + systemd + config
set -euo pipefail

REPO="4lador/razer-trinity-mapper"
PREFIX="$HOME/.local/bin"
WITH_UDEV=0
WITH_SERVICE=0
MODE="install"
REMOVE_ALL=0
BINARIES=(trinity-daemon trinity-gui trinity-ctl)

UDEV_RULE="/usr/lib/udev/rules.d/60-trinity-mapper.rules"
SERVICE_FILE="$HOME/.config/systemd/user/trinity-mapper.service"
CONFIG_DIR="$HOME/.config/razer-trinity-mapper"

for arg in "$@"; do
    case "$arg" in
        --system)       PREFIX="/usr/local/bin" ;;
        --with-udev)    WITH_UDEV=1 ;;
        --with-service) WITH_SERVICE=1 ;;
        --uninstall)    MODE="uninstall" ;;
        --all)          REMOVE_ALL=1 ;;
        *)              echo "unknown option: $arg"; exit 1 ;;
    esac
done

# --- Uninstall ---

uninstall() {
    echo "Uninstalling razer-trinity-mapper..."

    # Stop daemon if running
    if pgrep -x trinity-daemon >/dev/null 2>&1; then
        echo "  stopping daemon..."
        pkill -x trinity-daemon || true
        sleep 1
    fi

    # Remove binaries
    for bin in "${BINARIES[@]}"; do
        if [ -f "$PREFIX/$bin" ]; then
            if [ "$PREFIX" = "/usr/local/bin" ]; then
                sudo rm -f "$PREFIX/$bin"
            else
                rm -f "$PREFIX/$bin"
            fi
            echo "  removed: $PREFIX/$bin"
        fi
    done

    if [ "$REMOVE_ALL" = 1 ]; then
        # Remove systemd service
        if [ -f "$SERVICE_FILE" ]; then
            systemctl --user disable --now trinity-mapper.service 2>/dev/null || true
            rm -f "$SERVICE_FILE"
            systemctl --user daemon-reload
            echo "  removed: $SERVICE_FILE"
        fi

        # Remove udev rule
        if [ -f "$UDEV_RULE" ]; then
            sudo rm -f "$UDEV_RULE"
            sudo udevadm control --reload 2>/dev/null || true
            echo "  removed: $UDEV_RULE"
        fi

        # Remove config (profiles + calibration)
        if [ -d "$CONFIG_DIR" ]; then
            echo ""
            echo "  WARNING: this will delete your profiles and calibration:"
            echo "    $CONFIG_DIR"
            echo ""
            read -r -p "  Delete config? [y/N] " answer
            if [[ "$answer" =~ ^[Yy]$ ]]; then
                rm -rf "$CONFIG_DIR"
                echo "  removed: $CONFIG_DIR"
            else
                echo "  kept: $CONFIG_DIR"
            fi
        fi
    fi

    # Remove stale socket if present
    SOCK="${XDG_RUNTIME_DIR:-/tmp}/trinity-mapper.sock"
    if [ -S "$SOCK" ]; then
        rm -f "$SOCK"
        echo "  removed stale socket: $SOCK"
    fi

    echo ""
    if [ "$REMOVE_ALL" = 1 ]; then
        echo "Fully uninstalled."
    else
        echo "Binaries removed."
        if [ -d "$CONFIG_DIR" ]; then
            echo "Config preserved at $CONFIG_DIR (use --uninstall --all to remove)."
        fi
        if [ -f "$UDEV_RULE" ]; then
            echo "Udev rule preserved (use --uninstall --all to remove)."
        fi
        if [ -f "$SERVICE_FILE" ]; then
            echo "Systemd service preserved (use --uninstall --all to remove)."
        fi
    fi
    exit 0
}

# Route to uninstall mode before any download/install logic.
if [ "$MODE" = "uninstall" ]; then
    uninstall
fi

# --- Install ---

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)  ARCH="x86_64" ;;
    aarch64) ARCH="aarch64" ;;
    *) echo "unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "Fetching latest release..."
TAG="$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/.*"v\([^"]*\)"/\1/')"
if [ -z "$TAG" ]; then
    echo "error: could not determine latest release"
    echo "hint: check https://github.com/$REPO/releases"
    exit 1
fi
echo "Latest release: v$TAG"

URL="https://github.com/$REPO/releases/download/v$TAG/razer-trinity-mapper-v$TAG-$ARCH.tar.gz"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading $URL..."
curl -sL -o "$TMPDIR/release.tar.gz" "$URL"
if [ ! -s "$TMPDIR/release.tar.gz" ]; then
    echo "error: download failed or file is empty"
    echo "hint: prebuilt binaries may not be available yet for $ARCH"
    exit 1
fi

tar -xzf "$TMPDIR/release.tar.gz" -C "$TMPDIR"

# Install binaries
mkdir -p "$PREFIX"
if [ "$PREFIX" = "/usr/local/bin" ]; then
    sudo install -Dm755 "$TMPDIR/trinity-daemon" "$PREFIX/trinity-daemon"
    sudo install -Dm755 "$TMPDIR/trinity-gui" "$PREFIX/trinity-gui"
    sudo install -Dm755 "$TMPDIR/trinity-ctl" "$PREFIX/trinity-ctl"
else
    install -Dm755 "$TMPDIR/trinity-daemon" "$PREFIX/trinity-daemon"
    install -Dm755 "$TMPDIR/trinity-gui" "$PREFIX/trinity-gui"
    install -Dm755 "$TMPDIR/trinity-ctl" "$PREFIX/trinity-ctl"
fi
echo "Binaries installed to $PREFIX"

# Optional: udev rule
if [ "$WITH_UDEV" = 1 ]; then
    echo "Installing udev rule..."
    sudo install -Dm644 "$TMPDIR/60-trinity-mapper.rules" /usr/lib/udev/rules.d/60-trinity-mapper.rules
    sudo udevadm control --reload
    sudo udevadm trigger
fi

# Optional: systemd user service
if [ "$WITH_SERVICE" = 1 ]; then
    echo "Installing systemd user service..."
    mkdir -p "$HOME/.config/systemd/user"
    sed "s|%h/.local/bin|$PREFIX|" "$TMPDIR/trinity-mapper.service" \
        > "$HOME/.config/systemd/user/trinity-mapper.service"
    systemctl --user daemon-reload
    echo "Enable with: systemctl --user enable --now trinity-mapper.service"
fi

echo ""
echo "Done. Next steps:"
echo "  1. sudo modprobe uinput"
echo "     echo uinput | sudo tee /etc/modules-load.d/uinput.conf  # at boot"
echo "  2. sudo groupadd -r uinput 2>/dev/null || true"
echo "     sudo usermod -aG input,uinput \$USER"
echo "  3. Log out and back in"
echo "  4. Start the daemon: trinity-daemon"
echo "  5. Start the GUI: trinity-gui"
echo ""
echo "To uninstall: ./install.sh --uninstall"
echo "See README for full instructions."
