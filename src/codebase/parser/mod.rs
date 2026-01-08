pub mod extractor;
pub mod languages;

use std::path::Path;

use crate::codebase::scanner::detect_language;
use crate::types::symbol::{CodeReference, CodeSymbol};

use extractor::Extractor;

pub struct CodeParser;

impl CodeParser {
    pub fn parse_file(
        path: &Path,
        content: &str,
        project_id: &str,
    ) -> (Vec<CodeSymbol>, Vec<CodeReference>) {
        let language = detect_language(path);
        let Some(mut extractor) = Extractor::new(language) else {
            return (vec![], vec![]);
        };

        extractor.parse(content, path.to_string_lossy().as_ref(), project_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::symbol::CodeRelationType;
    use std::path::PathBuf;

    #[test]
    fn test_parser_crash() {
        let content = "fn test() {}";
        let path = PathBuf::from("test.rs");
        let (symbols, _) = CodeParser::parse_file(&path, content, "test");
        assert!(!symbols.is_empty());
    }

    #[test]
    fn test_rust_call_extraction() {
        let content = r#"
fn main() {
    let x = foo();
    bar(x);
}

fn foo() -> i32 { 42 }
fn bar(x: i32) {}
"#;
        let path = PathBuf::from("test.rs");
        let (symbols, refs) = CodeParser::parse_file(&path, content, "test");

        println!("=== SYMBOLS ===");
        for s in &symbols {
            println!(
                "  {} ({:?}) at line {}",
                s.name, s.symbol_type, s.start_line
            );
        }

        println!("\n=== REFERENCES ===");
        for r in &refs {
            println!(
                "  {} -> {} ({:?}) at line {}",
                r.from_symbol, r.to_symbol, r.relation_type, r.line
            );
        }

        // Should have 3 functions: main, foo, bar
        assert_eq!(symbols.len(), 3, "Expected 3 symbols");

        // Should have calls: main->foo, main->bar
        let calls: Vec<_> = refs
            .iter()
            .filter(|r| matches!(r.relation_type, CodeRelationType::Calls))
            .collect();

        println!("\n=== CALLS ONLY ===");
        for c in &calls {
            println!("  {} -> {}", c.from_symbol, c.to_symbol);
        }

        assert!(
            calls.len() >= 2,
            "Expected at least 2 calls, got {}",
            calls.len()
        );
    }
}
