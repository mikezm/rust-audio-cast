//! Driven adapters: concrete implementations of the domain ports.
//!
//! Chunk B ships *fakes* so the whole app runs end-to-end with no hardware:
//! an in-memory cast controller, a static device list, and a synthetic-tone
//! audio source. The real cpal / rust_cast / mdns-sd adapters replace these in
//! Chunks D and E without the inner layers changing.

mod audio_stream;
mod cast;
mod clock;
mod discovery;

pub use audio_stream::SyntheticAudioStream;
pub use cast::InMemoryCastController;
pub use clock::SystemClock;
pub use discovery::StaticDiscovery;
