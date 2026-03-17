//! Bridge to the tree-sitter-verilog grammar via extern C binding.
//!
//! The tree-sitter-verilog crate v1.0 uses the newer tree-sitter-language API
//! which is incompatible with tree-sitter 0.22. We call the underlying C
//! symbol directly and wrap it with Language::from_raw.

extern "C" {
    fn tree_sitter_verilog() -> *const std::ffi::c_void;
}

/// Get the tree-sitter Language for Verilog
pub fn language() -> tree_sitter::Language {
    unsafe { tree_sitter::Language::from_raw(tree_sitter_verilog() as *const _) }
}
