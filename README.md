# rust-audio-cast
## Raspberry Pi → Chromecast Audio streamer, in Rust

Reads USB audio input (e.g. a turntable) on a Raspberry Pi, hosts it as a low-latency
HTTP audio stream, discovers Google Chromecast Audio speakers on the LAN, and tells a
chosen speaker to play the stream. A touch-friendly web UI (usable from the Pi's own
touchscreen and from any phone on the LAN) controls play / stop / volume / speaker
selection.

Why buy the [Sonos Port](https://www.sonos.com/en-us/shop/port) for $449 when you can
do the same thing with a Raspberry Pi for half the price and twice the headache? This
is the Rust rewrite of [pi-stream-cast](https://github.com/mikezm/pi-stream-cast) —
same idea, fewer dependencies, and the two bugs that plagued the original fixed by
design.

## Why a rewrite?

The original Python version (Flask + PyAudio + PyChromecast, served by Apache/mod_wsgi)
had two problems this project exists to fix:

1. **USB audio couldn't be read under Apache.** The `www-data` user isn't in the
   `audio` group, so PortAudio couldn't open the ALSA devices. Here, a single binary
   runs as a **systemd service** under a user in the `audio` group — the permission
   problem disappears.
2. **The stream drifted out of sync over time.** The old code shipped ~6-second PCM
   chunks behind a fake 2 GB WAV header. Here, audio is captured continuously and
   streamed as small chunks with a proper streaming header, so clients track the live
   edge instead of drifting.

It also drops the heavy toolchain: no `python3.9-dev`, `portaudio19-dev`,
`python3-pyaudio`, no Apache/mod_wsgi, and no Node at runtime. Just one Rust binary.

## Architecture

A single self-contained Rust binary (one `tokio` process) that:

- **captures** USB audio continuously via [`cpal`](https://crates.io/crates/cpal)
  (ALSA backend) into a shared broadcast buffer;
- **controls** the Chromecast via [`rust_cast`](https://crates.io/crates/rust_cast),
  with mDNS discovery via [`mdns-sd`](https://crates.io/crates/mdns-sd);
- **serves** the REST API and the static touch UI via
  [`axum`](https://crates.io/crates/axum).

The backend follows hexagonal / onion (ports & adapters) layering with DDD, organized
as a Cargo workspace so the dependency direction is enforced by the compiler:

```
crates/
  domain/          # pure core: entities, value objects, ports (traits). No I/O.
  application/     # use cases (StartCasting, SetVolume, …). Depends only on domain.
  infrastructure/  # adapters: cpal, rust_cast, mdns-sd, clock. Implements the ports.
  bff/             # axum HTTP layer + UI-shaped DTOs (backend-for-frontend).
bin/
  audiocast/       # composition root: config, DI wiring, tokio main.
www/               # static touch UI (no build step).
deploy/            # systemd unit, kiosk launcher, install script.
```

Because every external dependency sits behind a port, the domain and application
layers are unit-testable with mock adapters — no real audio device, Chromecast, or
network required.

## Requirements

- A Raspberry Pi running **Raspberry Pi OS (64-bit)**. The Desktop image is needed only
  if you want the on-device touchscreen kiosk; headless (Lite) works fine if you only
  control it from phones on the LAN.
- A **USB audio input device** (e.g. a turntable via a USB Audio CODEC).
- A **Google Chromecast Audio** (or Chromecast-compatible speaker / group) on the same LAN.
- Rust toolchain ([`rustup`](https://rustup.rs)).
- `libasound2-dev` (ALSA headers) at build time — `libasound2` itself ships with Pi OS.

## Fork it

This is a personal appliance project, and you'll almost certainly need to tweak it for
your own hardware (at minimum the USB device name — see below). The easiest path is to
**fork the repo on GitHub** and clone your fork, so you can commit your config and any
changes and pull future updates from upstream:

```sh
git clone https://github.com/<your-username>/rust-audio-cast.git
cd rust-audio-cast
```

## Quick start (build & run)

Install the one system build dependency and compile:

```sh
sudo apt-get install -y libasound2-dev
cargo build --release
```

A Pi 4/5 is fast enough to build on-device (a few minutes; longer on first build).
Cross-compiling (`aarch64-unknown-linux-gnu`) is an optional later speedup.

Run it:

```sh
cargo run --release
```

Then open `http://<pi-ip>:8080/` for the touch UI. Configuration lives in `config.toml`
(see below); the stream URL is built from the auto-detected local IP, so nothing is
hard-coded to your network.

## Finding your USB audio device name

**This is the one setting almost everyone has to change.** `cpal` selects the capture
device by matching a substring against the **ALSA PCM name** — not the friendly product
name. The default in `config.toml` will likely not match your device.

To find the right value, first confirm the kernel sees your input:

```sh
arecord -l
```

You'll see something like `card 2: CODEC [USB Audio CODEC], device 0: ...` — note the
short card name in brackets (`CODEC` here; yours may be `Device`, `Audio`, etc.).

Then run the app once with capture enabled and watch the log. If the configured name
doesn't match, it logs the exact list of candidates it found:

```sh
AUDIOCAST_AUDIO=capture cargo run --release
# WARN ... no input device name matched; wanted="..." available=["hw:CARD=CODEC,DEV=0",
#         "plughw:CARD=CODEC,DEV=0", "default:CARD=CODEC", ...]
```

Pick the `plughw:CARD=<name>` entry for your device — the `plughw` plug layer
auto-converts sample format / rate / channels, so it's the most forgiving choice — and
set it in `config.toml`:

```toml
usb_device_name = "plughw:CARD=CODEC"   # <-- replace CODEC with your card name
```

When it matches, the log shows `opened USB capture device device=...` with the device's
actual sample rate and channels. You can verify capture by opening
`http://<pi-ip>:8080/audio-stream` in VLC and confirming you hear your source.

## Configuration

All settings live in `config.toml` (every field has a default). The app resolves config
from, in order: `$AUDIOCAST_CONFIG`, `/etc/audiocast/config.toml`, `./config.toml`, then
built-in defaults.

| Field | Purpose |
|---|---|
| `usb_device_name` | ALSA PCM name substring for your input device (see above). |
| `bind_addr` | Address/port the HTTP server binds to (default `0.0.0.0:8080`). |
| `sample_rate`, `channels` | Requested capture format (the device's actual format is used if it differs). |
| `backend` | `fake` (in-memory cast + static list, no hardware) or `live` (real Chromecast + mDNS). |
| `audio` | `synthetic` (440 Hz test tone, no device) or `capture` (real USB capture). |
| `discovery_timeout_secs` | How long an mDNS speaker scan runs. |
| `web_root` | Directory the static UI is served from (absolute path under the service). |

`backend` and `audio` are independent and can each be overridden at runtime via the
`AUDIOCAST_BACKEND` and `AUDIOCAST_AUDIO` environment variables.

### Test without hardware

The defaults (`backend = "fake"`, `audio = "synthetic"`) run anywhere — including a dev
laptop — with no USB device or Chromecast. To validate **real casting** before wiring up
the turntable, use the live cast backend with the synthetic tone:

```sh
AUDIOCAST_BACKEND=live cargo run --release
# open http://localhost:8080, find speakers, pick one, Play — you'll hear the 440 Hz tone
```

> On macOS the binary needs **Local Network** permission (System Settings → Privacy &
> Security → Local Network) for mDNS discovery, and you may get a firewall prompt to
> **allow incoming connections** so the Chromecast can fetch the stream. Both are
> one-time grants. (Linux/Pi has no such prompt.)

## Deploy as a service

One script does the whole install on the Pi — build, create a dedicated service user,
install the binary + UI + config, enable the systemd service, and (on a Desktop image)
set up a full-screen Chromium kiosk:

```sh
sudo deploy/install.sh
```

It is idempotent, so re-running after `git pull` redeploys (and preserves your
`/etc/audiocast/config.toml`). What it sets up:

- The binary runs as a **systemd service** under a dedicated `audiocast` user that is a
  member of the `audio` group — this is the structural fix for the original
  USB-permission bug. It reads `/etc/audiocast/config.toml` (the installed production
  config defaults to `backend = "live"`, `audio = "capture"`).
- The static UI is installed to `/usr/share/audiocast/www` and served via the absolute
  `web_root` in the config, so it works regardless of the service's working directory.
- On a Desktop image, the touchscreen shows the UI full-screen via a **Chromium kiosk**
  launched from a desktop icon and auto-started at login.

**After installing, edit `/etc/audiocast/config.toml` to set your `usb_device_name`**
(see "Finding your USB audio device name"), then `sudo systemctl restart audiocast`.

Useful afterwards:

```sh
systemctl status audiocast      # is it running?
journalctl -u audiocast -f      # live logs (device selected, stream URL, errors)
```

See `deploy/` for the unit file (`audiocast.service`), kiosk launcher (`kiosk.sh`,
`audiocast-kiosk.desktop`), production config (`config.toml`), and the installer. The
kiosk is for an X11 session; if your display uses Wayland, use your compositor's idle /
kiosk settings instead, or just control the app from a phone.

## API

The HTTP API preserves the endpoints of the original project (status, audio-stream,
list/refresh casts, start/stop cast, get/set/step/mute volume) so the UI contract is
stable. The live stream is served at `GET /audio-stream` as chunked `audio/wav`.

## License

See [LICENSE](LICENSE).
