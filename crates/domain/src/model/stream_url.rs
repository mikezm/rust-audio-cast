use std::fmt;

/// The HTTP URL the Chromecast pulls the audio stream from.
///
/// Built by the composition root from the auto-detected local IP, so nothing is
/// hard-coded to a particular network (the old app baked in `192.168.50.242`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamUrl(String);

impl StreamUrl {
    pub fn new(url: impl Into<String>) -> Self {
        Self(url.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StreamUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
