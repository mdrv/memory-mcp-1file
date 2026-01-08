use crate::types::EmbedTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedStrategy {
    Sync,
    Async,
}

pub struct EmbeddingPolicy;

impl EmbeddingPolicy {
    const SYNC_THRESHOLD: usize = 4096;

    pub fn decide(target: EmbedTarget, content_len: usize) -> EmbedStrategy {
        match target {
            EmbedTarget::Memory | EmbedTarget::Entity => {
                if content_len < Self::SYNC_THRESHOLD {
                    EmbedStrategy::Sync
                } else {
                    EmbedStrategy::Async
                }
            }
            EmbedTarget::CodeChunk | EmbedTarget::Symbol => EmbedStrategy::Async,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_small_sync() {
        assert_eq!(
            EmbeddingPolicy::decide(EmbedTarget::Memory, 100),
            EmbedStrategy::Sync
        );
    }

    #[test]
    fn test_memory_large_async() {
        assert_eq!(
            EmbeddingPolicy::decide(EmbedTarget::Memory, 5000),
            EmbedStrategy::Async
        );
    }

    #[test]
    fn test_code_always_async() {
        assert_eq!(
            EmbeddingPolicy::decide(EmbedTarget::CodeChunk, 10),
            EmbedStrategy::Async
        );
    }
}
