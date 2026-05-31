#!/usr/bin/env bash
#
# Install (or update) Rust Audio Cast on a Raspberry Pi: build the binary, create
# the dedicated service user, install the binary + UI + config, enable the
# systemd service, and set up the Chromium kiosk for the desktop user.
#
# Idempotent — safe to re-run to redeploy after a `git pull`. Run with sudo:
#     sudo deploy/install.sh
#
set -euo pipefail

# ----- resolve paths and users -------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ "${EUID}" -ne 0 ]]; then
  echo "error: run with sudo (needed to install the service and create the user)" >&2
  exit 1
fi

# The normal desktop user (who owns the build toolchain and runs the kiosk).
DESKTOP_USER="${SUDO_USER:-}"
if [[ -z "$DESKTOP_USER" || "$DESKTOP_USER" == "root" ]]; then
  echo "error: run via 'sudo deploy/install.sh' as your normal user, not as root directly" >&2
  exit 1
fi
DESKTOP_HOME="$(getent passwd "$DESKTOP_USER" | cut -d: -f6)"

SERVICE_USER="audiocast"
BIN_DST="/usr/local/bin/rust-audio-cast"
WWW_DST="/usr/share/audiocast/www"
CONFIG_DIR="/etc/audiocast"
CONFIG_DST="$CONFIG_DIR/config.toml"
KIOSK_DST="/usr/local/bin/audiocast-kiosk.sh"
UNIT_DST="/etc/systemd/system/audiocast.service"

echo "==> repo:         $REPO_DIR"
echo "==> desktop user: $DESKTOP_USER ($DESKTOP_HOME)"

# ----- 1. system build dependencies -------------------------------------------
echo "==> installing apt dependencies"
apt-get update -qq
apt-get install -y libasound2-dev pkg-config chromium-browser >/dev/null 2>&1 || \
  apt-get install -y libasound2-dev pkg-config chromium >/dev/null

# ----- 2. build the release binary as the desktop user ------------------------
echo "==> building (cargo build --release; first build is slow on a Pi)"
sudo -u "$DESKTOP_USER" -H bash -c \
  "source '$DESKTOP_HOME/.cargo/env' 2>/dev/null; cd '$REPO_DIR' && cargo build --release"

BIN_SRC="$REPO_DIR/target/release/rust-audio-cast"
[[ -x "$BIN_SRC" ]] || { echo "error: build did not produce $BIN_SRC" >&2; exit 1; }

# ----- 3. dedicated service user (in the audio group) -------------------------
if ! id "$SERVICE_USER" >/dev/null 2>&1; then
  echo "==> creating service user '$SERVICE_USER'"
  useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
fi
usermod -aG audio "$SERVICE_USER"

# ----- 4. install binary, UI, config, kiosk script ----------------------------
echo "==> installing binary -> $BIN_DST"
install -m 0755 "$BIN_SRC" "$BIN_DST"

echo "==> installing UI -> $WWW_DST"
mkdir -p "$WWW_DST"
cp -r "$REPO_DIR/www/." "$WWW_DST/"

echo "==> installing kiosk launcher -> $KIOSK_DST"
install -m 0755 "$SCRIPT_DIR/kiosk.sh" "$KIOSK_DST"

mkdir -p "$CONFIG_DIR"
if [[ -f "$CONFIG_DST" ]]; then
  echo "==> keeping existing config $CONFIG_DST"
else
  echo "==> installing config -> $CONFIG_DST"
  install -m 0644 "$SCRIPT_DIR/config.toml" "$CONFIG_DST"
fi

# ----- 5. systemd service ------------------------------------------------------
echo "==> installing + enabling systemd service"
install -m 0644 "$SCRIPT_DIR/audiocast.service" "$UNIT_DST"
systemctl daemon-reload
systemctl enable --now audiocast
systemctl restart audiocast

# ----- 6. kiosk autostart + desktop icon for the desktop user -----------------
echo "==> installing kiosk desktop entry for $DESKTOP_USER"
install_desktop_entry() {
  local dst_dir="$1"
  sudo -u "$DESKTOP_USER" mkdir -p "$dst_dir"
  sudo -u "$DESKTOP_USER" install -m 0755 "$SCRIPT_DIR/audiocast-kiosk.desktop" "$dst_dir/audiocast-kiosk.desktop"
}
# Clickable icon on the desktop, and autostart at login (appliance behavior).
install_desktop_entry "$DESKTOP_HOME/Desktop"
install_desktop_entry "$DESKTOP_HOME/.config/autostart"

# ----- done -------------------------------------------------------------------
IP="$(hostname -I | awk '{print $1}')"
echo
echo "==> done. Service status:"
systemctl --no-pager --lines=0 status audiocast || true
echo
echo "    UI:    http://${IP}:8080/   (and http://localhost:8080/ on the Pi)"
echo "    Logs:  journalctl -u audiocast -f"
echo "    Kiosk: tap the 'Audio Cast' desktop icon, or it auto-starts at login."
echo "    To disable kiosk autostart: rm $DESKTOP_HOME/.config/autostart/audiocast-kiosk.desktop"
