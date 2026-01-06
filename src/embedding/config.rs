#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    E5Small,
    E5Multi,
    Nomic,
    BgeM3,
}

impl ModelType {
    pub fn repo_id(&self) -> &'static str {
        match self {
            Self::E5Small => "intfloat/multilingual-e5-small",
            Self::E5Multi => "intfloat/multilingual-e5-base",
            Self::Nomic => "nomic-ai/nomic-embed-text-v1.5",
            Self::BgeM3 => "BAAI/bge-m3",
        }
    }

    pub fn dimensions(&self) -> usize {
        match self {
            Self::E5Small => 384,
            Self::E5Multi => 768,
            Self::Nomic => 768,
            Self::BgeM3 => 1024,
        }
    }
}

impl Default for ModelType {
    fn default() -> Self {
        Self::E5Multi
    }
}

impl std::str::FromStr for ModelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "e5_small" | "e5-small" => Ok(Self::E5Small),
            "e5_multi" | "e5-multi" | "e5_base" | "e5-base" => Ok(Self::E5Multi),
            "nomic" => Ok(Self::Nomic),
            "bge_m3" | "bge-m3" => Ok(Self::BgeM3),
            _ => Err(format!("Unknown model: {}", s)),
        }
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::E5Small => write!(f, "e5_small"),
            Self::E5Multi => write!(f, "e5_multi"),
            Self::Nomic => write!(f, "nomic"),
            Self::BgeM3 => write!(f, "bge_m3"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub model: ModelType,
    pub cache_size: usize,
    pub batch_size: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: ModelType::E5Multi,
            cache_size: 1000,
            batch_size: 32,
        }
    }
}
