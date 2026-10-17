//! Mathematical and geometric primitives with zero-copy and DoD alignment guarantees.

use bytemuck::{Pod, Zeroable};

/// Cache-aligned, zero-copy 3D single-precision float vector.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Vec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3f {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pod_zeroable_bytemuck_cast() {
        let v = Vec3f::new(1.0, 2.0, 3.0);
        let bytes: &[u8] = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 12);

        let roundtrip: &Vec3f = bytemuck::from_bytes(bytes);
        assert_eq!(*roundtrip, v);
    }

    #[test]
    fn vector_math_operations() {
        let a = Vec3f::new(3.0, 4.0, 0.0);
        assert_eq!(a.length(), 5.0);
    }
}
