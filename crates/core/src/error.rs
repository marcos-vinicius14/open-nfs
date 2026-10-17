//! Core domain error types.

use thiserror::Error;

/// Common foundational errors across the engine core.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Out of bounds buffer access: requested offset {offset} with length {length}, buffer size is {buffer_len}")]
    OutOfBounds {
        offset: usize,
        length: usize,
        buffer_len: usize,
    },

    #[error("Integer arithmetic overflow encountered in {operation}")]
    IntegerOverflow { operation: &'static str },

    #[error("Invalid invariant: {0}")]
    InvariantViolation(String),
}
