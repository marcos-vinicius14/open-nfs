//! Safe, bounded, panic-free parsers for legacy PC Need for Speed Underground 2 assets.

pub mod chunk;
pub mod error;
pub mod geometry;
pub mod synthetic;
pub mod texture;

pub use chunk::{ChunkIterator, RawChunk};
pub use error::{ChunkError, GeometryError, TextureError};
pub use geometry::{
    parse_geometry, ParsedCarPart, ParsedGeometryArchive, PartHeader, SubMeshSlice, Vertex36,
};
pub use texture::{decompress_jdlz, parse_texture_pack, TextureEntry, TextureFormat, TexturePack};
