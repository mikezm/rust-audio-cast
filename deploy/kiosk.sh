#!/usr/bin/env bash
# Launch the touch UI full-screen in Chromium kiosk mode. Runs as the desktop
# login user (not the audiocast service user) and only talks HTTP to localhost,
# so it needs no audio access.
#
# Assumes an X11 session (the SPI display setup switches the Pi to X11), so the
# `xset` screen-blank commands apply. Under Wayland use the compositor's idle
# settings instead.
set -euo pipefail

URL="${AUDIOCAST_KIOSK_URL:-http://localhost:8080/}"

# Always-on display: disable screen blanking and power management.
xset s off || true
xset -dpms || true
xset s noblank || true

for BROWSER in chromium-browser chromium; do
  if command -v "$BROWSER" >/dev/null 2>&1; then
    exec "$BROWSER" \
      --kiosk \
      --incognito \
      --noerrdialogs \
      --disable-infobars \
      --disable-session-crashed-bubble \
      --no-first-run \
      --password-store=basic \
      --check-for-update-interval=31536000 \
      --app="$URL"
  fi
done

echo "kiosk: no chromium browser found (tried chromium-browser, chromium)" >&2
exit 1
