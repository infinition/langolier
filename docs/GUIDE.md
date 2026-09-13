<div align="center">

<img src="langolier.png" alt="Langolier" width="120">

# Langolier guide

</div>

Reference manual. For a quick start, read the [README](../README.md).

- [Concepts](#concepts)
- [Ingestion](#ingestion)
- [Retrieval and tuning](#retrieval-and-tuning)
- [Engines](#engines)
- [Embeddings and reindexing](#embeddings-and-reindexing)
- [Watches](#watches)
- [Assistants](#assistants)
- [Export](#export)
- [Server mode](#server-mode)
- [Docker](#docker)
- [Telegram](#telegram)
- [Data and backup](#data-and-backup)
- [Troubleshooting](#troubleshooting)
- [Development](#development)

## Concepts

A **source** is one ingested file, pasted note or video link. Ingestion splits it into **passages** of about 1400 characters with 200 of overlap, each keeping a **locator** such as `Page 4` or `Slide 7` so a citation points somewhere real.

Each passage carries a **vector** produced by the embedding model, stored as a BLOB next to the text. Vectors record which model made them and are never mixed across models.

An **assistant** is a profile: a name, a mission, a knowledge scope and a set of settings that override the global ones. A **watch** is a folder Langolier keeps an eye on.

## Ingestion

Supported extensions:

```
txt md pdf docx pptx xlsx odt odp ods srt vtt
py rs js ts tsx jsx json csv toml yaml yml html css sql r jl ipynb
mp4 mkv mov webm avi m4v
mp3 wav m4a flac ogg
```

Office files are zip archives holding XML, read directly with no external tool. Word keeps text in `<w:t>`, PowerPoint in `<a:t>` with one section per slide, Excel through the shared string table with one section per sheet and tab separated rows. OpenDocument writes its text straight inside paragraphs in `content.xml`.

PDFs go through `pdf-extract`, one section per page. A page holding fewer than 30 alphanumeric characters is rendered and passed to Tesseract, and the excerpt keeps its page number and an OCR marker.

Video and audio are demuxed by FFmpeg then transcribed by `whisper-cli` with automatic language detection. Timestamps become locators. Video links are fetched with `yt-dlp`.

Limits: 256 MB per file, 20 MB of extracted text per source, 10000 pages per PDF. Deduplication is by content hash, so the same file added twice is ingested once.

## Retrieval and tuning

Every question runs two searches in parallel:

1. **Lexical.** SQLite FTS5 with `unicode61 remove_diacritics 2`, ranked by BM25.
2. **Dense.** Cosine similarity over the stored vectors.

Both lists are merged with Reciprocal Rank Fusion, then filtered for document diversity so one long file cannot take every slot.

Settings under **Engines**:

| Setting            | Default | Effect                                            |
| ------------------ | ------- | ------------------------------------------------- |
| Passages retrieved | 6       | How many passages reach the model                 |
| Minimum cosine     | 0.35    | Below this a dense hit does not count             |
| Minimum BM25       | 0       | Positive value drops weak lexical hits            |
| Relevance judge    | on      | A model scores each candidate from 0 to 1         |
| Judge threshold    | 0.35    | Under it the assistant abstains before writing    |
| Verify answer      | off     | A second call checks each claim, doubling latency |
| Strict grounding   | on      | Answer only from passages, otherwise abstain      |

The cosine floor matters more than it looks. EmbeddingGemma scores unrelated text around 0.19 to 0.26, so a floor below 0.3 lets noise through and the model answers from its own knowledge instead of abstaining.

**Modes.** Hybrid uses both searches. Semantic and lexical use one each. Exact serves only passages containing the literal phrase, with no judge. Free conversation skips retrieval entirely and is the only mode that is not grounded.

**Deep search** rewrites the question using the conversation history and merges two retrievals. It costs one extra model call.

## Engines

| Provider             | Endpoint                      | Notes                                         |
| -------------------- | ----------------------------- | --------------------------------------------- |
| Embedded (default)   | a curated tag or a GGUF path  | llama.cpp compiled in, Metal on Apple Silicon |
| Ollama               | `http://127.0.0.1:11434`      | Local daemon, optional                        |
| LM Studio, llama.cpp | `http://127.0.0.1:1234/v1`    | Any local OpenAI-compatible server            |
| OpenAI               | `https://api.openai.com/v1`   | Reasoning models refuse a forced temperature  |
| DeepSeek             | `https://api.deepseek.com/v1` |                                               |
| Anthropic            | `https://api.anthropic.com`   | Messages API, system prompt sent separately   |
| Custom               | your URL                      | Mistral, Groq, OpenRouter and similar         |

Remote providers require HTTPS. Local addresses stay allowed over plain HTTP. URLs carrying credentials or query parameters are rejected.

The context window and the maximum answer length are set per provider. With 24 GB of unified memory and an 8B model quantised to Q4, 8192 tokens of context is comfortable. LM Studio manages its own window inside the server.

Thinking models need care. Qwen3 variants receive `/no_think`, and anything the model still emits between `<think>` and `</think>` is filtered from the stream rather than shown.

### The embedded engine

Three chat tags are curated, each with a downloadable GGUF: `qwen3:8b` (5.0 GB, Q4_K_M, official Qwen build), `qwen3:4b-instruct` (2.5 GB, Q4_K_M, unsloth build since Qwen publishes no GGUF for it) and `qwen3:1.7b` (1.8 GB, Q8_0, official). The four embedding tags have theirs too. **Engines > Install the model** downloads a tag into `<data>/gguf/`; when Ollama already holds the same weights they are hard linked from its blob store instead of downloaded again.

Tags resolve to the cache, absolute paths are used as is, and relative paths are looked up next to the launcher, which is how exported kits find `models/chat.gguf`.

Models load on first use, in a few seconds from an SSD, and stay resident until **Unload idle models after** elapses (5 minutes by default, 0 keeps them forever). A 4B chat model plus EmbeddingGemma take about 3 GB resident. Requests are served one at a time on a dedicated thread; a long ingestion batch can delay a question by a few seconds.

Ollama remains a choice for people who already run it or share it between tools. It offers nothing the embedded engine lacks for Langolier: same llama.cpp, same Metal, same GGUF.

## Embeddings and reindexing

The embedding server is configured separately from the chat engine, so a cloud chat provider can sit next to a local embedder. Set the embedding endpoint to `embedded` to use a local GGUF instead of Ollama.

Changing the embedding model invalidates every vector. The app offers a reindex immediately, and **Engines > Reindex sources** reopens that dialog. Pick by type, by folder in the tree, or both. Reindexing rereads the original files, so they must still be where they were ingested from. Word search keeps working throughout.

Partial coverage is reported rather than hidden: if some passages were vectorised by another model, the answer carries a warning naming how many are compatible.

## Watches

A watch points at a folder. Two modes:

- **Sync** leaves files where they are and reprocesses the ones that change.
- **Hoover** moves each file into Langolier's data folder then indexes it. The watched folder empties and the original is kept for a future reindex.

Each watch can include subfolders, carry its own interval, or follow the global one. An interval of `0` means global. Hidden folders are skipped, except the watch root itself when it starts with a dot.

Network volumes are polled rather than notified. An unmounted volume is flagged on the card and picked up again when it reappears. A file touched within the last few seconds is left alone in case it is still being copied.

**Pause** stops ingestion without stopping detection. The queue keeps growing and empties when you resume. Useful when the machine is busy.

## Assistants

A profile holds its identity (name, mission, welcome message, avatar, theme) and its knowledge scope, which is either the whole memory or an explicit selection of watches and documents. Checking a watch brings everything it indexes, including files that arrive later.

Per profile overrides: provider, model, temperature, passages retrieved, abstention sentence, strict grounding, relevance judge. Anything left empty falls back to the global setting.

Two privacy controls:

- **Show sources** decides whether the exported chatbot lists cited passages under its answers.
- **Hide source file names** replaces every name with `Source 1`, `Source 2` and so on, for the model and for the page. Your document names cannot appear in an answer.

Two caps, both counted at roughly four characters per token: per conversation and per day across all conversations. Past a cap the chatbot replies with a fixed message without calling the model, which bounds an API bill.

## Export

**Export .langolier** writes the bundle alone: profile, settings and knowledge in one file. Drop it next to an already deployed launcher to update that bot. It is picked up in under 30 seconds, without a restart, and conversations are kept.

**Export chatbot** writes a ready-to-run folder:

```
my-assistant/
  my-assistant.langolier
  My assistant.app          window launcher (macOS) or chatbotgui
  My assistant Web.app      browser launcher or chatbotweb
  server.sh / server.bat    headless, port 8080
  models/                   GGUF files, embedded engine only
  README.md
```

Engine choice at export time:

| Choice             | Result                                                       |
| ------------------ | ------------------------------------------------------------ |
| Follow the profile | Cloud key embedded, or Ollama expected on the target machine |
| Complete           | llama.cpp plus GGUF models bundled, no prerequisites         |
| Complete light     | Same with `qwen3:4b-instruct`, about 2.9 GB total            |

A complete export re-vectorises the knowledge with the embedded engine so the bundle is self-consistent. A cloud profile bundles the embedder only.

Launchers are platform specific, the bundle is universal. Build launchers for other systems from the release workflow and drop them next to the `.langolier`.

An exported cloud profile carries its API key inside the bundle in clear text. Treat that file as a secret.

## Remote administration

An exported chatbot can be updated from its own page, for hosts where dropping a file is impractical: a NAS, a VPS, a Docker volume mounted read only.

Turn it on in the profile before exporting: **Remote administration**, a secret of at least 12 characters (the Generate button makes one), and two switches for import and restore. The secret is stored as a SHA-256 hash, both in the app and in the bundle; it is shown once.

On the chat page a small gear appears in the footer. With the secret:

- **Import** uploads a `.langolier` and installs it. A bundle that needs an embedding model absent from the host is refused with the reason; the page offers to continue with word search only.
- **Restore** lists every bundle the chatbot ever ran, named, timestamped and sized, marks the running one and the one the kit shipped with, and installs the selected one.
- **Delete** removes a stored version. The running one and the kit's own bundle cannot be deleted.

Rules worth knowing:

- Every install, whether by file drop, import or restore, lands in `<state>/bundles/`. The store keeps twenty entries, deduplicated by content, and never prunes the running one or the kit's.
- Newest wins. A file dropped in the bundle folder after an import is picked up on the next 20 second check, exactly as before.
- Conversations are kept across imports and restores. Telegram chats bound to an assistant that is no longer present start over.
- Administration belongs to the deployment, not to the content. Importing or restoring a bundle exported without administration does not lock you out; a bundle that brings its own secret replaces the current one, which is how a secret gets rotated.
- Five wrong secrets pause the admin routes for fifteen minutes. When administration is off, the routes do not exist.
- Uploads stream to disk with a 1 GB cap. The secret travels in a header, never in the URL, and is kept in the tab's session storage only.

There is deliberately no export: nothing leaves the server through the page.

Endpoints, all under the `X-Admin-Token` header: `GET /api/admin/bundles`, `POST /api/admin/import` (raw body, `?force=1` to accept a degraded embedder), `POST /api/admin/restore` and `POST /api/admin/delete` (JSON `{"id"}`).

For tests or a second instance, `LANGOLIER_DISABLE_TELEGRAM=1` keeps the server from polling the bot.

## Server mode

The same binary serves the chat page without a desktop environment.

```
langolier --serve --host 0.0.0.0 --port 8787 --bundle /data --data /state
langolier --gui                      native window instead of a browser
langolier --health-check host:port   exit code reflects health
```

`--bundle` is where `.langolier` files are looked for, also settable through `LANGOLIER_BUNDLE_DIR`. `--data` is the writable state directory. The server speaks raw HTTP/1.1 with SSE streaming and needs no reverse proxy for local use. If the requested port is taken it tries the next twenty.

Endpoints: `/` for the page, `/api/health`, `/api/config`, `/api/chat`.

## Docker

`docker/Dockerfile` builds the server with `--no-default-features`, which drops Tauri, GTK and WebKit. The result is a Debian slim image with a single static-ish binary.

```bash
mkdir -p docker/data docker/state
cp path/to/assistant.langolier docker/data/
cd docker
docker compose up -d --build
curl -fsS http://localhost:8787/api/health
```

Notes from running this on a Synology DS719+:

- Build the image on the host that will run it. llama.cpp compiles for the CPU in front of it, and a binary built on a modern laptop will use instructions an Atom or Celeron does not have.
- On a CPU without AVX2, embedding a question takes roughly 3 seconds and a grounded answer through a cloud provider about 4 seconds. Usable, not fast.
- A cloud profile keeps the container small. 2 GB is enough.
- `network_mode: host` avoids NAS firewall rules on Docker subnets and lets the Telegram bridge reach out.
- DSM ships Compose v1 under `/usr/local/bin/docker-compose` and needs `sudo` for the Docker socket.

`scripts/deploy-chatbot.sh` automates repeat deployments over SSH, reusing one prebuilt image for any number of bots. Set `LANGOLIER_SSH_HOST` and `LANGOLIER_NAS_IP` first.

## Telegram

Create a bot with `@BotFather`, paste the token into the profile, test it, then export. The bridge uses long polling, so no port is opened and no domain is needed. One Telegram chat maps to one conversation, and `/new` starts another.

The whitelist accepts numeric ids or `@handles`, comma separated. Empty means everyone can talk to it.

Caps and source privacy apply on Telegram exactly as they do on the web page. Messages travel through Telegram's servers, so keep it to content that may go there.

## Data and backup

| Platform | Location                                               |
| -------- | ------------------------------------------------------ |
| macOS    | `~/Library/Application Support/local.langolier.studio` |
| Linux    | `~/.local/share/local.langolier.studio`                |
| Windows  | `%APPDATA%\local.langolier.studio`                     |

`langolier.sqlite3` holds everything: settings, sources, passages, vectors, conversations, assistants and API keys. It runs in WAL mode. **Engines > Back up the database** takes a consistent copy through a checkpoint. Original media are not copied, back those up separately.

API keys are stored in clear text, like the rest of the settings. The vault saves you retyping them, it does not encrypt them.

## Troubleshooting

**The assistant answers from general knowledge.** Check that strict grounding is on and that the cosine floor is not too low. Below 0.3 unrelated passages clear the bar.

**Everything abstains.** The judge threshold or the cosine floor is too high, or the vectors were made by a different embedding model. Look for the partial index warning.

**Nothing gets indexed.** Check the watch is enabled and ingestion is not paused. Open the watch detail to see per-file errors.

**Transcription fails.** The Whisper ggml model path is empty or wrong, or FFmpeg is missing. Run the install script for your platform.

**The global shortcut does nothing.** It needs at least one modifier. The `fn` key cannot be captured by applications on macOS.

**Metal assertion on quit.** Fixed by shutting the engine down before exit. If you embed the engine elsewhere, call `engine::shutdown()` yourself.

## Development

```bash
npm ci
npm run desktop                 # Vite plus Tauri, hot reload
npm run build                   # typecheck and bundle the front end
cargo test  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt   --manifest-path src-tauri/Cargo.toml
```

Office format tests can run against real files:

```bash
LANGOLIER_OFFICE_FIXTURES=/path/to/files \
  cargo test --manifest-path src-tauri/Cargo.toml office_fixtures -- --ignored --nocapture
```

**Adding a translation.** English source strings are the keys. `src/locales/fr.ts` maps them to French, and `{name}` placeholders are filled at call time. A missing key falls back to English, so a partial locale is safe. To add a language, copy `fr.ts`, add the code to `Lang` in `src/i18n.tsx` and to the switch in the sidebar.

Backend messages are written in English and translated on display through the same dictionary, so a new user-facing string in Rust needs a matching entry in the locale file.
