import { useEffect, useState } from "react";
import { t } from "../i18n";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import {
  Bot,
  Plus,
  Trash2,
  Check,
  LoaderCircle,
  Image as ImageIcon,
  Package,
  FileDown,
  Star,
  FolderOpen,
} from "lucide-react";
import type { Assistant, Doc, Settings, Watch } from "../types";
import { PROVIDERS, THEMES } from "../types";
import { captureShortcut, formatShortcut } from "../shortcut";
import { Button, PageHeading, Empty } from "../components/Common";
import { confirm } from "../components/SourceEditor";
import LibraryTree from "../components/LibraryTree";
import ApiKeyField from "../components/ApiKeyField";
import { api, desktop, bytes, errorText } from "../api";

// Square-crops the image to 256 px.
function shrink(dataUrl: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const side = Math.min(img.width, img.height);
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 256;
      const ctx = canvas.getContext("2d")!;
      ctx.drawImage(
        img,
        (img.width - side) / 2,
        (img.height - side) / 2,
        side,
        side,
        0,
        0,
        256,
        256,
      );
      resolve(canvas.toDataURL("image/png"));
    };
    img.onerror = () => reject(new Error(t("Unreadable image")));
    img.src = dataUrl;
  });
}
const blank = (): Assistant => ({
  id: "",
  name: "",
  mission: "",
  welcome: "",
  avatar: "",
  theme: "nuit",
  scope: { all: true },
  overrides: {},
  show_sources: true,
  hide_source_names: false,
  max_conversation_tokens: 0,
  daily_token_budget: 0,
  telegram_token: "",
  telegram_allowed: "",
  admin_enabled: false,
  admin_secret: "",
  admin_secret_set: false,
  admin_import: true,
  admin_restore: true,
  language: "auto",
  tray_icon: false,
  palette_shortcut: "",
  start_hidden: false,
  created: 0,
});

export default function Assistants({
  assistants,
  docs,
  watches,
  settings,
  dataDir,
  onPreview,
  onError,
  onNotify,
  onChanged,
}: {
  assistants: Assistant[];
  docs: Doc[];
  watches: Watch[];
  settings: Settings;
  dataDir: string;
  onPreview: (d: Doc) => void;
  onError: (e: unknown) => void;
  onNotify: (m: string) => void;
  onChanged: () => Promise<void>;
}) {
  const [editing, setEditing] = useState<Assistant | null>(null);
  const [busy, setBusy] = useState(false);
  const [docFilter, setDocFilter] = useState("");
  const [engine, setEngine] = useState<
    "profile" | "embedded" | "embedded-light"
  >("profile");
  const [progress, setProgress] = useState("");
  const [telegramStatus, setTelegramStatus] = useState("");
  const [capturing, setCapturing] = useState(false);
  async function checkTelegram() {
    if (!editing) return;
    setTelegramStatus(t("Checking…"));
    try {
      const r = await api<{ username: string; name: string }>(
        "telegram_check",
        { token: editing.telegram_token },
      );
      setTelegramStatus(
        t("Bot recognised: @{username} ({name}).", {
          username: r.username,
          name: r.name,
        }),
      );
    } catch (e) {
      setTelegramStatus(errorText(e));
    }
  }
  useEffect(() => {
    if (!desktop) return;
    let dead = false;
    let cleanup: (() => void) | undefined;
    void listen<{ stage: string; done: number; total: number }>(
      "export-progress",
      (e) =>
        setProgress(
          `${e.payload.stage}${e.payload.total ? ` · ${Math.round((e.payload.done / e.payload.total) * 100)} %` : ""}`,
        ),
    ).then((fn) => {
      if (dead) fn();
      else cleanup = fn;
    });
    return () => {
      dead = true;
      cleanup?.();
    };
  }, []);
  // A list refresh never overwrites the draft.

  const field = <K extends keyof Assistant>(k: K, v: Assistant[K]) =>
    setEditing((a) => (a ? { ...a, [k]: v } : a));
  const override = (k: string, v: unknown) =>
    setEditing((a) => {
      if (!a) return a;
      const o = { ...a.overrides };
      if (v === undefined || v === "" || v === null) delete o[k];
      else o[k] = v;
      return { ...a, overrides: o };
    });
  const scopeAll = !!editing?.scope?.all;
  const scopeWatches: string[] = (editing?.scope?.watches as string[]) || [];
  const scopeDocs: string[] = (editing?.scope?.documents as string[]) || [];
  const setScope = (patch: Record<string, unknown>) =>
    setEditing((a) =>
      a ? { ...a, scope: { ...(a.scope.all ? {} : a.scope), ...patch } } : a,
    );

  async function save() {
    if (!editing) return;
    setBusy(true);
    try {
      const saved = await api<Assistant>("save_assistant", {
        assistant: editing,
      });
      setEditing(saved);
      await onChanged();
      onNotify(t('"{name}" saved.', { name: saved.name }));
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }
  async function remove(a: Assistant) {
    if (
      !(await confirm(
        t('Delete the profile "{name}"? Conversations and memory stay.', {
          name: a.name,
        }),
        t("Delete the assistant"),
      ))
    )
      return;
    try {
      await api("delete_assistant", { id: a.id });
      if (editing?.id === a.id) setEditing(null);
      await onChanged();
    } catch (e) {
      onError(e);
    }
  }
  async function activate(id: string) {
    try {
      await api("set_active_assistant", { id });
      await onChanged();
    } catch (e) {
      onError(e);
    }
  }
  async function pickAvatar() {
    try {
      const file = await open({
        multiple: false,
        filters: [
          { name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] },
        ],
      });
      if (typeof file !== "string") return;
      const data = await api<string>("read_image_data_url", { path: file });
      field("avatar", await shrink(data));
    } catch (e) {
      onError(e);
    }
  }
  async function exportAs(kind: "langolier" | "kit") {
    if (!editing?.id) return;
    try {
      const dest = await open({
        directory: true,
        multiple: false,
        title:
          kind === "kit"
            ? t("Folder to create the chatbot in")
            : t("Folder to write the .langolier file to"),
      });
      if (typeof dest !== "string") return;
      setBusy(true);
      const r = await api<{
        path: string;
        bytes?: number;
        payload?: number;
        launchers?: string[];
      }>("export_assistant", { id: editing.id, dest, kind, engine });
      onNotify(
        kind === "kit"
          ? t(
              "Chatbot exported to {path}: {launchers} + .langolier ({size}).",
              {
                path: r.path,
                launchers: (r.launchers || []).join(", "),
                size: bytes(r.payload || 0),
              },
            )
          : t("File written: {path} ({size}).", {
              path: r.path,
              size: bytes(r.bytes || 0),
            }),
      );
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
      setProgress("");
    }
  }

  const provider = PROVIDERS.find(
    (p) => p.id === (editing?.overrides.provider || settings.provider),
  );
  const visibleDocs = docs.filter((d) =>
    d.name.toLowerCase().includes(docFilter.toLowerCase()),
  );

  return (
    <main className="content-page">
      <PageHeading
        eyebrow={t("ONE VOICE PER MISSION")}
        title={t("The assistants.")}
        description={t(
          "Each profile has its own name, mission, knowledge scope and settings. Export it as a standalone chatbot when it is ready.",
        )}
        action={
          <Button primary onClick={() => setEditing(blank())}>
            <Plus size={16} /> {t("New assistant")}
          </Button>
        }
      />
      <div className="assistant-layout">
        <div className="assistant-list">
          <button
            className={`assistant-card${!settings.active_assistant ? " active" : ""}`}
            onClick={() => void activate("")}
          >
            <span className="assistant-avatar">L</span>
            <span>
              <b>Langolier</b>
              <small>{t("No profile · the whole memory")}</small>
            </span>
            {!settings.active_assistant && <Star size={14} className="lime" />}
          </button>
          {assistants.map((a) => (
            <div
              key={a.id}
              className={`assistant-card${settings.active_assistant === a.id ? " active" : ""}${editing?.id === a.id ? " editing" : ""}`}
            >
              <button
                className="assistant-main"
                onClick={() =>
                  setEditing({ ...a, overrides: { ...a.overrides } })
                }
              >
                {a.avatar ? (
                  <img className="assistant-avatar" src={a.avatar} alt="" />
                ) : (
                  <span className="assistant-avatar">
                    {a.name[0]?.toUpperCase()}
                  </span>
                )}
                <span>
                  <b>{a.name}</b>
                  <small>
                    {a.scope.all
                      ? t("The whole memory")
                      : t("{w} watch(es), {d} source(s)", {
                          w:
                            (a.scope.watches as string[] | undefined)?.length ||
                            0,
                          d:
                            (a.scope.documents as string[] | undefined)
                              ?.length || 0,
                        })}{" "}
                    · {t(THEMES.find((x) => x.id === a.theme)?.name || a.theme)}
                  </small>
                </span>
              </button>
              <button
                title={
                  settings.active_assistant === a.id
                    ? t("Active profile")
                    : t("Use this profile in the conversation")
                }
                onClick={() => void activate(a.id)}
              >
                <Star
                  size={14}
                  className={settings.active_assistant === a.id ? "lime" : ""}
                />
              </button>
              <button title={t("Delete")} onClick={() => void remove(a)}>
                <Trash2 size={14} />
              </button>
            </div>
          ))}
          {!assistants.length && (
            <Empty
              title={t("No profile")}
              text={t("Create an assistant: a mission, a scope, a tone.")}
            />
          )}
        </div>
        {editing ? (
          <section className="panel form-stack assistant-editor">
            <div className="panel-heading">
              <div>
                <h3>
                  {editing.id
                    ? editing.name || "Assistant"
                    : t("New assistant")}
                </h3>
                <p>{t("Identity and mission")}</p>
              </div>
              <Bot size={20} />
            </div>
            <div className="assistant-identity">
              <button
                className="avatar-pick"
                onClick={() => void pickAvatar()}
                disabled={!desktop}
                title={t("Choose an image")}
              >
                {editing.avatar ? (
                  <img src={editing.avatar} alt="" />
                ) : (
                  <ImageIcon size={22} />
                )}
              </button>
              <div className="form-stack grow">
                <label>
                  {t("Name")}
                  <input
                    value={editing.name}
                    onChange={(e) => field("name", e.target.value)}
                    placeholder={t("Nanny, Lawyer, Archivist…")}
                  />
                </label>
                <label>
                  {t("Welcome message")}
                  <input
                    value={editing.welcome}
                    onChange={(e) => field("welcome", e.target.value)}
                    placeholder={t("Hello! Ask me anything about…")}
                  />
                </label>
              </div>
            </div>
            {editing.avatar && (
              <button className="link" onClick={() => field("avatar", "")}>
                {t("Remove the image")}
              </button>
            )}
            <label>
              {t("Mission")}
              <textarea
                rows={5}
                value={editing.mission}
                onChange={(e) => field("mission", e.target.value)}
                placeholder={t(
                  "Who this assistant is, who it talks to, what it must and must not do, its tone.",
                )}
              />
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={editing.show_sources}
                onChange={(e) => field("show_sources", e.target.checked)}
              />
              {t("Show cited sources in the exported chatbot")}
              <small>
                {t(
                  "Unchecked: the chatbot answers without listing passages, useful for a non-technical audience.",
                )}
              </small>
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={editing.hide_source_names}
                onChange={(e) => field("hide_source_names", e.target.checked)}
              />
              {t("Hide source file names")}
              <small>
                {t(
                  'The model and the page only see "Source 1", "Source 2"… Your document names cannot appear in an answer.',
                )}
              </small>
            </label>
            <div className="two-fields">
              <label>
                {t("Cap per conversation (tokens, 0 = unlimited)")}
                <input
                  type="number"
                  min="0"
                  step="1000"
                  value={editing.max_conversation_tokens}
                  onChange={(e) =>
                    field(
                      "max_conversation_tokens",
                      Math.max(0, Number(e.target.value) || 0),
                    )
                  }
                />
              </label>
              <label>
                {t("Cap per day, all conversations (tokens, 0 = unlimited)")}
                <input
                  type="number"
                  min="0"
                  step="10000"
                  value={editing.daily_token_budget}
                  onChange={(e) =>
                    field(
                      "daily_token_budget",
                      Math.max(0, Number(e.target.value) || 0),
                    )
                  }
                />
              </label>
            </div>
            <p className="field-help">
              {t(
                "Recommended with an API key: the daily cap bounds the bill whatever happens. Estimated at 4 characters per token over exchanged messages; past the cap the chatbot replies with a fixed message without calling the model.",
              )}
            </p>
            <label>
              {t("Chat page theme")}
              <div className="theme-row">
                {THEMES.map((t) => (
                  <button
                    key={t.id}
                    type="button"
                    className={`theme-chip${editing.theme === t.id ? " selected" : ""}`}
                    style={{
                      background: t.bg,
                      color: t.fg,
                      borderColor: t.accent,
                    }}
                    onClick={() => field("theme", t.id)}
                  >
                    <span style={{ background: t.accent }} /> {t.name}
                  </button>
                ))}
              </div>
            </label>

            <div className="panel-heading">
              <div>
                <h3>{t("Knowledge scope")}</h3>
                <p>{t("What the assistant is allowed to read")}</p>
              </div>
              <FolderOpen size={18} />
            </div>
            <div className="segmented">
              <button
                type="button"
                className={scopeAll ? "selected" : ""}
                onClick={() => field("scope", { all: true })}
              >
                {t("The whole memory")}
              </button>
              <button
                type="button"
                className={!scopeAll ? "selected" : ""}
                onClick={() =>
                  field("scope", {
                    watches: scopeWatches,
                    documents: scopeDocs,
                  })
                }
              >
                {t("Selection")}
              </button>
            </div>
            {!scopeAll && (
              <div className="scope-pick">
                {watches.length > 0 && (
                  <div className="scope-watches">
                    <small>
                      {t(
                        "Watches: a checked watch brings everything it indexes, including files yet to come",
                      )}
                    </small>
                    <div className="scope-watch-list">
                      {watches.map((w) => (
                        <label key={w.id} className="inline-check">
                          <input
                            type="checkbox"
                            checked={scopeWatches.includes(w.id)}
                            onChange={(e) =>
                              setScope({
                                watches: e.target.checked
                                  ? [...scopeWatches, w.id]
                                  : scopeWatches.filter((x) => x !== w.id),
                                documents: scopeDocs,
                              })
                            }
                          />
                          <span className="truncate">{w.path}</span>
                          <em>{w.total}</em>
                        </label>
                      ))}
                    </div>
                  </div>
                )}
                <div className="scope-toolbar">
                  <strong>
                    {t("{n} source(s) chosen out of {total}", {
                      n: scopeDocs.length,
                      total: docs.length,
                    })}
                  </strong>
                  <input
                    placeholder={t("Filter sources…")}
                    value={docFilter}
                    onChange={(e) => setDocFilter(e.target.value)}
                  />
                  <Button
                    onClick={() =>
                      setScope({
                        watches: scopeWatches,
                        documents: [
                          ...new Set([
                            ...scopeDocs,
                            ...visibleDocs.map((d) => d.id),
                          ]),
                        ],
                      })
                    }
                  >
                    {t("Select all")}
                    {docFilter ? t(" (filtered)") : ""}
                  </Button>
                  <Button
                    onClick={() =>
                      setScope({ watches: scopeWatches, documents: [] })
                    }
                  >
                    {t("Clear all")}
                  </Button>
                </div>
                <div className="scope-tree">
                  <LibraryTree
                    docs={visibleDocs}
                    dataDir={dataDir}
                    sort="name"
                    selected={new Set(scopeDocs)}
                    onPreview={onPreview}
                    onSelect={(ids, checked) =>
                      setScope({
                        watches: scopeWatches,
                        documents: checked
                          ? [...new Set([...scopeDocs, ...ids])]
                          : scopeDocs.filter((x) => !ids.includes(x)),
                      })
                    }
                  />
                </div>
                <p className="field-help">
                  {t(
                    "Checking a folder takes everything inside it, subfolders included. Unchecking a subfolder or a file afterwards narrows the selection.",
                  )}
                </p>
              </div>
            )}

            <div className="panel-heading">
              <div>
                <h3>{t("Own settings")}</h3>
                <p>{t("Empty = Langolier's global setting")}</p>
              </div>
            </div>
            <div className="two-fields">
              <label>
                {t("Provider")}
                <select
                  value={(editing.overrides.provider as string) || ""}
                  onChange={(e) => {
                    override("provider", e.target.value || undefined);
                    const p = PROVIDERS.find((x) => x.id === e.target.value);
                    if (p) {
                      override("endpoint", p.endpoint);
                      if (p.models[0]) override("model", p.models[0]);
                    }
                  }}
                >
                  <option value="">
                    Global (
                    {PROVIDERS.find((p) => p.id === settings.provider)?.name &&
                      t(
                        PROVIDERS.find((p) => p.id === settings.provider)!.name,
                      )}
                    )
                  </option>
                  {PROVIDERS.map((p) => (
                    <option key={p.id} value={p.id}>
                      {t(p.name)}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                {t("Model")}
                <input
                  value={(editing.overrides.model as string) || ""}
                  onChange={(e) =>
                    override("model", e.target.value || undefined)
                  }
                  placeholder={settings.model}
                  list="installed-models"
                />
              </label>
            </div>
            {provider?.cloud &&
              (editing.overrides.provider || settings.provider) !==
                settings.provider && (
                <ApiKeyField
                  label={t("API key for this profile")}
                  value={(editing.overrides.api_key as string) || ""}
                  onChange={(k) => override("api_key", k || undefined)}
                  provider={
                    (editing.overrides.provider as string) || settings.provider
                  }
                />
              )}
            <div className="two-fields">
              <label>
                {t("Temperature")}
                <input
                  type="number"
                  step="0.1"
                  min="0"
                  max="2"
                  value={
                    (editing.overrides.temperature as number | undefined) ?? ""
                  }
                  onChange={(e) =>
                    override(
                      "temperature",
                      e.target.value === ""
                        ? undefined
                        : Number(e.target.value),
                    )
                  }
                  placeholder={String(settings.temperature)}
                />
              </label>
              <label>
                {t("Passages retrieved")}
                <input
                  type="number"
                  min="1"
                  max="10"
                  value={(editing.overrides.top_k as number | undefined) ?? ""}
                  onChange={(e) =>
                    override(
                      "top_k",
                      e.target.value === ""
                        ? undefined
                        : Number(e.target.value),
                    )
                  }
                  placeholder={String(settings.top_k)}
                />
              </label>
            </div>
            <label>
              {t("Abstention sentence for this profile")}
              <input
                value={(editing.overrides.abstain_text as string) || ""}
                onChange={(e) =>
                  override("abstain_text", e.target.value || undefined)
                }
                placeholder={settings.abstain_text}
              />
            </label>
            <div className="two-fields">
              <label className="check">
                <input
                  type="checkbox"
                  checked={
                    (editing.overrides.strict_grounding as
                      boolean | undefined) ?? settings.strict_grounding
                  }
                  onChange={(e) =>
                    override("strict_grounding", e.target.checked)
                  }
                />
                {t("Answer only from the sources")}
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={
                    (editing.overrides.rerank_enabled as boolean | undefined) ??
                    settings.rerank_enabled
                  }
                  onChange={(e) => override("rerank_enabled", e.target.checked)}
                />
                {t("Relevance judge")}
              </label>
            </div>

            <div className="panel-heading">
              <div>
                <h3>Telegram</h3>
                <p>{t("The exported chatbot also answers on Telegram")}</p>
              </div>
            </div>
            <ApiKeyField
              label={t("Bot token (created with @BotFather)")}
              value={editing.telegram_token}
              onChange={(k) => field("telegram_token", k)}
              provider="telegram"
              only="telegram"
              placeholder="123456789:AAH…"
            />
            <div className="telegram-check">
              <Button
                disabled={!editing.telegram_token.trim()}
                onClick={() => void checkTelegram()}
              >
                {t("Test the token")}
              </Button>
              {telegramStatus && <small>{telegramStatus}</small>}
            </div>
            <label>
              {t("Who may talk to it (empty = everyone)")}
              <input
                value={editing.telegram_allowed}
                onChange={(e) => field("telegram_allowed", e.target.value)}
                placeholder="@moi, @collegue, 123456789"
              />
            </label>
            <p className="field-help">
              {t(
                "The bridge polls the Telegram servers: no port to open, the web launcher or",
              )}{" "}
              <code>server.sh</code>{" "}
              {t(
                "just has to be running. One Telegram chat = one conversation;",
              )}{" "}
              <code>/new</code>{" "}
              {t(
                "opens another one. Caps and source privacy apply. Messages travel through Telegram, so keep it to content that may go there.",
              )}
            </p>
            <div className="panel-heading">
              <div>
                <h3>{t("Window launcher")}</h3>
                <p>{t("How the exported window app sits on the desktop")}</p>
              </div>
            </div>
            <label className="check inline-check">
              <input
                type="checkbox"
                checked={editing.tray_icon}
                onChange={(e) => field("tray_icon", e.target.checked)}
              />
              {t("Icon in the menu bar or system tray")}
              <small>
                {t(
                  "Open, Ask and Quit from the icon. Closing the window then hides it instead of quitting.",
                )}
              </small>
            </label>
            <label className="check inline-check">
              <input
                type="checkbox"
                checked={editing.start_hidden}
                onChange={(e) => {
                  field("start_hidden", e.target.checked);
                  if (e.target.checked) field("tray_icon", true);
                }}
              />
              {t("Start hidden, in the menu bar")}
            </label>
            <label>
              {t(
                "Global shortcut for the floating question bar (empty = none)",
              )}
              <div className="searchbox">
                <button
                  type="button"
                  className={`shortcut-capture${capturing ? " capturing" : ""}`}
                  onClick={() => setCapturing(true)}
                  onBlur={() => setCapturing(false)}
                  onKeyDown={(e) => {
                    if (!capturing) return;
                    const accel = captureShortcut(e.nativeEvent);
                    if (accel === null) return;
                    e.preventDefault();
                    if (accel) {
                      field("palette_shortcut", accel);
                      setCapturing(false);
                    }
                  }}
                >
                  {capturing
                    ? t("Press the combination…")
                    : editing.palette_shortcut
                      ? formatShortcut(editing.palette_shortcut)
                      : t("None")}
                </button>
                <Button
                  disabled={!editing.palette_shortcut}
                  onClick={() => field("palette_shortcut", "")}
                >
                  {t("Clear")}
                </Button>
              </div>
              <small className="muted">
                {t(
                  "Like Langolier's own palette: a floating bar over any application, Esc or a click outside closes it. Works on macOS, Windows and Linux.",
                )}
              </small>
            </label>
            <label>
              {t("Chat page language")}
              <select
                value={editing.language}
                onChange={(e) => field("language", e.target.value)}
              >
                <option value="auto">
                  {t("Follow the visitor's browser")}
                </option>
                <option value="en">English</option>
                <option value="fr">Français</option>
              </select>
              <small className="muted">
                {t(
                  "Buttons, placeholders and notices of the exported chatbot page. The assistant's own answers follow its mission and the question.",
                )}
              </small>
            </label>
            <div className="panel-heading">
              <div>
                <h3>{t("Remote administration")}</h3>
                <p>
                  {t(
                    "Update or roll back the exported chatbot from its own page",
                  )}
                </p>
              </div>
            </div>
            <label className="check inline-check">
              <input
                type="checkbox"
                checked={editing.admin_enabled}
                onChange={(e) => field("admin_enabled", e.target.checked)}
              />
              {t("Enable administration from the chat page")}
              <small>
                {t(
                  "A small gear appears in the page footer. Every action needs the secret below; with a wrong secret five times, the page pauses for fifteen minutes.",
                )}
              </small>
            </label>
            {editing.admin_enabled && (
              <>
                <label>
                  {editing.admin_secret_set
                    ? t("Admin secret (set; type a new one to replace it)")
                    : t("Admin secret (at least 12 characters)")}
                  <div className="searchbox">
                    <input
                      type="password"
                      autoComplete="off"
                      value={editing.admin_secret}
                      onChange={(e) => field("admin_secret", e.target.value)}
                      placeholder={
                        editing.admin_secret_set ? "••••••••••••" : ""
                      }
                    />
                    <Button
                      onClick={() => {
                        const bytes = new Uint8Array(18);
                        crypto.getRandomValues(bytes);
                        const secret = btoa(String.fromCharCode(...bytes))
                          .replace(/[+/=]/g, "")
                          .slice(0, 22);
                        field("admin_secret", secret);
                        onNotify(
                          t(
                            "Secret generated: {secret}. Copy it now, it will not be shown again.",
                            { secret },
                          ),
                        );
                      }}
                    >
                      {t("Generate")}
                    </Button>
                  </div>
                </label>
                <label className="check inline-check">
                  <input
                    type="checkbox"
                    checked={editing.admin_import}
                    onChange={(e) => field("admin_import", e.target.checked)}
                  />
                  {t("Allow importing a .langolier from the page")}
                </label>
                <label className="check inline-check">
                  <input
                    type="checkbox"
                    checked={editing.admin_restore}
                    onChange={(e) => field("admin_restore", e.target.checked)}
                  />
                  {t(
                    "Allow restoring or deleting earlier versions from the page",
                  )}
                </label>
                <p className="field-help">
                  {t(
                    "Every bundle the chatbot ever ran is kept on the server, twenty at most, named and timestamped. Nothing is ever downloaded from the page: no export, so the corpus and the API key stay on the server. The bundle the kit shipped with cannot be deleted.",
                  )}
                </p>
              </>
            )}
            <label>
              {t("Engine for the exported kit")}
              <select
                value={engine}
                onChange={(e) =>
                  setEngine(
                    e.target.value as "profile" | "embedded" | "embedded-light",
                  )
                }
              >
                <option value="profile">
                  {t(
                    "Follow the profile (cloud, or Ollama on the target machine)",
                  )}
                </option>
                <option value="embedded">
                  {t(
                    "Complete: llama.cpp + bundled GGUF models, no prerequisites",
                  )}
                </option>
                <option value="embedded-light">
                  {t(
                    "Complete light: same with qwen3:4b-instruct (≈ 2.9 GB total)",
                  )}
                </option>
              </select>
              <small className="muted">
                {engine === "embedded-light"
                  ? t(
                      "The profile's chat model is swapped for qwen3:4b-instruct (pulled through Ollama if missing): same behaviour on strict grounding in our tests, twice as fast, half the size. The profile's embedder is bundled and the knowledge re-vectorised.",
                    )
                  : engine === "embedded"
                    ? t(
                        "The chat model is copied from Ollama (several GB); EmbeddingGemma is downloaded once then bundled (330 MB). Knowledge is re-vectorised by the embedded engine. With a cloud profile, only the embedder is bundled.",
                      )
                    : t(
                        "Light kit. The manual lists the expected models; the launchers pull them through Ollama and report what is missing on the page.",
                      )}
              </small>
            </label>
            {progress && (
              <p className="field-help">
                <LoaderCircle className="spin" size={13} /> {progress}
              </p>
            )}
            <div className="assistant-actions">
              <Button
                primary
                disabled={busy || !editing.name.trim()}
                onClick={() => void save()}
              >
                {busy ? (
                  <LoaderCircle className="spin" size={15} />
                ) : (
                  <Check size={15} />
                )}{" "}
                {t("Save")}
              </Button>
              <Button onClick={() => setEditing(null)}>{t("Close")}</Button>
              <span className="spacer" />
              <Button
                disabled={busy || !editing.id}
                onClick={() => void exportAs("langolier")}
                title={t(
                  "The knowledge and profile file, to create or update an already deployed chatbot",
                )}
              >
                <FileDown size={15} /> {t("Export .langolier")}
              </Button>
              <Button
                disabled={busy || !editing.id}
                onClick={() => void exportAs("kit")}
                title={t(
                  "A ready-to-run folder: the .langolier, a window app, a web app, the server scripts and the manual",
                )}
              >
                <Package size={15} /> {t("Export the chatbot")}
              </Button>
            </div>
            {!editing.id && (
              <p className="field-help">
                {t("Save the profile first to be able to export it.")}
              </p>
            )}
            <p className="field-help">
              {t("The exported chatbot is an executable for this system (")}
              {navigator.platform.includes("Mac")
                ? "macOS Apple Silicon"
                : navigator.platform}
              {t(
                ") carrying the profile, its settings and its knowledge. With a cloud provider it runs anywhere; with Ollama the target machine must have it installed. To update a deployed chatbot, drop a new",
              )}{" "}
              <code>.langolier</code> {t("in its folder.")}
            </p>
          </section>
        ) : (
          <section className="panel assistant-placeholder">
            <Bot size={28} />
            <p>
              {t(
                "Pick a profile on the left, or create one. The star marks the profile used in the conversation and the palette.",
              )}
            </p>
          </section>
        )}
      </div>
    </main>
  );
}
