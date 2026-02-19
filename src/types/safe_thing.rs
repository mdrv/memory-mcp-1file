//! Safe SurrealDB Thing creation for symbol indexing.
//!
//! This module provides factory functions for creating SurrealDB Things
//! with validated, safe IDs that won't cause panics or SQL injection.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::{Thing, RecordId};

/// Creates a safe Thing for a code symbol using a deterministic hash.
///
/// The hash is computed from (project_id, file_path, name, line) to ensure:
/// 1. Deterministic: same input → same ID
/// 2. Safe: only hex characters, no special chars
/// 3. Unique: different symbols get different IDs
///
/// # Arguments
/// * `project_id` - The project identifier
/// * `file_path` - Path to the source file
/// * `name` - Symbol name (may contain `::`, special chars)
/// * `line` - Line number where symbol is defined
///
/// # Example
/// ```ignore
/// let thing = symbol_thing("myproject", "src/lib.rs", "std::io::Read", 42);
/// // Returns Thing { tb: "code_symbols", id: "a1b2c3d4e5f67890" }
/// ```
pub fn symbol_thing(project_id: &str, file_path: &str, name: &str, line: u32) -> Thing {
    let safe_id = symbol_hash(project_id, file_path, name, line);
    RecordId::new("code_symbols", safe_id)
}

/// Creates a safe Thing for a symbol relation endpoint.
///
/// Uses the same hashing as `symbol_thing` to ensure relations
/// can be created between symbols consistently.
pub fn symbol_relation_thing(project_id: &str, file_path: &str, name: &str, line: u32) -> Thing {
    symbol_thing(project_id, file_path, name, line)
}

/// Computes a deterministic hash for a symbol.
///
/// Returns a 16-character hex string that is safe for SurrealDB IDs.
pub fn symbol_hash(project_id: &str, file_path: &str, name: &str, line: u32) -> String {
    let mut hasher = DefaultHasher::new();
    project_id.hash(&mut hasher);
    file_path.hash(&mut hasher);
    name.hash(&mut hasher);
    line.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Creates a safe Thing for a code reference (caller/callee relationship).
///
/// For references, we hash based on the containing symbol's context
/// since references don't have their own unique identity.
pub fn reference_thing(
    project_id: &str,
    file_path: &str,
    from_symbol: &str,
    from_line: u32,
) -> Thing {
    symbol_thing(project_id, file_path, from_symbol, from_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_thing_with_colons() {
        // Should not panic with special characters
        let thing = symbol_thing("project", "file.rs", "std::io::Read", 42);
        assert_eq!(thing.table.as_str(), "code_symbols");
        // ID should be all hex characters
        assert!(crate::types::record_key_to_string(&thing.key).chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_symbol_thing_deterministic() {
        let thing1 = symbol_thing("p", "f.rs", "func", 10);
        let thing2 = symbol_thing("p", "f.rs", "func", 10);
        assert_eq!(crate::types::record_key_to_string(&thing1.key), crate::types::record_key_to_string(&thing2.key));
    }

    #[test]
    fn test_symbol_thing_different_for_different_inputs() {
        let thing1 = symbol_thing("p", "f.rs", "func1", 10);
        let thing2 = symbol_thing("p", "f.rs", "func2", 10);
        assert_ne!(crate::types::record_key_to_string(&thing1.key), crate::types::record_key_to_string(&thing2.key));
    }

    #[test]
    fn test_symbol_thing_different_lines() {
        let thing1 = symbol_thing("p", "f.rs", "func", 10);
        let thing2 = symbol_thing("p", "f.rs", "func", 20);
        assert_ne!(crate::types::record_key_to_string(&thing1.key), crate::types::record_key_to_string(&thing2.key));
    }

    #[test]
    fn test_symbol_hash_length() {
        let hash = symbol_hash("project", "file.rs", "symbol", 1);
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn test_special_characters_in_name() {
        // Various problematic characters that could cause issues
        let names = vec![
            "std::io::Read",
            "crate::module::func",
            "fn<T>",
            "impl Trait for Type",
            "async fn",
            "struct::method",
            "mod::inner::deep::func",
            "a'b",
            "a\"b",
            "a;b",
            "a/b",
            "a\\b",
        ];

        for name in names {
            let thing = symbol_thing("p", "f.rs", name, 1);
            // Should not panic and ID should be safe
            assert!(crate::types::record_key_to_string(&thing.key).chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_reference_thing() {
        let thing = reference_thing("project", "file.rs", "caller_func", 100);
        assert_eq!(thing.table.as_str(), "code_symbols");
    }
}
