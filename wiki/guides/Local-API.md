# Local API, Shortcuts and Siri

Langolier can answer over a local HTTP API, so Shortcuts, Siri or any local agent can
query your memory without opening the application.

## Switching it on

Under **Engines > Local API**, turn on **Answer local requests**. A token is generated
on the spot, and the address appears next to it. Use **Test the API** to confirm it
answers.

What the API is, precisely:

- It listens on `127.0.0.1` only. Nothing is exposed to your network, ever.
- It answers `POST` only, so a question never travels in a URL.
- Every request needs `Authorization: Bearer <token>`. Without a token configured, the
  endpoint refuses everything, including a lucky guess.
- Twelve questions a minute per address.
- Answering takes the same lock as the window, so a question from Siri and a question
  typed in the app never generate at the same time.

Starting and stopping takes effect immediately, without restarting the application.

## Two endpoints

`/api/ask` writes the answer. That takes a model, local or remote, the same one the
application uses.

```
POST http://127.0.0.1:8787/api/ask
Authorization: Bearer <token>

{ "question": "How do you purify water?" }
```

```json
{
  "content": "Boiling is the surest method [1]...",
  "text": "Boiling is the surest method [1]...\n\nSources\n[1] Guide.pdf, Page 222",
  "sources": [ { "name": "Guide.pdf", "locator": "Page 222", "text": "..." } ],
  "status": "ok",
  "warning": null
}
```

`content` is the answer alone, which is what you want read out loud. `text` is the same
answer with its sources listed underneath, which is what you want displayed. One request
gives you both.

`/api/search` stops before the writing and returns the passages, so whatever called it
can write instead.

```
POST http://127.0.0.1:8787/api/search
Authorization: Bearer <token>

{ "question": "How do you purify water?", "mode": "lexical" }
```

```json
{
  "sources": [
    { "n": 1, "name": "Guide.pdf", "locator": "Page 222", "text": "...", "score": 0.81 }
  ],
  "warning": null
}
```

## What each mode costs

This matters if you want Langolier to run without any model at all.

| Mode | What it needs |
|---|---|
| `exact`, `lexical` | Nothing. Retrieval is pure SQLite, no model involved |
| `hybrid`, `semantic` | The embedding engine, to turn your question into a vector |

The relevance judge, when left on under **Engines > La rigueur**, uses a model on top of
that. Turn it off if you want retrieval to stay free of any model.

So a shortcut calling `/api/search` in `lexical` mode asks nothing of any engine: it
reads your index and hands back passages. Whoever called it does the writing.

## Wiring it to Siri

In the Shortcuts app, create a shortcut named **Search my memory**:

1. **Ask for input**, text, with a prompt such as "What are you looking for?"
2. **Get contents of URL** on `http://127.0.0.1:8787/api/ask`
   - Method `POST`
   - Header `Authorization` set to `Bearer` followed by your token
   - Request body JSON, one field `question`, holding the provided input
3. **Get dictionary value** for `text`, then **Show result**
4. Optionally, a second **Get dictionary value** for `content` feeding **Speak text**,
   so the source list is displayed but not read aloud

Then say the shortcut's name to Siri. Siri asks your question, the shortcut calls
Langolier, and the answer comes back with its citations.

## Troubleshooting

**Nothing answers.** Check the application is running and the switch is on. The API
lives in the application process, so quitting Langolier stops it. The menu bar mode
keeps it alive without a window.

**401.** The token in the shortcut no longer matches the one in the settings.
Regenerating the token invalidates the old one.

**The answer mentions lexical search only.** The embedding engine is out of reach, so
hybrid search fell back to words. If you use Ollama for embeddings, it has to be running.
