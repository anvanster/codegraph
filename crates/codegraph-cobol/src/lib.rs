//! COBOL parser for CodeGraph
//!
//! This crate provides COBOL language support for the CodeGraph code analysis tool.
//! It parses standard COBOL constructs (programs, paragraphs, sections, COPY statements,
//! and CALL statements) using the vendored tree-sitter-cobol grammar.
//!
//! Supported file extensions: `.cob`, `.cbl`, `.cobol`, `.cpy`

mod extractor;
mod mapper;
mod parser_impl;
mod ts_cobol;
mod visitor;

pub use parser_impl::CobolParser;
