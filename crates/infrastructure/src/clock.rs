use std::time::Instant;

use domain::Clock;

/// Wall-clock implementation of the `Clock` port.
#[derive(Default)]
pub struct SystemClock;

impl SystemClock {
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
