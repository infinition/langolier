import { useEffect, useRef, useState } from "react";
import { t } from "../i18n";
import { CopyButton } from "../components/Common";
import { listen } from "@tauri-apps/api/event";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Sparkles, LoaderCircle, CornerDownLeft } from "lucide-react";
import { api, errorText } from "../api";
import type { Source } from "../types";

interface Turn {
  question: string;
  answer: string;
  error?: string;
  sources: Source[];
  done: boolean;
  hint?: string;
}

// Floating window opened by the global shortcut.
export default function Palette() {
  const [open, setOpen] = useState(false);
  const [question, setQuestion] = useState("");
  const [turns, setTurns] = useState<Turn[]>([]);
  const [phase, setPhase] = useState("");
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const thread = useRef<HTMLDivElement>(null);
  const busyRef = useRef(false);
  const conversation = useRef<string | null>(null);
  busyRef.current = busy;

  function reset() {
    conversation.current = null;
    setQuestion("");
    setTurns([]);
    setPhase("");
  }
  async function close() {
    setOpen(false);
    if (busyRef.current) void api("cancel_chat").catch(() => {});
    await api("hide_palette").catch(() => {});
  }
  // Edits the last exchange, the one in flight.
  function patchLast(fn: (t: Turn) => Turn) {
    setTurns((all) =>
      all.length ? [...all.slice(0, -1), fn(all[all.length - 1])] : all,
    );
  }
  useEffect(() => {
    const cleanups: (() => void)[] = [];
    let dead = false;
    const add = (p: Promise<() => void>) =>
      void p.then((fn) => (dead ? fn() : cleanups.push(fn)));
    add(
      listen("palette-open", () => {
        reset();
        setOpen(true);
        setTimeout(() => input.current?.focus(), 30);
      }),
    );
    add(
      listen<string>("chat-token", (e) =>
        patchLast((t) => ({ ...t, answer: t.answer + e.payload })),
      ),
    );
    add(listen<string>("chat-phase", (e) => setPhase(e.payload)));
    add(
      listen<{ sources: Source[] }>("chat-sources", (e) =>
        patchLast((t) => ({ ...t, sources: e.payload.sources })),
      ),
    );
    setOpen(true);
    setTimeout(() => input.current?.focus(), 30);
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") void close();
    };
    window.addEventListener("keydown", key);
    return () => {
      dead = true;
      cleanups.forEach((fn) => fn());
      window.removeEventListener("keydown", key);
    };
  }, []);
  useEffect(() => {
    thread.current?.scrollTo({
      top: thread.current.scrollHeight,
      behavior: "smooth",
    });
  }, [turns]);

  async function ask() {
    const q = question.trim();
    if (!q || busy) return;
    setBusy(true);
    setQuestion("");
    setTurns((all) => [
      ...all,
      { question: q, answer: "", sources: [], done: false },
    ]);
    setPhase(t("Connecting to the engine"));
    try {
      if (!conversation.current) {
        conversation.current = await api<string>("new_conversation");
      }
      const r = await api<{ content: string; status: string }>("chat", {
        conversationId: conversation.current,
        question: q,
        mode: "hybrid",
        assisted: false,
      });
      patchLast((turn) => ({
        ...turn,
        answer: r.content,
        done: true,
        hint:
          r.status === "abstained"
            ? t("Add a source, or narrow the question.")
            : undefined,
      }));
    } catch (e) {
      const error = errorText(e);
      patchLast((t) => ({ ...t, error, done: true }));
    } finally {
      setBusy(false);
      setPhase("");
      input.current?.focus();
    }
  }
  return (
    <div
      className={`palette-root${open ? " open" : ""}`}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) void close();
      }}
    >
      <form
        className="palette-bar"
        onSubmit={(e) => {
          e.preventDefault();
          void ask();
        }}
      >
        <Sparkles size={20} className="palette-icon" />
        <input
          ref={input}
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          placeholder={turns.length ? t("Go on…") : t("Ask your memory…")}
          autoFocus
          spellCheck={false}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void ask();
            }
          }}
        />
        {busy ? (
          <LoaderCircle size={18} className="spin palette-hint" />
        ) : (
          <span className="palette-hint">
            <CornerDownLeft size={14} />
          </span>
        )}
      </form>
      {turns.length > 0 && (
        <div
          className="palette-thread"
          ref={thread}
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) void close();
          }}
        >
          {turns.map((t, i) => (
            <div className="palette-turn" key={i}>
              <div className="palette-user">{t.question}</div>
              <div className={`palette-bubble${t.done ? "" : " writing"}`}>
                {t.error ? (
                  <p className="palette-error">{t.error}</p>
                ) : t.answer ? (
                  <div className="palette-answer">
                    {t.done && <CopyButton text={t.answer} />}
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>
                      {t.answer}
                    </ReactMarkdown>
                    {!t.done && <span className="caret" />}
                  </div>
                ) : (
                  <p className="palette-phase">{phase || "…"}</p>
                )}
                {t.hint && <p className="palette-hint-line">{t.hint}</p>}
                {t.done && t.sources.length > 0 && (
                  <p className="palette-sources">
                    {t.sources.slice(0, 4).map((s, j) => (
                      <span key={s.id}>
                        [{j + 1}] {s.name}
                      </span>
                    ))}
                  </p>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
