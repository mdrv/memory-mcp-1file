-- Memory MCP Server Schema
-- SurrealDB 2.x with kv-surrealkv
-- HNSW DIMENSION 768 for e5_multi model

-- Define analyzers for full-text search (BM25)
DEFINE ANALYZER IF NOT EXISTS simple TOKENIZERS blank, class FILTERS lowercase;

-- Memories table
DEFINE TABLE IF NOT EXISTS memories SCHEMAFULL;
DEFINE FIELD content          ON memories TYPE string;
DEFINE FIELD embedding        ON memories TYPE option<array<float>>;
DEFINE FIELD memory_type      ON memories TYPE string DEFAULT 'semantic';
DEFINE FIELD user_id          ON memories TYPE option<string>;
DEFINE FIELD metadata         ON memories FLEXIBLE TYPE option<object>;
DEFINE FIELD event_time       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD ingestion_time   ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_from       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON memories TYPE option<datetime>;
DEFINE FIELD importance_score ON memories TYPE float DEFAULT 1.0;
DEFINE FIELD invalidation_reason ON memories TYPE option<string>;
DEFINE FIELD content_hash     ON memories TYPE option<string>;
DEFINE FIELD embedding_state  ON memories TYPE string DEFAULT 'none';

DEFINE INDEX IF NOT EXISTS idx_memories_vec ON memories 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_memories_fts ON memories 
    FIELDS content SEARCH ANALYZER simple BM25;

-- Entities table
DEFINE TABLE IF NOT EXISTS entities SCHEMAFULL;
DEFINE FIELD name             ON entities TYPE string;
DEFINE FIELD entity_type      ON entities TYPE string DEFAULT 'unknown';
DEFINE FIELD description      ON entities TYPE option<string>;
DEFINE FIELD embedding        ON entities TYPE option<array<float>>;
DEFINE FIELD content_hash     ON entities TYPE option<string>;
DEFINE FIELD user_id          ON entities TYPE option<string>;
DEFINE FIELD created_at       ON entities TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_entities_vec ON entities 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_entities_fts ON entities 
    FIELDS name SEARCH ANALYZER simple BM25;

-- Relations table will be auto-created by RELATE statements
-- No explicit definition needed; RELATE creates edge tables automatically

-- Symbol relations table (code graph edges: calls, imports, etc.)
DEFINE TABLE IF NOT EXISTS symbol_relation;
DEFINE INDEX IF NOT EXISTS idx_sr_in ON symbol_relation FIELDS in;
DEFINE INDEX IF NOT EXISTS idx_sr_out ON symbol_relation FIELDS out;
DEFINE INDEX IF NOT EXISTS idx_sr_project ON symbol_relation FIELDS project_id;
DEFINE INDEX IF NOT EXISTS idx_sr_type ON symbol_relation FIELDS relation_type;

-- Code chunks table
DEFINE TABLE IF NOT EXISTS code_chunks SCHEMAFULL;
DEFINE FIELD file_path        ON code_chunks TYPE string;
DEFINE FIELD content          ON code_chunks TYPE string;
DEFINE FIELD language         ON code_chunks TYPE string DEFAULT 'unknown';
DEFINE FIELD start_line       ON code_chunks TYPE int;
DEFINE FIELD end_line         ON code_chunks TYPE int;
DEFINE FIELD chunk_type       ON code_chunks TYPE string DEFAULT 'other';
DEFINE FIELD name             ON code_chunks TYPE option<string>;
DEFINE FIELD embedding        ON code_chunks TYPE option<array<float>>;
DEFINE FIELD content_hash     ON code_chunks TYPE string;
DEFINE FIELD project_id       ON code_chunks TYPE option<string>;
DEFINE FIELD indexed_at       ON code_chunks TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_chunks_vec ON code_chunks 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_chunks_fts ON code_chunks 
    FIELDS content SEARCH ANALYZER simple BM25;
DEFINE INDEX IF NOT EXISTS idx_chunks_path ON code_chunks FIELDS file_path;
DEFINE INDEX IF NOT EXISTS idx_chunks_project ON code_chunks FIELDS project_id;
DEFINE INDEX IF NOT EXISTS idx_chunks_hash ON code_chunks FIELDS content_hash;

-- Index status table
DEFINE TABLE IF NOT EXISTS index_status SCHEMAFULL;
DEFINE FIELD project_id       ON index_status TYPE string;
DEFINE FIELD status           ON index_status TYPE string;
DEFINE FIELD total_files      ON index_status TYPE int DEFAULT 0;
DEFINE FIELD indexed_files    ON index_status TYPE int DEFAULT 0;
DEFINE FIELD total_chunks     ON index_status TYPE int DEFAULT 0;
DEFINE FIELD total_symbols    ON index_status TYPE int DEFAULT 0;
DEFINE FIELD started_at       ON index_status TYPE datetime;
DEFINE FIELD completed_at     ON index_status TYPE option<datetime>;
DEFINE FIELD error_message    ON index_status TYPE option<string>;

-- Code symbols table
DEFINE TABLE IF NOT EXISTS code_symbols SCHEMAFULL;
DEFINE FIELD name           ON code_symbols TYPE string;
DEFINE FIELD symbol_type    ON code_symbols TYPE string;
DEFINE FIELD file_path      ON code_symbols TYPE string;
DEFINE FIELD start_line     ON code_symbols TYPE int;
DEFINE FIELD end_line       ON code_symbols TYPE int;
DEFINE FIELD project_id     ON code_symbols TYPE string;
DEFINE FIELD signature      ON code_symbols TYPE option<string>;
DEFINE FIELD embedding      ON code_symbols TYPE option<array<float>>;
DEFINE FIELD indexed_at     ON code_symbols TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_symbols_name ON code_symbols FIELDS name;
DEFINE INDEX IF NOT EXISTS idx_symbols_project ON code_symbols FIELDS project_id;
DEFINE INDEX IF NOT EXISTS idx_symbols_path ON code_symbols FIELDS file_path;
DEFINE INDEX IF NOT EXISTS idx_symbols_vec ON code_symbols 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
