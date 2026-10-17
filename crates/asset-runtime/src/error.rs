//! Error types for runtime asset deserialization and loading.

use thiserror::Error;

/// Errors arising during `.onfs-model` deserialization or loading.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeAssetError {
    #[error("Invalid magic: expected b\"ONFM\", found {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("Unsupported runtime model format version: expected {expected}, found {found}")]
    UnsupportedVersion { expected: u32, found: u32 },

    #[error(
        "Unexpected EOF: required {expected} bytes at offset {offset:#x}, available {available}"
    )]
    UnexpectedEof {
        offset: usize,
        expected: usize,
        available: usize,
    },

    #[error("Payload corrupted: index {index} out of bounds for vertex count {vertex_count}")]
    IndexOutOfBounds { index: u16, vertex_count: u32 },

    #[error("Part name decode error: {0}")]
    InvalidPartName(String),

    #[error("IO error loading asset from {path}: {message}")]
    IoError { path: String, message: String },

    #[error("Integer overflow detected")]
    IntegerOverflow,
}
