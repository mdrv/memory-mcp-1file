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
    use std::path::PathBuf;

    #[test]
    fn test_parser_crash() {
        let content = "fn test() {}";
        let path = PathBuf::from("test.rs");
        let (symbols, _) = CodeParser::parse_file(&path, content, "test");
        assert!(!symbols.is_empty());
    }
}
