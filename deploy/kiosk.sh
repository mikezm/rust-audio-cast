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

# Idle blackout: after this many seconds with no touch, the UI paints a black
# overlay over itself; the next touch clears it. Default 1800s (30 min); set
# AUDIOCAST_SLEEP_SECS=0 to keep the screen on permanently.
#
# This is done IN THE PAGE rather than via `xset`, because the X screensaver
# can't draw over a fullscreen Chromium kiosk window. On panels whose backlight
# is hardwired on (many small SPI displays) this only blanks to black — no power
# saved — but it's the behavior available without backlight control.
SLEEP_SECS="${AUDIOCAST_SLEEP_SECS:-1800}"

# Keep the X screensaver/DPMS out of the way; the page handles blanking.
xset s off || true
xset -dpms || true
xset s noblank || true

# Tell the page it's the kiosk and how long to wait before blanking.
SEP="?"
case "$URL" in *\?*) SEP="&";; esac
URL="${URL}${SEP}kiosk&sleep=${SLEEP_SECS}"

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
