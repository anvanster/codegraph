//! CodeGraph Parser API
//!
//! Shared trait and types for building CodeGraph language parsers.
//!
//! This crate provides the foundation for implementing language parsers that work
//! with the CodeGraph database. It defines:
//!
//! - **CodeParser trait**: The core interface all parsers must implement
//! - **Entity types**: Language-agnostic representations of code elements (functions, classes, etc.)
//! - **Relationship types**: Representations of code dependencies (calls, imports, etc.)
//! - **Configuration**: Customizable parser behavior
//! - **Metrics**: Performance and success tracking
//! - **Error handling**: Comprehensive error types
//!
//! # Example
//!
//! ```rust,ignore
//! use codegraph_parser_api::{CodeParser, ParserConfig, ParserError, FileInfo};
//! use codegraph::CodeGraph;
//! use std::path::Path;
//!
//! struct MyParser {
//!     config: ParserConfig,
//! }
//!
//! impl CodeParser for MyParser {
//!     fn language(&self) -> &str {
//!         "mylang"
//!     }
//!
//!     fn file_extensions(&self) -> &[&str] {
//!         &[".my"]
//!     }
//!
//!     fn parse_file(&self, path: &Path, graph: &mut CodeGraph) -> Result<FileInfo, ParserError> {
//!         // Implementation here
//!         todo!()
//!     }
//!
//!     fn parse_source(&self, source: &str, file_path: &Path, graph: &mut CodeGraph)
//!         -> Result<FileInfo, ParserError> {
//!         // Implementation here
//!         todo!()
//!     }
//!
//!     fn config(&self) -> &ParserConfig {
//!         &self.config
//!     }
//!
//!     fn metrics(&self) -> ParserMetrics {
//!         ParserMetrics::default()
//!     }
//!
//!     fn reset_metrics(&mut self) {
//!         // Reset internal metrics
//!     }
//! }
//! ```

pub mod config;
pub mod entities;
pub mod errors;
pub mod ir;
pub mod metrics;
pub mod relationships;
pub mod traits;

// Re-export commonly used types
pub use config::ParserConfig;
pub use entities::{ClassEntity, Field, FunctionEntity, ModuleEntity, Parameter, TraitEntity};
pub use errors::{ParserError, ParserResult};
pub use ir::CodeIR;
pub use metrics::ParserMetrics;
pub use relationships::{
    CallRelation, ImplementationRelation, ImportRelation, InheritanceRelation,
};
pub use traits::{CodeParser, FileInfo, ProjectInfo};
