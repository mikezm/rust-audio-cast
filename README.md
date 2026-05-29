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

## Deploy as a service

Run the binary as a systemd service under a user in the `audio` group — this is the
fix for the original USB-permission bug:

```ini
# deploy/audiocast.service
[Service]
User=audiocast
SupplementaryGroups=audio                 # grants ALSA /dev/snd access
AmbientCapabilities=CAP_NET_BIND_SERVICE   # if binding :80
ExecStart=/usr/local/bin/rust-audio-cast
Restart=on-failure
```

The Pi's touchscreen shows the same web UI full-screen via Chromium kiosk mode,
launched from a desktop icon. See `deploy/` for the unit file, kiosk launcher, and
install script.

## API

The HTTP API preserves the endpoints of the original project (status, audio-stream,
list/refresh casts, start/stop cast, stop stream, get/set/step/mute volume) so the UI
contract is stable. The live stream is served at `GET /audio-stream` as chunked
`audio/wav`.

## License

See [LICENSE](LICENSE).
