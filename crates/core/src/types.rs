//! Domain primitives and newtype wrappers preventing primitive obsession.

use bytemuck::{Pod, Zeroable};
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

/// Normalized analog vehicle control state desacoupled from UI and physics.
/// Conforms to REQ-M3-09: can be updated via virtual touch controls or physical gamepads.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
pub struct VehicleControlState {
    /// Analog steering input: `[-1.0 (full left) .. 1.0 (full right)]`.
    pub steering: f32,
    /// Analog throttle input: `[0.0 (idle) .. 1.0 (full acceleration)]`.
    pub throttle: f32,
    /// Analog brake input: `[0.0 (released) .. 1.0 (full braking)]`.
    pub brake: f32,
    /// Digital handbrake / e-brake: `0` (released) or `1` (engaged).
    pub handbrake: u32,
}

impl VehicleControlState {
    /// Construct a clamped vehicle control state.
    #[inline]
    pub fn new(steering: f32, throttle: f32, brake: f32, handbrake: bool) -> Self {
        Self {
            steering: steering.clamp(-1.0, 1.0),
            throttle: throttle.clamp(0.0, 1.0),
            brake: brake.clamp(0.0, 1.0),
            handbrake: if handbrake { 1 } else { 0 },
        }
    }

    /// Reset all controls to neutral idle position.
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Return true if all controls are in neutral idle position.
    #[inline]
    pub fn is_neutral(&self) -> bool {
        self.steering == 0.0 && self.throttle == 0.0 && self.brake == 0.0 && self.handbrake == 0
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

    #[test]
    fn vehicle_control_state_clamping_and_pod() {
        let ctrl = VehicleControlState::new(-2.5, 1.5, -0.5, true);
        assert_eq!(ctrl.steering, -1.0);
        assert_eq!(ctrl.throttle, 1.0);
        assert_eq!(ctrl.brake, 0.0);
        assert_eq!(ctrl.handbrake, 1);
        assert!(!ctrl.is_neutral());

        let bytes: &[u8] = bytemuck::bytes_of(&ctrl);
        assert_eq!(bytes.len(), 16);

        let roundtrip: &VehicleControlState = bytemuck::from_bytes(bytes);
        assert_eq!(*roundtrip, ctrl);

        let mut reset_ctrl = ctrl;
        reset_ctrl.reset();
        assert!(reset_ctrl.is_neutral());
    }
}
