use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

fn default_datetime() -> Datetime {
    Datetime::default()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymbolType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Module,
    Trait,
    Import,
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolType::Function => write!(f, "function"),
            SymbolType::Method => write!(f, "method"),
            SymbolType::Class => write!(f, "class"),
            SymbolType::Struct => write!(f, "struct"),
            SymbolType::Enum => write!(f, "enum"),
            SymbolType::Interface => write!(f, "interface"),
            SymbolType::Module => write!(f, "module"),
            SymbolType::Trait => write!(f, "trait"),
            SymbolType::Import => write!(f, "import"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodeRelationType {
    Calls,
    Imports,
    Contains,
    Implements,
    Extends,
}

impl std::fmt::Display for CodeRelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeRelationType::Calls => write!(f, "calls"),
            CodeRelationType::Imports => write!(f, "imports"),
            CodeRelationType::Contains => write!(f, "contains"),
            CodeRelationType::Implements => write!(f, "implements"),
            CodeRelationType::Extends => write!(f, "extends"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub project_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    #[serde(default = "default_datetime")]
    pub indexed_at: Datetime,
}

impl CodeSymbol {
    pub fn new(
        name: String,
        symbol_type: SymbolType,
        file_path: String,
        start_line: u32,
        end_line: u32,
        project_id: String,
    ) -> Self {
        Self {
            id: None,
            name,
            symbol_type,
            file_path,
            start_line,
            end_line,
            project_id,
            signature: None,
            embedding: None,
            indexed_at: Datetime::default(),
        }
    }

    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }

    pub fn unique_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.project_id, self.file_path, self.name, self.start_line
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReference {
    pub name: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
}

impl CodeReference {
    pub fn new(name: String, file_path: String, line: u32, column: u32) -> Self {
        Self {
            name,
            file_path,
            line,
            column,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRelation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    #[serde(rename = "in")]
    pub from_symbol: Thing,

    #[serde(rename = "out")]
    pub to_symbol: Thing,

    pub relation_type: CodeRelationType,

    pub file_path: String,
    pub line: u32,

    #[serde(default = "default_datetime")]
    pub created_at: Datetime,
}

impl SymbolRelation {
    pub fn new(
        from_symbol: Thing,
        to_symbol: Thing,
        relation_type: CodeRelationType,
        file_path: String,
        line: u32,
    ) -> Self {
        Self {
            id: None,
            from_symbol,
            to_symbol,
            relation_type,
            file_path,
            line,
            created_at: Datetime::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredSymbol {
    #[serde(flatten)]
    pub symbol: CodeSymbol,
    pub score: f32,
}
