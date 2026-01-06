---
work_package_id: WP09
title: "Code Search Tools"
phase: "Phase 8"
priority: P3
subtasks: ["T055", "T056", "T057", "T058", "T059", "T060", "T061", "T062", "T062a"]
lane: planned
dependencies: ["WP05"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
---

# WP09: Code Search Tools

## Objective

Implement codebase indexing with tree-sitter AST chunking and semantic code search.

## Context

Code search enables developer agents to find relevant code semantically. This is a P3 feature - the core memory system works without it.

**Can run in parallel with WP07 and WP08** - only depends on WP05.

**Reference**:
- `kitty-specs/001-memory-mcp-server/research.md` - tree-sitter, code-splitter patterns

## Subtasks

### T055: Create codebase/scanner.rs

**Location**: `src/codebase/scanner.rs`

```rust
use std::path::{Path, PathBuf};
use ignore::WalkBuilder;

/// Scan directory for code files, respecting .gitignore and .memoryignore
pub fn scan_directory(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let walker = WalkBuilder::new(root)
        .hidden(true)  // Skip hidden files
        .git_ignore(true)  // Respect .gitignore
        .add_custom_ignore_filename(".memoryignore")  // Custom ignore file
        .build();
    
    let mut files = Vec::new();
    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && is_code_file(path) {
            files.push(path.to_path_buf());
        }
    }
    
    Ok(files)
}

/// Check if file is a code file based on extension
pub fn is_code_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    
    matches!(ext.to_lowercase().as_str(),
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | 
        "go" | "java" | "c" | "cpp" | "h" | "hpp" |
        "rb" | "php" | "swift" | "kt" | "scala" |
        "sh" | "bash" | "zsh" | "fish" |
        "json" | "yaml" | "yml" | "toml" | "xml" |
        "md" | "rst" | "txt"
    )
}

/// Detect language from file extension
pub fn detect_language(path: &Path) -> crate::types::Language {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return crate::types::Language::Unknown;
    };
    
    match ext.to_lowercase().as_str() {
        "rs" => crate::types::Language::Rust,
        "py" => crate::types::Language::Python,
        "js" | "jsx" => crate::types::Language::JavaScript,
        "ts" | "tsx" => crate::types::Language::TypeScript,
        "go" => crate::types::Language::Go,
        _ => crate::types::Language::Unknown,
    }
}
```

Add `ignore = "0.4"` to Cargo.toml.

---

### T056: Create codebase/chunker.rs

**Location**: `src/codebase/chunker.rs`

```rust
use code_splitter::{CodeSplitter, Language as SplitterLang};
use std::path::Path;

use crate::types::{ChunkType, CodeChunk, Language};

/// Chunk a code file using tree-sitter AST
pub fn chunk_file(
    path: &Path,
    content: &str,
    project_id: &str,
) -> anyhow::Result<Vec<CodeChunk>> {
    let language = super::scanner::detect_language(path);
    let file_path = path.to_string_lossy().to_string();
    
    // Get tree-sitter language
    let splitter_lang = match language {
        Language::Rust => Some(SplitterLang::Rust),
        Language::Python => Some(SplitterLang::Python),
        Language::JavaScript => Some(SplitterLang::JavaScript),
        Language::TypeScript => Some(SplitterLang::TypeScript),
        Language::Go => Some(SplitterLang::Go),
        _ => None,
    };
    
    if let Some(lang) = splitter_lang {
        // AST-aware chunking
        let splitter = CodeSplitter::new(lang)?;
        let chunks = splitter.split(content)?;
        
        Ok(chunks.into_iter().map(|chunk| {
            let content_hash = blake3::hash(chunk.content.as_bytes()).to_hex().to_string();
            
            CodeChunk {
                id: None,
                file_path: file_path.clone(),
                content: inject_context(&chunk, content),
                language: language.clone(),
                start_line: chunk.start_line as u32,
                end_line: chunk.end_line as u32,
                chunk_type: map_chunk_type(&chunk.kind),
                name: chunk.name.clone(),
                embedding: None,
                content_hash,
                project_id: Some(project_id.to_string()),
                indexed_at: chrono::Utc::now(),
            }
        }).collect())
    } else {
        // Fallback: fixed-size chunking (100 lines)
        fallback_chunk(path, content, project_id)
    }
}

/// Inject parent scope context into chunk
fn inject_context(chunk: &code_splitter::Chunk, _full_content: &str) -> String {
    // Prepend parent scope name if available
    if let Some(ref parent) = chunk.parent_name {
        format!("// Context: {}\n{}", parent, chunk.content)
    } else {
        chunk.content.clone()
    }
}

fn map_chunk_type(kind: &str) -> ChunkType {
    match kind.to_lowercase().as_str() {
        "function" | "fn" => ChunkType::Function,
        "class" => ChunkType::Class,
        "struct" => ChunkType::Struct,
        "module" | "mod" => ChunkType::Module,
        "impl" => ChunkType::Impl,
        _ => ChunkType::Other,
    }
}

fn fallback_chunk(path: &Path, content: &str, project_id: &str) -> anyhow::Result<Vec<CodeChunk>> {
    let language = super::scanner::detect_language(path);
    let file_path = path.to_string_lossy().to_string();
    let lines: Vec<&str> = content.lines().collect();
    let chunk_size = 100;
    
    let mut chunks = Vec::new();
    for (i, chunk_lines) in lines.chunks(chunk_size).enumerate() {
        let chunk_content = chunk_lines.join("\n");
        let content_hash = blake3::hash(chunk_content.as_bytes()).to_hex().to_string();
        
        chunks.push(CodeChunk {
            id: None,
            file_path: file_path.clone(),
            content: chunk_content,
            language: language.clone(),
            start_line: (i * chunk_size + 1) as u32,
            end_line: ((i + 1) * chunk_size).min(lines.len()) as u32,
            chunk_type: ChunkType::Other,
            name: None,
            embedding: None,
            content_hash,
            project_id: Some(project_id.to_string()),
            indexed_at: chrono::Utc::now(),
        });
    }
    
    Ok(chunks)
}
```

---

### T057: Create codebase/indexer.rs

**Location**: `src/codebase/indexer.rs`

```rust
use std::path::Path;
use std::sync::Arc;

use crate::embedding::EmbeddingService;
use crate::storage::SurrealStorage;
use crate::types::{CodeChunk, IndexState, IndexStatus};

/// Index a project directory
pub async fn index_directory(
    root: &Path,
    project_id: &str,
    storage: &SurrealStorage,
    embedding: &EmbeddingService,
    batch_size: usize,
) -> anyhow::Result<IndexResult> {
    // Scan files
    let files = super::scanner::scan_directory(root)?;
    let total_files = files.len() as u32;
    
    // Create initial status
    let status = IndexStatus {
        id: None,
        project_id: project_id.to_string(),
        status: IndexState::Indexing,
        total_files,
        indexed_files: 0,
        total_chunks: 0,
        started_at: chrono::Utc::now(),
        completed_at: None,
        error_message: None,
    };
    storage.update_index_status(status).await?;
    
    // Delete existing chunks for this project
    storage.delete_project_chunks(project_id).await?;
    
    let mut total_chunks = 0u32;
    let mut indexed_files = 0u32;
    let mut chunk_batch: Vec<CodeChunk> = Vec::new();
    
    for file_path in &files {
        // Read file content
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue, // Skip binary/unreadable files
        };
        
        // Chunk file
        let mut chunks = super::chunker::chunk_file(file_path, &content, project_id)?;
        
        // Generate embeddings for chunks
        for chunk in &mut chunks {
            if let Ok(emb) = embedding.embed(&chunk.content).await {
                chunk.embedding = Some(emb);
            }
        }
        
        chunk_batch.extend(chunks);
        indexed_files += 1;
        
        // Batch insert when batch is full
        if chunk_batch.len() >= batch_size {
            let count = storage.create_code_chunks_batch(std::mem::take(&mut chunk_batch)).await?;
            total_chunks += count as u32;
            
            // Update progress
            let progress = IndexStatus {
                id: None,
                project_id: project_id.to_string(),
                status: IndexState::Indexing,
                total_files,
                indexed_files,
                total_chunks,
                started_at: chrono::Utc::now(),
                completed_at: None,
                error_message: None,
            };
            storage.update_index_status(progress).await?;
        }
    }
    
    // Insert remaining chunks
    if !chunk_batch.is_empty() {
        let count = storage.create_code_chunks_batch(chunk_batch).await?;
        total_chunks += count as u32;
    }
    
    // Mark complete
    let final_status = IndexStatus {
        id: None,
        project_id: project_id.to_string(),
        status: IndexState::Completed,
        total_files,
        indexed_files,
        total_chunks,
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
        error_message: None,
    };
    storage.update_index_status(final_status).await?;
    
    Ok(IndexResult {
        project_id: project_id.to_string(),
        files_indexed: indexed_files,
        chunks_created: total_chunks,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct IndexResult {
    pub project_id: String,
    pub files_indexed: u32,
    pub chunks_created: u32,
}
```

---

### T058: Implement tool: index_project

```rust
    /// Index a codebase for semantic search.
    #[tool(description = "Index a codebase for semantic search.")]
    async fn index_project(
        &self,
        /// Path to project root directory
        path: String,
        /// Enable file watching for auto-reindex (optional, deferred)
        watch: Option<bool>,
    ) -> Result<CallToolResult, McpError> {
        // Check embedding ready
        if self.state.embedding.status() != EmbeddingStatus::Ready {
            return Ok(CallToolResult::error("Embedding service not ready. Please try again."));
        }
        
        let path = std::path::Path::new(&path);
        if !path.exists() || !path.is_dir() {
            return Ok(CallToolResult::error(format!("Invalid path: {}", path.display())));
        }
        
        // Derive project_id from directory name
        let project_id = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Index (this may take a while for large projects)
        match crate::codebase::indexer::index_directory(
            path,
            &project_id,
            &self.state.storage,
            &self.state.embedding,
            self.state.config.batch_size,
        ).await {
            Ok(result) => Ok(CallToolResult::success(serde_json::to_value(result).unwrap())),
            Err(e) => Err(McpError::internal(e.to_string())),
        }
    }
```

---

### T059: Implement tool: search_code

```rust
    /// Semantic search over indexed code.
    #[tool(description = "Semantic search over indexed code.")]
    async fn search_code(
        &self,
        /// Search query text
        query: String,
        /// Filter by project (optional)
        project_id: Option<String>,
        /// Maximum results (default: 10, max: 50)
        limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        // Check embedding ready
        if self.state.embedding.status() != EmbeddingStatus::Ready {
            return Ok(CallToolResult::error("Embedding service not ready. Please try again."));
        }
        
        let limit = limit.unwrap_or(10).min(50);
        
        // Generate query embedding
        let embedding = self.state.embedding.embed(&query).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        let results = self.state.storage
            .vector_search_code(&embedding, project_id.as_deref(), limit)
            .await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "results": results,
            "count": results.len(),
            "query": query
        })))
    }
```

---

### T060: Implement tool: get_index_status

```rust
    /// Get indexing status for a project.
    #[tool(description = "Get indexing status for a project.")]
    async fn get_index_status(
        &self,
        /// Project ID to check
        project_id: String,
    ) -> Result<CallToolResult, McpError> {
        match self.state.storage.get_index_status(&project_id).await {
            Ok(Some(status)) => Ok(CallToolResult::success(serde_json::to_value(status).unwrap())),
            Ok(None) => Ok(CallToolResult::error(format!("Project not found: {}", project_id))),
            Err(e) => Err(McpError::internal(e.to_string())),
        }
    }
```

---

### T061: Implement tool: list_projects

```rust
    /// List all indexed projects.
    #[tool(description = "List all indexed projects.")]
    async fn list_projects(&self) -> Result<CallToolResult, McpError> {
        let projects = self.state.storage.list_projects().await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "projects": projects,
            "count": projects.len()
        })))
    }
```

---

### T062: Implement tool: delete_project

```rust
    /// Delete all indexed code chunks for a project.
    #[tool(description = "Delete all indexed code chunks for a project.")]
    async fn delete_project(
        &self,
        /// Project ID to delete
        project_id: String,
    ) -> Result<CallToolResult, McpError> {
        let count = self.state.storage.delete_project_chunks(&project_id).await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "chunks_deleted": count
        })))
    }
```

---

---

### T062a: Write tests for WP09 components

**Goal**: Verify code search.

**Implementation**:
- Verify tree-sitter chunking boundaries
- Verify context injection
- Verify `.memoryignore` handling

**Pass Criteria**:
- `cargo test` passes

---

## Definition of Done

1. index_project processes 100 files in < 5 minutes
2. Tree-sitter chunks preserve function/class boundaries
3. Context injection includes parent scope name
4. search_code returns results with file_path, line numbers
5. .gitignore and .memoryignore respected
6. Fallback chunking works for unsupported languages

## Risks

| Risk | Mitigation |
|------|------------|
| tree-sitter compatibility | Fallback to fixed-size chunking |
| Large files OOM | Skip files > 1MB |
| Binary files | Use is_code_file filter |

## Reviewer Guidance

- Verify ignore patterns work correctly
- Check context injection format
- Confirm batch_size respected
- Test with real project (not just mocks)
