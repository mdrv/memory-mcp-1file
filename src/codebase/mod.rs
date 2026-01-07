pub mod chunker;
pub mod indexer;
pub mod manager;
pub mod scanner;
pub mod watcher;

pub use indexer::{incremental_index, index_project};
pub use manager::CodebaseManager;
pub use scanner::{detect_language, is_code_file, scan_directory};
pub use watcher::FileWatcher;
