# Settings

Where each setting lives, and what it actually changes. A setting sits on the page of
the thing it governs; what governs the engine itself stays under **Engines**.

## Sources > Processing

| Setting | Effect |
|---|---|
| Pause processing | Holds the whole queue. Watches keep spotting files, nothing is ingested, nothing is lost |
| Keep processing sources in the menu bar | Off by default. OCR, transcription and vectors are what heat the machine |
| Passage length | How long a passage is cut, in characters. Applies to sources indexed from then on |
| Overlap between passages | How much two consecutive passages share |

## Vigies

| Setting | Effect |
|---|---|
| Scan every | The interval every watch falls back to. A watch can override it |
| Keep watching folders in the menu bar | Off by default. A folder filled meanwhile is picked up when the window reopens |

## Engines > Le cerveau

The model that writes. Provider, address, model, answer length, and how long an idle
model stays in memory before it is unloaded.

Pointing the brain at a cloud provider sends your questions and the retrieved passages
to it. The interface says so next to the setting.

## Engines > La mémoire

The embedding engine, and how much is retrieved.

| Setting | Effect |
|---|---|
| Embedding server and model | What turns text into vectors. Changing it invalidates existing vectors |
| Passages retrieved | How many passages reach the answer |
| Candidates per search arm | How many each arm brings back before fusion |
| Passages per source | Caps what one document may contribute. Zero lifts the cap |

## Engines > La rigueur

| Setting | Effect |
|---|---|
| Answer only from the sources | Strict mode. Without a supporting passage, the abstention sentence is used |
| Abstention sentence | Yours to write. What it says when it does not know |
| Relevance judge | A model rereads the candidates and scores them. Below the threshold, it abstains |
| Judge model | Empty means the conversation model |
| Verify the answer after writing | A second call checks each claim. Safer, doubles the time |
| Minimum cosine, minimum BM25 | Raw floors, applied before the judge |

## Engines > La palette

The global shortcut that opens the floating question bar from anywhere.

## Engines > In the background

| Setting | Effect |
|---|---|
| Icon in the menu bar | Closing the window then releases it, and the app leaves the Dock |
| Start hidden | Opens without a window; the icon and the shortcut are there |
| Launch at login | Starts hidden at login |

## Engines > Local API

| Setting | Effect |
|---|---|
| Answer local requests | Starts the loopback server. See [Local API](Local-API) |
| Port | Default 8787 |
| Access token | Required on every request. Regenerating it invalidates the old one |

## Where settings are stored

In the same SQLite file as everything else, so they survive reinstalling the
application. Launch at login is the exception: it writes a system file, a LaunchAgent on
macOS, a registry key on Windows, a `.desktop` entry on Linux, because nothing inside the
application could start it.
