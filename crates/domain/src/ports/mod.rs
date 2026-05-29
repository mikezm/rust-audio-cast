//! Ports: the trait boundaries the application layer talks through. Adapters in
//! the `infrastructure` crate implement these; tests mock them. Signatures use
//! only domain types and `DomainError` — never an infra library's types.

mod audio_source;
mod cast;
mod clock;
mod discovery;

pub use audio_source::*;
pub use cast::*;
pub use clock::*;
pub use discovery::*;
