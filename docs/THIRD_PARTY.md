# Third-party components

Langolier uses third-party software under its respective license. Consult the exact locked versions and upstream license files before redistributing a release.

| Component               | Use                                        | Upstream                                   |
| ----------------------- | ------------------------------------------ | ------------------------------------------ |
| Tauri                   | Native shell and IPC                       | https://github.com/tauri-apps/tauri        |
| React                   | User interface                             | https://github.com/facebook/react          |
| Lucide                  | Interface icons                            | https://github.com/lucide-icons/lucide     |
| SQLite / rusqlite       | Persistent database and full-text search   | https://github.com/rusqlite/rusqlite       |
| pdf-extract             | PDF text extraction                        | https://github.com/jrmuizel/pdf-extract    |
| Ollama                  | External local inference service           | https://github.com/ollama/ollama           |
| whisper.cpp             | External speech recognizer                 | https://github.com/ggml-org/whisper.cpp    |
| FFmpeg                  | External audio/video decoder               | https://ffmpeg.org                         |
| yt-dlp                  | External media downloader                  | https://github.com/yt-dlp/yt-dlp           |
| Tesseract               | External OCR engine                        | https://github.com/tesseract-ocr/tesseract |
| Poppler                 | External PDF renderer                      | https://poppler.freedesktop.org            |
| llama.cpp (llama-cpp-2) | Embedded inference, Metal on Apple Silicon | https://github.com/utilityai/llama-cpp-rs  |
| flate2                  | Deflate for office archives                | https://github.com/rust-lang/flate2-rs     |
| quick-xml               | XML reader for office formats              | https://github.com/tafia/quick-xml         |
| reqwest                 | HTTP client                                | https://github.com/seanmonstar/reqwest     |
| MLX LM                  | Optional external fine-tuning workflow     | https://github.com/ml-explore/mlx-lm       |

Model weights have their own licenses and model cards. The application does not grant additional rights to Qwen, EmbeddingGemma, Whisper or other model artifacts. Default scripts download model weights separately instead of embedding them into the executable. External-tool distributions may carry additional license obligations, especially for codecs and PDF rendering.
