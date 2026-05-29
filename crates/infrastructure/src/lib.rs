//! Driven adapters: concrete implementations of the domain ports.
//!
//! Two backends are available, selected by config:
//! - *fake* — in-memory cast controller + static device list (+ synthetic-tone
//!   audio), so the whole app runs with no hardware.
//! - *live* — real `rust_cast` control + `mdns-sd` discovery (Chunk E). Audio is
//!   still the synthetic tone until the cpal capture lands in Chunk D.

mod audio_stream;
mod cast;
mod clock;
mod discovery;
mod mdns_discovery;
mod rust_cast_controller;

pub use audio_stream::SyntheticAudioStream;
pub use cast::InMemoryCastController;
pub use clock::SystemClock;
pub use discovery::StaticDiscovery;
pub use mdns_discovery::MdnsDiscovery;
pub use rust_cast_controller::RustCastController;
