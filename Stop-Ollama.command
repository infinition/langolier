#!/bin/bash
# Stops the Ollama daemon that Launch-Mac.command started, and Ollama.app if
# it is running, so no model stays resident in memory. Langolier itself is
# left alone.
set -uo pipefail

stopped=0
if pgrep -fq "Ollama.app"; then
  osascript -e 'tell application "Ollama" to quit' >/dev/null 2>&1
  echo "Ollama.app: quit"
  stopped=1
fi
if pgrep -x ollama >/dev/null; then
  pkill -x ollama
  for _ in 1 2 3 4 5; do pgrep -x ollama >/dev/null || break; sleep 1; done
  pgrep -x ollama >/dev/null && pkill -9 -x ollama
  echo "ollama serve: stopped"
  stopped=1
fi
[ "$stopped" = 1 ] || echo "Ollama was not running."

# Free the launcher log; /tmp is cleared on reboot anyway.
rm -f /tmp/langolier-ollama.log

osascript >/dev/null 2>&1 <<'EOF' &
tell application "Terminal"
  close (every window whose name contains "Stop-Ollama")
end tell
EOF
exit 0
