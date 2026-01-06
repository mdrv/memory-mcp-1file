use std::path::Path;

use crate::types::{ChunkType, CodeChunk};

use super::scanner::detect_language;

const MAX_CHUNK_LINES: usize = 100;

pub fn chunk_file(path: &Path, content: &str, project_id: &str) -> Vec<CodeChunk> {
    let language = detect_language(path);
    let file_path = path.to_string_lossy().to_string();

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut current_start = 0;

    while current_start < lines.len() {
        let end = (current_start + MAX_CHUNK_LINES).min(lines.len());
        let chunk_lines = &lines[current_start..end];
        let chunk_content = chunk_lines.join("\n");

        if !chunk_content.trim().is_empty() {
            let content_hash = blake3::hash(chunk_content.as_bytes())
                .to_hex()
                .to_string();

            chunks.push(CodeChunk {
                id: None,
                file_path: file_path.clone(),
                content: chunk_content,
                language: language.clone(),
                start_line: (current_start + 1) as u32,
                end_line: end as u32,
                chunk_type: ChunkType::Other,
                name: None,
                embedding: None,
                content_hash,
                project_id: Some(project_id.to_string()),
                indexed_at: chrono::Utc::now(),
            });
        }

        current_start = end;
    }

    chunks
}
