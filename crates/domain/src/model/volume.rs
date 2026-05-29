/// A speaker volume level, constrained to `0.0..=1.0`.
///
/// Construction always clamps into range and rounds to whole-percent precision,
/// so illegal or jittery values are unrepresentable and equality is reliable
/// (the old Python API accepted any float and could be driven out of range).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume(f32);

impl Volume {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 1.0;
    /// Increment used by volume-up / volume-down (1%), matching the original app.
    pub const STEP: f32 = 0.01;

    pub fn new(level: f32) -> Self {
        Self(round2(level.clamp(Self::MIN, Self::MAX)))
    }

    pub fn as_f32(self) -> f32 {
        self.0
    }

    /// One step louder, clamped at `MAX`.
    pub fn up(self) -> Self {
        Self::new(self.0 + Self::STEP)
    }

    /// One step quieter, clamped at `MIN`.
    pub fn down(self) -> Self {
        Self::new(self.0 - Self::STEP)
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self(0.0)
    }
}

fn round2(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_above_max() {
        assert_eq!(Volume::new(1.5).as_f32(), 1.0);
    }

    #[test]
    fn clamps_below_min() {
        assert_eq!(Volume::new(-0.3).as_f32(), 0.0);
    }

    #[test]
    fn up_steps_by_one_percent() {
        assert_eq!(Volume::new(0.50).up(), Volume::new(0.51));
    }

    #[test]
    fn up_is_noop_at_max() {
        assert_eq!(Volume::new(1.0).up(), Volume::new(1.0));
    }

    #[test]
    fn down_is_noop_at_min() {
        assert_eq!(Volume::new(0.0).down(), Volume::new(0.0));
    }

    #[test]
    fn rounds_to_whole_percent() {
        assert_eq!(Volume::new(0.514).as_f32(), 0.51);
    }
}
