# How retrieval works

Every answer starts by finding passages. Understanding this explains most of what the
settings do.

## Cutting sources into passages

A source is read, then cut into overlapping passages. The cut lands on a sentence
boundary when one is within reach, so a passage does not start mid-word.

Length and overlap are set under **Sources > Processing**. They apply to sources indexed
from then on; existing passages keep the length they were cut with until a reindex.

## Two searches, fused

| Arm | What it finds |
|---|---|
| BM25, through SQLite FTS5 | The passages holding your words |
| Dense vectors | The passages holding your meaning, whatever the words |

Both arms bring back candidates, and the two lists are fused with Reciprocal Rank
Fusion: a passage ranked high by either arm survives, a passage ranked high by both wins.

Vectors are stored normalised, so comparing two of them is a plain dot product.

**Candidates per search arm** decides how many each arm brings back before fusion. Wider
catches more and costs more to rerank.

**Passages per source** keeps one talkative document from filling the whole answer.

## The judge

With the relevance judge on, a model reads the surviving candidates and scores each one.
Below the threshold, the assistant abstains before writing anything.

This is the difference between an answer that quotes a vaguely related paragraph and one
that admits the corpus does not cover the question. It costs one extra model call.

## Grounding

In strict mode, the model may only use the retrieved passages for factual claims, and
must cite them inline. Citation markers are checked against the passages that were
actually supplied: a reference to a source that does not exist is caught.

That check is about reference bounds, not about truth. It confirms `[2]` points at a real
passage, not that the sentence faithfully reflects it.

**Verify the answer after writing** adds a second call that checks each claim against the
passages. Safer, and it doubles the response time.

## When vectors are missing

Changing the embedding model makes existing vectors incomparable: they were produced by
a different model. Langolier does not mix them. It says so, falls back to word search,
and offers to reindex.

Word search keeps working the whole time.
