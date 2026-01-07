# Code Graph Implementation Research

**Date:** 2026-01-07
**Status:** Active
**Goal:** Дослідити архітектуру для автоматичної побудови графових зв'язків коду в Memory MCP

## 1. Executive Summary

Memory MCP має інфраструктуру для Knowledge Graph (entities/relations), але **не використовує її для коду**. Tree-sitter є в залежностях, але **не інтегрований**. Потрібно:

1. Інтегрувати tree-sitter для AST парсингу
2. Екстрактити symbols (functions, classes) та їх зв'язки (calls, imports)
3. Зберігати як entities/relations в SurrealDB

## 2. Current State Analysis

### What Exists

| Component | Status | Details |
|-----------|--------|---------|
| tree-sitter | ✅ In Cargo.toml | `tree-sitter = "0.22"` but NOT used |
| PetGraph | ✅ Works | Used for Knowledge Graph |
| ChunkType enum | ✅ Exists | `Function, Class, Struct, Module, Impl, Other` |
| CodeChunk.name | ✅ Exists | Field for function/class name |
| Entity/Relation | ✅ Works | Full CRUD + graph traversal |

### What's Missing

| Component | Status | Details |
|-----------|--------|---------|
| AST parsing | ❌ Not used | chunker.rs splits by 100 lines |
| Symbol extraction | ❌ Missing | ChunkType always = Other |
| Call graph | ❌ Missing | No "A calls B" relationships |
| Code relations | ❌ Missing | No RELATE between code chunks |

### Current Flow

```
File → Scanner → Chunker (100 lines) → Embedding → Storage
                    ↓
              ChunkType::Other (always!)
              name: None (always!)
```

### Target Flow

```
File → Scanner → TreeSitter Parser → Symbol Extractor → Storage
                       ↓                    ↓
                 AST Nodes            Entities + Relations
                       ↓                    ↓
              Functions, Classes    "calls", "imports", "contains"
```

## 3. Architecture Decision

**Pattern: "Fact Database"** (NOT full LSP/compiler)

| Approach | Pros | Cons | Decision |
|----------|------|------|----------|
| Salsa (rust-analyzer) | Precise, incremental | In-memory only, complex | ❌ Too heavy |
| Stack Graphs (GitHub) | Precise name resolution | Complex DSL | ❌ Overkill |
| **Fact Database** | Simple, persistent, queryable | Less precise | ✅ **Selected** |

We extract facts (nodes/edges) during indexing → store in SurrealDB → query later.

## 4. Tree-sitter Query Patterns

### Source References

- **gossiphs** (github.com/williamfzc/gossiphs) - Apache-2.0, production rules
- **nvim-treesitter** (github.com/nvim-treesitter/nvim-treesitter/queries) - MIT, comprehensive

### Rust

```scheme
; Definitions (exports)
(function_item name: (identifier) @function.def)
(function_signature_item name: (identifier) @function.def)
(struct_item name: (type_identifier) @type.def)
(enum_item name: (type_identifier) @type.def)
(mod_item name: (identifier) @module.def)
(impl_item type: (type_identifier) @impl.def)

; References (calls/imports)
(call_expression function: (identifier) @function.call)
(call_expression function: (field_expression field: (field_identifier) @method.call))
(call_expression function: (scoped_identifier "::" name: (identifier) @function.call))
(use_declaration argument: (scoped_identifier name: (identifier) @import))
```

### Python

```scheme
; Definitions
(function_definition name: (identifier) @function.def)
(class_definition name: (identifier) @class.def)

; References
(call function: (identifier) @function.call)
(call function: (attribute attribute: (identifier) @method.call))
(import_statement name: (dotted_name (identifier) @import))
(import_from_statement name: (dotted_name (identifier) @import))
```

### TypeScript/JavaScript

```scheme
; Definitions
(function_declaration name: (identifier) @function.def)
(class_declaration name: (identifier) @class.def)
(method_definition name: (property_identifier) @method.def)
(arrow_function) @arrow.def
(export_statement (function_declaration name: (identifier) @export.def))

; References
(call_expression function: (identifier) @function.call)
(call_expression function: (member_expression property: (property_identifier) @method.call))
(import_statement) @import
```

### Go

```scheme
; Definitions
(function_declaration name: (identifier) @function.def)
(method_declaration name: (field_identifier) @method.def)
(type_declaration (type_spec name: (type_identifier) @type.def))

; References
(call_expression function: (identifier) @function.call)
(call_expression function: (selector_expression field: (field_identifier) @method.call))
(import_spec name: (package_identifier) @import)
```

### Java

```scheme
; Definitions
(class_declaration name: (identifier) @class.def)
(method_declaration name: (identifier) @method.def)
(interface_declaration name: (identifier) @interface.def)

; References
(method_invocation name: (identifier) @method.call)
(scoped_identifier (identifier) @import)
```

## 5. Required Changes

### 5.1 Cargo.toml

```toml
# Add language crates
tree-sitter-rust = "0.24"
tree-sitter-python = "0.25"
tree-sitter-typescript = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-go = "0.23"
tree-sitter-java = "0.23"
tree-sitter-language = "0.1"  # version compatibility
```

### 5.2 New Types (src/types/code.rs)

```rust
/// A symbol extracted from code (function, class, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub id: Option<Thing>,
    pub name: String,
    pub symbol_type: SymbolType,  // Function, Class, Method, etc.
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub project_id: String,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Module,
    Interface,
    Import,
}

/// A relation between code symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRelation {
    pub from_symbol: Thing,
    pub to_symbol: Thing,
    pub relation_type: CodeRelationType,
    pub file_path: String,  // where the relation was found
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeRelationType {
    Calls,      // function A calls function B
    Imports,    // file A imports from file B
    Contains,   // class A contains method B
    Implements, // struct A implements trait B
    Extends,    // class A extends class B
}
```

### 5.3 New Module (src/codebase/parser.rs)

```rust
use tree_sitter::{Parser, Query, QueryCursor, Language};

pub struct CodeParser {
    parsers: HashMap<Language, Parser>,
    queries: HashMap<String, LanguageQueries>,
}

struct LanguageQueries {
    definitions: Query,
    references: Query,
}

impl CodeParser {
    pub fn new() -> Self { ... }
    
    pub fn parse_file(&self, path: &Path, content: &str) 
        -> Result<(Vec<CodeSymbol>, Vec<CodeRelation>)> 
    {
        let language = detect_language(path);
        let tree = self.parse(language, content)?;
        
        let symbols = self.extract_definitions(&tree, language)?;
        let relations = self.extract_references(&tree, language, &symbols)?;
        
        Ok((symbols, relations))
    }
}
```

### 5.4 Modified Indexer (src/codebase/indexer.rs)

```rust
pub async fn index_project(state: Arc<AppState>, project_path: &Path) -> Result<IndexStatus> {
    let parser = CodeParser::new();
    
    for file_path in files {
        let content = fs::read_to_string(&file_path).await?;
        
        // NEW: Parse with tree-sitter
        let (symbols, relations) = parser.parse_file(&file_path, &content)?;
        
        // Store symbols as entities
        for symbol in symbols {
            let entity = Entity::from_symbol(symbol);
            state.storage.create_entity(entity).await?;
        }
        
        // Store relations
        for relation in relations {
            state.storage.create_code_relation(relation).await?;
        }
        
        // Also create chunks for semantic search (existing logic)
        let chunks = chunk_file(&file_path, &content, &project_id);
        // ...
    }
}
```

### 5.5 Storage Trait Extension (src/storage/traits.rs)

```rust
// Add to StorageBackend trait:

/// Create a code symbol, returns the generated ID
async fn create_code_symbol(&self, symbol: CodeSymbol) -> Result<String>;

/// Create a relation between code symbols
async fn create_code_relation(&self, relation: CodeRelation) -> Result<String>;

/// Find all symbols that call a given symbol
async fn get_callers(&self, symbol_id: &str) -> Result<Vec<CodeSymbol>>;

/// Find all symbols called by a given symbol
async fn get_callees(&self, symbol_id: &str) -> Result<Vec<CodeSymbol>>;

/// Find symbols by name pattern
async fn search_symbols(&self, query: &str, project_id: Option<&str>) -> Result<Vec<CodeSymbol>>;
```

### 5.6 Schema Extension (src/storage/schema.surql)

```sql
-- Code symbols table
DEFINE TABLE IF NOT EXISTS code_symbols SCHEMAFULL;
DEFINE FIELD name           ON code_symbols TYPE string;
DEFINE FIELD symbol_type    ON code_symbols TYPE string;
DEFINE FIELD file_path      ON code_symbols TYPE string;
DEFINE FIELD start_line     ON code_symbols TYPE int;
DEFINE FIELD end_line       ON code_symbols TYPE int;
DEFINE FIELD project_id     ON code_symbols TYPE string;
DEFINE FIELD embedding      ON code_symbols TYPE option<array<float>>;
DEFINE FIELD indexed_at     ON code_symbols TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_symbols_name ON code_symbols FIELDS name;
DEFINE INDEX IF NOT EXISTS idx_symbols_project ON code_symbols FIELDS project_id;
DEFINE INDEX IF NOT EXISTS idx_symbols_path ON code_symbols FIELDS file_path;
DEFINE INDEX IF NOT EXISTS idx_symbols_vec ON code_symbols 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;

-- Code relations are created via RELATE statements
-- Example: RELATE code_symbols:fn_a -> calls -> code_symbols:fn_b
```

## 6. Implementation Plan

### Phase 1: Foundation (MVP)
1. Add tree-sitter language crates to Cargo.toml
2. Create `CodeSymbol` and `CodeRelation` types
3. Implement basic `CodeParser` for Rust only
4. Extend storage trait and SurrealDB implementation
5. Modify indexer to use new parser

### Phase 2: Multi-language
1. Add query patterns for Python, TypeScript, Go, Java
2. Create language detection based on file extension
3. Add comprehensive tests

### Phase 3: Graph Queries
1. Add MCP tools: `get_callers`, `get_callees`, `get_imports`
2. Integrate with `recall` for hybrid search
3. Add graph visualization export

### Phase 4: Optimization
1. Incremental parsing (pass old_tree)
2. Parallel file processing
3. Caching of parsed trees

## 7. Reference Implementations

| Project | URL | License | Notes |
|---------|-----|---------|-------|
| gossiphs | github.com/williamfzc/gossiphs | Apache-2.0 | Rule struct, multi-lang |
| nvim-treesitter | github.com/nvim-treesitter/nvim-treesitter | MIT | Comprehensive queries |
| Sourcegraph | github.com/sourcegraph/sourcegraph | Enterprise | Production scale |
| stack-graphs | github.com/github/stack-graphs | MIT | Precise resolution |

## 8. Open Questions

- [ ] How to handle cross-file name resolution without full semantic analysis?
- [ ] Should we store call relations with line numbers or just symbol-to-symbol?
- [ ] How to handle dynamic languages (Python) where calls can't be statically resolved?
- [ ] Should incremental updates delete old relations or use temporal validity?

## 9. Risks

| Risk | Mitigation |
|------|------------|
| Tree-sitter version conflicts | Use `tree-sitter-language` crate |
| Performance on large codebases | Batch processing, incremental parsing |
| Incomplete call graph (dynamic code) | Accept imprecision, document limitations |
| Schema migration for existing data | Add new tables, don't modify existing |

---

*Last updated: 2026-01-07*
