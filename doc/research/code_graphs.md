# Research: Code Graph Architectures & Intelligence

**Date:** 2026-01-07
**Status:** Completed
**Goal:** Identify architectural patterns for code dependency graphs to inform Memory MCP implementation.

## 1. Top Open Source Projects

### A. GitHub Stack Graphs
*   **Repo:** `github/stack-graphs` (Rust)
*   **Purpose:** Precise code navigation (jump-to-def, find-refs) at GitHub scale.
*   **Key Innovation:** "Stack Graphs" extend Scope Graphs to handle complex name binding rules incrementally.
*   **Architecture:**
    *   **Incremental:** Each file is parsed into an isolated subgraph.
    *   **Stitching:** Subgraphs are "stitched" together at query time (or pre-calculated partial paths).
    *   **Language Agnostic:** Rules defined via `tree-sitter-graph` DSL.
    *   **Storage:** Highly serializable; optimizes for "write once, read many".

### B. Rust Analyzer (Salsa)
*   **Repo:** `rust-lang/rust-analyzer`, `salsa-rs/salsa`
*   **Purpose:** Fast, incremental IDE intelligence.
*   **Key Innovation:** "Query-based compiler" architecture.
*   **Architecture:**
    *   **Database Pattern:** The code is a database. Queries (e.g., "type of symbol X") trigger on-demand computation.
    *   **Memoization:** Results are cached. Inputs are tracked. If input changes, only dependent queries re-run.
    *   **In-Memory:** Designed for live editing sessions, not long-term storage (though experimental persistence exists).

### C. Kuzu (Embedded Graph DB)
*   **Repo:** `kuzudb/kuzu`
*   **Purpose:** High-performance embedded graph database (like SQLite for graphs).
*   **Key Innovation:** Columnar storage for graphs, vectorized query execution.
*   **Relevance:** strong candidate for the *persistence layer* of a memory graph, distinct from the *compiler layer*.

## 2. Architectural Patterns

### Pattern 1: The "Compiler Database" (Salsa)
*   **Concept:** Code is data. Functions are queries.
*   **Pros:** Extremely reactive, correct incremental updates, handles complexity well.
*   **Cons:** High complexity to implement, typically in-memory only, "persistence" is difficult (requires serializing the whole query graph).

### Pattern 2: The "Stitched Graph" (Stack Graphs)
*   **Concept:** Parse files locally into independent graphs. "Stitch" them only when a specific path is requested.
*   **Pros:** Massive scalability (O(1) indexing per file), perfect for "distributed" code.
*   **Cons:** Querying is path-finding (can be slow without pre-computation), complex "graph language" to define rules.

### Pattern 3: The "Fact Database" (Datalog / SCIP)
*   **Concept:** Extract facts (edges, nodes) during build -> Insert into DB -> Query DB.
*   **Pros:** Simple storage (SQLite/Kuzu), standard query languages (SQL/Cypher/Datalog).
*   **Cons:** "Stale" data (needs re-indexing), hard to handle "lazy" resolution (must resolve everything upfront).

## 3. Recommendations for Memory MCP

For a **Memory MCP Server** (which needs to remember context across sessions and maybe even across reboots/projects), purely in-memory approaches (Salsa) are insufficient for the *storage* layer, though excellent for the *compute* layer.

**Proposed Hybrid Architecture:**

1.  **Storage Layer (The "Memory"):**
    *   Use an embedded Graph DB (e.g., **Kuzu** or **SurrealDB**) to store semantic entities (Files, Functions, User Notes, Tasks).
    *   This provides persistence, querying capability, and "backup" out of the box.

2.  **Intelligence Layer (The "Processor"):**
    *   Use a simplified **Stack Graph** approach for linking.
    *   Do not try to build a full compiler. Instead, extract "Symbols" and "References" and store them as nodes/edges in Kuzu.
    *   Use **Tree-sitter** for robust parsing (standard in Rust ecosystem).

**Specific Rust Crates:**
*   `tree-sitter`: Parsing.
*   `kuzu` or `surrealdb`: Embedded graph storage.
*   `petgraph`: If in-memory graph algorithms are needed before storage.
*   `stack-graphs`: Only if we need precise, language-agnostic name resolution logic (high complexity cost).

## 4. Conclusion
We should **not** build a full LSP (Salsa approach). We should build a **Knowledge Graph** of code. The "Fact Database" pattern (Pattern 3) backed by an embedded graph DB is the most pragmatic fit for "Memory", as it prioritizes storage, retrieval, and relationships over sub-millisecond editor responsiveness.
