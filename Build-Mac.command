#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"
for tool in node npm cargo; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Missing $tool. Install Node.js LTS, Rust, and Xcode command-line tools first."
    exit 1
  fi
done
npm ci
npm run release -- --bundles app
echo "Built: $(pwd)/src-tauri/target/release/bundle/macos/Langolier.app"
