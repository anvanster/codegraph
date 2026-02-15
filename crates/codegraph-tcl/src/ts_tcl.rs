//! Bindings to the vendored tree-sitter-tcl grammar

extern "C" {
    fn tree_sitter_tcl() -> *const std::ffi::c_void;
}

/// Get the tree-sitter Language for Tcl
pub fn language() -> tree_sitter::Language {
    unsafe { tree_sitter::Language::from_raw(tree_sitter_tcl() as *const _) }
}
