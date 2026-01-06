---
work_package_id: WP04
title: "Embedding Layer"
phase: "Phase 3"
priority: P1
subtasks: ["T026", "T027", "T028", "T029", "T030", "T031", "T032"]
lane: planned
dependencies: ["WP02"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
---

# WP04: Embedding Layer

## Objective

Implement the full embedding service with Candle integration, LRU caching, and background model loading.

## Context

This layer provides vector embeddings for memories, entities, and code chunks. The server must start within 1 second, so model loading happens in background.

**Can run in parallel with WP03** (Storage Layer).

**Reference**:
- `kitty-specs/001-memory-mcp-server/research.md` - Candle patterns, model selection

## Subtasks

### T026: Create embedding/config.rs

**Location**: `src/embedding/config.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    E5Small,   // 384 dimensions
    E5Multi,   // 768 dimensions (default)
    Nomic,     // 768 dimensions
    BgeM3,     // 1024 dimensions
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

impl std::str::FromStr for ModelType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "e5_small" | "e5-small" => Ok(Self::E5Small),
            "e5_multi" | "e5-multi" => Ok(Self::E5Multi),
            "nomic" => Ok(Self::Nomic),
            "bge_m3" | "bge-m3" => Ok(Self::BgeM3),
            _ => Err(format!("Unknown model: {}", s)),
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
```

---

### T027: Implement EmbeddingEngine - Model Loading

**Location**: `src/embedding/engine.rs`

```rust
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::BertModel;
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;

pub struct EmbeddingEngine {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    dimensions: usize,
}

impl EmbeddingEngine {
    pub fn new(model_type: ModelType) -> anyhow::Result<Self> {
        let device = Device::Cpu;
        let api = Api::new()?;
        let repo = api.model(model_type.repo_id().to_string());
        
        // Download files
        let tokenizer_path = repo.get("tokenizer.json")?;
        let weights_path = repo.get("model.safetensors")?;
        let config_path = repo.get("config.json")?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Tokenizer error: {}", e))?;
        
        // Load model config
        let config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&config_path)?
        )?;
        
        // Load weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &device)?
        };
        
        // Build model (BertModel for E5/BGE)
        let bert_config = candle_transformers::models::bert::Config::from_json(&config)?;
        let model = BertModel::load(vb, &bert_config)?;
        
        Ok(Self {
            model,
            tokenizer,
            device,
            dimensions: model_type.dimensions(),
        })
    }
}
```

---

### T028: Implement embed() with mean pooling

**Location**: `src/embedding/engine.rs` (continued)

```rust
impl EmbeddingEngine {
    pub fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        // Tokenize
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenizer error: {}", e))?;
        
        let ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        
        // Convert to tensors
        let input_ids = Tensor::new(ids, &self.device)?.unsqueeze(0)?;
        let attention = Tensor::new(attention_mask, &self.device)?.unsqueeze(0)?;
        let token_type_ids = input_ids.zeros_like()?;
        
        // Forward pass
        let output = self.model.forward(&input_ids, &token_type_ids, Some(&attention))?;
        
        // Mean pooling
        let sum = output.sum(1)?;
        let count = attention.sum(1)?.to_dtype(candle_core::DType::F32)?;
        let mean = sum.broadcast_div(&count.unsqueeze(1)?)?;
        
        // L2 normalization
        let norm = mean.sqr()?.sum_keepdim(1)?.sqrt()?;
        let normalized = mean.broadcast_div(&norm)?;
        
        // Extract as Vec<f32>
        let vec: Vec<f32> = normalized.squeeze(0)?.to_vec1()?;
        
        assert_eq!(vec.len(), self.dimensions);
        Ok(vec)
    }
    
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}
```

---

### T029: Implement EmbeddingCache

**Location**: `src/embedding/cache.rs`

```rust
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

pub struct EmbeddingCache {
    cache: Mutex<LruCache<String, Vec<f32>>>,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl EmbeddingCache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(cap)),
            hits: Default::default(),
            misses: Default::default(),
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
            self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(vec.clone())
        } else {
            self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
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
```

---

### T030: Implement EmbeddingService

**Location**: `src/embedding/service.rs`

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{EmbeddingCache, EmbeddingConfig, EmbeddingEngine, EmbeddingStatus, ModelType};
use crate::{AppError, Result};

pub struct EmbeddingService {
    engine: Arc<RwLock<Option<EmbeddingEngine>>>,
    cache: EmbeddingCache,
    config: EmbeddingConfig,
    status: Arc<std::sync::atomic::AtomicU8>,
}

impl EmbeddingService {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            engine: Arc::new(RwLock::new(None)),
            cache: EmbeddingCache::new(config.cache_size),
            config,
            status: Arc::new(std::sync::atomic::AtomicU8::new(0)), // Loading
        }
    }
    
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        let model_ver = self.config.model.repo_id();
        if let Some(cached) = self.cache.get(text, model_ver) {
            return Ok(cached);
        }
        
        // Get engine (blocking read)
        let guard = self.engine.read().await;
        let engine = guard.as_ref().ok_or(AppError::EmbeddingNotReady)?;
        
        // Embed
        let embedding = engine.embed(text)
            .map_err(|e| AppError::Embedding(e.to_string()))?;
        
        // Cache result
        self.cache.put(text, model_ver, embedding.clone());
        
        Ok(embedding)
    }
    
    pub fn status(&self) -> EmbeddingStatus {
        match self.status.load(std::sync::atomic::Ordering::Relaxed) {
            0 => EmbeddingStatus::Loading,
            1 => EmbeddingStatus::Ready,
            _ => EmbeddingStatus::Error,
        }
    }
    
    pub fn model(&self) -> ModelType {
        self.config.model
    }
    
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.cache.stats()
    }
}
```

---

### T031: Implement background model loading

**Location**: `src/embedding/service.rs` (continued)

```rust
impl EmbeddingService {
    pub fn start_loading(&self) {
        let engine = self.engine.clone();
        let status = self.status.clone();
        let model = self.config.model;
        
        std::thread::spawn(move || {
            tracing::info!("Loading embedding model: {:?}", model);
            
            match EmbeddingEngine::new(model) {
                Ok(e) => {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        let mut guard = engine.write().await;
                        *guard = Some(e);
                    });
                    status.store(1, std::sync::atomic::Ordering::Relaxed);
                    tracing::info!("Embedding model ready");
                }
                Err(e) => {
                    tracing::error!("Failed to load embedding model: {}", e);
                    status.store(2, std::sync::atomic::Ordering::Relaxed);
                }
            }
        });
    }
}
```

---

### T032: Implement status tracking

**Location**: `src/embedding/mod.rs`

```rust
mod cache;
mod config;
mod engine;
mod service;

pub use cache::{CacheStats, EmbeddingCache};
pub use config::{EmbeddingConfig, ModelType};
pub use engine::EmbeddingEngine;
pub use service::EmbeddingService;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingStatus {
    Loading,
    Ready,
    Error,
}
```

---

## Definition of Done

1. `EmbeddingService::embed("text")` returns correct dimension vector
2. Cache hit/miss tracking works
3. Server starts in < 1 second (model loads in background)
4. Status transitions: Loading -> Ready (or Error)
5. L2 norm of output vectors ≈ 1.0

## Risks

| Risk | Mitigation |
|------|------------|
| Candle API changes | Pin 0.9.1, use exact patterns from PoC |
| Model download failures | Graceful error, status = Error, non-embedding tools still work |
| Memory usage during load | CPU-only, no GPU tensors |

## Reviewer Guidance

- Verify L2 normalization is applied
- Check cache key includes model version
- Confirm background loading doesn't block startup
- Test with e5_small (faster download for testing)
