use std::path::{Path, PathBuf};

use ignore::{overrides::OverrideBuilder, WalkBuilder};

use crate::types::Language;

pub fn scan_directory(root: &Path) -> crate::Result<Vec<PathBuf>> {
    let mut overrides = OverrideBuilder::new(root);
    let _ = overrides.add("!**/*.g.dart");
    let _ = overrides.add("!**/*.freezed.dart");
    let _ = overrides.add("!**/*.gr.dart");
    let _ = overrides.add("!**/*.config.dart");
    let _ = overrides.add("!**/*.mocks.dart");
    let _ = overrides.add("!**/*.arb");
    let _ = overrides.add("!build/**");
    let _ = overrides.add("!.dart_tool/**");
    let _ = overrides.add("!dist/**");
    let _ = overrides.add("!out/**");
    let _ = overrides.add("!target/**");
    let _ = overrides.add("!node_modules/**");

    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .add_custom_ignore_filename(".memoryignore")
        .overrides(
            overrides
                .build()
                .unwrap_or_else(|_| ignore::overrides::Override::empty()),
        )
        .build();

    let mut files = Vec::new();
    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && !is_ignored_file(path) && is_code_file(path) {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

pub fn is_ignored_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    if path_str.contains("/node_modules/")
        || path_str.contains("\\node_modules\\")
        || path_str.contains("/target/")
        || path_str.contains("\\target\\")
        || path_str.contains("/build/")
        || path_str.contains("\\build\\")
        || path_str.contains("/.dart_tool/")
        || path_str.contains("\\.dart_tool\\")
        || path_str.contains("/dist/")
        || path_str.contains("\\dist\\")
        || path_str.contains("/out/")
        || path_str.contains("\\out\\")
    {
        return true;
    }

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    if name.starts_with('.') && name != "." {
        return true;
    }

    matches!(
        name.as_str(),
        "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "composer.lock"
            | "cargo.lock"
            | "pubspec.lock"
            | "gemfile.lock"
            | "poetry.lock"
    ) || name.ends_with(".g.dart")
        || name.ends_with(".freezed.dart")
        || name.ends_with(".gr.dart")
        || name.ends_with(".config.dart")
        || name.ends_with(".mocks.dart")
        || name.ends_with(".arb")
        || name.ends_with(".min.js")
        || name.ends_with(".min.css")
        || name.ends_with(".bundle.js")
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
            | "dart"
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
        "dart" => Language::Dart,
        _ => Language::Unknown,
    }
}
