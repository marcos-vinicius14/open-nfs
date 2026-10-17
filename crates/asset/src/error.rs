//! Strongly-typed, panic-free error definitions for legacy binary asset parsing.

use open_nfs_core::types::ChunkId;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChunkError {
    #[error("Unexpected EOF at offset {offset:#x}: required {expected} bytes, but only {available} available")]
    UnexpectedEof {
        offset: usize,
        expected: usize,
        available: usize,
    },

    #[error("Chunk size {size} exceeds remaining container buffer {remaining}")]
    InvalidChunkSize { size: usize, remaining: usize },

    #[error("Integer overflow detected during {context}")]
    IntegerOverflow { context: &'static str },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GeometryError {
    #[error("Chunk container error: {0}")]
    Chunk(#[from] ChunkError),

    #[error("Missing mandatory chunk in hierarchy: {0:?}")]
    MissingMandatoryChunk(ChunkId),

    #[error("Invalid vertex payload: length {payload_bytes} bytes is not a multiple of 36")]
    InvalidVertexStride { payload_bytes: usize },

    #[error("Triangle index out of bounds: index {index} >= vertex count {vertex_count}")]
    IndexOutOfBounds { index: u16, vertex_count: u32 },

    #[error("Failed to decode ASCII part name: {0}")]
    InvalidPartName(&'static str),
}

/// Errors arising during texture archive and JDLZ decompression.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TextureError {
    #[error("Chunk container error: {0}")]
    Chunk(#[from] ChunkError),

    #[error("Invalid JDLZ magic: expected b\"JDLZ\", found {0:?}")]
    InvalidJdlzMagic([u8; 4]),

    #[error("Unsupported JDLZ compression version: expected 2, found {0}")]
    UnsupportedJdlzVersion(u8),

    #[error(
        "JDLZ output buffer overflow: decompressed bytes {actual} exceeded header limit {expected}"
    )]
    DecompressionOverflow { expected: usize, actual: usize },

    #[error("JDLZ back-reference out of bounds: offset {offset} exceeds current decoded buffer length {current_len}")]
    InvalidBackReference { offset: usize, current_len: usize },

    #[error("Unexpected EOF in texture data stream at offset {offset:#x}: needed {expected}, got {available}")]
    UnexpectedEof {
        offset: usize,
        expected: usize,
        available: usize,
    },
}
