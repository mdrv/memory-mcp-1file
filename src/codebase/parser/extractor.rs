use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Parser, Query, QueryCursor};

use crate::types::symbol::{CodeReference, CodeSymbol};
use crate::types::Language;

use super::languages::{get_language_support, LanguageSupport};

pub struct Extractor {
    parser: Parser,
    language: Language,
    support: Box<dyn LanguageSupport>,
}

impl Extractor {
    pub fn new(language: Language) -> Option<Self> {
        let support = get_language_support(language.clone())?;
        let mut parser = Parser::new();
        parser
            .set_language(&support.get_language())
            .expect("Error loading grammar");

        Some(Self {
            parser,
            language,
            support,
        })
    }

    pub fn parse(
        &mut self,
        content: &str,
        file_path: &str,
        project_id: &str,
    ) -> (Vec<CodeSymbol>, Vec<CodeReference>) {
        let tree = match self.parser.parse(content, None) {
            Some(t) => t,
            None => return (vec![], vec![]),
        };

        let symbols = self.extract_symbols(&tree, content, file_path, project_id);
        let references = self.extract_references(&tree, content, file_path);

        (symbols, references)
    }

    fn extract_symbols(
        &self,
        tree: &tree_sitter::Tree,
        content: &str,
        file_path: &str,
        project_id: &str,
    ) -> Vec<CodeSymbol> {
        let query_source = self.support.get_definition_query();
        let query = match Query::new(&self.support.get_language(), query_source) {
            Ok(q) => q,
            Err(e) => {
                tracing::error!("Invalid definition query for {:?}: {}", self.language, e);
                return vec![];
            }
        };

        let mut query_cursor = QueryCursor::new();
        let mut matches = query_cursor.matches(&query, tree.root_node(), content.as_bytes());

        let mut symbols = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                let capture_name = query.capture_names()[capture.index as usize];

                if let Ok(name) = node.utf8_text(content.as_bytes()) {
                    let symbol_type = self.support.map_symbol_type(capture_name);
                    let start_line = node.start_position().row as u32 + 1;
                    let end_line = node.end_position().row as u32 + 1;

                    let signature: Option<&str> = node
                        .parent()
                        .and_then(|p: Node| p.utf8_text(content.as_bytes()).ok());

                    let mut symbol = CodeSymbol::new(
                        name.to_string(),
                        symbol_type,
                        file_path.to_string(),
                        start_line,
                        end_line,
                        project_id.to_string(),
                    );

                    if let Some(sig) = signature {
                        let truncated = sig
                            .lines()
                            .next()
                            .unwrap_or(sig)
                            .chars()
                            .take(200)
                            .collect::<String>();
                        symbol = symbol.with_signature(truncated);
                    }

                    symbols.push(symbol);
                }
            }
        }

        symbols
    }

    fn extract_references(
        &self,
        tree: &tree_sitter::Tree,
        content: &str,
        file_path: &str,
    ) -> Vec<CodeReference> {
        let query_source = self.support.get_reference_query();
        let query = match Query::new(&self.support.get_language(), query_source) {
            Ok(q) => q,
            Err(e) => {
                tracing::error!("Invalid reference query for {:?}: {}", self.language, e);
                return vec![];
            }
        };

        let mut query_cursor = QueryCursor::new();
        let mut matches = query_cursor.matches(&query, tree.root_node(), content.as_bytes());

        let mut references = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;

                if let Ok(name) = node.utf8_text(content.as_bytes()) {
                    let start_line = node.start_position().row as u32 + 1;
                    let column = node.start_position().column as u32;

                    references.push(CodeReference::new(
                        name.to_string(),
                        file_path.to_string(),
                        start_line,
                        column,
                    ));
                }
            }
        }

        references
    }
}
