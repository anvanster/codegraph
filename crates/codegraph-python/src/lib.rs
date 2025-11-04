//! # codegraph-python
//!
//! Python parser plugin for CodeGraph - extracts code entities and relationships
//! from Python source files.
//!
//! ## Features
//!
//! - Parse single Python files or entire projects
//! - Extract functions, classes, methods with full metadata
//! - Track relationships (calls, imports, inheritance)
//! - Configurable behavior (visibility filtering, parallel processing)
//! - Safe: No panics, graceful error handling
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use codegraph_python::Parser;
//! # use codegraph_python::error::Result;
//!
//! # fn main() -> Result<()> {
//! // Create parser with default configuration
//! let parser = Parser::new();
//!
//! // Parse will be implemented in Phase 3
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod entities;
pub mod error;
pub mod relationships;

mod builder;
mod extractor;
mod parser;
// mod visitor; // Temporarily disabled - will be re-enabled in a future task

pub use config::ParserConfig;
pub use error::{ParseError, Result};
pub use parser::{FileInfo, Parser, ProjectInfo};
