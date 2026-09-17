import { useState, useEffect } from "react";
import { t } from "../i18n";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  LoaderCircle,
  Check,
  RefreshCw,
  MessageCircle,
  Library,
  CircleAlert,
  Video,
  FolderOpen,
  CircleCheck,
  Download,
  HardDrive,
  ShieldCheck,
  Command,
  KeyRound,
  Radio,
  Copy,
} from "lucide-react";
import {
  PROVIDERS,
  ABSTAIN_TEXT,
  type Settings,
  type Health,
  type EmbedModel,
  CHAT_MODELS,
} from "../types";
import { formatShortcut, captureShortcut } from "../shortcut";
import { Button, PageHeading, ShortcutCapture } from "../components/Common";
import ApiKeyField from "../components/ApiKeyField";
import ReindexModal from "../components/ReindexModal";
import type { Doc } from "../types";
import { api, desktop, errorText, number } from "../api";
export default function EngineSettings({
  settings,
  health,
  docs,
  dataDir,
  onError,
  onSaved,
  onRefresh,
}: {
  settings: Settings;
  health: Health | null;
  docs: Doc[];
  dataDir: string;
  onError: (e: unknown) => void;
  onSaved: () => void;
  onRefresh: () => Promise<void>;
}) {
  const [s, setS] = useState(settings);
  const [saving, setSaving] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [maintenance, setMaintenance] = useState("");
  const [model, setModel] = useState("qwen3:8b");
  const [progress, setProgress] = useState("");
  const [embedModels, setEmbedModels] = useState<EmbedModel[]>([]);
  const [reindexing, setReindexing] = useState(false);
  const [apiProbe, setApiProbe] = useState("");
  const [tokenShown, setTokenShown] = useState(false);
  useEffect(() => {
    if (!desktop) return;
    void api<EmbedModel[]>("embed_models")
      .then(setEmbedModels)
      .catch(() => {});
  }, []);
  function field<K extends keyof Settings>(key: K, value: Settings[K]) {
    setS((v) => ({ ...v, [key]: value }));
  }
  useEffect(() => {
    if (!desktop) return;
    let dead = false;
    let cleanup: (() => void) | undefined;
    void listen<{ status: string; completed?: number; total?: number }>(
      "model-progress",
      (e) =>
        setProgress(
          `${e.payload.status}${e.payload.total ? ` · ${Math.round(((e.payload.completed || 0) / e.payload.total) * 100)} %` : ""}`,
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
  async function saveSettings() {
    setSaving(true);
    try {
      const embeddingChanged =
        s.embedding_model.trim() !== settings.embedding_model.trim() ||
        s.embedding_endpoint.trim() !== settings.embedding_endpoint.trim();
      await api("save_settings", { settings: s });
      onSaved();
      if (embeddingChanged && desktop) {
        const { ask } = await import("@tauri-apps/plugin-dialog");
        const go = await ask(
          t(
            "The embedding model changed: existing vectors are no longer comparable. Pick the sources to reindex now? (Word search keeps working meanwhile.)",
          ),
          { title: t("Reindexing needed"), kind: "warning" },
        );
        if (go) setReindexing(true);
      }
    } catch (e) {
      onError(e);
    } finally {
      setSaving(false);
    }
  }
  async function pull() {
    setPulling(true);
    setProgress(t("Connecting to the Ollama registry"));
    try {
      await api("pull_model", { model });
      setProgress(t("Model installed."));
      await onRefresh();
    } catch (e) {
      onError(e);
    } finally {
      setPulling(false);
    }
  }
  async function chooseWhisper() {
    try {
      const file = await open({
        title: t("Whisper ggml model"),
        filters: [{ name: "Whisper", extensions: ["bin"] }],
      });
      if (typeof file === "string") field("whisper_model", file);
    } catch (e) {
      onError(e);
    }
  }
  const embeddingCount = settings.embedding_model;
  const provider = PROVIDERS.find((p) => p.id === s.provider);
  return (
    <main className="content-page">
      {reindexing && (
        <ReindexModal
          docs={docs}
          dataDir={dataDir}
          onClose={() => setReindexing(false)}
          onError={onError}
          onDone={setMaintenance}
        />
      )}
      <PageHeading
        eyebrow={t("THE POWER IS YOURS")}
        title={t("Choose your engine.")}
        description={t(
          "Swap models, keep your memory. Everything runs on your machine.",
        )}
        action={
          <Button primary disabled={saving} onClick={() => void saveSettings()}>
            {saving ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <Check size={16} />
            )}{" "}
            {t("Save")}
          </Button>
        }
      />
      <div className="engine-status">
        <span
          className={`status-dot ${health?.engine_ok ? "online" : "warning"}`}
        />
        <div>
          <strong>
            {health?.engine_ok
              ? t("Your engine is answering.")
              : t("Let's connect your local engine.")}
          </strong>
          <p>
            {health?.engine_ok
              ? t("{n} model(s) available", { n: health.models?.length || 0 }) +
                ` · ${settings.endpoint}`
              : t("Start Ollama or a local OpenAI-compatible server.")}
          </p>
        </div>
        <Button onClick={() => void onRefresh()}>
          <RefreshCw size={14} /> {t("Check")}
        </Button>
      </div>
      <div className="settings-grid">
        <section className="panel form-stack">
          <div className="panel-heading">
            <div>
              <h3>{t("The brain")}</h3>
              <p>{t("Generation and conversation")}</p>
            </div>
            <MessageCircle size={20} />
          </div>
          <label>
            {t("Provider")}
            <select
              value={s.provider}
              onChange={(e) => {
                const p = PROVIDERS.find((x) => x.id === e.target.value);
                if (!p) return;
                field("provider", p.id);
                field("endpoint", p.endpoint);
                if (p.models[0]) field("model", p.models[0]);
              }}
            >
              {PROVIDERS.map((p) => (
                <option key={p.id} value={p.id}>
                  {t(p.name)}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t("Server address")}
            <input
              value={s.endpoint}
              onChange={(e) => field("endpoint", e.target.value)}
            />
          </label>
          {provider?.cloud && (
            <ApiKeyField
              value={s.api_key}
              onChange={(k) => field("api_key", k)}
              provider={s.provider}
              placeholder={s.provider === "anthropic" ? "sk-ant-…" : "sk-…"}
            />
          )}
          <label>
            {s.provider === "embedded"
              ? t("Chat model (curated tag, or path to a GGUF file)")
              : t("Chat model")}
            <input
              list="installed-models"
              value={s.model}
              onChange={(e) => field("model", e.target.value)}
            />
            <datalist id="installed-models">
              {(s.provider === "embedded"
                ? CHAT_MODELS.map((m) => m.tag)
                : health?.models?.length
                  ? health.models.map((m) => m.name)
                  : provider?.models || []
              ).map((name) => (
                <option key={name} value={name} />
              ))}
            </datalist>
          </label>
          {s.provider === "embedded" && (
            <p className="field-help">
              {t(
                "A curated tag is downloaded once into the app cache with the button below, then loaded on demand. llama.cpp runs inside Langolier: nothing to install, nothing listening on a port.",
              )}
            </p>
          )}
          {provider?.cloud && (
            <div className="setting-note">
              <CircleAlert size={16} />
              <p>
                {t(
                  "Your questions and the retrieved passages are sent to this provider. The memory (index, vectors) stays local. The key is stored in the application database.",
                )}
              </p>
            </div>
          )}
          <div className="two-fields">
            <label>
              {t("Context window")}
              <select
                value={s.context_size}
                onChange={(e) => field("context_size", Number(e.target.value))}
              >
                <option value={4096}>{number(4096)} tokens</option>
                <option value={8192}>{number(8192)} tokens</option>
                <option value={16384}>{number(16384)} tokens</option>
                <option value={32768}>{number(32768)} tokens</option>
              </select>
            </label>
            <label>
              {t("Temperature")}
              <input
                type="number"
                step="0.1"
                min="0"
                max="2"
                value={s.temperature}
                onChange={(e) => field("temperature", Number(e.target.value))}
              />
            </label>
          </div>
          <label>
            {t("Maximum answer length: {n} tokens", { n: s.max_tokens })}
            <input
              type="range"
              min="256"
              max="8192"
              step="256"
              value={s.max_tokens}
              onChange={(e) => field("max_tokens", Number(e.target.value))}
            />
          </label>
          {(s.provider === "embedded" ||
            s.embedding_endpoint.trim() === "embedded") && (
            <label>
              {t("Unload idle models after (minutes, 0 = never)")}
              <input
                type="number"
                min="0"
                max="1440"
                value={s.engine_idle_minutes}
                onChange={(e) =>
                  field(
                    "engine_idle_minutes",
                    Math.max(0, Number(e.target.value)),
                  )
                }
              />
              <small className="muted">
                {t(
                  "Memory is released when the engine has not been used for this long, and the next question reloads the model in a few seconds.",
                )}
              </small>
            </label>
          )}
          <p className="field-help">
            {CHAT_MODELS.map((m) => (
              <span key={m.tag}>
                <code>{m.tag}</code> — {t(m.note)}
                <br />
              </span>
            ))}
            {t("Watch out for the bare")} <code>qwen3:4b</code>{" "}
            {t(
              "(the Thinking-2507 variant): it always reasons and leaks that reasoning into the answer. Take",
            )}{" "}
            <code>qwen3:4b-instruct</code>.
          </p>
          <p className="field-help">
            {t(
              "The starting profile uses a quantised 8B model. With 24 GB, keep the context window reasonable. LM Studio manages its window in the server.",
            )}
          </p>
        </section>
        <section className="panel form-stack">
          <div className="panel-heading">
            <div>
              <h3>{t("The memory")}</h3>
              <p>{t("Representation and search")}</p>
            </div>
            <Library size={20} />
          </div>
          <label>
            {t('Ollama embedding server (or "embedded" for a local GGUF)')}
            <input
              value={s.embedding_endpoint}
              onChange={(e) => field("embedding_endpoint", e.target.value)}
            />
          </label>
          {s.embedding_endpoint.trim() === "embedded" ? (
            <label>
              {t("Embedding model (curated tag, or path to a GGUF file)")}
              <input
                list="embed-tags"
                value={s.embedding_model}
                onChange={(e) => field("embedding_model", e.target.value)}
                placeholder="embeddinggemma"
              />
              <datalist id="embed-tags">
                {embedModels.map((m) => (
                  <option key={m.tag} value={m.tag}>
                    {m.label}
                  </option>
                ))}
              </datalist>
            </label>
          ) : (
            <label>
              {t("Embedding model")}
              <select
                value={
                  embedModels.some((m) => m.tag === s.embedding_model.trim())
                    ? s.embedding_model.trim()
                    : "__other"
                }
                onChange={(e) => {
                  if (e.target.value !== "__other")
                    field("embedding_model", e.target.value);
                }}
              >
                {embedModels.map((m) => (
                  <option key={m.tag} value={m.tag}>
                    {m.label} · {m.dims} dim.
                  </option>
                ))}
                <option value="__other">
                  {t("Other")} ({s.embedding_model})
                </option>
              </select>
              <small className="muted">
                {t(
                  "Curated list: every model has an official GGUF for the complete export. Changing model invalidates the vectors, and a reindex will be offered.",
                )}
              </small>
            </label>
          )}
          <label>
            {t("Passages retrieved: {n}", { n: s.top_k })}
            <input
              type="range"
              min="1"
              max="10"
              value={s.top_k}
              onChange={(e) => field("top_k", Number(e.target.value))}
            />
          </label>
          <label>
            {t("Candidates per search arm: {n}", { n: s.candidate_pool })}
            <input
              type="range"
              min="8"
              max="256"
              step="8"
              value={s.candidate_pool}
              onChange={(e) => field("candidate_pool", Number(e.target.value))}
            />
            <small>
              {t(
                "Passages each arm brings back before lexical and dense results are merged. Wider catches more, and costs more to rerank.",
              )}
            </small>
          </label>
          <label>
            {s.passages_per_source === 0
              ? t("Passages per source: no cap")
              : t("Passages per source: {n}", { n: s.passages_per_source })}
            <input
              type="range"
              min="0"
              max="10"
              value={s.passages_per_source}
              onChange={(e) =>
                field("passages_per_source", Number(e.target.value))
              }
            />
            <small>
              {t(
                "Keeps one talkative source from filling the whole answer. Zero lifts the cap.",
              )}
            </small>
          </label>

          <div className="setting-note">
            <CircleAlert size={16} />
            <p>
              {t(
                "Changing the embedding means reindexing the sources. Vectors from different models are never mixed. Current configured index:",
              )}{" "}
              <b>{embeddingCount}</b>.
            </p>
          </div>
          <p className="field-help">
            {t(
              "BM25 plus vector search, RRF fusion, document diversity and section context. Deep search adds a conversational rewrite.",
            )}
          </p>
        </section>
      </div>
      <div className="settings-grid">
        <section className="panel form-stack">
          <div className="panel-heading">
            <div>
              <h3>{t("The discipline")}</h3>
              <p>{t("What the assistant is allowed to answer")}</p>
            </div>
            <ShieldCheck size={20} />
          </div>
          <label className="check">
            <input
              type="checkbox"
              checked={s.strict_grounding}
              onChange={(e) => field("strict_grounding", e.target.checked)}
            />
            {t("Answer only from the sources")}
            <small>
              {t(
                "With no convincing passage, the model replies with the abstention sentence instead of drawing on its own knowledge.",
              )}
            </small>
          </label>
          <label>
            {t("Abstention sentence")}
            <textarea
              rows={2}
              value={s.abstain_text}
              onChange={(e) => field("abstain_text", e.target.value)}
              onBlur={() => {
                if (!s.abstain_text.trim()) field("abstain_text", ABSTAIN_TEXT);
              }}
            />
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={s.rerank_enabled}
              onChange={(e) => field("rerank_enabled", e.target.checked)}
            />
            {t("Relevance judge (reranker)")}
            <small>
              {t(
                "A model rereads the candidates and scores each passage. Below the threshold the assistant abstains before writing anything.",
              )}
            </small>
          </label>
          <div className="two-fields">
            <label>
              Seuil d’abstention : {Math.round(s.rerank_threshold * 100)} %
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                disabled={!s.rerank_enabled}
                value={s.rerank_threshold}
                onChange={(e) =>
                  field("rerank_threshold", Number(e.target.value))
                }
              />
            </label>
            <label>
              Candidats relus : {s.rerank_candidates}
              <input
                type="range"
                min="5"
                max="40"
                disabled={!s.rerank_enabled}
                value={s.rerank_candidates}
                onChange={(e) =>
                  field("rerank_candidates", Number(e.target.value))
                }
              />
            </label>
          </div>
          <label>
            {t("Judge model (empty = chat model)")}
            <input
              list="installed-models"
              disabled={!s.rerank_enabled}
              value={s.rerank_model}
              onChange={(e) => field("rerank_model", e.target.value)}
              placeholder="qwen3:8b"
            />
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={s.verify_answer}
              onChange={(e) => field("verify_answer", e.target.checked)}
            />
            {t("Verify the answer after writing")}
            <small>
              {t(
                "A second call checks that every claim is supported by the passages. Safer, but it doubles the response time.",
              )}
            </small>
          </label>
          <p className="field-help">
            {t("Raw search thresholds, before the judge: minimum cosine")}{" "}
            {s.min_dense_score.toFixed(2)}
            <input
              type="range"
              min="0"
              max="0.9"
              step="0.05"
              value={s.min_dense_score}
              onChange={(e) => field("min_dense_score", Number(e.target.value))}
            />
            {t("Minimum BM25 score (0 = off)")}
            <input
              type="number"
              min="0"
              max="100"
              step="0.5"
              value={s.min_lexical_score}
              onChange={(e) =>
                field("min_lexical_score", Number(e.target.value))
              }
            />
          </p>
        </section>
        <section className="panel form-stack">
          <div className="panel-heading">
            <div>
              <h3>{t("The palette")}</h3>
              <p>{t("Ask a question from anywhere")}</p>
            </div>
            <Command size={20} />
          </div>
          <label>
            {t("Global shortcut")}
            <ShortcutCapture
              value={s.shortcut}
              onChange={(accel) => field("shortcut", accel)}
            />
          </label>
          <p className="field-help">
            {t(
              "A combination with at least one modifier (⌘, ⌃, ⌥). The fn key cannot be captured by applications. The shortcut opens a floating bar; Esc or a click outside closes it.",
            )}
          </p>
          <Button onClick={() => void api("open_palette")}>
            <Command size={14} /> {t("Try the palette")}
          </Button>
        </section>
        <section className="panel form-stack">
          <div className="panel-heading">
            <div>
              <h3>{t("In the background")}</h3>
              <p>{t("Where Langolier waits when you are not using it")}</p>
            </div>
            <Radio size={20} />
          </div>
          <label className="check inline-check">
            <input
              type="checkbox"
              checked={s.tray_icon}
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
              checked={s.start_hidden}
              onChange={(e) => {
                field("start_hidden", e.target.checked);
                if (e.target.checked) field("tray_icon", true);
              }}
            />
            {t("Start hidden, in the menu bar")}
            <small>
              {t(
                "Langolier opens without a window; the icon and the shortcut are there.",
              )}
            </small>
          </label>
          <label className="check inline-check">
            <input
              type="checkbox"
              checked={s.launch_at_login}
              onChange={(e) => field("launch_at_login", e.target.checked)}
            />
            {t("Launch at login")}
            <small>
              {t(
                "Starts hidden at login, so the palette is always one shortcut away.",
              )}
            </small>
          </label>
          {s.tray_icon && (
            <>
              <p className="field-help">
                {t(
                  "Closing the window leaves Langolier in the menu bar, out of the Dock: the interface is released and about 240 MB with it. Click the icon to ask a question, right click for the menu. The local API keeps answering.",
                )}
              </p>
            </>
          )}
        </section>
        <section className="panel form-stack">
          <div className="panel-heading">
            <div>
              <h3>{t("Local API")}</h3>
              <p>
                {t(
                  "Let Shortcuts, Siri and local apps query your Langolier memory.",
                )}
              </p>
            </div>
            <Radio size={20} />
          </div>
          <label className="check inline-check">
            <input
              type="checkbox"
              checked={s.local_api}
              onChange={async (e) => {
                const on = e.target.checked;
                setApiProbe("");
                if (on && !s.local_api_token) {
                  try {
                    field("local_api_token", await api<string>("new_local_token"));
                  } catch (err) {
                    onError(err);
                    return;
                  }
                }
                field("local_api", on);
              }}
            />
            {t("Answer local requests")}
            <small>
              {t(
                "Loopback only, never the network. Answers come from the same engine as the window, one question at a time.",
              )}
            </small>
          </label>
          {s.local_api && (
            <>
              <label>
                {t("Port")}
                <input
                  type="number"
                  min="1024"
                  max="65535"
                  value={s.local_api_port}
                  onChange={(e) =>
                    field("local_api_port", Number(e.target.value))
                  }
                />
              </label>
              <label>
                {t("Access token")}
                <input
                  type={tokenShown ? "text" : "password"}
                  readOnly
                  value={s.local_api_token}
                  onFocus={(e) => e.target.select()}
                />
                <small>
                  {t(
                    "Sent as Authorization: Bearer. Without it the API answers nothing.",
                  )}
                </small>
              </label>
              <div className="button-row">
                <Button onClick={() => setTokenShown(!tokenShown)}>
                  {tokenShown ? t("Hide") : t("Show")}
                </Button>
                <Button
                  onClick={() =>
                    void navigator.clipboard
                      .writeText(s.local_api_token)
                      .then(() => setApiProbe(t("Token copied.")))
                      .catch(onError)
                  }
                >
                  <Copy size={14} /> {t("Copy the token")}
                </Button>
                <Button
                  onClick={async () => {
                    try {
                      field("local_api_token", await api<string>("new_local_token"));
                      setApiProbe(t("New token. Save, then update your shortcut."));
                    } catch (e) {
                      onError(e);
                    }
                  }}
                >
                  <RefreshCw size={14} /> {t("Regenerate")}
                </Button>
              </div>
              <p className="field-help">
                POST http://127.0.0.1:{s.local_api_port}/api/ask
                <br />
                {'{ "question": "…" }'} → {'{ "content": "…", "sources": [ … ] }'}
              </p>
              <div className="button-row">
                <Button
                  onClick={() =>
                    void navigator.clipboard
                      .writeText(
                        `http://127.0.0.1:${s.local_api_port}/api/ask`,
                      )
                      .then(() => setApiProbe(t("Address copied.")))
                      .catch(onError)
                  }
                >
                  <Copy size={14} /> {t("Copy the address")}
                </Button>
                <Button
                  onClick={async () => {
                    setApiProbe(t("Testing…"));
                    try {
                      const r = await api<{ ok: boolean; ms: number }>(
                        "test_local_api",
                      );
                      setApiProbe(
                        r.ok
                          ? t("The API answers in {n} ms.", { n: r.ms })
                          : t("The API answered an error."),
                      );
                    } catch (e) {
                      setApiProbe(errorText(e));
                    }
                  }}
                >
                  {t("Test the API")}
                </Button>
              </div>
              {apiProbe && <p className="field-help">{apiProbe}</p>}
            </>
          )}
        </section>
      </div>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h3>{t("The ears")}</h3>
            <p>{t("Local multilingual transcription through whisper.cpp")}</p>
          </div>
          <Video size={20} />
        </div>
        <div className="whisper-path">
          <label>
            {t("Whisper model file")}
            <input
              value={s.whisper_model}
              onChange={(e) => field("whisper_model", e.target.value)}
              placeholder="/chemin/vers/ggml-base.bin"
            />
          </label>
          <Button onClick={() => void chooseWhisper()}>
            <FolderOpen size={16} /> {t("Choose")}
          </Button>
        </div>
        <div className="dependency-list">
          {["ffmpeg", "yt-dlp", "whisper-cli"].map((name) => (
            <span key={name}>
              {health?.dependencies[name] ? (
                <CircleCheck size={15} className="lime" />
              ) : (
                <CircleAlert size={15} />
              )}{" "}
              {name}
            </span>
          ))}
          <span>
            {health?.whisper_model_exists ? (
              <CircleCheck size={15} className="lime" />
            ) : (
              <CircleAlert size={15} />
            )}{" "}
            Poids Whisper
          </span>
        </div>
        <p className="field-help">
          {t(
            "Run Install-Mac.command or Install-Windows.bat to prepare the engines. The multilingual base model favours speed; small or medium can improve transcription. This version indexes speech, not video images.",
          )}
        </p>
      </section>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h3>{t("A new model, in one click")}</h3>
            <p>
              {s.provider === "embedded" ||
              s.embedding_endpoint.trim() === "embedded"
                ? t(
                    "Curated tags land in the app cache; other names go through Ollama",
                  )
                : t(
                    "Downloaded by the Ollama server configured for embeddings",
                  )}
            </p>
          </div>
          <Download size={20} />
        </div>
        <div className="download-model">
          <input
            aria-label={t("Model to download")}
            value={model}
            onChange={(e) => setModel(e.target.value)}
            list="suggested-models"
          />
          <datalist id="suggested-models">
            {CHAT_MODELS.map((m) => (
              <option key={m.tag} value={m.tag}>
                {t(m.note)}
              </option>
            ))}
            {embedModels.map((m) => (
              <option key={m.tag} value={m.tag}>
                {m.label}
              </option>
            ))}
          </datalist>
          <Button disabled={pulling || !model} onClick={() => void pull()}>
            {pulling ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <Download size={15} />
            )}{" "}
            {pulling ? t("Downloading…") : t("Install the model")}
          </Button>
        </div>
        {progress && <p className="download-progress">{progress}</p>}
        <p className="field-help">
          {t(
            "An Internet connection is needed to download the weights. Then select the model in its configuration and save.",
          )}
        </p>
      </section>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h3>{t("Maintain your memory")}</h3>
            <p>
              {t(
                "Rebuild the index or take a consistent backup of the database.",
              )}
            </p>
          </div>
          <HardDrive size={20} />
        </div>
        <div className="download-model">
          <Button onClick={() => setReindexing(true)}>
            <RefreshCw size={15} />
            {t("Reindex sources…")}
          </Button>
          <Button
            onClick={() =>
              void (async () => {
                try {
                  const path = await save({
                    defaultPath: "langolier-backup.sqlite3",
                  });
                  if (path) {
                    await api("backup_database", { path });
                    setMaintenance(
                      t(
                        "Database backed up. Original media must be backed up separately.",
                      ),
                    );
                  }
                } catch (e) {
                  onError(e);
                }
              })()
            }
          >
            <Download size={15} />
            {t("Back up the database")}
          </Button>
        </div>
        {maintenance && <p className="field-help">{maintenance}</p>}
      </section>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h3>{t("Scanned pages")}</h3>
            <p>
              {t(
                "Local optical recognition, switched on automatically page by page.",
              )}
            </p>
          </div>
          <Library size={20} />
        </div>
        <div className="dependency-list">
          {["pdftoppm", "tesseract"].map((name) => (
            <span key={name}>
              {health?.dependencies[name] ? (
                <CircleCheck size={15} className="lime" />
              ) : (
                <CircleAlert size={15} />
              )}{" "}
              {name}
            </span>
          ))}
        </div>
        <p className="field-help">
          {t(
            "Pages holding fewer than 30 alphanumeric characters are rendered then read by Tesseract. French and English are installed. Every excerpt keeps its page and the OCR note. Diagrams and formulas may need checking.",
          )}
        </p>
      </section>
      <div className="data-location">
        <HardDrive size={16} />
        <div>
          <strong>{t("Your memory lives here")}</strong>
          <code>{dataDir || t("Local application directory")}</code>
        </div>
        <span className="badge">SQLITE · WAL</span>
      </div>
    </main>
  );
}
