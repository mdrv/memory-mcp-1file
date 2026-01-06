# Embedding Pipeline — Design Document

## Overview

Pure Rust embedding engine using Candle. No ONNX, no Python dependencies.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    EmbeddingService                          │
├─────────────────────────────────────────────────────────────┤
│ Startup: Background preload (non-blocking)                  │
│ L1 Cache: LRU in-memory (1000 items, search queries)        │
│ L2 Cache: SurrealDB embedding field (persistent)            │
│ Cache Key: blake3(normalize(text) || model_version)         │
│ Batching: 32 default, 256 max                               │
│ Pooling: Mean pooling                                       │
│ Recovery: Auto-reload on transient errors                   │
└─────────────────────────────────────────────────────────────┘
```

## Components

### EmbeddingConfig

```rust
pub struct EmbeddingConfig {
    pub model: ModelType,
    pub cache_size: usize,      // default: 1000
    pub batch_size: usize,      // default: 32
    pub timeout_secs: u64,      // default: 60
    pub preload: bool,          // default: true
}

pub enum ModelType {
    E5Small,    // 384 dims, ~134 MB
    E5Multi,    // 768 dims, ~1.1 GB (default)
    Nomic,      // 768 dims, ~1.9 GB
    BgeM3,      // 1024 dims, ~2.3 GB
}
```

### EmbeddingEngine (Sync, CPU-bound)

```rust
pub struct EmbeddingEngine {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    config: EmbeddingConfig,
}

impl EmbeddingEngine {
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let device = Device::Cpu;  // GPU optional
        let (model, tokenizer) = load_model(&config.model)?;
        Ok(Self { model, tokenizer, device, config })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self.tokenizer.encode(text, true)?;
        let input_ids = Tensor::new(tokens.get_ids(), &self.device)?;
        let attention_mask = Tensor::new(tokens.get_attention_mask(), &self.device)?;
        
        let embeddings = self.model.forward(&input_ids, &attention_mask)?;
        let pooled = mean_pooling(&embeddings, &attention_mask)?;
        let normalized = normalize(&pooled)?;
        
        normalized.to_vec1()
    }
    
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Batch processing with padding
    }
}
```

### EmbeddingCache (LRU)

```rust
pub struct EmbeddingCache {
    cache: RwLock<LruCache<String, Vec<f32>>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl EmbeddingCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: RwLock::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }
    
    pub fn get(&self, key: &str) -> Option<Vec<f32>> {
        let mut cache = self.cache.write().unwrap();
        if let Some(v) = cache.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(v.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
    
    pub fn insert(&self, key: String, value: Vec<f32>) {
        let mut cache = self.cache.write().unwrap();
        cache.put(key, value);
    }
    
    fn cache_key(text: &str, model: &str) -> String {
        let normalized = text.trim().to_lowercase();
        let hash = blake3::hash(format!("{}:{}", normalized, model).as_bytes());
        hash.to_hex().to_string()
    }
}
```

### EmbeddingService (Async wrapper)

```rust
pub struct EmbeddingService {
    engine: Arc<RwLock<Option<EmbeddingEngine>>>,
    cache: Arc<EmbeddingCache>,
    config: EmbeddingConfig,
    status: Arc<AtomicU8>,  // 0=Loading, 1=Ready, 2=Error
}

impl EmbeddingService {
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        let service = Self {
            engine: Arc::new(RwLock::new(None)),
            cache: Arc::new(EmbeddingCache::new(config.cache_size)),
            config: config.clone(),
            status: Arc::new(AtomicU8::new(0)),
        };
        
        if config.preload {
            service.preload_background().await;
        }
        
        Ok(service)
    }
    
    async fn preload_background(&self) {
        let engine = self.engine.clone();
        let config = self.config.clone();
        let status = self.status.clone();
        
        tokio::task::spawn_blocking(move || {
            match EmbeddingEngine::new(config) {
                Ok(e) => {
                    *engine.write().unwrap() = Some(e);
                    status.store(1, Ordering::Release);
                    tracing::info!("Embedding model loaded");
                }
                Err(e) => {
                    status.store(2, Ordering::Release);
                    tracing::error!("Failed to load model: {}", e);
                }
            }
        });
    }
    
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        let key = EmbeddingCache::cache_key(text, &self.config.model.name());
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached);
        }
        
        // Wait for model if loading
        self.wait_ready().await?;
        
        // Compute embedding
        let engine = self.engine.clone();
        let text = text.to_string();
        let embedding = tokio::task::spawn_blocking(move || {
            let guard = engine.read().unwrap();
            guard.as_ref().unwrap().embed(&text)
        }).await??;
        
        // Cache result
        self.cache.insert(key, embedding.clone());
        
        Ok(embedding)
    }
    
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.wait_ready().await?;
        
        let engine = self.engine.clone();
        let texts = texts.to_vec();
        
        tokio::task::spawn_blocking(move || {
            let guard = engine.read().unwrap();
            let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
            guard.as_ref().unwrap().embed_batch(&refs)
        }).await?
    }
    
    async fn wait_ready(&self) -> Result<()> {
        let timeout = Duration::from_secs(self.config.timeout_secs);
        let start = Instant::now();
        
        loop {
            match self.status.load(Ordering::Acquire) {
                1 => return Ok(()),
                2 => return Err(anyhow!("Model failed to load")),
                _ => {
                    if start.elapsed() > timeout {
                        return Err(anyhow!("Model load timeout"));
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }
}
```

## Model Loading

Models are downloaded from HuggingFace Hub on first use:

```rust
fn load_model(model_type: &ModelType) -> Result<(BertModel, Tokenizer)> {
    let repo = match model_type {
        ModelType::E5Small => "intfloat/multilingual-e5-small",
        ModelType::E5Multi => "intfloat/multilingual-e5-base",
        ModelType::Nomic => "nomic-ai/nomic-embed-text-v1.5",
        ModelType::BgeM3 => "BAAI/bge-m3",
    };
    
    let api = hf_hub::api::sync::Api::new()?;
    let model_repo = api.model(repo.to_string());
    
    let config_path = model_repo.get("config.json")?;
    let tokenizer_path = model_repo.get("tokenizer.json")?;
    let weights_path = model_repo.get("model.safetensors")?;
    
    let config: BertConfig = serde_json::from_str(&fs::read_to_string(config_path)?)?;
    let tokenizer = Tokenizer::from_file(tokenizer_path)?;
    
    let vb = VarBuilder::from_safetensors(&[weights_path], DType::F32, &Device::Cpu)?;
    let model = BertModel::load(vb, &config)?;
    
    Ok((model, tokenizer))
}
```

## Pooling Strategy

Mean pooling with attention mask:

```rust
fn mean_pooling(embeddings: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
    let mask = attention_mask.unsqueeze(2)?;
    let masked = embeddings.broadcast_mul(&mask)?;
    let sum = masked.sum(1)?;
    let count = mask.sum(1)?.clamp(1e-9, f64::MAX)?;
    sum.broadcast_div(&count)
}

fn normalize(tensor: &Tensor) -> Result<Tensor> {
    let norm = tensor.sqr()?.sum_keepdim(1)?.sqrt()?;
    tensor.broadcast_div(&norm)
}
```

## Error Recovery

```rust
enum EmbeddingState {
    Loading,
    Ready,
    Recovering,
    Failed(String),
}

impl EmbeddingService {
    async fn recover(&self) {
        self.status.store(0, Ordering::Release);  // Loading
        self.preload_background().await;
    }
}
```

## Performance Considerations

1. **CPU vs GPU**: Default is CPU. GPU requires `candle-core/cuda` feature.
2. **Batch size**: 32 is optimal for CPU. Increase for GPU.
3. **Cache hit rate**: Target >80% for search queries.
4. **Model size**: e5_small for constrained environments, e5_multi for quality.

## Token Protection

Embeddings are NEVER serialized in MCP responses:

```rust
#[serde(skip_serializing)]
pub embedding: Option<Vec<f32>>,
```

Savings: ~3KB per result (768 floats × 4 bytes).
