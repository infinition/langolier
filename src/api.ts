import { invoke, isTauri } from "@tauri-apps/api/core";
import { getLang, t } from "./i18n";
import { ABSTAIN_TEXT, type Snapshot } from "./types";
export const desktop = isTauri();
export const initial: Snapshot = {
  settings: {
    provider: "ollama",
    endpoint: "http://127.0.0.1:11434",
    model: "qwen3:8b",
    embedding_endpoint: "http://127.0.0.1:11434",
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
    watch_interval: 30,
    active_assistant: "",
    ingestion_paused: false,
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
export const number = (n: number) =>
  new Intl.NumberFormat(getLang() === "fr" ? "fr-FR" : "en-US").format(n);
