//! `open-nfs-core` — Foundation crate providing primitive types, invariant models,
//! and common traits for the Open-NFS engine.
//!
//! Standards conform to `AGENTS.md` and `docs/adr.md`.

#![deny(unsafe_code)]

pub mod error;
pub mod math;
pub mod types;

pub use error::CoreError;
pub use math::{RuntimeVertex, Vec3f};
pub use types::{ChunkId, FixedDelta, Rpm};
