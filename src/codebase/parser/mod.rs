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

    #[test]
    fn test_dart_symbol_extraction() {
        let content = r#"
import 'package:flutter/material.dart';

class MyWidget extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Container();
  }

  void _handleTap() {}
}

void main() {
  runApp(MyApp());
}

enum AppState {
  loading,
  ready,
  error,
}

mixin LoggingMixin {
  void log(String message) {
    print(message);
  }
}

extension StringExt on String {
  String capitalize() {
    return '${this[0].toUpperCase()}${substring(1)}';
  }
}
"#;
        let path = PathBuf::from("test.dart");
        let (symbols, refs) = CodeParser::parse_file(&path, content, "test");

        println!("=== DART SYMBOLS ===");
        for s in &symbols {
            println!(
                "  {} ({:?}) at lines {}-{}",
                s.name, s.symbol_type, s.start_line, s.end_line
            );
        }

        println!("\n=== DART REFERENCES ===");
        for r in &refs {
            println!(
                "  {} -> {} ({:?}) at line {}",
                r.from_symbol, r.to_symbol, r.relation_type, r.line
            );
        }

        assert!(
            symbols.len() >= 5,
            "Expected at least 5 symbols, got {}. Names: {:?}",
            symbols.len(),
            symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );

        assert!(
            symbols.iter().any(|s| s.name == "MyWidget"),
            "Should find class MyWidget"
        );
        assert!(
            symbols.iter().any(|s| s.name == "main"),
            "Should find function main"
        );
        assert!(
            symbols.iter().any(|s| s.name == "AppState"),
            "Should find enum AppState"
        );
    }

    #[test]
    fn test_dart_ast_dump() {
        use tree_sitter::Parser;

        let mut parser = Parser::new();
        let lang: tree_sitter::Language = tree_sitter_dart_orchard::LANGUAGE.into();
        parser.set_language(&lang).unwrap();

        let code = r#"
import 'package:flutter/material.dart';

class MyService {
  final ApiClient client;

  void doWork() {
    print("hello");
    someFunction(42);
    client.fetchData(url);
    widget?.build(context);
    Navigator.of(context).push(route);
    list..add(1)..add(2);
    setState(() {});
    Future.delayed(Duration(seconds: 1));
  }
}

void topLevelFunction() {
  final result = compute(42);
}
"#;

        let tree = parser.parse(code, None).unwrap();
        dump_node(tree.root_node(), code, 0);
    }

    #[test]
    fn test_dart_reference_extraction() {
        let content = r#"
import 'package:flutter/material.dart';

class MyWidget extends StatelessWidget {
  final ApiClient client;

  Widget build(BuildContext context) {
    print("hello");
    client.fetchData("url");
    widget?.rebuild(context);
    setState(() {});
    list..add(1)..add(2);
    return Container();
  }
}

void main() {
  runApp(MyApp());
}
"#;
        let path = PathBuf::from("test.dart");
        let (symbols, refs) = CodeParser::parse_file(&path, content, "test");

        println!("=== DART SYMBOLS ===");
        for s in &symbols {
            println!(
                "  {} ({:?}) L{}-{}",
                s.name, s.symbol_type, s.start_line, s.end_line
            );
        }

        println!("\n=== DART REFERENCES ===");
        for r in &refs {
            println!(
                "  {} -> {} ({:?}) L{}",
                r.from_symbol, r.to_symbol, r.relation_type, r.line
            );
        }

        let calls: Vec<_> = refs
            .iter()
            .filter(|r| matches!(r.relation_type, CodeRelationType::Calls))
            .collect();

        let imports: Vec<_> = refs
            .iter()
            .filter(|r| matches!(r.relation_type, CodeRelationType::Imports))
            .collect();

        println!("\n=== CALLS ({}) ===", calls.len());
        for c in &calls {
            println!("  {} -> {}", c.from_symbol, c.to_symbol);
        }
        println!("\n=== IMPORTS ({}) ===", imports.len());
        for i in &imports {
            println!("  {}", i.to_symbol);
        }

        // Import works
        assert!(!imports.is_empty(), "Should find at least 1 import");

        // Function calls found
        assert!(
            calls.len() >= 2,
            "Should find at least 2 calls, got {}. All refs: {:?}",
            calls.len(),
            refs.iter()
                .map(|r| (&r.from_symbol, &r.to_symbol, &r.relation_type))
                .collect::<Vec<_>>()
        );

        // Specific calls
        assert!(
            calls.iter().any(|c| c.to_symbol == "print"),
            "Should find call to 'print'"
        );
        assert!(
            calls.iter().any(|c| c.to_symbol == "runApp"),
            "Should find call to 'runApp'"
        );
    }

    #[test]
    fn test_dart_real_project_references() {
        // Test on real Dart file from mobile-odoo project
        let content = r#"
import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:sentry_flutter/sentry_flutter.dart';

class ErrorHandler {
  const ErrorHandler({GlobalErrorHandler? sentryHandler, GlobalErrorHandler? customLogger})
      : _sentryHandler = sentryHandler, _customLogger = customLogger;
  final GlobalErrorHandler? _sentryHandler;
  final GlobalErrorHandler? _customLogger;

  Future<void> handle(Object error, StackTrace stackTrace) async {
    await _customLogger?.call(error, stackTrace);
    await _sentryHandler?.call(error, stackTrace);
  }
}

Future<void> bootstrap({required Widget child}) async {
  FlutterError.onError = (FlutterErrorDetails details) {
    FlutterError.presentError(details);
    errorHandler.handle(details.exception, details.stack ?? StackTrace.empty);
  };

  await runZonedGuarded(
    () async {
      await _initializeWithMonitoring(dsn: dsn, child: child);
    },
    (error, stackTrace) async {
      await errorHandler.handle(error, stackTrace);
    },
  );
}

Future<void> _initializeWithMonitoring({required String dsn, required Widget child}) async {
  if (dsn.isNotEmpty) {
    await SentryFlutter.init((options) {
      options.dsn = dsn;
    }, appRunner: () => _initPostHogAndRun(child));
  }
}

void _initPostHogAndRun(Widget child) {
  final config = PostHogConfig(posthogKey);
  config.host = 'https://example.com';
  Posthog().setup(config);
  runApp(ProviderScope(overrides: [], child: child));
}
"#;
        let path = PathBuf::from("lib/app/bootstrap.dart");
        let (symbols, refs) = CodeParser::parse_file(&path, content, "test");

        println!("=== REAL PROJECT: SYMBOLS ({}) ===", symbols.len());
        for s in &symbols {
            println!(
                "  {} ({:?}) L{}-{}",
                s.name, s.symbol_type, s.start_line, s.end_line
            );
        }

        println!("\n=== REAL PROJECT: ALL REFERENCES ({}) ===", refs.len());
        for r in &refs {
            println!(
                "  {} -> {} ({:?}) L{}",
                r.from_symbol, r.to_symbol, r.relation_type, r.line
            );
        }

        let calls: Vec<_> = refs
            .iter()
            .filter(|r| matches!(r.relation_type, CodeRelationType::Calls))
            .collect();
        let imports: Vec<_> = refs
            .iter()
            .filter(|r| matches!(r.relation_type, CodeRelationType::Imports))
            .collect();

        println!("\n=== CALLS ({}) ===", calls.len());
        for c in &calls {
            println!("  {} -> {} (L{})", c.from_symbol, c.to_symbol, c.line);
        }
        println!("\n=== IMPORTS ({}) ===", imports.len());
        for i in &imports {
            println!("  {} (L{})", i.to_symbol, i.line);
        }

        // Imports
        assert!(
            imports.len() >= 3,
            "Should find at least 3 imports, got {}",
            imports.len()
        );

        // At least some calls should be found
        assert!(
            calls.len() >= 3,
            "Should find at least 3 calls, got {}",
            calls.len()
        );

        // Specific expected calls
        assert!(
            calls.iter().any(|c| c.to_symbol == "handle"),
            "Should find 'handle' method call"
        );
        assert!(
            calls.iter().any(|c| c.to_symbol == "runApp"),
            "Should find 'runApp' call"
        );
    }

    fn dump_node(node: tree_sitter::Node, source: &str, indent: usize) {
        if !node.is_named() {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                dump_node(child, source, indent);
            }
            return;
        }
        let kind = node.kind();
        if kind == "comment" || kind == "documentation_comment" {
            return;
        }

        let text = node.utf8_text(source.as_bytes()).unwrap_or("???");
        let short = if text.len() > 60 {
            format!("{}...", &text[..60])
        } else {
            text.to_string()
        };
        let short = short.replace('\n', "\\n");

        println!(
            "{}{} [L{}] {:?}",
            "  ".repeat(indent),
            kind,
            node.start_position().row + 1,
            short
        );

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            dump_node(child, source, indent + 1);
        }
    }
}
