//! Orbit camera controller and view-projection matrix math.

use bytemuck::{Pod, Zeroable};

/// GPU uniform buffer structure for camera matrices, lighting, and model transform.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub light_dir: [f32; 4],
}

impl Default for CameraUniforms {
    fn default() -> Self {
        Self {
            view_proj: IDENTITY_MAT4,
            model: IDENTITY_MAT4,
            camera_pos: [0.0, 1.5, 4.0, 1.0],
            light_dir: [0.577, 0.577, 0.577, 0.0],
        }
    }
}

pub const IDENTITY_MAT4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// World orientation matrix transforming NFSU2 coordinates (X=length, Y=width, Z=height)
/// to standard engine world coordinates (X=right, Y=up, Z=-length).
pub const NFSU2_TO_WORLD_MAT4: [[f32; 4]; 4] = [
    [0.0, 0.0, -1.0, 0.0],
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Turntable / Orbit camera controller centered around target vehicle.
#[derive(Debug, Clone)]
pub struct OrbitCamera {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: [0.0, 0.4, 0.0],
            yaw: 0.6,
            pitch: 0.3,
            distance: 4.8,
            fov_y: 45.0_f32.to_radians(),
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 100.0,
        }
    }
}

impl OrbitCamera {
    /// Create a new camera with specific aspect ratio.
    pub fn new(aspect: f32) -> Self {
        Self {
            aspect,
            ..Default::default()
        }
    }

    /// Rotate camera by delta yaw and pitch (in radians).
    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-1.45, 1.45);
    }

    /// Zoom camera by delta distance factor.
    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(1.5, 20.0);
    }

    /// Reset camera to default orientation.
    pub fn reset(&mut self) {
        *self = Self {
            aspect: self.aspect,
            ..Default::default()
        };
    }

    /// Update aspect ratio on window resize.
    pub fn set_aspect(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.aspect = width as f32 / height as f32;
        }
    }

    /// Calculate the eye position in world space.
    pub fn eye_position(&self) -> [f32; 3] {
        let cos_pitch = self.pitch.cos();
        let sin_pitch = self.pitch.sin();
        let cos_yaw = self.yaw.cos();
        let sin_yaw = self.yaw.sin();

        [
            self.target[0] + self.distance * cos_pitch * sin_yaw,
            self.target[1] + self.distance * sin_pitch,
            self.target[2] + self.distance * cos_pitch * cos_yaw,
        ]
    }

    /// Compute View Matrix (LookAt).
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let eye = self.eye_position();
        let forward = normalize([
            self.target[0] - eye[0],
            self.target[1] - eye[1],
            self.target[2] - eye[2],
        ]);
        let up_guide = [0.0, 1.0, 0.0];
        let right = normalize(cross(forward, up_guide));
        let up = cross(right, forward);

        [
            [right[0], up[0], -forward[0], 0.0],
            [right[1], up[1], -forward[1], 0.0],
            [right[2], up[2], -forward[2], 0.0],
            [-dot(right, eye), -dot(up, eye), dot(forward, eye), 1.0],
        ]
    }

    /// Compute Perspective Projection Matrix (DirectX / Vulkan [0, 1] depth range).
    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        let tan_half_fov = (self.fov_y * 0.5).tan();
        let f = 1.0 / tan_half_fov;
        let range = self.far - self.near;

        [
            [f / self.aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, -self.far / range, -1.0],
            [0.0, 0.0, -(self.near * self.far) / range, 0.0],
        ]
    }

    /// Compute combined View-Projection matrix.
    pub fn view_projection_matrix(&self) -> [[f32; 4]; 4] {
        mat4_mul(&self.projection_matrix(), &self.view_matrix())
    }

    /// Build camera uniforms for shader binding.
    pub fn build_uniforms(&self) -> CameraUniforms {
        let eye = self.eye_position();
        CameraUniforms {
            view_proj: self.view_projection_matrix(),
            model: NFSU2_TO_WORLD_MAT4,
            camera_pos: [eye[0], eye[1], eye[2], 1.0],
            light_dir: normalize_4([0.5, 1.0, 0.7, 0.0]),
        }
    }
}

// Minimal vector and matrix helper functions
#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-6 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}

#[inline]
fn normalize_4(v: [f32; 4]) -> [f32; 4] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-6 {
        [v[0] / len, v[1] / len, v[2] / len, 0.0]
    } else {
        [0.0, 1.0, 0.0, 0.0]
    }
}

#[inline]
pub fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            out[col][row] = a[0][row] * b[col][0]
                + a[1][row] * b[col][1]
                + a[2][row] * b[col][2]
                + a[3][row] * b[col][3];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_default_matrices_finite() {
        let cam = OrbitCamera::default();
        let vp = cam.view_projection_matrix();
        for col in vp {
            for val in col {
                assert!(val.is_finite());
            }
        }
    }

    #[test]
    fn camera_rotation_clamps_pitch() {
        let mut cam = OrbitCamera::default();
        cam.rotate(0.0, 10.0);
        assert!((cam.pitch - 1.45).abs() < 1e-4);

        cam.rotate(0.0, -20.0);
        assert!((cam.pitch - (-1.45)).abs() < 1e-4);
    }
}
