use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

fn default_datetime() -> Datetime {
    Datetime::default()
}

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
    pub chunk_type: ChunkType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    pub content_hash: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    #[serde(default = "default_datetime")]
    pub indexed_at: Datetime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChunkType {
    Function,
    Class,
    Struct,
    Module,
    Impl,
    #[default]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Dart,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub project_id: String,
    pub status: IndexState,

    #[serde(default)]
    pub total_files: u32,

    #[serde(default)]
    pub indexed_files: u32,

    #[serde(default)]
    pub total_chunks: u32,

    #[serde(default)]
    pub total_symbols: u32,

    #[serde(default = "default_datetime")]
    pub started_at: Datetime,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<Datetime>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IndexState {
    Indexing,
    EmbeddingPending,
    Completed,
    Failed,
}

impl std::fmt::Display for IndexState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexState::Indexing => write!(f, "indexing"),
            IndexState::EmbeddingPending => write!(f, "embedding_pending"),
            IndexState::Completed => write!(f, "completed"),
            IndexState::Failed => write!(f, "failed"),
        }
    }
}

impl IndexStatus {
    pub fn new(project_id: String) -> Self {
        Self {
            id: None,
            project_id,
            status: IndexState::Indexing,
            total_files: 0,
            indexed_files: 0,
            total_chunks: 0,
            total_symbols: 0,
            started_at: Datetime::default(),
            completed_at: None,
            error_message: None,
        }
    }
}
