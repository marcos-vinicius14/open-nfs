//! Runtime format (.onfs-model) data structures and safe parsers.

pub mod error;
pub mod format;
pub mod model;

pub use error::RuntimeAssetError;
pub use format::{deserialize_runtime_model, serialize_runtime_model, ONFM_MAGIC, ONFM_VERSION};
pub use model::{RuntimeBounds, RuntimeModel, RuntimePart, RuntimeSubmesh, RuntimeVertex};
