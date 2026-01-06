pub mod chunker;
pub mod indexer;
pub mod scanner;
pub mod watcher;

pub use indexer::index_project;
pub use scanner::{detect_language, is_code_file, scan_directory};
pub use watcher::FileWatcher;
