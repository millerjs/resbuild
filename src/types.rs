//! Types
//!
//! This module is defines misc types used for enormalization
use serde_json::{Value, Map};

/// An alias of serde's Map, a Doc represents a denormalized JSON
/// document
pub type Doc = Map<String, Value>;
