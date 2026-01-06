use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub struct EmbeddingCache {
    cache: Mutex<LruCache<String, Vec<f32>>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl EmbeddingCache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(cap)),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    fn cache_key(text: &str, model_version: &str) -> String {
        let normalized = text.trim().to_lowercase();
        let hash = blake3::hash(format!("{}:{}", normalized, model_version).as_bytes());
        hash.to_hex().to_string()
    }

    pub fn get(&self, text: &str, model_version: &str) -> Option<Vec<f32>> {
        let key = Self::cache_key(text, model_version);
        let mut cache = self.cache.lock().unwrap();
        if let Some(vec) = cache.get(&key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(vec.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn put(&self, text: &str, model_version: &str, embedding: Vec<f32>) {
        let key = Self::cache_key(text, model_version);
        let mut cache = self.cache.lock().unwrap();
        cache.put(key, embedding);
    }

    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            size: cache.len(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = EmbeddingCache::new(2);
        let model = "test-model";
        let vec1 = vec![1.0, 2.0, 3.0];

        assert!(cache.get("hello", model).is_none());
        assert_eq!(cache.stats().misses, 1);

        cache.put("hello", model, vec1.clone());
        assert_eq!(cache.get("hello", model), Some(vec1));
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().size, 1);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = EmbeddingCache::new(1);
        let model = "test-model";

        cache.put("a", model, vec![1.0]);
        cache.put("b", model, vec![2.0]);

        assert!(cache.get("a", model).is_none());
        assert!(cache.get("b", model).is_some());
        assert_eq!(cache.stats().size, 1);
    }

    #[test]
    fn test_cache_normalization() {
        let cache = EmbeddingCache::new(10);
        let model = "test-model";
        let vec = vec![1.0];

        cache.put("  Hello  ", model, vec.clone());
        assert_eq!(cache.get("hello", model), Some(vec));
    }
}
