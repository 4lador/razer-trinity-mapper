#!/usr/bin/env bash
# razer-trinity-mapper installer — downloads the latest GitHub release.
#
# Usage:
#   ./install.sh                     # user install (~/.local/bin)
#   ./install.sh --system            # system install (/usr/local/bin, sudo)
#   ./install.sh --with-udev         # also install the udev rule
#   ./install.sh --with-service      # also install the systemd user service
set -euo pipefail

REPO="4lador/razer-trinity-mapper"
PREFIX="$HOME/.local/bin"
WITH_UDEV=0
WITH_SERVICE=0

for arg in "$@"; do
    case "$arg" in
        --system)      PREFIX="/usr/local/bin" ;;
        --with-udev)   WITH_UDEV=1 ;;
        --with-service) WITH_SERVICE=1 ;;
        *)             echo "unknown option: $arg"; exit 1 ;;
    esac
done

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
    # Fix ExecStart path for user install
    sed "s|%h/.local/bin|$PREFIX|" "$TMPDIR/trinity-mapper.service" \
        > "$HOME/.config/systemd/user/trinity-mapper.service"
    systemctl --user daemon-reload
    echo "Enable with: systemctl --user enable --now trinity-mapper.service"
fi

echo ""
echo "Done. Next steps:"
echo "  1. sudo modprobe uinput"
echo "  echo uinput | sudo tee /etc/modules-load.d/uinput.conf  # at boot"
echo "  2. sudo groupadd -r uinput 2>/dev/null || true"
echo "     sudo usermod -aG input,uinput \$USER"
echo "  3. Log out and back in"
echo "  4. Start the daemon: trinity-daemon"
echo "  5. Start the GUI: trinity-gui"
echo ""
echo "See README for full instructions."
