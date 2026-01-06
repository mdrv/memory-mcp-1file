#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelType {
    E5Small,
    #[default]
    E5Multi,
    Nomic,
    BgeM3,
    Mock,
}

impl ModelType {
    pub fn repo_id(&self) -> &'static str {
        match self {
            Self::E5Small => "intfloat/multilingual-e5-small",
            Self::E5Multi => "intfloat/multilingual-e5-base",
            Self::Nomic => "nomic-ai/nomic-embed-text-v1.5",
            Self::BgeM3 => "BAAI/bge-m3",
            Self::Mock => "mock",
        }
    }

    pub fn dimensions(&self) -> usize {
        match self {
            Self::E5Small => 384,
            Self::E5Multi => 768,
            Self::Nomic => 768,
            Self::BgeM3 => 1024,
            Self::Mock => 768,
        }
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
            "mock" => Ok(Self::Mock),
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
            Self::Mock => write!(f, "mock"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_model_type_from_str() {
        assert_eq!(ModelType::from_str("e5-small").unwrap(), ModelType::E5Small);
        assert_eq!(ModelType::from_str("E5_MULTI").unwrap(), ModelType::E5Multi);
        assert_eq!(ModelType::from_str("nomic").unwrap(), ModelType::Nomic);
        assert_eq!(ModelType::from_str("bge-m3").unwrap(), ModelType::BgeM3);
        assert!(ModelType::from_str("unknown").is_err());
    }

    #[test]
    fn test_model_type_display() {
        assert_eq!(format!("{}", ModelType::E5Small), "e5_small");
        assert_eq!(format!("{}", ModelType::E5Multi), "e5_multi");
    }

    #[test]
    fn test_model_dimensions() {
        assert_eq!(ModelType::E5Small.dimensions(), 384);
        assert_eq!(ModelType::E5Multi.dimensions(), 768);
        assert_eq!(ModelType::BgeM3.dimensions(), 1024);
    }

    #[test]
    fn test_default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.model, ModelType::E5Multi);
        assert_eq!(config.cache_size, 1000);
    }
}
