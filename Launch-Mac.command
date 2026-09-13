#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"

# Start the Ollama daemon only when nothing answers yet (Ollama.app counts).
if command -v ollama >/dev/null 2>&1; then
  if ! curl -fsS http://127.0.0.1:11434/api/version >/dev/null 2>&1; then
    nohup ollama serve > /tmp/langolier-ollama.log 2>&1 &
  fi
fi

# Installed app first, local build second, compile only as a last resort.
app=""
for candidate in \
  "/Applications/Langolier.app" \
  "$HOME/Applications/Langolier.app" \
  "src-tauri/target/release/bundle/macos/Langolier.app"; do
  if [ -d "$candidate" ]; then app="$candidate"; break; fi
done
if [ -z "$app" ]; then
  echo "No Langolier.app found. Install the release .dmg from"
  echo "https://github.com/infinition/langolier/releases, or build it now (10 min, needs Rust and Node)."
  read -r -p "Build now? [y/N] " answer
  [ "${answer:-n}" = "y" ] || exit 0
  ./Build-Mac.command
  app="src-tauri/target/release/bundle/macos/Langolier.app"
fi
open "$app"

# Close the Terminal window this script opened, leave every other one alone.
osascript >/dev/null 2>&1 <<'EOF' &
tell application "Terminal"
  close (every window whose name contains "Launch-Mac")
end tell
EOF
exit 0
