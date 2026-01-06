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
