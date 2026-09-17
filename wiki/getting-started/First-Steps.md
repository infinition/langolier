# First steps

## Add your first sources

Open **Sources** and press **Add sources**. You can drop a folder, pick files, paste a
piece of text, or give the address of a video.

Supported without any extra tool: PDF, DOCX, PPTX, XLSX, ODT, ODP, ODS, Markdown, plain
text, CSV, JSON, notebooks, source files and subtitles. Audio and video go through
transcription, which needs whisper.cpp and FFmpeg.

Each file is read, cut into passages, and turned into vectors. The table shows where
each source stands: its language, its passage count, and how many of those passages
already carry a vector. A source is usable for word search as soon as its passages
exist, before the vectors are finished.

## Ask a question

Open **Conversation** and ask. The answer carries numbered citations, and the chips
below it open the exact passage behind each claim.

The mode selector under the question decides how the memory is searched:

| Mode | What it does |
|---|---|
| Hybrid | Words and meaning, fused. The default, and the right answer most of the time |
| Lexical | Words only, through BM25. No model needed to search |
| Semantic | Meaning only, through vectors |
| General | Answers from the model alone, without your sources |

## When it says it does not know

In strict mode, an answer with no supporting passage is refused rather than guessed.
That refusal is a feature: it means the corpus does not hold what you asked for.

Three things usually explain it. The source may not be indexed yet, which the Sources
table tells you. The question may use words absent from your documents, which hybrid
search handles better than lexical. Or the abstention threshold may be set high, under
**Engines > La rigueur**.

The abstention sentence is yours to write. Change it to something that sounds like you.

## What to do next

- Point Langolier at a folder that keeps filling, with [Watches](Watches)
- Give a subject its own profile, with [Assistants](Assistants)
- Ask from outside the app, with [the local API](Local-API)
