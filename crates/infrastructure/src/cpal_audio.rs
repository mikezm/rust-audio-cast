use std::sync::mpsc;
use std::sync::Arc;

use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use domain::{AudioFormat, DomainError};
use tokio::sync::broadcast;

use crate::pcm_source::PcmSource;

const CHANNEL_CAPACITY: usize = 64;

/// Real USB audio capture via `cpal` (the ALSA backend on the Pi).
///
/// `cpal::Stream` is `!Send`, so it is built and owned by a dedicated thread
/// that parks to keep the stream alive for the life of the process. The capture
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
/// resolved format (or an error) back to `start`, then park forever so the
/// `!Send` stream is never dropped.
fn run_capture(
    device_substr: String,
    tx: broadcast::Sender<Bytes>,
    ready_tx: mpsc::Sender<Result<AudioFormat, DomainError>>,
) {
    match build_stream(&device_substr, tx) {
        Ok((stream, format)) => {
            if let Err(e) = stream.play() {
                let _ = ready_tx
                    .send(Err(DomainError::Audio(format!("could not start capture: {e}"))));
                return;
            }
            let _ = ready_tx.send(Ok(format));
            // Keep the stream alive; dropping it stops capture.
            loop {
                std::thread::park();
            }
        }
        Err(e) => {
            let _ = ready_tx.send(Err(e));
        }
    }
}

// cpal 0.17 deprecates `Device::name()`, but the ALSA device name is exactly
// what we match the configured substring against, so we keep using it.
#[allow(deprecated)]
fn build_stream(
    device_substr: &str,
    tx: broadcast::Sender<Bytes>,
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
        SampleFormat::F32 => build_typed::<f32>(&device, &config, tx, |s| {
            (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
        })?,
        SampleFormat::I16 => build_typed::<i16>(&device, &config, tx, |s| s)?,
        SampleFormat::U16 => {
            build_typed::<u16>(&device, &config, tx, |s| (s as i32 - 32_768) as i16)?
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
            err_fn,
            None,
        )
        .map_err(|e| DomainError::Audio(format!("could not build input stream: {e}")))
}

fn err_fn(e: cpal::StreamError) {
    tracing::error!(error = %e, "cpal capture stream error");
}
