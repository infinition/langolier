#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"
if command -v ollama >/dev/null 2>&1; then
  if ! curl -fsS http://127.0.0.1:11434/api/version >/dev/null 2>&1; then
    nohup ollama serve > /tmp/langolier-ollama.log 2>&1 &
  fi
fi
if [ ! -d "src-tauri/target/release/bundle/macos/Langolier.app" ]; then
  ./Build-Mac.command
fi
open "src-tauri/target/release/bundle/macos/Langolier.app"
