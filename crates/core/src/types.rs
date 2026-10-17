//! Domain primitives and newtype wrappers preventing primitive obsession.

use std::time::Duration;

/// Strongly-typed 32-bit chunk identifier used in legacy PC container streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ChunkId(pub u32);

impl ChunkId {
    /// Create a new ChunkId from a raw little-endian integer.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return the raw integer value of the chunk id.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Strongly-typed engine revolutions per minute (RPM).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rpm(pub f32);

impl Rpm {
    /// Construct an RPM value, clamped to non-negative numbers.
    #[inline]
    pub fn new(val: f32) -> Self {
        Self(val.max(0.0))
    }

    /// Return the raw floating-point value.
    #[inline]
    pub fn raw(self) -> f32 {
        self.0
    }
}

/// Strongly-typed fixed simulation delta step (e.g. 60 Hz or 120 Hz).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedDelta(pub Duration);

impl FixedDelta {
    /// 60 Hz simulation step (16.666... ms).
    pub const STEP_60HZ: Self = Self(Duration::from_nanos(16_666_667));

    /// 120 Hz simulation step (8.333... ms).
    pub const STEP_120HZ: Self = Self(Duration::from_nanos(8_333_333));

    /// Return the timestep as fractional seconds (`f32`).
    #[inline]
    pub fn as_secs_f32(self) -> f32 {
        self.0.as_secs_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_id_equality_and_raw() {
        let chunk = ChunkId::new(0x0003_4000);
        assert_eq!(chunk.raw(), 0x0003_4000);
        assert_eq!(chunk, ChunkId(0x0003_4000));
    }

    #[test]
    fn rpm_clamped_non_negative() {
        let neg = Rpm::new(-500.0);
        assert_eq!(neg.raw(), 0.0);

        let pos = Rpm::new(6500.0);
        assert_eq!(pos.raw(), 6500.0);
    }

    #[test]
    fn fixed_delta_conversions() {
        let delta = FixedDelta::STEP_60HZ;
        let secs = delta.as_secs_f32();
        assert!((secs - 0.0166666).abs() < 1e-4);
    }
}
