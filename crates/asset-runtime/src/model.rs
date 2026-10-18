//! Strongly typed runtime model data structures optimized for mobile GPU staging and rendering.

use bytemuck::{Pod, Zeroable};

/// Axis-aligned bounding box.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct RuntimeBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Default for RuntimeBounds {
    fn default() -> Self {
        Self {
            min: [f32::MAX; 3],
            max: [f32::MIN; 3],
        }
    }
}

pub use open_nfs_core::math::RuntimeVertex;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct RuntimeSubmesh {
    pub material_id: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub shader_flags: u32,
    pub bounds: RuntimeBounds,
}

/// A logical vehicle part or sub-component containing vertices and draw calls.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePart {
    pub name: String,
    pub lod_level: u16,
    pub part_hash: u32,
    pub bounds: RuntimeBounds,
    pub vertices: Vec<RuntimeVertex>,
    pub indices: Vec<u16>,
    pub submeshes: Vec<RuntimeSubmesh>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuntimeModel {
    pub source_sha256: [u8; 32],
    pub parts: Vec<RuntimePart>,
}

impl RuntimeModel {
    pub fn find_part(&self, name: &str) -> Option<&RuntimePart> {
        self.parts.iter().find(|p| p.name == name)
    }

    pub fn total_vertices(&self) -> usize {
        self.parts.iter().map(|p| p.vertices.len()).sum()
    }

    pub fn total_indices(&self) -> usize {
        self.parts.iter().map(|p| p.indices.len()).sum()
    }

    /// Assemble matching parts into a unified vertex, index, and submesh buffer for GPU staging.
    pub fn assemble_parts<'a, I>(
        &self,
        parts: I,
    ) -> (Vec<RuntimeVertex>, Vec<u32>, Vec<RuntimeSubmesh>)
    where
        I: IntoIterator<Item = &'a RuntimePart>,
    {
        let mut total_verts = Vec::new();
        let mut total_indices = Vec::new();
        let mut total_submeshes = Vec::new();

        for part in parts {
            let v_offset = total_verts.len() as u32;
            let i_offset = total_indices.len() as u32;

            total_verts.extend_from_slice(&part.vertices);

            for &idx in &part.indices {
                total_indices.push(idx as u32 + v_offset);
            }

            for sm in &part.submeshes {
                let mut shifted = *sm;
                shifted.index_offset += i_offset;
                total_submeshes.push(shifted);
            }
        }

        (total_verts, total_indices, total_submeshes)
    }

    /// Assemble all parts matching a specific LOD level (0 for LOD A, 1 for LOD B, etc.).
    pub fn assemble_lod(
        &self,
        lod_level: u16,
    ) -> (Vec<RuntimeVertex>, Vec<u32>, Vec<RuntimeSubmesh>) {
        self.assemble_parts(self.parts.iter().filter(|p| p.lod_level == lod_level))
    }

    /// Assemble the primary stock vehicle exterior (base + stock kit 00 LOD A parts + 4 positioned wheels & brakes).
    pub fn assemble_stock_car(&self) -> (Vec<RuntimeVertex>, Vec<u32>, Vec<RuntimeSubmesh>) {
        let stock_parts = self.parts.iter().filter(|p| {
            let name = &p.name;
            let is_lod_a = name.ends_with("_A");
            let is_stock =
                name.ends_with("_BASE_A") || name.contains("_KIT00_") || name.starts_with("KIT00_");
            let is_excluded = name.contains("DECAL")
                || name.contains("_UNDER_")
                || name.contains("_UNLIT_")
                || name.contains("TRUNK_AUDIO")
                || name.contains("_WHEEL_")
                || name.contains("_BRAKE_");
            is_lod_a && is_stock && !is_excluded
        });

        let (mut v, mut i, mut s) = self.assemble_parts(stock_parts);
        if v.is_empty() {
            return self.assemble_lod(0);
        }

        // Instance the stock wheel and brake rotors at the 4 wheel well mount points (FL, FR, RL, RR).
        // In local wheel space, y=0.0 is the outer alloy rim face (pointing -Y) and y=+0.184 is the inner barrel.
        const FRONT_AXLE_X: f32 = 1.373;
        const REAR_AXLE_X: f32 = -1.293;
        const TRACK_OUTER_Y: f32 = 0.850;
        const AXLE_HEIGHT_Z: f32 = 0.185;
        const WHEEL_SCALE: f32 = 0.90;

        if let Some(wheel) = self
            .parts
            .iter()
            .find(|p| p.name.ends_with("KIT00_FRONT_WHEEL_A"))
        {
            Self::append_transformed_part(
                &mut v,
                &mut i,
                &mut s,
                wheel,
                [FRONT_AXLE_X, TRACK_OUTER_Y, AXLE_HEIGHT_Z],
                WHEEL_SCALE,
                true,
            );
            Self::append_transformed_part(
                &mut v,
                &mut i,
                &mut s,
                wheel,
                [FRONT_AXLE_X, -TRACK_OUTER_Y, AXLE_HEIGHT_Z],
                WHEEL_SCALE,
                false,
            );
            Self::append_transformed_part(
                &mut v,
                &mut i,
                &mut s,
                wheel,
                [REAR_AXLE_X, TRACK_OUTER_Y, AXLE_HEIGHT_Z],
                WHEEL_SCALE,
                true,
            );
            Self::append_transformed_part(
                &mut v,
                &mut i,
                &mut s,
                wheel,
                [REAR_AXLE_X, -TRACK_OUTER_Y, AXLE_HEIGHT_Z],
                WHEEL_SCALE,
                false,
            );
        }

        if let Some(fbrake) = self
            .parts
            .iter()
            .find(|p| p.name.ends_with("KIT00_FRONT_BRAKE_A"))
        {
            Self::append_transformed_part(
                &mut v,
                &mut i,
                &mut s,
                fbrake,
                [FRONT_AXLE_X, TRACK_OUTER_Y - 0.06, AXLE_HEIGHT_Z],
                WHEEL_SCALE,
                true,
            );
            Self::append_transformed_part(
                &mut v,
                &mut i,
                &mut s,
                fbrake,
                [FRONT_AXLE_X, -TRACK_OUTER_Y + 0.06, AXLE_HEIGHT_Z],
                WHEEL_SCALE,
                false,
            );
        }

        if let Some(rbrake) = self
            .parts
            .iter()
            .find(|p| p.name.ends_with("KIT00_REAR_BRAKE_A"))
        {
            Self::append_transformed_part(
                &mut v,
                &mut i,
                &mut s,
                rbrake,
                [REAR_AXLE_X, TRACK_OUTER_Y - 0.06, AXLE_HEIGHT_Z],
                WHEEL_SCALE,
                true,
            );
            Self::append_transformed_part(
                &mut v,
                &mut i,
                &mut s,
                rbrake,
                [REAR_AXLE_X, -TRACK_OUTER_Y + 0.06, AXLE_HEIGHT_Z],
                WHEEL_SCALE,
                false,
            );
        }

        (v, i, s)
    }

    fn append_transformed_part(
        total_verts: &mut Vec<RuntimeVertex>,
        total_indices: &mut Vec<u32>,
        total_submeshes: &mut Vec<RuntimeSubmesh>,
        part: &RuntimePart,
        offset: [f32; 3],
        scale: f32,
        mirror_y: bool,
    ) {
        let v_offset = total_verts.len() as u32;
        let i_offset = total_indices.len() as u32;
        let y_sign = if mirror_y { -1.0 } else { 1.0 };

        for vert in &part.vertices {
            let mut tv = *vert;
            tv.position[0] = tv.position[0] * scale + offset[0];
            tv.position[1] = tv.position[1] * scale * y_sign + offset[1];
            tv.position[2] = tv.position[2] * scale + offset[2];
            tv.normal[1] *= y_sign;
            total_verts.push(tv);
        }

        if mirror_y {
            for tri in part.indices.chunks_exact(3) {
                total_indices.push(tri[0] as u32 + v_offset);
                total_indices.push(tri[2] as u32 + v_offset);
                total_indices.push(tri[1] as u32 + v_offset);
            }
        } else {
            for &idx in &part.indices {
                total_indices.push(idx as u32 + v_offset);
            }
        }

        for sm in &part.submeshes {
            let mut shifted = *sm;
            shifted.index_offset += i_offset;
            total_submeshes.push(shifted);
        }
    }
}
