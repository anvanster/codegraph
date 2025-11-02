//! # codegraph
//!
//! A fast, reliable, and flexible graph database optimized for storing and querying code relationships.
//!
//! ## Core Principles
//!
//! - **Parser Agnostic**: Bring your own parser, we handle the graph
//! - **Performance First**: Sub-100ms queries for responsive tooling
//! - **Test-Driven**: Comprehensive test coverage ensures reliability
//! - **Zero Magic**: Explicit over implicit, always
//! - **Persistence Primary**: Durable storage with RocksDB
//!
//! ## Architecture
//!
//! codegraph is organized in layers:
//!
//! ```text
//! User Tools (parsers, analysis)
//!     ↓
//! Code Helpers (convenience API)
//!     ↓
//! Query Builder (fluent interface)
//!     ↓
//! Core Graph (nodes, edges, algorithms)
//!     ↓
//! Storage Backend (RocksDB, memory)
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use codegraph::{CodeGraph, helpers};
//! use std::path::Path;
//!
//! // Explicit graph creation with persistent storage
//! let mut graph = CodeGraph::open(Path::new("./my_project.graph")).unwrap();
//!
//! // Explicitly add a file to the graph using helper functions
//! let file_id = helpers::add_file(&mut graph, "main.rs", "rust").unwrap();
//!
//! // Users explicitly parse and add entities (no magic scanning)
//! // Parser integration is up to the user
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod error;
pub mod storage;
pub mod graph;
pub mod query;
pub mod helpers;
pub mod export;

// Re-export main types
pub use error::{GraphError, Result};
pub use graph::{CodeGraph, Node, Edge, NodeType, EdgeType, NodeId, EdgeId, Direction, PropertyMap, PropertyValue};
pub use storage::{StorageBackend, RocksDBBackend, MemoryBackend};
pub use query::QueryBuilder;
