//! Tcl/SDC/UPF parser for CodeGraph
//!
//! This crate provides Tcl language support for the CodeGraph code analysis tool.
//! It parses standard Tcl constructs (procedures, namespaces, imports) and
//! additionally classifies EDA/VLSI commands (SDC constraints, design file
//! reads/writes, Synopsys tool flow commands, OpenROAD patterns).
//!
//! Supported file extensions: `.tcl`, `.sdc`, `.upf`

mod eda;
mod extractor;
mod mapper;
mod parser_impl;
mod sdc;
mod ts_tcl;
mod visitor;

pub use parser_impl::TclParser;
