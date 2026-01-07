use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use moka::future::Cache;
use redb::{Database, ReadableDatabase, TableDefinition};

const CACHE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("embeddings");

#[derive(Clone)]
pub struct EmbeddingStore {
    // L1 Cache: RAM (Fastest)
    ram_cache: Cache<String, Vec<f32>>,
    // L2 Cache: Disk (Persistent)
    disk_cache: Arc<Database>,
}

impl EmbeddingStore {
    pub fn new(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("cache.redb");
        let disk_cache = Database::create(db_path)?;

        // Initialize table if not exists
        let write_txn = disk_cache.begin_write()?;
        {
            let _table = write_txn.open_table(CACHE_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self {
            // Store up to 10,000 embeddings in RAM
            ram_cache: Cache::builder().max_capacity(10_000).build(),
            disk_cache: Arc::new(disk_cache),
        })
    }

    pub async fn get(&self, hash: &str) -> Option<Vec<f32>> {
        // 1. Check RAM
        if let Some(vec) = self.ram_cache.get(hash).await {
            return Some(vec);
        }

        // 2. Check Disk
        // redb operations are synchronous (blocking I/O), so strictly speaking
        // they should be wrapped in spawn_blocking for max performance,
        // but redb is very fast (memory mapped), so direct access is often acceptable.
        // For strict correctness in async context:
        let db = self.disk_cache.clone();
        let hash_owned = hash.to_string();

        let vec_opt = tokio::task::spawn_blocking(move || -> Result<Option<Vec<f32>>> {
            let read_txn = db.begin_read()?;
            let table = read_txn.open_table(CACHE_TABLE)?;

            if let Some(value) = table.get(hash_owned.as_str())? {
                let vec: Vec<f32> =
                    bincode::serde::decode_from_slice(value.value(), bincode::config::standard())?
                        .0;
                Ok(Some(vec))
            } else {
                Ok(None)
            }
        })
        .await
        .ok()?
        .ok()?;

        // 3. Populate RAM (Promote to L1)
        if let Some(ref vec) = vec_opt {
            self.ram_cache.insert(hash.to_string(), vec.clone()).await;
        }

        vec_opt
    }

    pub async fn put(&self, hash: String, embedding: Vec<f32>) -> Result<()> {
        // 1. Write to RAM
        self.ram_cache.insert(hash.clone(), embedding.clone()).await;

        // 2. Write to Disk
        let db = self.disk_cache.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let write_txn = db.begin_write()?;
            {
                let mut table = write_txn.open_table(CACHE_TABLE)?;
                let bytes = bincode::serde::encode_to_vec(&embedding, bincode::config::standard())?;
                table.insert(hash.as_str(), bytes.as_slice())?;
            }
            write_txn.commit()?;
            Ok(())
        })
        .await??;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_embedding_store_persistence() {
        let dir = tempdir().unwrap();
        let store = EmbeddingStore::new(dir.path()).unwrap();

        let hash = "abc123hash".to_string();
        let embedding = vec![0.1, 0.2, 0.3];

        // 1. Put
        store.put(hash.clone(), embedding.clone()).await.unwrap();

        // 2. Get (from RAM)
        let retrieved = store.get(&hash).await.unwrap();
        assert_eq!(retrieved, embedding);

        // 3. Re-open to test Disk persistence
        drop(store);
        let store2 = EmbeddingStore::new(dir.path()).unwrap();

        // 4. Get (from Disk)
        let retrieved2 = store2.get(&hash).await.unwrap();
        assert_eq!(retrieved2, embedding);
    }
}
