//! Core graph types and operations.
//!
//! This module defines the fundamental building blocks:
//! - [`Node`]: Graph nodes representing code entities
//! - [`Edge`]: Directed relationships between nodes
//! - [`CodeGraph`]: The main graph database interface

mod types;
mod property;
mod codegraph;
pub mod algorithms;

pub use types::{
    Node, Edge, NodeId, EdgeId, NodeType, EdgeType, Direction,
};
pub use property::{PropertyMap, PropertyValue};
pub use codegraph::CodeGraph;
