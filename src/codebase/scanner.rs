use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::types::Language;

pub fn scan_directory(root: &Path) -> crate::Result<Vec<PathBuf>> {
    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .add_custom_ignore_filename(".memoryignore")
        .build();

    let mut files = Vec::new();
    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && is_code_file(path) {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

pub fn is_code_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };

    matches!(
        ext.to_lowercase().as_str(),
        "rs" | "py"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "go"
            | "java"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "rb"
            | "php"
            | "swift"
            | "kt"
            | "scala"
            | "sh"
            | "bash"
            | "zsh"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "xml"
            | "md"
    )
}

pub fn detect_language(path: &Path) -> Language {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Language::Unknown;
    };

    match ext.to_lowercase().as_str() {
        "rs" => Language::Rust,
        "py" => Language::Python,
        "js" | "jsx" => Language::JavaScript,
        "ts" | "tsx" => Language::TypeScript,
        "go" => Language::Go,
        "java" => Language::Java,
        _ => Language::Unknown,
    }
}
