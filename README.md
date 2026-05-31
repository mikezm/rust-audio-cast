# rust-audio-cast
## Raspberry Pi → Chromecast Audio streamer, in Rust

Reads USB audio input (e.g. a turntable) on a Raspberry Pi, hosts it as a low-latency
HTTP audio stream, discovers Google Chromecast Audio speakers on the LAN, and tells a
chosen speaker to play the stream. A touch-friendly web UI (and the Pi's own
touchscreen) controls play / stop / volume / speaker selection.

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

- Raspberry Pi (developed on a Pi 5) running **Raspberry Pi OS Desktop (64-bit, Bookworm)**
- A USB audio input device (e.g. a turntable via a USB Audio CODEC)
- A Google Chromecast Audio device on the same LAN
- Rust toolchain ([`rustup`](https://rustup.rs))
- `libasound2-dev` (ALSA headers) at build time — `libasound2` ships with Pi OS

## Build

Install the one system build dependency and compile:

```sh
sudo apt-get install libasound2-dev
cargo build --release
```

The Pi 5 is fast enough to build on-device; cross-compiling
(`aarch64-unknown-linux-gnu`) is an optional later speedup.

## Run

```sh
cargo run --release
```

Then open `http://<pi-ip>/` for the touch UI, or point a Chromecast at the stream via
the UI. Configuration lives in `config.toml` (USB device name, bind address, sample
rate, optional default Chromecast UUID); the stream URL is built from the
auto-detected local IP, so nothing is hard-coded to your network.

### Backends: fake vs. live

`config.toml`'s `backend` (or env `AUDIOCAST_BACKEND`) selects the adapters:

- `fake` (default) — in-memory cast + a static device list. Runs anywhere, no hardware.
- `live` — real Chromecast control (`rust_cast`) + LAN discovery (`mdns-sd`). The
  audio is still the synthetic 440 Hz tone until the cpal USB capture lands, so this
  lets you validate casting end-to-end from any dev machine before wiring up the
  turntable:

  ```sh
  AUDIOCAST_BACKEND=live cargo run
  # open http://localhost:8080, Rescan, pick your speaker, Play
  ```

  On macOS the binary needs **Local Network** permission (System Settings → Privacy &
  Security → Local Network) for mDNS discovery to see devices, and you may get a
  firewall prompt to **allow incoming connections** so the Chromecast can fetch the
  stream. Both are one-time grants.

## Deploy as a service

One script does the whole install on the Pi — build, dedicated service user, binary +
UI + config, systemd service, and the Chromium kiosk:

```sh
sudo deploy/install.sh
```

It is idempotent, so re-running after `git pull` redeploys (and preserves
`/etc/audiocast/config.toml`). What it sets up:

- The binary runs as a **systemd service** under the dedicated `audiocast` user, which
  is a member of the `audio` group (`SupplementaryGroups=audio`) — this is the
  structural fix for the original USB-permission bug. The service reads its config from
  `/etc/audiocast/config.toml` (`backend = "live"`, `audio = "capture"`).
- The static UI is installed to `/usr/share/audiocast/www` and served via the absolute
  `web_root` in the config, so it works regardless of the service's working directory.
- The Pi's touchscreen shows the UI full-screen via **Chromium kiosk** (`deploy/kiosk.sh`),
  launched from an "Audio Cast" desktop icon and auto-started at login. The kiosk assumes
  an **X11** session (which the SPI-display setup uses) so the `xset` screen-blank
  disabling applies.

See `deploy/` for the unit file (`audiocast.service`), kiosk launcher (`kiosk.sh`,
`audiocast-kiosk.desktop`), production config (`config.toml`), and the installer.
Useful afterwards: `systemctl status audiocast` and `journalctl -u audiocast -f`.

## API

The HTTP API preserves the endpoints of the original project (status, audio-stream,
list/refresh casts, start/stop cast, stop stream, get/set/step/mute volume) so the UI
contract is stable. The live stream is served at `GET /audio-stream` as chunked
`audio/wav`.

## License

See [LICENSE](LICENSE).
