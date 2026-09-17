# Choosing an engine

Langolier separates the brain from the memory. The model that writes answers and the
model that turns text into vectors are configured independently, so a cloud chat model
can sit next to a local embedder.

## The embedded engine

llama.cpp, built into the application. Pick a curated tag under **Engines**, press
Install, and the GGUF lands in the application cache.

| Model | Size | Notes |
|---|---|---|
| `qwen3:4b-instruct` | 2.5 GB | Recommended, around 30 tokens per second |
| `qwen3:8b` | 5.0 GB | Surer, around 17 tokens per second |
| `qwen3:1.7b` | 1.4 GB | Very fast, may drift into English |

Any GGUF path works too. Weights already pulled by Ollama are linked into the cache
rather than downloaded again.

Idle models are unloaded after a configurable pause, five minutes by default, so memory
comes back without quitting the application.

Prefer `qwen3:4b-instruct` over the bare `qwen3:4b`: the latter reasons out loud and lets
its thinking leak into the answer.

## Ollama, LM Studio and compatible servers

Point the address at a local server. Ollama on `http://127.0.0.1:11434`, LM Studio on its
own port. The model has to be pulled on that side first.

If you use Ollama for embeddings, it has to be running when Langolier searches, or
hybrid search falls back to words only. The embedded engine has no such dependency,
since it starts with the application.

## A cloud provider

Supported providers are listed in the settings. Your API key is stored in the
application database, and your questions and the retrieved passages travel to the
provider. The interface states this next to the setting.

Token caps on an assistant profile bound the bill.

## Changing the embedding model

Vectors produced by one model cannot be compared with vectors produced by another.
Langolier refuses to mix them: it says so, falls back to word search, and offers to
reindex the sources.

Word search keeps working throughout, so the application stays usable while vectors are
rebuilt.
