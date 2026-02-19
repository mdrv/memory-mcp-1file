use serde::{Deserialize, Serialize};
use super::{Datetime, SurrealValue, Thing};

fn default_weight() -> f32 {
    1.0
}

fn default_datetime() -> Datetime {
    Datetime::default()
}

fn default_entity_type() -> String {
    "unknown".to_string()
}

fn default_name() -> String {
    String::new()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, SurrealValue)]
pub struct Entity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_entity_type")]
    pub entity_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content_hash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    #[serde(default = "default_datetime")]
    pub created_at: Datetime,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
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

    #[serde(default = "default_datetime")]
    pub valid_from: Datetime,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<Datetime>,
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
            content_hash: None,
            user_id: None,
            created_at: Datetime::default(),
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

impl std::str::FromStr for Direction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "outgoing" | "out" => Ok(Direction::Outgoing),
            "incoming" | "in" => Ok(Direction::Incoming),
            "both" => Ok(Direction::Both),
            _ => Ok(Direction::default()),
        }
    }
}
