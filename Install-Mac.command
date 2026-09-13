#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"
if ! command -v brew >/dev/null 2>&1; then
  echo "Install Homebrew from https://brew.sh, then run this installer again."
  exit 1
fi
brew install ffmpeg yt-dlp whisper-cpp poppler tesseract
model_dir="$HOME/Library/Application Support/local.langolier.studio/models"
mkdir -p "$model_dir"
./scripts/install-ocr-models.sh "$model_dir"
if [ ! -f "$model_dir/ggml-base.bin" ]; then
  curl --fail --location --retry 3 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin' -o "$model_dir/ggml-base.bin.part"
  printf '%s  %s\n' '60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe' "$model_dir/ggml-base.bin.part" | shasum -a 256 -c -
  mv "$model_dir/ggml-base.bin.part" "$model_dir/ggml-base.bin"
fi
echo 'Media tools are ready. Launch Langolier, then install the chat and embedding models from Engines.'
echo 'Ollama is optional: brew install ollama, if you prefer it to the built-in engine.'
