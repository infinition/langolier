import {
  CopyButton,
  Logo,
  Modal,
  SourceButton,
} from "./components/Common";
import { t, useLang } from "./i18n";
import ImportModal from "./components/ImportModal";
import SourceEditor from "./components/SourceEditor";
import LibraryPage from "./pages/Library";
import Watches from "./pages/Watches";
import Assistants from "./pages/Assistants";
import Metrics from "./pages/Metrics";
import Lab from "./pages/Lab";
import EngineSettings from "./pages/EngineSettings";
import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  ArrowUp,
  ArrowUpRight,
  Plus,
  MessageCircle,
  Library,
  Activity,
  FlaskConical,
  SlidersHorizontal,
  FileText,
  Video,
  Code2,
  Link2,
  Clipboard,
  Upload,
  X,
  ChevronDown,
  ChevronRight,
  CircleCheck,
  CircleAlert,
  LoaderCircle,
  Trash2,
  Eye,
  Bot,
  Square,
  Globe,
  Github,
  Coffee,
  ShieldCheck,
  Cpu,
  HardDrive,
  ThumbsUp,
  ThumbsDown,
  PanelLeftClose,
  PanelLeftOpen,
  Sparkles,
} from "lucide-react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  api,
  initial,
  desktop,
  errorText,
  bytes,
  number,
} from "./api";
import type {
  Chunk,
  Doc,
  Health,
  Message,
  Page,
  Snapshot,
  Source,
} from "./types";
const navigation: {
  id: Page;
  name: string;
  icon: typeof Library;
  shortcut: string;
}[] = [
  { id: "chat", name: "Conversation", icon: MessageCircle, shortcut: "01" },
  { id: "library", name: "Sources", icon: Library, shortcut: "02" },
  { id: "watch", name: "Watches", icon: Eye, shortcut: "03" },
  { id: "assistants", name: "Assistants", icon: Bot, shortcut: "04" },
  { id: "metrics", name: "Observatory", icon: Activity, shortcut: "05" },
  { id: "lab", name: "Laboratory", icon: FlaskConical, shortcut: "06" },
  { id: "settings", name: "Engines", icon: SlidersHorizontal, shortcut: "07" },
];
function LangSwitch() {
  const { lang, setLang } = useLang();
  return (
    <div className="lang-switch" role="group" aria-label={t("Language")}>
      {(["en", "fr"] as const).map((l) => (
        <button
          key={l}
          className={l === lang ? "on" : ""}
          aria-pressed={l === lang}
          onClick={() => {
            setLang(l);
            // Native menus live outside the interface's translations and
            // have to be told; failing is harmless, they stay as they were.
            if (desktop) void api("set_interface_language", { lang: l }).catch(() => {});
          }}
        >
          {l.toUpperCase()}
        </button>
      ))}
    </div>
  );
}

export default function App() {
  const [data, setData] = useState<Snapshot>(initial);
  const [health, setHealth] = useState<Health | null>(null);
  const [page, setPage] = useState<Page>("chat");
  const [sidebar, setSidebar] = useState(true);
  const [importTab, setImportTab] = useState<"files" | "text" | "url" | null>(
    null,
  );
  const [toast, setToast] = useState("");
  const [error, setError] = useState("");
  const [dragging, setDragging] = useState(false);
  const [conversation, setConversation] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  const [stream, setStream] = useState("");
  const [phase, setPhase] = useState("");
  const [sources, setSources] = useState<Source[]>([]);
  const [mode, setMode] = useState("hybrid");
  const [assisted, setAssisted] = useState(false);
  const [preview, setPreview] = useState<{
    title: string;
    docId?: string;
    highlight?: string;
    chunks: Chunk[];
  } | null>(null);
  async function openFromSearch(s: Source, highlight: string) {
    try {
      const chunks = await api<Chunk[]>("document_chunks", { id: s.doc_id });
      setPreview({ title: s.name, docId: s.doc_id, highlight, chunks });
    } catch (e) {
      reportError(e);
    }
  }
  const bottom = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLTextAreaElement>(null);
  const reportError = useCallback((e: unknown) => setError(errorText(e)), []);
  const refresh = useCallback(async () => {
    try {
      setData(await api<Snapshot>("snapshot"));
    } catch (e) {
      reportError(e);
    }
  }, [reportError]);
  const checkHealth = useCallback(async () => {
    try {
      setHealth(await api<Health>("health"));
    } catch (e) {
      if (desktop) reportError(e);
    }
  }, [reportError]);
  const notify = useCallback((text: string) => {
    setToast(text);
  }, []);
  const imported = useCallback(
    async (command: string, args: Record<string, unknown>) => {
      try {
        const r = await api<{ added: number; skipped: string[] }>(
          command,
          args,
        );
        notify(
          t("{n} source(s) added", { n: r.added }) +
            (r.skipped.length
              ? t(" · {n} skipped", { n: r.skipped.length })
              : ""),
        );
        await refresh();
        setImportTab(null);
      } catch (e) {
        reportError(e);
        throw e;
      }
    },
    [refresh, notify, reportError],
  );
  useEffect(() => {
    void refresh();
    void checkHealth();
    const timer = setInterval(() => void refresh(), 3000);
    const healthTimer = setInterval(() => void checkHealth(), 30000);
    return () => {
      clearInterval(timer);
      clearInterval(healthTimer);
    };
  }, [refresh, checkHealth]);
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(""), 5000);
    return () => clearTimeout(t);
  }, [toast]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement | null)?.closest?.(".shortcut-capture"))
        return;
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setImportTab("files");
      }
      if (e.key === "Escape") {
        setImportTab(null);
        setPreview(null);
        setError("");
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    const unlisten: (() => void)[] = [];
    const add = async (p: Promise<() => void>) => {
      const fn = await p;
      if (disposed) fn();
      else unlisten.push(fn);
    };
    void add(
      listen<string>("chat-token", (e) => setStream((t) => t + e.payload)),
    );
    void add(listen<string>("chat-phase", (e) => setPhase(e.payload)));
    void add(
      listen<{ sources: Source[]; warning?: string }>("chat-sources", (e) => {
        setSources(e.payload.sources);
        if (e.payload.warning) setError(e.payload.warning);
      }),
    );
    void add(
      getCurrentWebviewWindow().onDragDropEvent((e) => {
        if (e.payload.type === "over" || e.payload.type === "enter")
          setDragging(true);
        else {
          setDragging(false);
          if (e.payload.type === "drop")
            void imported("import_files", { paths: e.payload.paths }).catch(
              () => {},
            );
        }
      }),
    );
    return () => {
      disposed = true;
      unlisten.forEach((fn) => fn());
    };
  }, [imported]);
  useEffect(() => {
    bottom.current?.scrollIntoView({ behavior: "smooth" });
  }, [stream, messages.length]);
  async function pickFiles(directory = false) {
    try {
      const paths = await open({
        multiple: !directory,
        directory,
        title: directory ? t("Add a folder") : t("Feed Langolier"),
      });
      if (paths)
        await imported("import_files", {
          paths: Array.isArray(paths) ? paths : [paths],
        });
    } catch (e) {
      reportError(e);
    }
  }
  async function viewDoc(doc: Doc) {
    try {
      const chunks = await api<Chunk[]>("document_chunks", { id: doc.id });
      setPreview({ title: doc.name, docId: doc.id, chunks });
    } catch (e) {
      reportError(e);
    }
  }
  /// Immediate deletion, no confirmation.
  async function removeConversation(id: string) {
    try {
      await api("delete_conversation", { id });
      if (conversation === id) {
        setConversation(null);
        setMessages([]);
      }
      await refresh();
    } catch (e) {
      reportError(e);
    }
  }
  async function selectConversation(id: string) {
    if (busy) return;
    try {
      const m = await api<Message[]>("messages", { id });
      setConversation(id);
      setMessages(m);
      setPage("chat");
    } catch (e) {
      reportError(e);
    }
  }
  async function send() {
    const q = question.trim();
    if (!q || busy) return;
    setBusy(true);
    setError("");
    setStream("");
    setSources([]);
    setPhase(t("Connecting to the engine"));
    setQuestion("");
    let id = conversation;
    try {
      if (!id) {
        id = await api<string>("new_conversation");
        setConversation(id);
      }
      setMessages((m) => [
        ...m,
        { id: "pending", role: "user", content: q, sources: [] },
      ]);
      const response = await api<{
        status: string;
        citation_check: { invalid: number[]; missing: boolean };
      }>("chat", { conversationId: id, question: q, mode, assisted });
      setMessages(await api<Message[]>("messages", { id }));
      if (response.status === "abstained")
        notify(
          t(
            "Nothing in your sources supports an answer. Add a source, narrow the question, or switch to Free conversation.",
          ),
        );
      if (
        response.citation_check.invalid.length ||
        response.citation_check.missing
      )
        notify(
          t("Check the citations: some references are missing or invalid."),
        );
      await refresh();
    } catch (e) {
      reportError(e);
      setQuestion(q);
      if (id)
        setMessages(await api<Message[]>("messages", { id }).catch(() => []));
    } finally {
      setBusy(false);
      setStream("");
      setPhase("");
    }
  }
  async function rate(id: string, value: number) {
    try {
      await api("feedback", { id, value });
      setMessages((m) =>
        m.map((x) => (x.id === id ? { ...x, feedback: value } : x)),
      );
      void refresh();
    } catch (e) {
      reportError(e);
    }
  }
  async function exportData(kind: string) {
    try {
      const path = await save({
        defaultPath:
          kind === "metrics"
            ? "langolier-metrics.json"
            : "langolier-training.jsonl",
      });
      if (path) {
        await api("export_data", { kind, path });
        notify(t("Export saved."));
      }
    } catch (e) {
      reportError(e);
    }
  }
  const docs = data.documents;
  // Something is being read, transcribed or vectorised: the creature eats.
  const ingesting = docs.some((d) =>
    ["queued", "processing"].includes(d.status),
  );
  const ready = docs.filter((d) => d.status === "ready").length;
  const chunks = docs.reduce((a, d) => a + d.chunks, 0);
  const activeNav = navigation.find((n) => n.id === page)!;
  return (
    <div className={`app ${sidebar ? "" : "collapsed"}`}>
      <aside className="sidebar">
        <button className="brand" onClick={() => setPage("chat")}>
          <Logo interactive busy={ingesting} />
          <span>
            langolier<span className="brand-dot">.</span>
          </span>
        </button>
        <div className="nav-label">{t("YOUR SPACE")}</div>
        <nav>
          {navigation.map(({ id, name, icon: Icon, shortcut }) => (
            <button
              key={id}
              className={page === id ? "active" : ""}
              onClick={() => setPage(id)}
            >
              <Icon size={17} />
              <span>{t(name)}</span>
              {id === "library" && docs.length > 0 ? (
                <b>{docs.length}</b>
              ) : (
                <small>{shortcut}</small>
              )}
            </button>
          ))}
        </nav>
        <div className="history">
          <div className="nav-label">
            {t("CONVERSATIONS")} <MessageCircle size={12} />
          </div>
          <button
            className="new-chat"
            disabled={busy}
            onClick={() => {
              setConversation(null);
              setMessages([]);
              setPage("chat");
              setQuestion("");
            }}
          >
            <Plus size={16} /> {t("New conversation")}
          </button>
          <div className="history-list">
          {data.conversations.length ? (
            data.conversations.map((c) => (
              <div
                key={c.id}
                className={`history-item${conversation === c.id ? " selected" : ""}`}
              >
                <button
                  disabled={busy}
                  className={conversation === c.id ? "selected" : ""}
                  onClick={() => void selectConversation(c.id)}
                >
                  <span className="truncate">{c.title}</span>
                </button>
                <button
                  className="history-delete"
                  title={t("Delete this conversation")}
                  aria-label={t('Delete "{title}"', { title: c.title })}
                  disabled={busy}
                  onClick={() => void removeConversation(c.id)}
                >
                  <Trash2 size={13} />
                </button>
              </div>
            ))
          ) : (
            <p>
              {t("Your next ideas")}
              <br />
              {t("start here.")}
            </p>
          )}
          </div>
        </div>
        <div className="sidebar-bottom">
          <div className="local-status">
            <span
              className={`status-dot ${health?.engine_ok ? "online" : ""}`}
            />
            <strong>
              {health?.engine_ok
                ? t("Engine connected")
                : desktop
                  ? t("Engine needs setup")
                  : t("Interface preview")}
            </strong>
            <span>LOCAL</span>
          </div>
          <LangSwitch />
          <button className="machine" onClick={() => setPage("settings")}>
            <Cpu size={17} />
            <div>
              <strong>
                {health
                  ? `${health.platform} · ${health.arch}`
                  : t("Your machine, your AI")}
              </strong>
              <span>
                {health
                  ? t("{size} of unified memory / RAM", {
                      size: bytes(health.memory_total),
                    })
                  : "Rust + Tauri · v0.2.0"}
              </span>
            </div>
            <ChevronRight size={14} />
          </button>
          <div className="sidebar-links">
            {(
              [
                ["site", Globe, t("The website")],
                ["source", Github, t("The source code")],
                ["support", Coffee, t("Support the project")],
              ] as const
            ).map(([target, Icon, label]) => (
              <button
                key={target}
                className={target === "support" ? "support" : ""}
                title={label}
                aria-label={label}
                onClick={() =>
                  void api("open_link", { target }).catch(reportError)
                }
              >
                <Icon size={15} />
              </button>
            ))}
          </div>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div>
            <button
              className="icon-button"
              aria-label={t("Show or hide the menu")}
              onClick={() => setSidebar((v) => !v)}
            >
              {sidebar ? (
                <PanelLeftClose size={17} />
              ) : (
                <PanelLeftOpen size={17} />
              )}
            </button>
            <span className="breadcrumb">{t("Personal space")}</span>
            <span className="slash">/</span>
            <strong>{activeNav.name}</strong>
          </div>
          <div>
            <span className="privacy">
              <ShieldCheck size={14} /> {t("Your data stays here")}
            </span>
            <button
              className="top-import"
              onClick={() => setImportTab("files")}
            >
              <Plus size={15} /> {t("Add sources")} <kbd>⌘ K</kbd>
            </button>
          </div>
        </header>
        {!desktop && (
          <div className="preview-banner">
            {t(
              "Visual preview. Sources, models and metrics become active in the native app.",
            )}
          </div>
        )}
        {page === "chat" && (
          <main
            className={`chat-page ${messages.length ? "has-messages" : ""}`}
          >
            {!messages.length && !busy ? (
              <div className="welcome">
                <div className="eyebrow">
                  <span className="status-dot online" />{" "}
                  {t("ONE MEMORY. A THOUSAND CONNECTIONS.")}
                </div>
                <h1>
                  {t("Make sense of")}
                  <br />
                  <span>{t("everything you know.")}</span>
                </h1>
                <p className="hero-description">
                  {t(
                    "Your documents, your code, your videos, your voice notes.",
                  )}
                  <br />
                  {t("One place to connect them. An AI to explore them.")}
                </p>
                <div
                  className="ingest-hero"
                  onClick={() => setImportTab("files")}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ")
                      setImportTab("files");
                  }}
                >
                  <div className="orbit" aria-hidden="true">
                    <div className="orbit-ring r1" />
                    <div className="orbit-ring r2" />
                    <span className="orbit-file f1">
                      <FileText size={20} />
                    </span>
                    <span className="orbit-file f2">
                      <Code2 size={20} />
                    </span>
                    <span className="orbit-file f3">
                      <Video size={20} />
                    </span>
                    <div className="orbit-core">
                      <Logo />
                    </div>
                    <span className="orbit-spark" />
                  </div>
                  <div className="ingest-copy">
                    <div className="mini-label">{t("FEED YOUR CURIOSITY")}</div>
                    <h2>{t("Drag. Drop. Connect.")}</h2>
                    <p>
                      {t("A whole folder, a video or a passing thought.")}
                      <br />
                      {t("Langolier turns it into a memory you can explore.")}
                    </p>
                    <div className="format-tags">
                      <span>PDF</span>
                      <span>{t("OFFICE")}</span>
                      <span>{t("CODE & NOTEBOOKS")}</span>
                      <span>{t("VIDEO")}</span>
                      <span>{t("+ TEXT")}</span>
                    </div>
                  </div>
                  <button
                    className="circle-add"
                    aria-label={t("Add sources")}
                    onClick={(e) => {
                      e.stopPropagation();
                      setImportTab("files");
                    }}
                  >
                    <Plus size={24} />
                  </button>
                </div>
                <div className="quick-actions">
                  <button onClick={() => void pickFiles()}>
                    <Upload size={15} /> {t("Import files")}{" "}
                    <ArrowUpRight size={13} />
                  </button>
                  <button onClick={() => setImportTab("url")}>
                    <Link2 size={15} /> {t("Add a video link")}{" "}
                    <ArrowUpRight size={13} />
                  </button>
                  <button onClick={() => setImportTab("text")}>
                    <Clipboard size={15} /> {t("Paste text")}{" "}
                    <ArrowUpRight size={13} />
                  </button>
                </div>
                <div className="memory-strip">
                  <div>
                    <strong>{number(ready)}</strong>
                    <span>{t("sources absorbed")}</span>
                  </div>
                  <div>
                    <strong>{number(chunks)}</strong>
                    <span>{t("passages in memory")}</span>
                  </div>
                  <div>
                    <strong>
                      {
                        new Set(docs.map((d) => d.language).filter(Boolean))
                          .size
                      }
                    </strong>
                    <span>{t("languages detected")}</span>
                  </div>
                  <div>
                    <span className="memory-icon">
                      <HardDrive size={19} />
                    </span>
                    <span>
                      {t("Your knowledge.")}
                      <br />
                      <b>{t("On your machine.")}</b>
                    </span>
                  </div>
                </div>
              </div>
            ) : (
              <div className="messages">
                <div className="conversation-heading">
                  <div className="eyebrow">
                    {t("THE THREAD OF YOUR THINKING")}
                  </div>
                  <span>{data.settings.model}</span>
                </div>
                {messages.map((m) => (
                  <article key={m.id} className={`message ${m.role}`}>
                    <div className="message-avatar">
                      {m.role === "assistant" ? <Logo small /> : t("YOU")}
                    </div>
                    <div className="message-body">
                      <div className="message-label">
                        {m.role === "assistant" ? "Langolier" : t("You")}
                      </div>
                      {m.role === "assistant" && m.content && (
                        <CopyButton text={m.content} />
                      )}
                      <Markdown
                        remarkPlugins={[remarkGfm]}
                        components={{
                          a: ({ children }) => (
                            <span className="markdown-link">{children}</span>
                          ),
                          img: () => (
                            <span>{t("[External image not loaded]")}</span>
                          ),
                        }}
                      >
                        {m.content}
                      </Markdown>
                      {m.sources.length > 0 && (
                        <div className="source-list">
                          {m.sources.map((s, i) => (
                            <SourceButton
                              key={s.id}
                              source={s}
                              index={i}
                              onClick={() =>
                                setPreview({
                                  title: s.name,
                                  docId: s.doc_id,
                                  chunks: [s],
                                })
                              }
                            />
                          ))}
                        </div>
                      )}
                      {m.role === "assistant" && (
                        <div className="message-feedback">
                          <button
                            className={m.feedback === 1 ? "selected" : ""}
                            aria-label={t("Helpful answer")}
                            onClick={() => void rate(m.id, 1)}
                          >
                            <ThumbsUp size={13} />
                          </button>
                          <button
                            className={m.feedback === -1 ? "selected" : ""}
                            aria-label={t("Answer needs work")}
                            onClick={() => void rate(m.id, -1)}
                          >
                            <ThumbsDown size={13} />
                          </button>
                          <span>
                            {t("Check the sources for anything that matters.")}
                          </span>
                        </div>
                      )}
                    </div>
                  </article>
                ))}
                {busy && (
                  <article className="message assistant streaming">
                    <div className="message-avatar">
                      <Logo small />
                    </div>
                    <div className="message-body">
                      <div className="message-label">
                        Langolier{" "}
                        <span>
                          <LoaderCircle size={12} className="spin" />
                          {phase}
                        </span>
                      </div>
                      <Markdown
                        remarkPlugins={[remarkGfm]}
                        components={{
                          a: ({ children }) => <span>{children}</span>,
                          img: () => null,
                        }}
                      >
                        {stream || " "}
                      </Markdown>
                      {!stream && (
                        <div className="thinking-dots">
                          <i />
                          <i />
                          <i />
                        </div>
                      )}
                      <div className="source-list">
                        {sources.map((s, i) => (
                          <SourceButton
                            key={s.id}
                            source={s}
                            index={i}
                            onClick={() =>
                              setPreview({
                                title: s.name,
                                docId: s.doc_id,
                                chunks: [s],
                              })
                            }
                          />
                        ))}
                      </div>
                    </div>
                  </article>
                )}
                <div ref={bottom} />
              </div>
            )}
            <div className="composer-wrap">
              <div className="composer">
                <textarea
                  ref={input}
                  aria-label={t("Your question")}
                  placeholder={
                    ready
                      ? t("Ask your memory a question…")
                      : t("Your next idea starts with a question…")
                  }
                  value={question}
                  onChange={(e) => setQuestion(e.target.value)}
                  onKeyDown={(e) => {
                    if (
                      e.key === "Enter" &&
                      !e.shiftKey &&
                      !e.nativeEvent.isComposing
                    ) {
                      e.preventDefault();
                      void send();
                    }
                  }}
                  rows={2}
                />
                <div className="composer-toolbar">
                  <div>
                    <button
                      className="icon-button"
                      aria-label={t("Add a source")}
                      onClick={() => setImportTab("files")}
                    >
                      <Plus size={18} />
                    </button>
                    <span className="toolbar-divider" />
                    <select
                      aria-label={t("Conversation mode")}
                      value={mode}
                      onChange={(e) => setMode(e.target.value)}
                    >
                      <option value="hybrid">{t("Hybrid memory")}</option>
                      <option value="semantic">{t("Semantic search")}</option>
                      <option value="lexical">{t("Lexical search")}</option>
                      <option value="general">{t("Free conversation")}</option>
                    </select>
                    <button
                      className={`assist-toggle ${assisted ? "active" : ""}`}
                      title={t(
                        "Rewrites the question using the history and merges two searches. Costs one extra model call.",
                      )}
                      onClick={() => setAssisted((a) => !a)}
                    >
                      <Sparkles size={13} /> {t("Deep search")}
                    </button>
                  </div>
                  <div>
                    <select
                      className="model-label assistant-select"
                      aria-label={t("Assistant profile")}
                      title={t("Profile used for new conversations")}
                      value={data.settings.active_assistant}
                      disabled={busy}
                      onChange={(e) =>
                        void api("set_active_assistant", { id: e.target.value })
                          .then(() => {
                            setConversation(null);
                            setMessages([]);
                          })
                          .then(refresh)
                          .catch(reportError)
                      }
                    >
                      <option value="">Langolier</option>
                      {data.assistants.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.name}
                        </option>
                      ))}
                    </select>
                    <button
                      className="model-label"
                      onClick={() => setPage("settings")}
                    >
                      <span
                        className={`status-dot ${health?.engine_ok ? "online" : ""}`}
                      />
                      {data.settings.model}
                      <ChevronDown size={12} />
                    </button>
                    {busy ? (
                      <button
                        className="send stop"
                        aria-label={t("Stop generating")}
                        onClick={() =>
                          void api("cancel_chat").catch(reportError)
                        }
                      >
                        <Square size={15} fill="currentColor" />
                      </button>
                    ) : (
                      <button
                        className="send"
                        aria-label={t("Send")}
                        disabled={!question.trim()}
                        onClick={() => void send()}
                      >
                        <ArrowUp size={20} />
                      </button>
                    )}
                  </div>
                </div>
              </div>
              <div className="composer-note">
                <span>
                  <ShieldCheck size={11} />{" "}
                  {t("Local inference · Traceable sources")}
                </span>
                <span>
                  {t("Enter to send")} <span className="dim">·</span>{" "}
                  {t("⇧ Enter for a new line")}
                </span>
              </div>
            </div>
          </main>
        )}
        {page === "library" && (
          <LibraryPage
            docs={docs}
            dataDir={data.data_dir}
            settings={data.settings}
            onImport={() => setImportTab("files")}
            onOpenDoc={(d) => void viewDoc(d)}
            onOpenPassage={openFromSearch}
            onError={reportError}
            onNotify={notify}
            onRefresh={refresh}
            onShowDetail={setError}
          />
        )}
        {page === "watch" && (
          <Watches
            watches={data.watches}
            docs={docs}
            settings={data.settings}
            onError={reportError}
            onChanged={refresh}
          />
        )}
        {page === "assistants" && (
          <Assistants
            assistants={data.assistants}
            docs={docs}
            watches={data.watches}
            settings={data.settings}
            dataDir={data.data_dir}
            onPreview={(d) => void viewDoc(d)}
            onError={reportError}
            onNotify={notify}
            onChanged={refresh}
          />
        )}
        {page === "metrics" && (
          <Metrics
            data={data}
            health={health}
            onExport={() => void exportData("metrics")}
          />
        )}
        {page === "lab" && (
          <Lab
            data={data}
            refresh={refresh}
            onError={reportError}
            onExport={() => void exportData("training")}
            onSource={(s) =>
              setPreview({ title: s.name, docId: s.doc_id, chunks: [s] })
            }
          />
        )}
        {page === "settings" && (
          <EngineSettings
            settings={data.settings}
            health={health}
            docs={docs}
            dataDir={data.data_dir}
            onError={reportError}
            onSaved={() => {
              void refresh();
              void checkHealth();
              notify(t("Settings saved."));
            }}
            onRefresh={checkHealth}
          />
        )}
      </div>
      {importTab && (
        <ImportModal
          tab={importTab}
          setTab={setImportTab}
          close={() => setImportTab(null)}
          pickFiles={pickFiles}
          imported={imported}
        />
      )}
      {preview && (
        <Modal title={preview.title} wide onClose={() => setPreview(null)}>
          <SourceEditor
            key={preview.docId || preview.title}
            docId={preview.docId}
            highlight={preview.highlight}
            chunks={preview.chunks}
            onError={reportError}
            onChanged={(chunks, note) => {
              setPreview((p) => (p ? { ...p, chunks } : p));
              if (note) notify(note);
              void refresh();
            }}
          />
        </Modal>
      )}
      {error && (
        <div className="error-banner" role="alert">
          <CircleAlert size={18} />
          <div>
            <strong>{t("Something to check")}</strong>
            <p>{error}</p>
          </div>
          <button aria-label={t("Dismiss error")} onClick={() => setError("")}>
            <X size={17} />
          </button>
        </div>
      )}
      {toast && (
        <div className="toast" role="status">
          <CircleCheck size={16} />
          {toast}
        </div>
      )}
      {dragging && (
        <div className="drop-overlay">
          <div>
            <Logo />
            <h2>{t("Shall we turn this into something?")}</h2>
            <p>{t("Drop your files to add them to your memory.")}</p>
            <Upload size={30} />
          </div>
        </div>
      )}
    </div>
  );
}
