# Assistants

An assistant is a named profile: a mission, a scope of knowledge, an engine, a set of
caps and a theme. Langolier without a profile answers from everything with the global
settings; a profile narrows and specialises that.

## Creating one

Open **Assistants** and create a profile. What each field changes:

| Field | Effect |
|---|---|
| Mission | Prepended to the instructions, so the assistant knows what it is for |
| Welcome | The first message a visitor sees on the exported chat page |
| Scope | Which sources it may answer from: everything, or a chosen set |
| Engine | Its own model and provider, overriding the global one |
| Caps | A ceiling on tokens per conversation and per day |
| Theme | The colours of its exported page |
| Language | English, French, or the visitor's own |

## Scope

Scope is the difference between a general assistant and a useful one. A profile bound
to a handful of folders answers about those folders and abstains elsewhere, which is
usually what you want when you hand it to someone else.

Sources outside the scope are not searched at all. They are not hidden after the fact,
they never enter the retrieval.

## Caps and privacy

Token caps bound an API bill. When a conversation reaches its cap, the assistant says
so instead of silently continuing to spend.

**Hide source names** keeps document titles out of the answers. The citations still
work and still point at real passages, but the reader does not learn that your
`2024-negotiation-notes.pdf` exists.

## Exporting

Export produces a folder holding a `.langolier` bundle, a window launcher, a web
launcher and server scripts. See [Exporting a chatbot](Export).

The profile travels with its scope, its engine and its caps. What does not travel: your
other sources, your other profiles, your conversations.
