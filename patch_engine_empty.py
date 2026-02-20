import re

content = open('src/embedding/engine.rs').read()

new_content = content.replace(
    "let embedding = hidden.narrow(1, actual_len - 1, 1)?.squeeze(1)?;",
    """if actual_len == 0 { return Err(anyhow::anyhow!("Cannot embed empty token sequence")); }
                    let embedding = hidden.narrow(1, actual_len - 1, 1)?.squeeze(1)?;"""
)

open('src/embedding/engine.rs', 'w').write(new_content)
