//! Domain core for rust-audio-cast.
//!
//! This is the innermost layer of the onion: pure types and trait definitions
//! ("ports") with no knowledge of cpal, rust_cast, axum, or any async runtime.
//! Everything else depends inward on this crate; this crate depends on nothing
//! but `async-trait` and `thiserror`.

pub mod error;
pub mod model;
pub mod ports;

pub use error::DomainError;
pub use model::*;
pub use ports::*;
