//! Rust bindings for the Tea Tree-sitter grammar.

use tree_sitter::Language;

extern "C" {
    fn tree_sitter_tea() -> Language;
}

/// Get the parsed [`Language`] for the Tea grammar.
pub fn language() -> Language {
    unsafe { tree_sitter_tea() }
}

/// The contents of the generated node types JSON.
pub const NODE_TYPES: &str = include_str!("../../../src/node-types.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_load_language() {
        let language = language();
        let version = language.version();
        assert!(version > 0);
    }
}
