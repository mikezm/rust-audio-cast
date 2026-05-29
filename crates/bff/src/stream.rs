use std::convert::Infallible;

use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use domain::AudioFormat;
use futures::StreamExt;

use crate::state::AppState;

/// Serve the live audio as a chunked WAV stream: one canonical header, then PCM
/// frames straight off the broadcast. No Content-Length and a streaming-size
/// header (`0xFFFFFFFF`) so the Chromecast treats it as a live stream, not a
/// finite seekable file — the fix for the original drift/desync bug.
pub async fn audio_stream(State(state): State<AppState>) -> Response {
    let header = wav_header(state.format);

    let head = futures::stream::once(async move { Ok::<Bytes, Infallible>(header) });
    let frames = state.audio.subscribe().map(Ok::<Bytes, Infallible>);
    let body = Body::from_stream(head.chain(frames));

    ([(header::CONTENT_TYPE, AudioFormat::MIME)], body).into_response()
}

/// Canonical 44-byte WAV/PCM header with streaming (unknown) sizes.
fn wav_header(format: AudioFormat) -> Bytes {
    let channels = format.channels;
    let sample_rate = format.sample_rate;
    let bits = format.bits_per_sample;
    let byte_rate = sample_rate * channels as u32 * (bits as u32 / 8);
    let block_align = channels * (bits / 8);
    const STREAMING_SIZE: u32 = 0xFFFF_FFFF;

    let mut h = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&STREAMING_SIZE.to_le_bytes());
    h.extend_from_slice(b"WAVE");
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    h.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    h.extend_from_slice(&channels.to_le_bytes());
    h.extend_from_slice(&sample_rate.to_le_bytes());
    h.extend_from_slice(&byte_rate.to_le_bytes());
    h.extend_from_slice(&block_align.to_le_bytes());
    h.extend_from_slice(&bits.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&STREAMING_SIZE.to_le_bytes());
    Bytes::from(h)
}
