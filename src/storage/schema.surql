-- Memory MCP Server Schema
-- SurrealDB 2.x with kv-surrealkv
-- HNSW DIMENSION 768 for e5_multi model

-- Memories table
DEFINE TABLE IF NOT EXISTS memories SCHEMAFULL;
DEFINE FIELD content          ON memories TYPE string;
DEFINE FIELD embedding        ON memories TYPE option<array<float>>;
DEFINE FIELD memory_type      ON memories TYPE string DEFAULT 'semantic';
DEFINE FIELD user_id          ON memories TYPE option<string>;
DEFINE FIELD metadata         ON memories TYPE option<object>;
DEFINE FIELD event_time       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD ingestion_time   ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_from       ON memories TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON memories TYPE option<datetime>;
DEFINE FIELD importance_score ON memories TYPE float DEFAULT 1.0;
DEFINE FIELD invalidation_reason ON memories TYPE option<string>;

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
DEFINE FIELD user_id          ON entities TYPE option<string>;
DEFINE FIELD created_at       ON entities TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_entities_vec ON entities 
    FIELDS embedding HNSW DIMENSION 768 DIST COSINE;
DEFINE INDEX IF NOT EXISTS idx_entities_fts ON entities 
    FIELDS name SEARCH ANALYZER simple BM25;

-- Relations table (graph edges)
DEFINE TABLE IF NOT EXISTS relations TYPE RELATION IN entities OUT entities SCHEMAFULL;
DEFINE FIELD relation_type    ON relations TYPE string;
DEFINE FIELD weight           ON relations TYPE float DEFAULT 1.0;
DEFINE FIELD valid_from       ON relations TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until      ON relations TYPE option<datetime>;

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
DEFINE FIELD started_at       ON index_status TYPE datetime;
DEFINE FIELD completed_at     ON index_status TYPE option<datetime>;
DEFINE FIELD error_message    ON index_status TYPE option<string>;
