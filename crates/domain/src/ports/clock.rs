use std::time::Instant;

/// Injected time source, so any future time-dependent domain logic stays
/// deterministic under test instead of reading the wall clock directly.
#[cfg_attr(feature = "mock", mockall::automock)]
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}
