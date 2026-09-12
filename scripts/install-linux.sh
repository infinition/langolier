#!/usr/bin/env bash
set -euo pipefail
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf ffmpeg git cmake python3-venv curl poppler-utils tesseract-ocr tesseract-ocr-fra
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
python3 -m venv "$repo_dir/work/media-env"
"$repo_dir/work/media-env/bin/pip" install 'yt-dlp==2026.8.19'
mkdir -p "$HOME/.local/bin"
ln -sf "$repo_dir/work/media-env/bin/yt-dlp" "$HOME/.local/bin/yt-dlp"
if [ ! -d "$repo_dir/work/whisper.cpp" ]; then
  git clone --depth 1 --branch v1.9.2 https://github.com/ggml-org/whisper.cpp.git "$repo_dir/work/whisper.cpp"
fi
cmake -S "$repo_dir/work/whisper.cpp" -B "$repo_dir/work/whisper.cpp/build" -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF
cmake --build "$repo_dir/work/whisper.cpp/build" -j 4
cp "$repo_dir/work/whisper.cpp/build/bin/whisper-cli" "$HOME/.local/bin/whisper-cli"
model_dir="${XDG_DATA_HOME:-$HOME/.local/share}/local.langolier.studio/models"
mkdir -p "$model_dir"
if [ ! -f "$model_dir/ggml-base.bin" ]; then
  curl -fL --retry 3 https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin -o "$model_dir/ggml-base.bin.part"
  printf '%s  %s\n' '60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe' "$model_dir/ggml-base.bin.part" | sha256sum -c -
  mv "$model_dir/ggml-base.bin.part" "$model_dir/ggml-base.bin"
fi
echo 'Install Ollama from https://ollama.com/download/linux, start ollama serve, then:'
echo 'ollama pull qwen3:8b && ollama pull embeddinggemma'
echo 'Ensure ~/.local/bin is on PATH. Run ./Launch-Linux.sh.'
