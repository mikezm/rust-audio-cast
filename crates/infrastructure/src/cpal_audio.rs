use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use domain::{AudioFormat, DomainError};
use tokio::sync::broadcast;

use crate::pcm_source::PcmSource;

const CHANNEL_CAPACITY: usize = 64;

// Backoff between capture rebuild attempts while the device is gone.
const REBUILD_BACKOFF_MIN: Duration = Duration::from_secs(1);
const REBUILD_BACKOFF_MAX: Duration = Duration::from_secs(30);

/// Real USB audio capture via `cpal` (the ALSA backend on the Pi).
///
/// `cpal::Stream` is `!Send`, so it is built and owned by a dedicated thread
/// that keeps the stream alive for the life of the process, rebuilding it if
/// it dies (see [`run_capture`]). The capture
/// callback converts samples to interleaved i16 PCM and pushes them onto the
/// same lossy `broadcast` channel the synthetic source uses — so everything
/// downstream (the WAV stream handler, the Chromecast) is unchanged; only the
/// bytes are now real turntable audio.
pub struct CpalAudioStream {
    tx: broadcast::Sender<Bytes>,
    format: AudioFormat,
}

impl CpalAudioStream {
    /// Open the first input device whose name contains `device_substr` (falling
    /// back to the host default input if none match), start capture on a
    /// dedicated thread, and return a handle once the stream is running.
    ///
    /// The advertised [`AudioFormat`] reflects the device's *actual* sample rate
    /// and channel count, so the WAV header and Chromecast playback stay in
    /// tune even if the device doesn't run at the configured 44.1 kHz.
    pub fn start(device_substr: String) -> Result<Arc<Self>, DomainError> {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        let tx_thread = tx.clone();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<AudioFormat, DomainError>>();

        std::thread::Builder::new()
            .name("cpal-capture".into())
            .spawn(move || run_capture(device_substr, tx_thread, ready_tx))
            .map_err(|e| DomainError::Audio(format!("could not spawn capture thread: {e}")))?;

        // Block until the thread either opens the device or reports why it
        // couldn't. This only happens once, at startup, before we serve.
        let format = ready_rx
            .recv()
            .map_err(|_| {
                DomainError::Audio("capture thread exited before opening device".into())
            })??;

        Ok(Arc::new(Self { tx, format }))
    }
}

impl PcmSource for CpalAudioStream {
    fn format(&self) -> AudioFormat {
        self.format
    }

    fn subscribe(&self) -> broadcast::Receiver<Bytes> {
        self.tx.subscribe()
    }
}

/// Body of the dedicated capture thread: build and play the stream, report the
/// resolved format (or an error) back to `start`, then block until the stream
/// reports a fatal error (e.g. ALSA POLLERR after a USB hiccup), rebuild it,
/// and carry on. The `!Send` stream lives and dies on this thread.
///
/// A failure *before* the first successful start is reported through
/// `ready_tx` and ends the thread: at startup a missing device should fail the
/// service loudly. After that, failures retry with backoff so a re-plugged
/// device resumes capture without a service restart.
fn run_capture(
    device_substr: String,
    tx: broadcast::Sender<Bytes>,
    ready_tx: mpsc::Sender<Result<AudioFormat, DomainError>>,
) {
    let mut ready_tx = Some(ready_tx);
    let mut started_format: Option<AudioFormat> = None;
    let mut backoff = REBUILD_BACKOFF_MIN;

    loop {
        let (err_tx, err_rx) = mpsc::channel::<cpal::StreamError>();
        let attempt = build_stream(&device_substr, tx.clone(), err_tx).and_then(
            |(stream, format)| match stream.play() {
                Ok(()) => Ok((stream, format)),
                Err(e) => Err(DomainError::Audio(format!("could not start capture: {e}"))),
            },
        );

        let (stream, format) = match attempt {
            Ok(ok) => ok,
            Err(e) => match ready_tx.take() {
                Some(ready) => {
                    let _ = ready.send(Err(e));
                    return;
                }
                None => {
                    tracing::warn!(error = %e, retry_in = ?backoff, "capture rebuild failed; retrying");
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(REBUILD_BACKOFF_MAX);
                    continue;
                }
            },
        };

        match started_format {
            None => {
                started_format = Some(format);
                if let Some(ready) = ready_tx.take() {
                    let _ = ready.send(Ok(format));
                }
            }
            // The WAV header and any live cast session were negotiated with
            // the original format, so we can't silently switch: exit and let
            // systemd (Restart=on-failure) bring us back up clean.
            Some(started) if started != format => {
                tracing::error!(
                    ?started,
                    now = ?format,
                    "device came back with a different format; exiting for a clean restart"
                );
                std::process::exit(1);
            }
            Some(_) => tracing::info!("capture stream rebuilt"),
        }
        backoff = REBUILD_BACKOFF_MIN;

        // Block until the stream errors. The cpal ALSA backend repeats the
        // error callback in a tight loop, so log once here and drop the stream
        // to silence it before rebuilding.
        match err_rx.recv() {
            Ok(e) => tracing::error!(error = %e, "capture stream failed; rebuilding"),
            Err(mpsc::RecvError) => tracing::error!("capture stream vanished; rebuilding"),
        }
        drop(stream);
    }
}

// cpal 0.17 deprecates `Device::name()`, but the ALSA device name is exactly
// what we match the configured substring against, so we keep using it.
#[allow(deprecated)]
fn build_stream(
    device_substr: &str,
    tx: broadcast::Sender<Bytes>,
    err_tx: mpsc::Sender<cpal::StreamError>,
) -> Result<(cpal::Stream, AudioFormat), DomainError> {
    let host = cpal::default_host();
    let device = find_device(&host, device_substr)?;
    let name = device.name().unwrap_or_else(|_| "<unknown>".into());

    let supported = device
        .default_input_config()
        .map_err(|e| DomainError::Audio(format!("no default input config for '{name}': {e}")))?;
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    // In cpal 0.17 `SampleRate`/`ChannelCount` are plain `u32`/`u16` aliases.
    let format = AudioFormat {
        sample_rate: config.sample_rate,
        channels: config.channels,
        bits_per_sample: 16,
    };

    tracing::info!(
        device = %name,
        sample_rate = format.sample_rate,
        channels = format.channels,
        ?sample_format,
        "opened USB capture device"
    );

    // Convert each native sample to i16 PCM. Explicit per-format closures avoid
    // depending on cpal's optional sample-conversion traits.
    let stream = match sample_format {
        SampleFormat::F32 => build_typed::<f32>(&device, &config, tx, err_tx, |s| {
            (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
        })?,
        SampleFormat::I16 => build_typed::<i16>(&device, &config, tx, err_tx, |s| s)?,
        SampleFormat::U16 => {
            build_typed::<u16>(&device, &config, tx, err_tx, |s| (s as i32 - 32_768) as i16)?
        }
        other => {
            return Err(DomainError::Audio(format!(
                "unsupported sample format: {other:?}"
            )))
        }
    };

    Ok((stream, format))
}

/// Find an input device whose name contains `substr`. Logs what's available and
/// falls back to the host default if nothing matches — ALSA names from cpal
/// don't always match `arecord -l`, so the fallback keeps capture working while
/// the warning tells us to fix the configured substring.
#[allow(deprecated)]
fn find_device(host: &cpal::Host, substr: &str) -> Result<cpal::Device, DomainError> {
    let devices = host
        .input_devices()
        .map_err(|e| DomainError::Audio(format!("could not enumerate input devices: {e}")))?;

    let mut available = Vec::new();
    for device in devices {
        match device.name() {
            Ok(name) if name.contains(substr) => return Ok(device),
            Ok(name) => available.push(name),
            Err(_) => {}
        }
    }

    tracing::warn!(
        wanted = substr,
        available = ?available,
        "no input device name matched; falling back to default input"
    );
    host.default_input_device()
        .ok_or_else(|| DomainError::Audio("no default input device available".into()))
}

/// Build a typed input stream that converts each sample to i16 little-endian PCM
/// and broadcasts it. `data` is already channel-interleaved, so we preserve it.
fn build_typed<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: broadcast::Sender<Bytes>,
    err_tx: mpsc::Sender<cpal::StreamError>,
    convert: fn(T) -> i16,
) -> Result<cpal::Stream, DomainError>
where
    T: cpal::SizedSample + 'static,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut buf = Vec::with_capacity(data.len() * 2);
                for &sample in data {
                    buf.extend_from_slice(&convert(sample).to_le_bytes());
                }
                // Err just means no current subscribers; keep capturing.
                let _ = tx.send(Bytes::from(buf));
            },
            // Hand the error to the capture thread, which logs it and
            // rebuilds the stream. Err means it already got one; drop.
            move |e| {
                let _ = err_tx.send(e);
            },
            None,
        )
        .map_err(|e| DomainError::Audio(format!("could not build input stream: {e}")))
}
