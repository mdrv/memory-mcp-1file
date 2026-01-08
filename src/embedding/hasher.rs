pub struct ContentHasher;

impl ContentHasher {
    pub fn hash(content: &str) -> String {
        blake3::hash(content.as_bytes()).to_hex().to_string()
    }

    pub fn needs_reembed(old_hash: Option<&str>, new_content: &str) -> bool {
        let new_hash = Self::hash(new_content);
        match old_hash {
            Some(old) => old != new_hash,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let h1 = ContentHasher::hash("hello");
        let h2 = ContentHasher::hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_needs_reembed_none() {
        assert!(ContentHasher::needs_reembed(None, "content"));
    }

    #[test]
    fn test_needs_reembed_same() {
        let hash = ContentHasher::hash("content");
        assert!(!ContentHasher::needs_reembed(Some(&hash), "content"));
    }

    #[test]
    fn test_needs_reembed_different() {
        let hash = ContentHasher::hash("old");
        assert!(ContentHasher::needs_reembed(Some(&hash), "new"));
    }
}
