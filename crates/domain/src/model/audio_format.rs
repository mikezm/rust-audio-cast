/// PCM audio format of the captured stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl AudioFormat {
    /// CD-quality stereo: the turntable's native format and the project default.
    pub const CD: AudioFormat = AudioFormat {
        sample_rate: 44_100,
        channels: 2,
        bits_per_sample: 16,
    };

    /// MIME type advertised for the WAV stream and passed to the Chromecast.
    pub const MIME: &'static str = "audio/wav";
}
