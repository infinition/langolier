# Exporting a chatbot

Export turns an assistant into something that runs on another machine: a folder holding
a `.langolier` bundle, a window launcher, a web launcher and server scripts.

## What travels

The bundle carries the profile, the sources in its scope, their passages and their
vectors, and the settings it needs. It does not carry your other sources, your other
profiles, or your conversations.

## Running it

```bash
./server.sh                 # chat page on 0.0.0.0:8080
./chatbotweb --serve --port 8787 --data ./state
```

The window launcher opens a real window instead of a browser tab. It can sit in the menu
bar or system tray, start hidden, and pop a floating question bar on a global shortcut,
on macOS, Windows and Linux.

The page speaks the visitor's language, English or French, or the one pinned in the
profile.

## Updating a deployed bot

Drop a new `.langolier` next to the launcher. It is picked up in under thirty seconds
without a restart, and conversations survive the swap.

## When you have no file access

Turn on **Remote administration** in the profile before exporting, with a secret. A gear
then sits in the chat page footer: with the secret, you can import a bundle from the
browser, pick any earlier version from a list and restore it, or delete versions you no
longer need.

Every bundle the chatbot ever ran is kept, twenty at most, named, timestamped and
deduplicated. The running version and the one the kit shipped with cannot be deleted.

Five wrong secrets pause the admin routes for fifteen minutes. When administration is
off, the routes do not exist at all.

There is deliberately no export from the page: nothing leaves the server through it, so
the corpus and the API key stay where you put them.

## Docker

The server build needs no desktop libraries. `docker/` holds a Dockerfile and a Compose
file.

```bash
mkdir -p docker/data docker/state
cp ~/Desktop/my-assistant/my-assistant.langolier docker/data/
cd docker && docker compose up -d --build
curl -fsS http://localhost:8787/api/health
```

| Mount | Contents |
|---|---|
| `./data` | The `.langolier`, and for an embedded export, `models/*.gguf`. Replace the bundle here to update |
| `./state` | The working database and the conversations. Back this up |

llama.cpp tunes itself for the processor that compiles it, so build the image on the
host that will run it. On a NAS or a small VPS, a cloud provider profile is lighter: the
container then only embeds text, and a 2 GB memory cap is enough. With
`network_mode: host`, the Telegram bridge works without extra plumbing.
