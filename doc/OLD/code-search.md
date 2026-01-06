# Code Search — Design Document

## Overview

Semantic code search for Memory MCP Server. Features:
- Automatic codebase indexing
- Tree-sitter AST-based chunking with hierarchical context injection
- Integration with existing `recall` tool via Z-score normalization and RRF merge
- Git-based multi-project isolation via namespaces
- Real-time file watching for auto-reindexing

## Architecture

### High-Level Flow

```
┌──────────────────┐     ┌────────────────┐     ┌──────────────┐
│  index_project   │ ──▶ │  File Scanner  │ ──▶ │ Tree-sitter  │
│   (MCP Tool)     │     │   (walkdir)    │     │   Parser     │
└──────────────────┘     └────────────────┘     └──────────────┘
                                                       │
                              ┌────────────────────────┘
                              ▼
                    ┌──────────────────┐     ┌──────────────┐
                    │  Code Chunker    │ ──▶ │  Embeddings  │
                    │ (code-splitter)  │     │   (Candle)   │
                    └──────────────────┘     └──────────────┘
                                                    │
                                                    ▼
                                          ┌──────────────────┐
                                          │   SurrealDB      │
                                          │  Namespace per   │
                                          │    project       │
                                          └──────────────────┘
```

### Multi-Project Namespace Strategy

Each Git repository = separate namespace:

```
Git repo: /home/user/my-app/.git
Project ID: "my-app"
Namespace: "project:my-app"

Tables in namespace:
- memories
- entities
- relations
- code_chunks
```

## Components

### 1. Project Detection

```rust
pub struct ProjectInfo {
    pub id: String,        // "my-app"
    pub path: PathBuf,     // /home/user/my-app
    pub namespace: String, // "project:my-app"
}

pub fn detect_project(path: &Path) -> Result<ProjectInfo> {
    let git_root = find_git_root(path)?;
    let repo_name = git_root.file_name()?;
    
    Ok(ProjectInfo {
        id: repo_name.to_string(),
        path: git_root,
        namespace: format!("project:{}", repo_name),
    })
}
```

### 2. File Scanner

```rust
pub fn scan_directory(path: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(path)
        .add_custom_ignore_filename(".memoryignore")
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| is_code_file(e.path()))
        .map(|e| e.into_path())
        .collect()
}

fn is_code_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go")
    )
}
```

### 3. Code Chunker (Hybrid)

Primary: `code-splitter` crate for AST-based chunking
Fallback: Fixed-size chunking for unknown languages

```rust
use code_splitter::{Splitter, CharCounter};

pub fn chunk_file(path: &Path, content: &str) -> Vec<CodeChunk> {
    let ts_lang = detect_tree_sitter_language(path);
    
    if let Some(lang) = ts_lang {
        if let Ok(splitter) = Splitter::new(lang, CharCounter) {
            let splitter = splitter
                .with_max_size(MAX_TOKENS)
                .with_overlap(OVERLAP_TOKENS);
                
            if let Ok(chunks) = splitter.split(content.as_bytes()) {
                return chunks.into_iter()
                    .map(|c| inject_parent_context(c, content))
                    .collect();
            }
        }
    }
    
    fixed_size_chunks(content, MAX_LINES)
}

fn inject_parent_context(chunk: RawChunk, source: &str) -> CodeChunk {
    let parent_name = find_enclosing_scope(&chunk, source);
    
    let enriched_content = match parent_name {
        Some(scope) => format!("// Context: {}\n{}", scope, chunk.content),
        None => chunk.content.to_string(),
    };
    
    CodeChunk {
        content: enriched_content,
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        ..Default::default()
    }
}

fn find_enclosing_scope(chunk: &RawChunk, source: &str) -> Option<String> {
    // Tree-sitter cursor walk to find parent scope
    // Returns: "impl Foo", "class Bar", "mod baz", etc.
}
```

### 4. Indexer

```rust
pub async fn index_directory(
    path: &Path,
    storage: Arc<dyn StorageBackend>,
    embedder: Arc<EmbeddingService>,
    project_id: Option<String>,
) -> Result<IndexStats> {
    let files = scan_directory(path);
    
    // 1. Parallel scan + chunk (CPU-bound, rayon)
    let chunks: Vec<CodeChunk> = files
        .par_iter()
        .filter_map(|f| should_reindex(f))
        .flat_map(|f| chunk_file(f))
        .collect();
    
    // 2. Adaptive batch embedding (token-based)
    for batch in adaptive_batches(&chunks, MAX_BATCH_TOKENS) {
        let embeddings = embed_with_retry(&batch, MAX_RETRIES).await?;
        storage.create_code_chunks_batch(&batch, embeddings).await?;
    }
    
    Ok(stats)
}

fn adaptive_batches(chunks: &[CodeChunk], max_tokens: usize) -> Vec<Vec<&CodeChunk>> {
    // Batch by total tokens (content.len() / 4), not count
}

async fn embed_with_retry(batch: &[&CodeChunk], max_retries: u32) -> Result<Vec<Vec<f32>>> {
    // Exponential backoff: 100ms, 200ms, 400ms...
}
```

### 5. Reindexing Strategy

mtime → hash optimization:

```rust
fn should_reindex(file: &Path, cached: &IndexState) -> ReindexDecision {
    // 1. Fast mtime check (no file read)
    let meta = file.metadata()?;
    if meta.modified()? <= cached.mtime && meta.len() == cached.size {
        return ReindexDecision::Skip;  // 99% of cases
    }
    
    // 2. If mtime changed, check hash
    let content = fs::read_to_string(file)?;
    let hash = blake3::hash(content.as_bytes());
    
    if hash.to_hex().as_str() == cached.content_hash {
        return ReindexDecision::UpdateMeta;  // Only mtime
    }
    
    // 3. Real change
    ReindexDecision::Reindex(content, hash.to_hex().to_string())
}
```

### 6. File Watcher

```rust
pub struct CodebaseWatcher {
    watcher: RecommendedWatcher,
}

impl CodebaseWatcher {
    pub fn watch(path: &Path) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        
        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        tx.send(FileEvent::Modified(event.paths)).ok();
                    }
                    EventKind::Remove(_) => {
                        tx.send(FileEvent::Deleted(event.paths)).ok();
                    }
                    _ => {}
                }
            }
        })?;
        
        watcher.watch(path, RecursiveMode::Recursive)?;
        
        // Debounced at 500ms
        let debounced_rx = debounce_stream(rx, Duration::from_millis(500));
        
        tokio::spawn(async move {
            while let Ok(events) = debounced_rx.recv_batch().await {
                handle_debounced_events(events).await;
            }
        });
        
        Ok(Self { watcher })
    }
}
```

## Data Model

### CodeChunk Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    pub file_path: String,
    pub content: String,
    
    #[serde(default)]
    pub language: Language,
    
    pub start_line: u32,
    pub end_line: u32,
    
    #[serde(default)]
    pub chunk_type: ChunkType,  // function/class/module
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    // NEVER serialized (token protection)
    #[serde(skip_serializing)]
    pub embedding: Option<Vec<f32>>,
    
    pub content_hash: String,   // blake3 hex
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<Datetime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChunkType {
    Function, Class, Struct, Module, Impl, 
    #[default]
    Other
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Language {
    Rust, Python, JavaScript, TypeScript, Go, 
    #[default]
    Unknown
}
```

### SurrealDB Schema

```sql
DEFINE TABLE OVERWRITE code_chunks SCHEMAFULL;

DEFINE FIELD file_path     ON code_chunks TYPE string;
DEFINE FIELD content       ON code_chunks TYPE string;
DEFINE FIELD language      ON code_chunks TYPE string;
DEFINE FIELD start_line    ON code_chunks TYPE int;
DEFINE FIELD end_line      ON code_chunks TYPE int;
DEFINE FIELD chunk_type    ON code_chunks TYPE string;
DEFINE FIELD name          ON code_chunks TYPE option<string>;
DEFINE FIELD embedding     ON code_chunks TYPE option<array<float>>;
DEFINE FIELD content_hash  ON code_chunks TYPE string;
DEFINE FIELD project_id    ON code_chunks TYPE option<string>;
DEFINE FIELD indexed_at    ON code_chunks TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_chunks_path ON code_chunks FIELDS file_path;
DEFINE INDEX idx_chunks_hash ON code_chunks FIELDS content_hash;
DEFINE INDEX idx_chunks_project ON code_chunks FIELDS project_id;

DEFINE INDEX IF NOT EXISTS idx_chunks_vec ON code_chunks 
  FIELDS embedding HNSW DIMENSION 768 DIST COSINE;

DEFINE INDEX idx_chunks_fts ON code_chunks 
  FIELDS content SEARCH ANALYZER simple BM25;

-- Progress tracking
DEFINE TABLE OVERWRITE index_status SCHEMAFULL;
DEFINE FIELD project_id    ON index_status TYPE string;
DEFINE FIELD status        ON index_status TYPE string;
DEFINE FIELD total_files   ON index_status TYPE int;
DEFINE FIELD indexed_files ON index_status TYPE int;
DEFINE FIELD total_chunks  ON index_status TYPE int;
DEFINE FIELD started_at    ON index_status TYPE datetime;
DEFINE FIELD completed_at  ON index_status TYPE option<datetime>;
```

## MCP Tools

### index_project

```rust
#[tool(description = "Index a codebase project for semantic search")]
async fn index_project(params: IndexProjectArgs) {
    let project = detect_project(&args.path)?;
    storage.use_namespace(&project.namespace).await?;
    let stats = index_directory(path, storage, embedder, Some(project.id)).await?;
}
```

Args: path (String), watch (Option<bool>)

### search_code

```rust
const RRF_K: f32 = 60.0;

#[tool(description = "Semantic search over indexed code")]
async fn search_code(params: SearchCodeArgs) {
    let limit = args.limit.min(50);  // Hard cap
    
    let query_emb = embedder.embed(&args.query).await?;
    let vec_results = storage.vector_search_code(query_emb, 50).await?;
    let bm25_results = storage.bm25_search_code(&args.query, 50).await?;
    
    let merged = rrf_merge(&[vec_results, bm25_results], RRF_K, limit);
}
```

Args: query (String), project_id (Option<String>), limit (Option<usize>)

### get_index_status

Args: project_id (String)
Returns: project_id, status, chunks_count

### list_projects

No args. Returns list of project namespaces (max 100).

### delete_project

Args: project_id (String)
Deletes all code_chunks for project.

## Token Protection Strategy

### 1. Embedding Exclusion

```rust
#[serde(skip_serializing)]
pub embedding: Option<Vec<f32>>,
```

Savings: ~3KB per result (768 floats × 4 bytes)

### 2. Hard Result Limits

| Tool | User Limit | Hard Cap |
|------|------------|----------|
| search_code | unlimited | 50 |
| recall | unlimited | 50 |
| list_projects | unlimited | 100 |

### 3. Response Structure

```json
{
  "results": [/* CodeChunk without embedding */],
  "count": 10,
  "limit": 50
}
```

## Dependencies

```toml
[dependencies]
# Parsing
code-splitter = "0.1"
tree-sitter = "0.26"
tree-sitter-rust = "0.24"
tree-sitter-python = "0.25"
tree-sitter-javascript = "0.25"
tree-sitter-typescript = "0.23"

# File operations
walkdir = "2"
ignore = "0.4"
notify = "8"

# Parallelism
rayon = "1.10"

# Existing
blake3, tokio, surrealdb, candle, rmcp, petgraph
```
