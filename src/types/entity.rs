use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

fn default_weight() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    pub name: String,
    
    #[serde(default)]
    pub entity_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    #[serde(skip_serializing)]
    pub embedding: Option<Vec<f32>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    #[serde(rename = "in")]
    pub from_entity: Thing,
    
    #[serde(rename = "out")]
    pub to_entity: Thing,
    
    pub relation_type: String,
    
    #[serde(default = "default_weight")]
    pub weight: f32,
    
    #[serde(default = "Utc::now")]
    pub valid_from: DateTime<Utc>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Outgoing,
    Incoming,
    Both,
}

impl Entity {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            entity_type: "unknown".to_string(),
            description: None,
            embedding: None,
            user_id: None,
            created_at: Utc::now(),
        }
    }
    
    pub fn with_type(mut self, entity_type: String) -> Self {
        self.entity_type = entity_type;
        self
    }
    
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}
