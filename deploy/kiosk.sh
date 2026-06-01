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

# Screen sleep: blank / power down the display after this many seconds of
# inactivity; any touch (or key) wakes it. Default 1800s (30 min). Set
# AUDIOCAST_SLEEP_SECS=0 to keep the screen on permanently.
#
# NOTE: whether this actually saves power depends on the panel. DPMS-capable
# displays (most HDMI/DSI) truly power down. Panels whose backlight is hardwired
# on (many small SPI displays) only blank to BLACK — cosmetic, no power saved —
# since they expose no backlight control. Either way a touch wakes it.
# (X11 only — under Wayland use the compositor's idle settings.)
SLEEP_SECS="${AUDIOCAST_SLEEP_SECS:-1800}"
if [ "${SLEEP_SECS}" -gt 0 ] 2>/dev/null; then
  xset s "${SLEEP_SECS}" "${SLEEP_SECS}" || true
  xset +dpms || true
  xset dpms "${SLEEP_SECS}" "${SLEEP_SECS}" "${SLEEP_SECS}" || true
else
  xset s off || true
  xset -dpms || true
  xset s noblank || true
fi

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
