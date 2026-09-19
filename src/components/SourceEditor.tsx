import { useEffect, useRef, useState } from "react";
import { message, t } from "../i18n";
import { ask } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import {
  FileText,
  Pencil,
  Trash2,
  Check,
  X,
  Replace,
  LoaderCircle,
  Wand2,
  ChevronUp,
  ChevronDown,
} from "lucide-react";
import Highlight, { countMatches } from "./Highlight";
import type { Chunk } from "../types";
import { Button, Empty } from "./Common";
import { api, desktop } from "../api";

export async function confirm(question: string, title: string) {
  if (desktop) return ask(question, { title, kind: "warning" });
  return window.confirm(question);
}

/// A source's passages, fixable one by one or by global replace.
export default function SourceEditor({
  docId,
  chunks,
  highlight = "",
  onError,
  onChanged,
}: {
  docId?: string;
  chunks: Chunk[];
  highlight?: string;
  onError: (e: unknown) => void;
  onChanged: (chunks: Chunk[], note?: string) => void;
}) {
  const [editing, setEditing] = useState<number | null>(null);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [from, setFrom] = useState(highlight);
  // Navigation between matches of the replaced term.
  const [current, setCurrent] = useState(0);
  const container = useRef<HTMLDivElement>(null);
  const terms = from ? [from] : [];
  const total = terms.length
    ? chunks.reduce((n, c) => n + countMatches(c.text, terms), 0)
    : 0;
  useEffect(() => {
    setCurrent(0);
  }, [from, chunks]);
  useEffect(() => {
    if (!total) return;
    const el = container.current?.querySelector(`mark[data-hit="${current}"]`);
    el?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [current, total]);
  const step = (d: number) =>
    setCurrent((c) => (total ? (c + d + total) % total : 0));
  const [to, setTo] = useState("");
  const [count, setCount] = useState<{
    chunks: number;
    occurrences: number;
  } | null>(null);
  const [note, setNote] = useState("");
  const [proposal, setProposal] = useState(false);
  const [progress, setProgress] = useState("");
  useEffect(() => {
    if (!desktop) return;
    let dead = false;
    let cleanup: (() => void) | undefined;
    void listen<{
      doc_id: string;
      done: number;
      total: number;
      changed: number;
    }>("polish-progress", (e) => {
      if (e.payload.doc_id !== docId) return;
      setProgress(
        t("Fixing {done}/{total} · {changed} passage(s) changed", {
          done: e.payload.done,
          total: e.payload.total,
          changed: e.payload.changed,
        }),
      );
    }).then((fn) => {
      if (dead) fn();
      else cleanup = fn;
    });
    return () => {
      dead = true;
      cleanup?.();
    };
  }, [docId]);
  async function proposeFix(c: Chunk) {
    setBusy(true);
    try {
      const r = await api<{ proposal: string; changed: boolean }>(
        "polish_chunk",
        { id: c.id },
      );
      if (!r.changed) {
        setNote(t("No correction proposed for this passage."));
        return;
      }
      setEditing(c.id);
      setDraft(r.proposal);
      setProposal(true);
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }
  async function polishAll() {
    if (
      !docId ||
      !(await confirm(
        t(
          "Have the model reread the {n} passage(s) of this source? Only recognition errors are fixed; changed passages are reindexed. The original on disk stays intact.",
          { n: chunks.length },
        ),
        t("Fix the source with AI"),
      ))
    )
      return;
    setBusy(true);
    setProgress(t("Fixing…"));
    try {
      const r = await api<{
        chunks: number;
        changed: number;
        rejected: number;
        cancelled: boolean;
        warning: string | null;
      }>("polish_document", { docId });
      setNote(
        t("{changed} passage(s) fixed out of {chunks}", {
          changed: r.changed,
          chunks: r.chunks,
        }) +
          (r.rejected
            ? t(", {n} proposal(s) rejected", { n: r.rejected })
            : "") +
          (r.cancelled ? t(" (interrupted)") : "") +
          "." +
          (r.warning ? ` ${message(r.warning)}` : ""),
      );
      await reload(chunks);
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
      setProgress("");
    }
  }

  async function reload(fallback: Chunk[], warning?: string | null) {
    const fresh = docId
      ? await api<Chunk[]>("document_chunks", { id: docId })
      : fallback;
    onChanged(fresh, warning || undefined);
    if (warning) setNote(warning);
  }
  async function saveChunk(id: number) {
    setBusy(true);
    try {
      const r = await api<{ warning: string | null }>("update_chunk", {
        id,
        text: draft,
      });
      setEditing(null);
      setProposal(false);
      await reload(
        chunks.map((c) => (c.id === id ? { ...c, text: draft.trim() } : c)),
        r.warning,
      );
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }
  async function removeChunk(c: Chunk) {
    if (
      !(await confirm(
        t(
          "Delete this passage ({locator}) from memory? The original on disk is not modified.",
          { locator: c.locator },
        ),
        t("Delete a passage"),
      ))
    )
      return;
    setBusy(true);
    try {
      await api("delete_chunk", { id: c.id });
      await reload(chunks.filter((x) => x.id !== c.id));
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }
  async function replace(apply: boolean) {
    setBusy(true);
    try {
      const r = await api<{
        chunks: number;
        occurrences: number;
        warning: string | null;
      }>("replace_in_document", { docId: docId || "", from, to, apply });
      setCount({ chunks: r.chunks, occurrences: r.occurrences });
      if (apply) {
        setNote(
          t("{n} occurrence(s) replaced across {chunks} passage(s).", {
            n: r.occurrences,
            chunks: r.chunks,
          }) + (r.warning ? ` ${message(r.warning)}` : ""),
        );
        setCount(null);
        await reload(
          chunks.map((c) => ({ ...c, text: c.text.split(from).join(to) })),
        );
      }
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }

  // Mark offsets, passage by passage.
  const offsets: number[] = [];
  chunks.reduce((n, c) => {
    offsets.push(n);
    return n + (terms.length ? countMatches(c.text, terms) : 0);
  }, 0);
  return (
    <div className="source-preview" ref={container}>
      <div className="replace-bar">
        <Replace size={15} />
        <input
          placeholder={t("Text to highlight / replace")}
          value={from}
          onChange={(e) => {
            setFrom(e.target.value);
            setCount(null);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              step(e.shiftKey ? -1 : 1);
            }
          }}
        />
        {from && (
          <span className="hit-nav" aria-live="polite">
            <button
              type="button"
              title={t("Previous match (Shift+Enter)")}
              disabled={!total}
              onClick={() => step(-1)}
            >
              <ChevronUp size={14} />
            </button>
            <b>{total ? `${current + 1}/${total}` : "0"}</b>
            <button
              type="button"
              title={t("Next match (Enter)")}
              disabled={!total}
              onClick={() => step(1)}
            >
              <ChevronDown size={14} />
            </button>
          </span>
        )}
        <input
          placeholder={t("Replace with")}
          value={to}
          onChange={(e) => setTo(e.target.value)}
        />
        {count ? (
          <Button
            primary
            disabled={busy || !count.occurrences}
            onClick={() => void replace(true)}
          >
            {busy ? (
              <LoaderCircle className="spin" size={14} />
            ) : (
              <Check size={14} />
            )}{" "}
            Remplacer {count.occurrences} occ. / {count.chunks} passage(s)
          </Button>
        ) : (
          <Button disabled={busy || !from} onClick={() => void replace(false)}>
            {t("Count")}
          </Button>
        )}
        <small>
          {docId ? t("In this source") : t("Across the whole memory")}
        </small>
      </div>
      {docId && (
        <div className="polish-bar">
          <Button disabled={busy} onClick={() => void polishAll()}>
            {busy && progress ? (
              <LoaderCircle className="spin" size={14} />
            ) : (
              <Wand2 size={14} />
            )}{" "}
            Corriger toute la source par l’IA
          </Button>
          <small>
            {progress ||
              t(
                "Transcription and OCR errors only: misheard words, technical terms, punctuation. Nothing is rewritten.",
              )}
          </small>
        </div>
      )}
      {note && <p className="muted">{note}</p>}
      {chunks.length ? (
        chunks.map((c, i) => (
          <section key={c.id} className={editing === c.id ? "editing" : ""}>
            <span className="locator">
              <FileText size={12} />
              {c.locator}
              <span className="chunk-actions">
                {editing === c.id ? (
                  <>
                    <button
                      title={t("Save")}
                      disabled={busy || !draft.trim()}
                      onClick={() => void saveChunk(c.id)}
                    >
                      <Check size={14} />
                    </button>
                    <button
                      title={t("Cancel")}
                      disabled={busy}
                      onClick={() => {
                        setEditing(null);
                        setProposal(false);
                      }}
                    >
                      <X size={14} />
                    </button>
                  </>
                ) : (
                  <>
                    <button
                      title={t("Propose an AI correction")}
                      disabled={busy}
                      onClick={() => void proposeFix(c)}
                    >
                      <Wand2 size={14} />
                    </button>
                    <button
                      title={t("Fix this passage")}
                      disabled={busy}
                      onClick={() => {
                        setEditing(c.id);
                        setDraft(c.text);
                        setProposal(false);
                      }}
                    >
                      <Pencil size={14} />
                    </button>
                    <button
                      title={t("Delete this passage")}
                      disabled={busy}
                      onClick={() => void removeChunk(c)}
                    >
                      <Trash2 size={14} />
                    </button>
                  </>
                )}
              </span>
            </span>
            {editing === c.id && proposal && (
              <p className="proposal-note">
                <Wand2 size={12} /> Proposition de l’IA — relisez avant
                d’enregistrer.
              </p>
            )}
            {editing === c.id ? (
              <textarea
                autoFocus
                rows={Math.min(18, Math.max(4, draft.split("\n").length + 1))}
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
              />
            ) : (
              <pre>
                <Highlight
                  text={c.text}
                  terms={terms}
                  offset={offsets[i]}
                  current={current}
                />
              </pre>
            )}
          </section>
        ))
      ) : (
        <Empty
          title={t("No indexed text yet")}
          text={t(
            "Processing is queued or needs attention. Check its status under Sources.",
          )}
        />
      )}
      {chunks.length === 500 && (
        <p className="muted">
          {t("Preview limited to the first 500 passages.")}
        </p>
      )}
    </div>
  );
}
