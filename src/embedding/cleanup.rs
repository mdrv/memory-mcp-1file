//! Cleanup utilities for stale lock files and incomplete downloads.
//!
//! HuggingFace Hub uses `.lock` files for atomic downloads. When a process
//! crashes or is killed, these locks can become stale and block future downloads.
//! This module provides utilities to detect and clean up such artifacts.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use super::config::ModelType;

/// Result of cleanup operation
#[derive(Debug, Default)]
pub struct CleanupResult {
    /// Number of stale lock files removed
    pub locks_removed: usize,
    /// Number of incomplete files removed
    pub incomplete_removed: usize,
    /// Errors encountered (non-fatal)
    pub errors: Vec<String>,
}

impl CleanupResult {
    pub fn is_empty(&self) -> bool {
        self.locks_removed == 0 && self.incomplete_removed == 0
    }
}

/// Configuration for cleanup behavior
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// Lock files older than this are considered stale (default: 5 minutes)
    pub stale_threshold: Duration,
    /// Whether to try file locking to detect active locks
    pub use_flock: bool,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            stale_threshold: Duration::from_secs(5 * 60), // 5 minutes
            use_flock: true,
        }
    }
}

/// Clean up stale artifacts for a specific model in the cache directory.
///
/// This should be called before attempting to load a model to ensure
/// no stale locks block the download process.
pub fn cleanup_model_cache(
    cache_dir: &Path,
    model: ModelType,
    config: &CleanupConfig,
) -> CleanupResult {
    let mut result = CleanupResult::default();

    if model == ModelType::Mock {
        return result;
    }

    // HuggingFace Hub stores models in: {cache_dir}/models--{org}--{repo}/blobs/
    let repo_id = model.repo_id();
    let repo_dir_name = format!("models--{}", repo_id.replace('/', "--"));
    let blobs_dir = cache_dir.join(&repo_dir_name).join("blobs");

    if !blobs_dir.exists() {
        tracing::debug!("Blobs directory does not exist: {:?}", blobs_dir);
        return result;
    }

    tracing::info!("Checking for stale artifacts in {:?}", blobs_dir);

    // Scan for lock files and incomplete downloads
    match fs::read_dir(&blobs_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(".lock") {
                        if should_remove_lock(&path, config) {
                            if let Err(e) = fs::remove_file(&path) {
                                result
                                    .errors
                                    .push(format!("Failed to remove lock file {:?}: {}", path, e));
                            } else {
                                tracing::info!("Removed stale lock file: {:?}", path);
                                result.locks_removed += 1;
                            }
                        }
                    } else if file_name.ends_with(".incomplete") {
                        if let Err(e) = fs::remove_file(&path) {
                            result.errors.push(format!(
                                "Failed to remove incomplete file {:?}: {}",
                                path, e
                            ));
                        } else {
                            tracing::info!("Removed incomplete file: {:?}", path);
                            result.incomplete_removed += 1;
                        }
                    }
                }
            }
        }
        Err(e) => {
            result
                .errors
                .push(format!("Failed to read blobs directory: {}", e));
        }
    }

    // Also check snapshots directory for incomplete refs
    let snapshots_dir = cache_dir.join(&repo_dir_name).join("snapshots");
    cleanup_incomplete_snapshots(&snapshots_dir, &mut result);

    if !result.is_empty() {
        tracing::info!(
            "Cleanup complete: removed {} lock files, {} incomplete files",
            result.locks_removed,
            result.incomplete_removed
        );
    }

    result
}

/// Determine if a lock file should be removed.
fn should_remove_lock(path: &Path, config: &CleanupConfig) -> bool {
    // Strategy 1: Check file age
    if is_lock_stale_by_age(path, config.stale_threshold) {
        tracing::debug!("Lock file {:?} is stale by age", path);
        return true;
    }

    // Strategy 2: Try to acquire exclusive lock (if enabled)
    if config.use_flock && is_lock_stale_by_flock(path) {
        tracing::debug!("Lock file {:?} is stale by flock test", path);
        return true;
    }

    false
}

/// Check if lock file is older than threshold
fn is_lock_stale_by_age(path: &Path, threshold: Duration) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => match metadata.modified() {
            Ok(modified) => match SystemTime::now().duration_since(modified) {
                Ok(age) => age > threshold,
                Err(_) => false, // Clock went backwards, be conservative
            },
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Try to acquire an exclusive lock on the file.
/// If we can acquire it, the lock is stale (no other process holds it).
#[cfg(unix)]
fn is_lock_stale_by_flock(path: &Path) -> bool {
    use std::fs::OpenOptions;
    use std::os::unix::io::AsRawFd;

    let file = match OpenOptions::new().read(true).open(path) {
        Ok(f) => f,
        Err(_) => return false, // Can't open, be conservative
    };

    let fd = file.as_raw_fd();

    // Try non-blocking exclusive lock
    // LOCK_EX | LOCK_NB = exclusive + non-blocking
    let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };

    if result == 0 {
        // We got the lock, which means no one else holds it → stale
        // Unlock before returning
        unsafe { libc::flock(fd, libc::LOCK_UN) };
        true
    } else {
        // Lock is held by another process
        false
    }
}

#[cfg(not(unix))]
fn is_lock_stale_by_flock(_path: &Path) -> bool {
    // On non-Unix systems, fall back to age-based detection only
    false
}

/// Clean up incomplete snapshot references
fn cleanup_incomplete_snapshots(snapshots_dir: &Path, result: &mut CleanupResult) {
    if !snapshots_dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(snapshots_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check for .incomplete marker files inside snapshot dirs
                if let Ok(files) = fs::read_dir(&path) {
                    for file in files.flatten() {
                        let file_path = file.path();
                        if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
                            if name.ends_with(".incomplete") {
                                if let Err(e) = fs::remove_file(&file_path) {
                                    result.errors.push(format!(
                                        "Failed to remove incomplete snapshot file {:?}: {}",
                                        file_path, e
                                    ));
                                } else {
                                    tracing::debug!(
                                        "Removed incomplete snapshot file: {:?}",
                                        file_path
                                    );
                                    result.incomplete_removed += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_structure(temp: &TempDir, model: ModelType) -> PathBuf {
        let repo_id = model.repo_id();
        let repo_dir_name = format!("models--{}", repo_id.replace('/', "--"));
        let blobs_dir = temp.path().join(&repo_dir_name).join("blobs");
        fs::create_dir_all(&blobs_dir).unwrap();
        blobs_dir
    }

    #[test]
    fn test_cleanup_empty_dir() {
        let temp = TempDir::new().unwrap();
        let config = CleanupConfig::default();
        let result = cleanup_model_cache(temp.path(), ModelType::E5Multi, &config);
        assert!(result.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_cleanup_removes_old_locks() {
        let temp = TempDir::new().unwrap();
        let blobs_dir = create_test_structure(&temp, ModelType::E5Multi);

        // Create a lock file
        let lock_path = blobs_dir.join("test.lock");
        fs::File::create(&lock_path).unwrap();

        // Set modification time to 10 minutes ago
        let old_time =
            filetime::FileTime::from_system_time(SystemTime::now() - Duration::from_secs(10 * 60));
        filetime::set_file_mtime(&lock_path, old_time).unwrap();

        let config = CleanupConfig {
            stale_threshold: Duration::from_secs(5 * 60),
            use_flock: false, // Don't use flock in test
        };

        let result = cleanup_model_cache(temp.path(), ModelType::E5Multi, &config);
        assert_eq!(result.locks_removed, 1);
        assert!(!lock_path.exists());
    }

    #[test]
    fn test_cleanup_keeps_fresh_locks() {
        let temp = TempDir::new().unwrap();
        let blobs_dir = create_test_structure(&temp, ModelType::E5Multi);

        // Create a fresh lock file
        let lock_path = blobs_dir.join("fresh.lock");
        fs::File::create(&lock_path).unwrap();

        let config = CleanupConfig {
            stale_threshold: Duration::from_secs(5 * 60),
            use_flock: false,
        };

        let result = cleanup_model_cache(temp.path(), ModelType::E5Multi, &config);
        assert_eq!(result.locks_removed, 0);
        assert!(lock_path.exists());
    }

    #[test]
    fn test_cleanup_removes_incomplete_files() {
        let temp = TempDir::new().unwrap();
        let blobs_dir = create_test_structure(&temp, ModelType::E5Multi);

        // Create incomplete files
        let incomplete_path = blobs_dir.join("model.safetensors.incomplete");
        let mut f = fs::File::create(&incomplete_path).unwrap();
        f.write_all(b"partial data").unwrap();

        let config = CleanupConfig::default();
        let result = cleanup_model_cache(temp.path(), ModelType::E5Multi, &config);

        assert_eq!(result.incomplete_removed, 1);
        assert!(!incomplete_path.exists());
    }

    #[test]
    fn test_mock_model_skips_cleanup() {
        let temp = TempDir::new().unwrap();
        let config = CleanupConfig::default();
        let result = cleanup_model_cache(temp.path(), ModelType::Mock, &config);
        assert!(result.is_empty());
    }
}
