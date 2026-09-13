<div align="center">

<img src=".github/langolier.png" alt="Langolier" width="160">

# Langolier

**A local knowledge studio. Feed it documents, code, videos and voice notes, ask questions, get answers with citations. Nothing leaves the machine unless you point it at a cloud API.**

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-orange.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/infinition/langolier?include_prereleases)](https://github.com/infinition/langolier/releases)
[![Rust](https://img.shields.io/badge/Rust-Tauri%202-CE422B.svg)](https://v2.tauri.app/)

[English](#english) | [Français](#français)

</div>

---

## English

Langolier indexes what you already own and answers from it. Retrieval runs on SQLite: BM25 for words, vectors for meaning, fused with RRF, optionally reranked by a judge model. When nothing in the corpus supports an answer, it says so instead of inventing one.

The desktop app is Rust plus Tauri 2 with a React front end. The same binary also runs headless as a chatbot server, which is how an exported assistant gets deployed.

<p align="center">
  <img src="docs/screenshots/conversation.png" alt="Langolier answering with citations" width="100%" />
  <br>
  <sub><strong>Conversation:</strong> Every factual sentence carries a numbered citation, and the chips under the answer open the exact passage that backed it. The footer shows which memory mode and which model produced it.</sub>
</p>

### A look around

<p align="center">
  <img src="docs/screenshots/sources.png" alt="Source library" width="100%" />
  <br>
  <sub><strong>Sources:</strong> One row per ingested file, with its language, its passage count and how many of those passages already carry a vector. List, tree and full-text search share the same view, and a folder in the tree can be reindexed or removed in one click.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/watches.png" alt="Watched folders" width="100%" />
  <br>
  <sub><strong>Watches:</strong> Folders that feed themselves into the memory. Sync leaves files in place, hoover moves them in. Each card shows what is ready, queued or failed, when it was last scanned, and can be paused without losing anything.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/assistant-editor.png" alt="Assistant editor" width="100%" />
  <br>
  <sub><strong>Assistants:</strong> A profile is a mission, a knowledge scope, an engine and a set of caps. The token caps bound an API bill, and hiding source names keeps document titles out of the answers.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/engines.png" alt="Engine settings" width="100%" />
  <br>
  <sub><strong>Engines:</strong> The brain and the memory are configured separately, so a cloud chat model can sit next to a local embedder. Below, the grounding rules: abstention sentence, relevance judge, thresholds.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/observatory.png" alt="Observatory metrics" width="100%" />
  <br>
  <sub><strong>Observatory:</strong> Latency, tokens per second, success rate and the activity log, measured on the machine. No simulated data.</sub>
</p>

### What it does

| Feature       | Detail                                                                                              |
| ------------- | --------------------------------------------------------------------------------------------------- |
| Ingest        | PDF, DOCX, PPTX, XLSX, ODT, ODP, ODS, Markdown, text, CSV, JSON, notebooks, source files, subtitles |
| Transcribe    | Video and audio through whisper.cpp, including voice memos (`m4a`, `mp3`, `wav`, `flac`, `ogg`)     |
| Watch folders | Local or mounted NAS paths, polled on a timer, sync or hoover mode                                  |
| Retrieve      | FTS5 BM25 and dense vectors, RRF fusion, optional LLM reranker, abstention thresholds               |
| Ground        | Strict mode answers only from retrieved passages, with inline citations                             |
| Assistants    | Named profiles with their own mission, scope, engine, caps and theme                                |
| Export        | A `.langolier` bundle plus standalone launchers, updatable by dropping in a new bundle              |
| Bridge        | Telegram long polling, no open port required                                                        |

### Install

Grab a build from [Releases](https://github.com/infinition/langolier/releases):

| Platform              | File                  |
| --------------------- | --------------------- |
| macOS (Apple Silicon) | `.dmg`                |
| Windows               | `.exe` installer      |
| Linux                 | `.AppImage` or `.deb` |

macOS builds are ad hoc signed. On first launch, right click the app and pick Open.

### Build from source

Requires [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), Rust stable, Node 20 or newer, and `cmake` for the embedded llama.cpp.

```bash
git clone https://github.com/infinition/langolier.git
cd langolier
npm ci
npm run tauri build
```

For development, `npm run desktop` starts Vite and the Tauri shell with hot reload.

On macOS, `Launch-Mac.command` opens the app and starts Ollama when needed; `Stop-Ollama.command` shuts Ollama down so no model stays in memory.

### Pick an engine

Langolier never hardcodes a model. Set the provider under **Engines**.

**Fully local, Ollama.** The simplest path. Install Ollama, then:

```bash
ollama pull qwen3:8b
ollama pull embeddinggemma
```

Point the chat endpoint at `http://127.0.0.1:11434`. `qwen3:8b` is the safest of the three tested models on strict grounding. `qwen3:4b-instruct` is twice as fast for half the footprint. Avoid the bare `qwen3:4b` Thinking variant, it leaks its reasoning into answers.

**Fully local, no daemon.** Select the embedded provider and point it at a GGUF file. llama.cpp is compiled into the binary, with Metal on Apple Silicon. Nothing else to install and nothing listening on a port.

**Cloud API.** OpenAI, DeepSeek, Anthropic, or any OpenAI-compatible endpoint such as Mistral, Groq or OpenRouter. Your questions and the retrieved passages go to that provider. The index and the vectors stay local. Keys live in the app database in clear text, like the rest of the settings, so protect your session.

### Embeddings

Embeddings decide what retrieval can find, so they matter more than the chat model. Four are wired in, each with an official GGUF for offline export:

| Model                  | Size | Notes                                |
| ---------------------- | ---- | ------------------------------------ |
| `embeddinggemma`       | 300M | Multilingual, light, the default     |
| `qwen3-embedding:0.6b` | 600M | Multilingual, more accurate, heavier |
| `nomic-embed-text`     | 137M | Mostly English, very light           |
| `bge-m3`               | 560M | Multilingual, long context           |

Vectors from different models are never mixed. Switching models invalidates the existing ones, and the app offers a reindex, by type or by folder. Word search keeps working meanwhile.

### Run a chatbot locally

Build an assistant under **Assistants**, then export it. You get a folder holding a `.langolier` bundle, a window launcher, a web launcher and server scripts.

```bash
./server.sh                 # chat page on 0.0.0.0:8080
./chatbotweb --serve --port 8787 --data ./state
```

To update a deployed bot, drop a new `.langolier` next to the launcher. It is picked up in under 30 seconds without a restart, and conversations survive.

### Run a chatbot in Docker

The server build needs no desktop libraries. `docker/` holds a Dockerfile and a Compose file.

```bash
mkdir -p docker/data docker/state
cp ~/Desktop/my-assistant/my-assistant.langolier docker/data/
cd docker && docker compose up -d --build
curl -fsS http://localhost:8787/api/health
```

Mount points:

- `./data` holds the `.langolier` and, for an embedded export, `models/*.gguf`. Replace the bundle here to update the bot.
- `./state` holds the working database and conversations. Back this up.

llama.cpp tunes itself for the CPU that compiles it, so build the image on the host that will run it. On a NAS or a small VPS, a cloud provider profile is the lighter option: the container then only needs to embed text, and a 2 GB memory cap is enough. With `network_mode: host` the Telegram bridge works without extra plumbing.

### Project layout

| Path             | Contents                                                                                                                     |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `src/`           | React interface, `locales/fr.ts` holds the French strings                                                                    |
| `src-tauri/src/` | Rust core: ingestion, retrieval, pipeline, engines, server, bundles                                                          |
| `docker/`        | Server image and Compose file                                                                                                |
| `docs/`          | [Detailed guide](docs/GUIDE.md), also published at [infinition.github.io/langolier](https://infinition.github.io/langolier/) |
| `scripts/`       | Install helpers, dataset tooling, deployment                                                                                 |

### Language

The interface ships in English and French. The switch sits at the bottom of the sidebar and the choice persists. English source strings are the keys; French lives in `src/locales/fr.ts`.

### License

[MPL 2.0](LICENSE).

---

## Français

Langolier indexe ce que vous possédez déjà et répond à partir de ça. La recherche tourne sur SQLite : BM25 pour les mots, vecteurs pour le sens, fusionnés par RRF, avec un modèle juge en option. Quand rien dans le corpus ne soutient une réponse, il le dit au lieu d'inventer.

L'application de bureau est en Rust avec Tauri 2 et une interface React. Le même binaire tourne aussi sans écran comme serveur de chatbot, ce qui sert à déployer un assistant exporté.

<p align="center">
  <img src="docs/screenshots/conversation.png" alt="Langolier répond avec ses citations" width="100%" />
  <br>
  <sub><strong>Conversation:</strong> Chaque phrase factuelle porte une citation numérotée, et les pastilles sous la réponse ouvrent le passage exact qui la soutient. Le pied de page indique le mode de mémoire et le modèle utilisés.</sub>
</p>

### Tour du propriétaire

<p align="center">
  <img src="docs/screenshots/sources.png" alt="Bibliothèque de sources" width="100%" />
  <br>
  <sub><strong>Sources:</strong> Une ligne par fichier ingéré, avec sa langue, son nombre de passages et combien portent déjà un vecteur. Liste, arborescence et recherche plein texte partagent la même vue, et un dossier de l'arbre se réindexe ou se retire d'un clic.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/watches.png" alt="Dossiers surveillés" width="100%" />
  <br>
  <sub><strong>Vigies:</strong> Des dossiers qui alimentent la mémoire tout seuls. Suivi laisse les fichiers en place, aspiration les déplace. Chaque carte montre ce qui est prêt, en file ou en erreur, la date du dernier passage, et se met en pause sans rien perdre.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/assistant-editor.png" alt="Éditeur d'assistant" width="100%" />
  <br>
  <sub><strong>Assistants:</strong> Un profil, c'est une mission, un périmètre de connaissances, un moteur et des plafonds. Les plafonds de tokens bornent une facture d'API, et masquer les noms de sources garde vos titres de documents hors des réponses.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/engines.png" alt="Réglages des moteurs" width="100%" />
  <br>
  <sub><strong>Moteurs:</strong> Le cerveau et la mémoire se règlent séparément : un modèle de conversation cloud peut cohabiter avec un embeddeur local. En dessous, les règles d'ancrage : phrase d'abstention, juge de pertinence, seuils.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/observatory.png" alt="Métriques de l'observatoire" width="100%" />
  <br>
  <sub><strong>Observatoire:</strong> Latence, tokens par seconde, taux de succès et journal d'activité, mesurés sur la machine. Aucune donnée simulée.</sub>
</p>

### Ce que ça fait

| Fonction      | Détail                                                                                                    |
| ------------- | --------------------------------------------------------------------------------------------------------- |
| Ingestion     | PDF, DOCX, PPTX, XLSX, ODT, ODP, ODS, Markdown, texte, CSV, JSON, notebooks, fichiers source, sous-titres |
| Transcription | Vidéo et audio via whisper.cpp, mémos vocaux compris (`m4a`, `mp3`, `wav`, `flac`, `ogg`)                 |
| Vigies        | Dossiers locaux ou NAS monté, scrutés par minuteur, mode suivi ou aspiration                              |
| Recherche     | BM25 FTS5 et vecteurs denses, fusion RRF, juge LLM optionnel, seuils d'abstention                         |
| Ancrage       | Le mode strict ne répond que depuis les passages retrouvés, avec citations                                |
| Assistants    | Profils nommés, chacun sa mission, son périmètre, son moteur, ses plafonds et son thème                   |
| Export        | Un `.langolier` et des lanceurs autonomes, mis à jour en déposant un nouveau fichier                      |
| Pont          | Telegram par interrogation longue, aucun port à ouvrir                                                    |

### Installation

Prenez une version dans [Releases](https://github.com/infinition/langolier/releases) :

| Système               | Fichier               |
| --------------------- | --------------------- |
| macOS (Apple Silicon) | `.dmg`                |
| Windows               | Installateur `.exe`   |
| Linux                 | `.AppImage` ou `.deb` |

Les versions macOS sont signées ad hoc. Au premier lancement, clic droit sur l'app puis Ouvrir.

### Compiler depuis les sources

Il faut les [prérequis Tauri](https://v2.tauri.app/start/prerequisites/), Rust stable, Node 20 ou plus, et `cmake` pour le llama.cpp embarqué.

```bash
git clone https://github.com/infinition/langolier.git
cd langolier
npm ci
npm run tauri build
```

Pour développer, `npm run desktop` lance Vite et la coquille Tauri avec rechargement à chaud.

Sur macOS, `Launch-Mac.command` ouvre l'app et démarre Ollama si besoin ; `Stop-Ollama.command` l'arrête pour qu'aucun modèle ne reste en mémoire.

### Choisir un moteur

Langolier ne fige aucun modèle. Le fournisseur se règle dans **Moteurs**.

**Tout local, avec Ollama.** Le plus simple. Installez Ollama, puis :

```bash
ollama pull qwen3:8b
ollama pull embeddinggemma
```

Pointez l'adresse de conversation sur `http://127.0.0.1:11434`. `qwen3:8b` est le plus sûr des trois modèles testés sur l'ancrage strict. `qwen3:4b-instruct` va deux fois plus vite pour moitié moins lourd. Évitez le `qwen3:4b` nu, variante Thinking : il laisse fuir son raisonnement dans la réponse.

**Tout local, sans démon.** Choisissez le fournisseur embarqué et donnez-lui un fichier GGUF. llama.cpp est compilé dans le binaire, avec Metal sur Apple Silicon. Rien d'autre à installer, rien qui écoute sur un port.

**API cloud.** OpenAI, DeepSeek, Anthropic, ou n'importe quelle adresse compatible OpenAI comme Mistral, Groq ou OpenRouter. Vos questions et les passages retrouvés partent chez ce fournisseur. L'index et les vecteurs restent locaux. Les clés sont dans la base de l'application en clair, comme le reste des réglages : protégez votre session.

### Embeddings

Les embeddings décident de ce que la recherche peut trouver, donc ils comptent plus que le modèle de conversation. Quatre sont câblés, chacun avec un GGUF officiel pour l'export hors ligne :

| Modèle                 | Taille | Notes                                |
| ---------------------- | ------ | ------------------------------------ |
| `embeddinggemma`       | 300M   | Multilingue, léger, le défaut        |
| `qwen3-embedding:0.6b` | 600M   | Multilingue, plus précis, plus lourd |
| `nomic-embed-text`     | 137M   | Surtout anglais, très léger          |
| `bge-m3`               | 560M   | Multilingue, contexte long           |

Les vecteurs de modèles différents ne sont jamais mélangés. Changer de modèle invalide les vecteurs existants, et l'application propose une réindexation, par type ou par dossier. La recherche par mots continue de fonctionner pendant ce temps.

### Faire tourner un chatbot en local

Créez un assistant dans **Assistants**, puis exportez-le. Vous obtenez un dossier avec un `.langolier`, un lanceur fenêtre, un lanceur web et les scripts serveur.

```bash
./server.sh                 # page de conversation sur 0.0.0.0:8080
./chatbotweb --serve --port 8787 --data ./state
```

Pour mettre à jour un bot déployé, déposez un nouveau `.langolier` à côté du lanceur. Il est pris en compte en moins de 30 secondes sans redémarrage, et les conversations sont conservées.

### Faire tourner un chatbot dans Docker

La compilation serveur n'a besoin d'aucune bibliothèque de bureau. `docker/` contient un Dockerfile et un fichier Compose.

```bash
mkdir -p docker/data docker/state
cp ~/Bureau/mon-assistant/mon-assistant.langolier docker/data/
cd docker && docker compose up -d --build
curl -fsS http://localhost:8787/api/health
```

Points de montage :

- `./data` contient le `.langolier` et, pour un export embarqué, `models/*.gguf`. Remplacez le bundle ici pour mettre à jour le bot.
- `./state` contient la base de travail et les conversations. C'est ce qu'il faut sauvegarder.

llama.cpp s'optimise pour le processeur qui le compile : construisez l'image sur la machine qui va la faire tourner. Sur un NAS ou un petit VPS, un profil cloud est plus léger : le conteneur ne fait plus que vectoriser du texte, et 2 Go de mémoire suffisent. Avec `network_mode: host`, le pont Telegram fonctionne sans plomberie supplémentaire.

### Organisation du dépôt

| Chemin           | Contenu                                                                                                                     |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `src/`           | Interface React, `locales/fr.ts` porte les chaînes françaises                                                               |
| `src-tauri/src/` | Cœur Rust : ingestion, recherche, pipeline, moteurs, serveur, bundles                                                       |
| `docker/`        | Image serveur et fichier Compose                                                                                            |
| `docs/`          | [Guide détaillé](docs/GUIDE.md), publié aussi sur [infinition.github.io/langolier](https://infinition.github.io/langolier/) |
| `scripts/`       | Scripts d'installation, outillage dataset, déploiement                                                                      |

### Langue

L'interface existe en anglais et en français. Le sélecteur est en bas de la barre latérale et le choix est conservé. Les chaînes anglaises servent de clés, le français vit dans `src/locales/fr.ts`.

### Licence

[MPL 2.0](LICENSE).
