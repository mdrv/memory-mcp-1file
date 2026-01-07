# Tree-sitter Usage in Rust

Research into using `tree-sitter` for code analysis, parsing, and AST querying within a Rust application.

## 1. Setup and Installation

To use tree-sitter in Rust, you need the core library and the specific language grammars you want to parse.

### Cargo.toml
```toml
[dependencies]
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
tree-sitter-python = "0.20"
tree-sitter-typescript = "0.20"
tree-sitter-javascript = "0.20"
```

> **Note**: Ensure the versions of language crates are compatible with the core `tree-sitter` crate version.

## 2. Basic Parsing Workflow

The standard workflow involves creating a `Parser`, setting its language, and parsing a string into a `Tree`.

```rust
use tree_sitter::{Parser, Language};

fn main() {
    // 1. Create a parser
    let mut parser = Parser::new();

    // 2. Set the language (e.g., Rust)
    let language = tree_sitter_rust::language();
    parser.set_language(language).expect("Error loading Rust grammar");

    // 3. Parse source code
    let source_code = "fn main() { println!(\"Hello\"); }";
    let tree = parser.parse(source_code, None).unwrap();

    // 4. Inspect the tree
    let root_node = tree.root_node();
    println!("{}", root_node.to_sexp()); 
    // Output: (source_file (function_item name: (identifier) parameters: (parameters) body: (block ...)))
}
```

## 3. Querying the AST

Tree-sitter provides a Lisp-like query language (S-expressions) to match patterns in the AST.

### Rust Example: Extracting Functions

```rust
use tree_sitter::{Parser, Query, QueryCursor};

fn extract_functions(source_code: &str) {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::language();
    parser.set_language(language).unwrap();
    let tree = parser.parse(source_code, None).unwrap();

    // Define query
    let query_str = "(function_item name: (identifier) @func_name)";
    let query = Query::new(language, query_str).unwrap();
    
    // Execute query
    let mut query_cursor = QueryCursor::new();
    let matches = query_cursor.matches(&query, tree.root_node(), source_code.as_bytes());

    for m in matches {
        for capture in m.captures {
            let node = capture.node;
            let func_name = node.utf8_text(source_code.as_bytes()).unwrap();
            println!("Found function: {}", func_name);
        }
    }
}
```

## 4. Common Query Patterns

Different languages use different node names. Here are the common patterns for extraction:

### Rust
| Concept | Query Pattern | Node Name |
|---------|---------------|-----------|
| Function Definition | `(function_item) @func` | `function_item` |
| Function Call | `(call_expression) @call` | `call_expression` |
| Import | `(use_declaration) @import` | `use_declaration` |
| Struct | `(struct_item) @struct` | `struct_item` |

### Python
| Concept | Query Pattern | Node Name |
|---------|---------------|-----------|
| Function Definition | `(function_definition) @func` | `function_definition` |
| Function Call | `(call) @call` | `call` |
| Import | `(import_statement) @import`<br>`(import_from_statement) @import` | `import_statement`<br>`import_from_statement` |
| Class | `(class_definition) @class` | `class_definition` |

### TypeScript / JavaScript
| Concept | Query Pattern | Node Name |
|---------|---------------|-----------|
| Function Definition | `(function_declaration) @func`<br>`(method_definition) @method`<br>`(arrow_function) @arrow` | `function_declaration`<br>`method_definition`<br>`arrow_function` |
| Function Call | `(call_expression) @call` | `call_expression` |
| Import | `(import_statement) @import` | `import_statement` |
| Class | `(class_declaration) @class` | `class_declaration` |

## 5. Finding Node Names

If you are unsure of a node name for a specific language:
1. **Parse a snippet** and print the S-expression:
   ```rust
   println!("{}", root_node.to_sexp());
   ```
2. **Check `nvim-treesitter` queries**: The [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter/tree/master/runtime/queries) repository contains comprehensive query files (`locals.scm`, `highlights.scm`) for almost all supported languages.

## 6. Available Grammars

Popular Rust crates for tree-sitter grammars:
- [`tree-sitter-rust`](https://crates.io/crates/tree-sitter-rust)
- [`tree-sitter-python`](https://crates.io/crates/tree-sitter-python)
- [`tree-sitter-javascript`](https://crates.io/crates/tree-sitter-javascript)
- [`tree-sitter-typescript`](https://crates.io/crates/tree-sitter-typescript)
- [`tree-sitter-go`](https://crates.io/crates/tree-sitter-go)
- [`tree-sitter-java`](https://crates.io/crates/tree-sitter-java)
- [`tree-sitter-c`](https://crates.io/crates/tree-sitter-c)
- [`tree-sitter-cpp`](https://crates.io/crates/tree-sitter-cpp)
- [`tree-sitter-json`](https://crates.io/crates/tree-sitter-json)
- [`tree-sitter-bash`](https://crates.io/crates/tree-sitter-bash)

## 7. Best Practices

1. **Reuse Parsers**: Creating a `Parser` is cheap, but reloading the language *might* have overhead. Reusing the parser instance is generally good.
2. **Incremental Parsing**: If editing files, pass `Some(&old_tree)` to `parser.parse()` for significant performance gains.
3. **Byte Handling**: Always pass `source_code.as_bytes()` to query cursors and text extraction methods to ensure correct UTF-8 handling.
4. **Error Handling**: `parser.parse()` returns `Option<Tree>`. It handles syntax errors gracefully (producing error nodes) rather than panicking.
