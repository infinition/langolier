import {
  Logo,
  Button,
  Empty,
  Modal,
  SourceButton,
  PageHeading,
} from "./components/Common";
import { t, useLang } from "./i18n";
import ImportModal from "./components/ImportModal";
import SourceEditor, { confirm } from "./components/SourceEditor";
import LibraryTree from "./components/LibraryTree";
import MemorySearch from "./components/MemorySearch";
import Watches from "./pages/Watches";
import Assistants from "./pages/Assistants";
import Metrics from "./pages/Metrics";
import Lab from "./pages/Lab";
import EngineSettings from "./pages/EngineSettings";
import {
  useState,
  useEffect,
  useRef,
  useCallback,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  ArrowUp,
  ArrowUpRight,
  ArrowRight,
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
  Search,
  ChevronDown,
  ChevronRight,
  CircleCheck,
  CircleAlert,
  LoaderCircle,
  RefreshCw,
  Trash2,
  Eye,
  ListTree,
  List,
  Bot,
  Download,
  Square,
  ShieldCheck,
  Cpu,
  HardDrive,
  Check,
  ThumbsUp,
  ThumbsDown,
  PanelLeftClose,
  PanelLeftOpen,
  Command,
  Sparkles,
  FolderOpen,
  ExternalLink,
  MoreHorizontal,
} from "lucide-react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { api, initial, desktop, errorText, bytes, number } from "./api";
import type {
  Chunk,
  Doc,
  Evaluation,
  Health,
  Message,
  Page,
  Report,
  Settings,
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
const kindIcon = (kind: string) =>
  [
    "mp4",
    "mkv",
    "mov",
    "webm",
    "mp3",
    "wav",
    "url",
    "srt",
    "vtt",
    "m4a",
  ].includes(kind)
    ? Video
    : ["py", "ipynb", "rs", "js", "ts"].includes(kind)
      ? Code2
      : FileText;
function LangSwitch() {
  const { lang, setLang } = useLang();
  return (
    <div className="lang-switch" role="group" aria-label={t("Language")}>
      {(["en", "fr"] as const).map((l) => (
        <button
          key={l}
          className={l === lang ? "on" : ""}
          aria-pressed={l === lang}
          onClick={() => setLang(l)}
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
  const [filter, setFilter] = useState("");
  const [typeFilter, setTypeFilter] = useState("all");
  const [libraryView, setLibraryView] = useState<"list" | "tree" | "search">(
    "list",
  );
  const [treeSort, setTreeSort] = useState<"name" | "date">("date");
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
  /// Removes a source or a whole folder, after confirmation.
  async function removeDocs(ids: string[], label: string) {
    const many = ids.length > 1;
    if (
      !(await confirm(
        many
          ? t(
              'Remove the {n} sources under "{label}" from the index? The original files are untouched.',
              { n: ids.length, label },
            )
          : t(
              'Remove "{label}" from the index? The original file is untouched.',
              { label },
            ),
        many ? t("Remove a folder") : t("Remove a source"),
      ))
    )
      return;
    try {
      const r = await api<{ removed: number; busy: number }>(
        "delete_documents",
        { ids },
      );
      if (r.busy)
        notify(
          t(
            "{n} source(s) removed; {busy} still processing, try again after.",
            { n: r.removed, busy: r.busy },
          ),
        );
      await refresh();
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
        citation_check: { invalid: number[]; missing: boolean };
      }>("chat", { conversationId: id, question: q, mode, assisted });
      setMessages(await api<Message[]>("messages", { id }));
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
  const ready = docs.filter((d) => d.status === "ready").length;
  const queued = docs.filter((d) =>
    ["queued", "processing"].includes(d.status),
  ).length;
  const chunks = docs.reduce((a, d) => a + d.chunks, 0);
  const indexed = docs.reduce((a, d) => a + d.embedded, 0);
  const visibleDocs = docs.filter(
    (d) =>
      d.name.toLowerCase().includes(filter.toLowerCase()) &&
      (typeFilter === "all" ||
        (typeFilter === "video"
          ? kindIcon(d.kind) === Video
          : typeFilter === "code"
            ? kindIcon(d.kind) === Code2
            : kindIcon(d.kind) === FileText)),
  );
  const activeNav = navigation.find((n) => n.id === page)!;
  return (
    <div className={`app ${sidebar ? "" : "collapsed"}`}>
      <aside className="sidebar">
        <button className="brand" onClick={() => setPage("chat")}>
          <Logo />
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
          {data.conversations.length ? (
            data.conversations.slice(0, 8).map((c) => (
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
                  : "Rust + Tauri · v0.1.0"}
              </span>
            </div>
            <ChevronRight size={14} />
          </button>
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
                        aria-label="Envoyer"
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
          <main className="content-page">
            <PageHeading
              eyebrow={t("YOUR KNOWLEDGE, NO SILOS")}
              title={t("The raw material.")}
              description={t(
                "Every source becomes a starting point. Your memory grows with you.",
              )}
              action={
                <Button primary onClick={() => setImportTab("files")}>
                  <Plus size={16} /> {t("Add sources")}
                </Button>
              }
            />
            <div className="library-stats">
              <span>
                <b>{number(docs.length)}</b> {t("sources")}
              </span>
              <span>
                <b>{number(chunks)}</b> {t("passages")}
              </span>
              <span>
                <b>{number(indexed)}</b> {t("vectors")}
              </span>
              <span className={queued ? "lime" : ""}>
                <span
                  className={`status-dot ${queued ? "online pulse" : ""}`}
                />
                {queued
                  ? `${queued} en attente ou en cours`
                  : t("Queue up to date")}
              </span>
            </div>
            <div className="library-toolbar">
              <div className="segmented view-switch" role="tablist">
                <button
                  className={libraryView === "list" ? "selected" : ""}
                  onClick={() => setLibraryView("list")}
                  title={t("List")}
                >
                  <List size={14} /> {t("List")}
                </button>
                <button
                  className={libraryView === "tree" ? "selected" : ""}
                  onClick={() => setLibraryView("tree")}
                  title="Arborescence"
                >
                  <ListTree size={14} /> {t("Tree")}
                </button>
                <button
                  className={libraryView === "search" ? "selected" : ""}
                  onClick={() => setLibraryView("search")}
                  title={t("Search inside passages")}
                >
                  <Search size={14} /> {t("Search")}
                </button>
              </div>
              {libraryView === "tree" && (
                <div className="segmented">
                  <button
                    className={treeSort === "date" ? "selected" : ""}
                    onClick={() => setTreeSort("date")}
                  >
                    {t("Date")}
                  </button>
                  <button
                    className={treeSort === "name" ? "selected" : ""}
                    onClick={() => setTreeSort("name")}
                  >
                    {t("Name")}
                  </button>
                </div>
              )}
              {libraryView !== "search" && (
                <div className="searchbox">
                  <Search size={16} />
                  <input
                    value={filter}
                    onChange={(e) => setFilter(e.target.value)}
                    placeholder={t("Find a source…")}
                    aria-label={t("Filter sources")}
                  />
                </div>
              )}
              <div className="segmented">
                {[
                  ["all", t("All")],
                  ["document", "Documents"],
                  ["video", t("Media")],
                  ["code", "Code"],
                ].map(([id, label]) => (
                  <button
                    key={id}
                    className={typeFilter === id ? "selected" : ""}
                    onClick={() => setTypeFilter(id)}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
            {libraryView === "search" ? (
              <MemorySearch
                onOpen={openFromSearch}
                onError={reportError}
                onChanged={() => void refresh()}
              />
            ) : libraryView === "tree" && docs.length ? (
              <LibraryTree
                docs={visibleDocs}
                dataDir={data.data_dir}
                sort={treeSort}
                onOpen={(d) => void viewDoc(d)}
                onDelete={(ids, label) => void removeDocs(ids, label)}
              />
            ) : !docs.length ? (
              <Empty
                title={t("A whole memory to build.")}
                text={t(
                  "Drop a folder, paste text or add a video. Processing continues in the background.",
                )}
                action={
                  <Button onClick={() => setImportTab("files")}>
                    <Plus size={15} /> {t("Add your first source")}
                  </Button>
                }
              />
            ) : (
              <div className="source-table">
                <div className="table-head">
                  <span>{t("SOURCE")}</span>
                  <span>{t("LANGUAGE")}</span>
                  <span>{t("PASSAGES")}</span>
                  <span>{t("PROCESSING")}</span>
                  <span />
                </div>
                {visibleDocs.map((d) => {
                  const Icon = kindIcon(d.kind);
                  return (
                    <div key={d.id} className="table-row">
                      <button
                        className="source-name"
                        onClick={() => void viewDoc(d)}
                      >
                        <div
                          className={`file-icon ${Icon === Video ? "video" : Icon === Code2 ? "code" : ""}`}
                        >
                          <Icon size={20} />
                        </div>
                        <div>
                          <strong>{d.name}</strong>
                          <small>
                            {d.kind.toUpperCase()} · {bytes(d.bytes)}
                          </small>
                        </div>
                      </button>
                      <span className="language">
                        {d.language?.toUpperCase() || "..."}
                      </span>
                      <span>
                        {d.chunks ? number(d.chunks) : "0"}
                        <small className="vector-count">
                          {d.embedded}/{d.chunks} {t("vectors")}
                        </small>
                      </span>
                      <div className={`doc-status ${d.status}`}>
                        <span>
                          {d.status === "processing" ? (
                            <LoaderCircle className="spin" size={13} />
                          ) : d.status === "ready" ? (
                            <CircleCheck size={13} />
                          ) : d.status === "error" ? (
                            <CircleAlert size={13} />
                          ) : (
                            <span className="status-dot" />
                          )}
                          {d.stage}
                        </span>
                        {d.error && (
                          <button
                            className="error-detail"
                            onClick={() => setError(d.error!)}
                          >
                            {t("See details")}
                          </button>
                        )}
                      </div>
                      <div className="row-actions">
                        <button
                          title={t("Reindex")}
                          disabled={d.status === "processing"}
                          onClick={() =>
                            void api("retry_document", { id: d.id })
                              .then(refresh)
                              .catch(reportError)
                          }
                        >
                          <RefreshCw size={14} />
                        </button>
                        <button
                          title={t("Remove from the index")}
                          disabled={d.status === "processing"}
                          onClick={() =>
                            void api("delete_document", { id: d.id })
                              .then(refresh)
                              .catch(reportError)
                          }
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </div>
                  );
                })}
                {!visibleDocs.length && (
                  <Empty
                    title={t("No matching source")}
                    text={t("Try another name or another filter.")}
                  />
                )}
              </div>
            )}
            <div className="info-note">
              <ShieldCheck size={15} />
              <p>
                {t(
                  "Originals stay where they are. Removing a source drops its passages and vectors from the index. Older messages and their citations stay in the history.",
                )}
              </p>
            </div>
          </main>
        )}
        {page === "watch" && (
          <Watches
            watches={data.watches}
            docs={docs}
            interval={data.settings.watch_interval}
            paused={data.settings.ingestion_paused}
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
