use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

fn default_memory_type() -> MemoryType {
    MemoryType::Semantic
}

fn default_importance() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    pub content: String,
    
    #[serde(skip_serializing)]
    pub embedding: Option<Vec<f32>>,
    
    #[serde(default = "default_memory_type")]
    pub memory_type: MemoryType,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    
    #[serde(default = "Utc::now")]
    pub event_time: DateTime<Utc>,
    
    #[serde(default = "Utc::now")]
    pub ingestion_time: DateTime<Utc>,
    
    #[serde(default = "Utc::now")]
    pub valid_from: DateTime<Utc>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
    
    #[serde(default = "default_importance")]
    pub importance_score: f32,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalidation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Episodic,
    #[default]
    Semantic,
    Procedural,
}

impl std::str::FromStr for MemoryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "episodic" => Ok(Self::Episodic),
            "semantic" => Ok(Self::Semantic),
            "procedural" => Ok(Self::Procedural),
            _ => Err(format!("Unknown memory type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<MemoryType>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Memory {
    pub fn new(content: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            content,
            embedding: None,
            memory_type: MemoryType::Semantic,
            user_id: None,
            metadata: None,
            event_time: now,
            ingestion_time: now,
            valid_from: now,
            valid_until: None,
            importance_score: 1.0,
            invalidation_reason: None,
        }
    }
    
    pub fn with_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = memory_type;
        self
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}
