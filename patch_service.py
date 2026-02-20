import re

content = open('src/embedding/service.rs').read()

# Make sure mrl_dim is captured before thread spawn and used inside
# We removed let mrl_dim = self.config.mrl_dim; with sed globally. Let's restore it before the spawn.

new_content = content.replace(
    "let model = self.config.model;\n        let cache_dir = self.config.cache_dir.clone();",
    "let model = self.config.model;\n        let mrl_dim = self.config.mrl_dim;\n        let cache_dir = self.config.cache_dir.clone();"
)

open('src/embedding/service.rs', 'w').write(new_content)
