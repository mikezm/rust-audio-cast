//! Driven adapters: concrete implementations of the domain ports.
//!
//! Control/discovery backends are selected by config:
//! - *fake* — in-memory cast controller + static device list, so the whole app
//!   runs with no hardware.
//! - *live* — real `rust_cast` control + `mdns-sd` discovery (Chunk E).
//!
//! The audio source is selected independently: a [`SyntheticAudioStream`] 440 Hz
//! tone for hardware-free dev, or real USB capture via [`CpalAudioStream`]
//! (Chunk D). Both implement [`PcmSource`].

mod audio_stream;
mod cast;
mod clock;
mod cpal_audio;
mod discovery;
mod mdns_discovery;
mod pcm_source;
mod rust_cast_controller;

pub use audio_stream::SyntheticAudioStream;
pub use cast::InMemoryCastController;
pub use clock::SystemClock;
pub use cpal_audio::CpalAudioStream;
pub use discovery::StaticDiscovery;
pub use mdns_discovery::MdnsDiscovery;
pub use pcm_source::PcmSource;
pub use rust_cast_controller::RustCastController;
