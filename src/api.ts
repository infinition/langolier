import { invoke, isTauri } from "@tauri-apps/api/core";
import { getLang, t } from "./i18n";
import { ABSTAIN_TEXT, type Settings, type Snapshot } from "./types";
export const desktop = isTauri();
export const initial: Snapshot = {
  settings: {
    provider: "embedded",
    endpoint: "http://127.0.0.1:11434",
    model: "qwen3:4b-instruct",
    embedding_endpoint: "embedded",
    embedding_model: "embeddinggemma",
    whisper_model: "",
    top_k: 6,
    temperature: 0.2,
    context_size: 8192,
    api_key: "",
    max_tokens: 2048,
    min_dense_score: 0.35,
    min_lexical_score: 0,
    rerank_enabled: true,
    rerank_model: "",
    rerank_candidates: 20,
    rerank_threshold: 0.35,
    strict_grounding: true,
    abstain_text: ABSTAIN_TEXT,
    verify_answer: false,
    shortcut: "CommandOrControl+Shift+Space",
    candidate_pool: 48,
    passages_per_source: 3,
    chunk_size: 1400,
    chunk_overlap: 200,
    local_api: false,
    local_api_port: 8787,
    local_api_token: "",
    interface_language: "en",
    auto_subtitles: false,
    watch_in_background: false,
    ingest_in_background: false,
    watch_interval: 30,
    active_assistant: "",
    ingestion_paused: false,
    engine_idle_minutes: 5,
    tray_icon: false,
    start_hidden: false,
    launch_at_login: false,
  },
  documents: [],
  conversations: [],
  watches: [],
  assistants: [],
  runs: [],
  evaluations: [],
  totals: [
    { requests: 0, successful: null, latency_ms: null, tps: null, tokens: 0 },
  ],
  feedback: [{ rated: 0, positive: null }],
  data_dir: "",
};
export async function api<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (desktop) return invoke<T>(command, args);
  if (command === "snapshot") return initial as T;
  throw new Error(
    t(
      "Open the Langolier app to reach the Rust engine. This page is an interface preview.",
    ),
  );
}
export const errorText = (e: unknown) =>
  t(e instanceof Error ? e.message : String(e));
export const bytes = (n: number) =>
  n >= 1e9
    ? `${(n / 1e9).toFixed(1)} ${t("GB")}`
    : n >= 1e6
      ? `${(n / 1e6).toFixed(1)} ${t("MB")}`
      : `${Math.ceil(n / 1e3)} ${t("KB")}`;
const DUPLICATE = "Duplicate of ";
/// A duplicate names the source already holding those bytes, so the name is
/// kept out of the lookup key.
export const docError = (e: string) =>
  e.startsWith(DUPLICATE)
    ? t("Duplicate of {name}", { name: e.slice(DUPLICATE.length) })
    : t(e);
/// Changes a few settings without holding the whole form: a page that owns
/// one switch should not have to know about every other field.
export async function patchSettings(
  current: Settings,
  changes: Partial<Settings>,
): Promise<void> {
  await api("save_settings", { settings: { ...current, ...changes } });
}
export const locale = () => (getLang() === "fr" ? "fr-FR" : "en-US");
export const number = (n: number) =>
  new Intl.NumberFormat(locale()).format(n);
