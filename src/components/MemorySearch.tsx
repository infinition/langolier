import { useState } from "react";
import { message, t } from "../i18n";
import {
  Search,
  LoaderCircle,
  Replace,
  Check,
  Pencil,
  X,
  ExternalLink,
} from "lucide-react";
import type { Source } from "../types";
import { Button, Empty } from "./Common";
import Highlight, { termsOf, countMatches } from "./Highlight";
import { api } from "../api";

const MODES: [string, string][] = [
  ["exact", "Exact"],
  ["lexical", "Mots"],
  ["semantic", "Sens"],
  ["hybrid", t("Hybrid")],
];

// Whole-memory search, highlighted results, batch replace.
export default function MemorySearch({
  onOpen,
  onError,
  onChanged,
}: {
  onOpen: (s: Source, highlight: string) => void;
  onError: (e: unknown) => void;
  onChanged: () => void;
}) {
  const [query, setQuery] = useState("");
  const [mode, setMode] = useState("exact");
  const [results, setResults] = useState<Source[] | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [latency, setLatency] = useState(0);
  const [busy, setBusy] = useState(false);
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [editing, setEditing] = useState<number | null>(null);
  const [draft, setDraft] = useState("");
  const [note, setNote] = useState("");
  const terms = termsOf(query, mode === "exact");

  async function run() {
    const q = query.trim();
    if (!q) return;
    setBusy(true);
    setNote("");
    try {
      const r = await api<{
        sources: Source[];
        warning: string | null;
        latency_ms: number;
      }>("search_sources", { query: q, mode });
      setResults(r.sources);
      setWarning(r.warning);
      setLatency(r.latency_ms);
      if (mode === "exact" && !from) setFrom(q);
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }
  async function replaceAll() {
    if (!results?.length || !from) return;
    setBusy(true);
    try {
      const r = await api<{
        chunks: number;
        occurrences: number;
        warning: string | null;
      }>("replace_in_chunks", { ids: results.map((s) => s.id), from, to });
      setNote(
        t("{n} occurrence(s) replaced across {chunks} passage(s).", {
          n: r.occurrences,
          chunks: r.chunks,
        }) + (r.warning ? ` ${message(r.warning)}` : ""),
      );
      setResults(
        results.map((s) => ({ ...s, text: s.text.split(from).join(to) })),
      );
      onChanged();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }
  async function save(s: Source) {
    setBusy(true);
    try {
      const r = await api<{ warning: string | null }>("update_chunk", {
        id: s.id,
        text: draft,
      });
      setResults(
        (all) =>
          all?.map((x) => (x.id === s.id ? { ...x, text: draft.trim() } : x)) ||
          null,
      );
      setEditing(null);
      if (r.warning) setNote(r.warning);
      onChanged();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }
  const occurrences =
    results?.reduce(
      (n, s) => n + countMatches(s.text, from ? [from] : []),
      0,
    ) || 0;
  const byDoc = new Map<string, Source[]>();
  results?.forEach((s) =>
    byDoc.set(s.doc_id, [...(byDoc.get(s.doc_id) || []), s]),
  );

  return (
    <div className="memory-search">
      <form
        className="watch-form-row"
        onSubmit={(e) => {
          e.preventDefault();
          void run();
        }}
      >
        <div className="searchbox grow">
          <Search size={16} />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("Search a word, an exact phrase, or an idea…")}
            aria-label={t("Search in memory")}
          />
        </div>
        <div className="segmented">
          {MODES.map(([id, label]) => (
            <button
              type="button"
              key={id}
              className={mode === id ? "selected" : ""}
              onClick={() => setMode(id)}
            >
              {label}
            </button>
          ))}
        </div>
        <Button
          primary
          disabled={busy || !query.trim()}
          onClick={() => void run()}
        >
          {busy ? (
            <LoaderCircle className="spin" size={15} />
          ) : (
            <Search size={15} />
          )}{" "}
          Chercher
        </Button>
      </form>
      {results && (
        <>
          <div className="replace-bar">
            <Replace size={15} />
            <input
              placeholder={t("Text to replace in the results")}
              value={from}
              onChange={(e) => setFrom(e.target.value)}
            />
            <input
              placeholder={t("Replace with")}
              value={to}
              onChange={(e) => setTo(e.target.value)}
            />
            <Button
              primary
              disabled={busy || !from || !occurrences}
              onClick={() => void replaceAll()}
            >
              <Check size={14} /> Remplacer {occurrences} occ. dans{" "}
              {results.length} passage(s)
            </Button>
            <small>
              {results.length} passage(s) · {byDoc.size} source(s) · {latency}{" "}
              ms
              {warning ? ` · ${warning}` : ""}
            </small>
          </div>
          {note && <p className="muted">{note}</p>}
          {results.length ? (
            <div className="search-results">
              {[...byDoc.entries()].map(([docId, list]) => (
                <section key={docId} className="search-doc">
                  <h4>
                    <button
                      className="link"
                      onClick={() => onOpen(list[0], from || query)}
                      title={t("Open the source")}
                    >
                      {list[0].name} <ExternalLink size={12} />
                    </button>
                    <small>{list.length} passage(s)</small>
                  </h4>
                  {list.map((s) => (
                    <article
                      key={s.id}
                      className={`search-hit${editing === s.id ? " editing" : ""}`}
                    >
                      <span className="locator">
                        {s.locator}
                        {mode !== "exact" && (
                          <small>· score {(s.score * 100).toFixed(0)} %</small>
                        )}
                        <span className="chunk-actions">
                          {editing === s.id ? (
                            <>
                              <button
                                title={t("Save")}
                                disabled={busy || !draft.trim()}
                                onClick={() => void save(s)}
                              >
                                <Check size={14} />
                              </button>
                              <button
                                title={t("Cancel")}
                                onClick={() => setEditing(null)}
                              >
                                <X size={14} />
                              </button>
                            </>
                          ) : (
                            <button
                              title={t("Edit this passage")}
                              disabled={busy}
                              onClick={() => {
                                setEditing(s.id);
                                setDraft(s.text);
                              }}
                            >
                              <Pencil size={14} />
                            </button>
                          )}
                        </span>
                      </span>
                      {editing === s.id ? (
                        <textarea
                          autoFocus
                          rows={Math.min(
                            16,
                            Math.max(4, draft.split("\n").length + 1),
                          )}
                          value={draft}
                          onChange={(e) => setDraft(e.target.value)}
                        />
                      ) : (
                        <pre>
                          <Highlight
                            text={s.text}
                            terms={
                              from ? [...new Set([...terms, from])] : terms
                            }
                          />
                        </pre>
                      )}
                    </article>
                  ))}
                </section>
              ))}
            </div>
          ) : (
            <Empty
              title={t("Nothing found")}
              text={t(
                "Try another mode: Words ignores order, Meaning looks for the idea.",
              )}
            />
          )}
        </>
      )}
    </div>
  );
}
