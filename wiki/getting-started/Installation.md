# Installation

## Download a build

Builds are published on the [releases page](https://github.com/infinition/langolier/releases).

| Platform | File |
|---|---|
| macOS (Apple Silicon) | `.dmg` |
| Windows | `.exe` installer |
| Linux | `.AppImage`, `.deb` or `.rpm` |

macOS builds are ad hoc signed. On first launch, right click the app and pick Open,
otherwise Gatekeeper refuses it.

## What else you need

Langolier answers out of the box with its embedded engine. Some sources need external
tools, and the application tells you which one is missing when it needs it.

| Tool | Needed for |
|---|---|
| Poppler | PDF page counts and rendering scanned pages |
| Tesseract | OCR on scanned pages |
| FFmpeg | audio and video |
| whisper.cpp | transcription |
| yt-dlp | adding a video by URL |

On macOS, `Install-Mac.command` installs all of them through Homebrew and fetches the
transcription model. On Windows, `Install-Windows.bat` does the same.

## Build from source

Requires the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), Rust
stable, Node 20 or newer, and `cmake` for the embedded llama.cpp.

```bash
git clone https://github.com/infinition/langolier.git
cd langolier
npm ci
npm run tauri build
```

For development, `npm run desktop` starts Vite and the Tauri shell with hot reload.

llama.cpp tunes itself for the processor that compiles it, so build on the machine
that will run it.

## Where your data lives

One SQLite file in the application data folder, holding sources, passages, vectors and
conversations. Original files stay where they are: removing a source drops its passages
and vectors from the index, never the file.

| System | Path |
|---|---|
| macOS | `~/Library/Application Support/local.langolier.studio` |
| Windows | `%APPDATA%\local.langolier.studio` |
| Linux | `~/.local/share/local.langolier.studio` |

Back up that folder and you have backed up everything.
