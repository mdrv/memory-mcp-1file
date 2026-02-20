import re

readme = open('README.md').read()

old_config_table = """| Arg | Env | Default | Description |
|-----|-----|---------|-------------|
| `--data-dir` | `DATA_DIR` | `./data` | DB location |
| `--model` | `EMBEDDING_MODEL` | `e5_multi` | Embedding model (`e5_small`, `e5_multi`, `nomic`, `bge_m3`) |
| `--log-level` | `LOG_LEVEL` | `info` | Verbosity |"""

new_config_table = """| Arg | Env | Default | Description |
|-----|-----|---------|-------------|
| `--data-dir` | `DATA_DIR` | `./data` | DB location |
| `--model` | `EMBEDDING_MODEL` | `qwen3` | Embedding model (`qwen3`, `gemma`, `bge_m3`, `nomic`, `e5_multi`, `e5_small`) |
| `--mrl-dim` | `MRL_DIM` | *(native)* | Output dimension for MRL-supported models (e.g. 64, 128, 256, 512, 1024 for Qwen3). Defaults to the model's native maximum dimension (1024 for Qwen3). |
| `--batch-size` | `BATCH_SIZE` | `8` | Maximum batch size for embedding inference |
| `--cache-size` | `CACHE_SIZE` | `1000` | LRU cache capacity for embeddings |
| `--timeout` | `TIMEOUT_MS` | `30000` | Timeout in milliseconds |
| `--idle-timeout` | `IDLE_TIMEOUT` | `0` | Idle timeout in minutes. 0 = disabled |
| `--log-level` | `LOG_LEVEL` | `info` | Verbosity |"""

old_models_table = """| Argument Value | HuggingFace Repo | Dimensions | Size | Use Case |
| :--- | :--- | :--- | :--- | :--- |
| `e5_small` | `intfloat/multilingual-e5-small` | 384 | 134 MB | Fastest, minimal RAM. Good for dev/testing. |
| `e5_multi` | `intfloat/multilingual-e5-base` | 768 | 1.1 GB | **Default**. Best balance of quality/speed. |
| `nomic` | `nomic-ai/nomic-embed-text-v1.5` | 768 | 1.9 GB | High quality long-context embeddings. |
| `bge_m3` | `BAAI/bge-m3` | 1024 | 2.3 GB | State-of-the-art multilingual quality. Heavy. |"""

new_models_table = """| Argument Value | HuggingFace Repo | Dimensions | Size | Use Case |
| :--- | :--- | :--- | :--- | :--- |
| `qwen3` | `Qwen/Qwen3-Embedding-0.6B` | 1024 (MRL) | 1.2 GB | **Default**. Top open-source 2026 model, 32K context, MRL support. |
| `gemma` | `onnx-community/embeddinggemma-300m-ONNX` | 768 (MRL) | ~195 MB | Lighter alternative with MRL support. (Requires proprietary license agreement) |
| `bge_m3` | `BAAI/bge-m3` | 1024 | 2.3 GB | State-of-the-art multilingual hybrid retrieval. Heavy. |
| `nomic` | `nomic-ai/nomic-embed-text-v1.5` | 768 | 1.9 GB | High quality long-context BERT-compatible. |
| `e5_multi` | `intfloat/multilingual-e5-base` | 768 | 1.1 GB | Legacy; kept for backward compatibility. |
| `e5_small` | `intfloat/multilingual-e5-small` | 384 | 134 MB | Fastest, minimal RAM. Good for dev/testing. |

### 📉 Matryoshka Representation Learning (MRL)

Models marked with **(MRL)** support dynamically truncating the output embedding vector to a smaller dimension (e.g., 512, 256, 128) with minimal loss of accuracy. This saves database storage and speeds up vector search.

Use the `--mrl-dim` argument to specify the desired size. If omitted, the default is the model's native base dimension (e.g., 1024 for Qwen3).

**Warning:** Once your database is created with a specific dimension, you cannot change it without wiping the data directory."""

readme = readme.replace(old_config_table, new_config_table)
readme = readme.replace(old_models_table, new_models_table)

open('README.md', 'w').write(readme)
