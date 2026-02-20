import re
content = open('src/embedding/config.rs').read()

# remove old manual Default impl
old_impl = """impl Default for ModelType {
    fn default() -> Self {
        Self::Qwen3
    }
}"""
content = content.replace(old_impl, "")

# add derive default to the enum
content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq)]", "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]")

# add #[default] to Qwen3
content = content.replace("    Qwen3,", "    #[default]\n    Qwen3,")

open('src/embedding/config.rs', 'w').write(content)
