#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
# Ollama is optional; start it only when installed and not yet answering.
if command -v ollama >/dev/null 2>&1 && ! curl -fsS http://127.0.0.1:11434/api/version >/dev/null 2>&1; then
  nohup ollama serve >/tmp/langolier-ollama.log 2>&1 &
fi
if [ ! -x src-tauri/target/release/langolier ]; then
  npm ci
  npm run release -- --bundles deb,appimage
fi
exec src-tauri/target/release/langolier
