import re

content = open('src/main.rs').read()

old_mrl_help = '#[arg(long, env = "MRL_DIM")]'
new_mrl_help = '#[arg(long, env = "MRL_DIM", help = "MRL output dimension (Qwen3/Gemma only). Defaults to model native dim (1024 for qwen3)")]'

content = content.replace(old_mrl_help, new_mrl_help)

old_log = 'tracing::warn!("Dimension check: {}", e);\n    }'
new_log = 'tracing::warn!("Dimension check: {}", e);\n    }\n\n    tracing::info!(output_dim = embedding_config.output_dim(), model = %embedding_config.model, "Embedding engine configured");'

content = content.replace(old_log, new_log)

open('src/main.rs', 'w').write(content)
