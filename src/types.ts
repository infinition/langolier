export type Page =
  "chat" | "library" | "watch" | "assistants" | "metrics" | "lab" | "settings";
export interface Settings {
  provider: string;
  endpoint: string;
  model: string;
  embedding_endpoint: string;
  embedding_model: string;
  whisper_model: string;
  top_k: number;
  temperature: number;
  context_size: number;
  api_key: string;
  max_tokens: number;
  min_dense_score: number;
  min_lexical_score: number;
  rerank_enabled: boolean;
  rerank_model: string;
  rerank_candidates: number;
  rerank_threshold: number;
  strict_grounding: boolean;
  abstain_text: string;
  verify_answer: boolean;
  shortcut: string;
  watch_interval: number;
  active_assistant: string;
  ingestion_paused: boolean;
  engine_idle_minutes: number;
}
export interface Assistant {
  id: string;
  name: string;
  mission: string;
  welcome: string;
  avatar: string;
  theme: string;
  scope: Record<string, unknown>;
  overrides: Record<string, unknown>;
  show_sources: boolean;
  hide_source_names: boolean;
  max_conversation_tokens: number;
  daily_token_budget: number;
  telegram_token: string;
  telegram_allowed: string;
  admin_enabled: boolean;
  admin_secret: string;
  admin_secret_set: boolean;
  admin_import: boolean;
  admin_restore: boolean;
  created: number;
}
export interface EmbedModel {
  tag: string;
  label: string;
  dims: number;
  gguf_file: string;
}
export const THEMES: {
  id: string;
  name: string;
  bg: string;
  fg: string;
  accent: string;
}[] = [
  {
    id: "nuit",
    name: "Night",
    bg: "#111310",
    fg: "#e9ebe3",
    accent: "#d7f98a",
  },
  {
    id: "clair",
    name: "Light",
    bg: "#f6f5f0",
    fg: "#1e211b",
    accent: "#2f6b2a",
  },
  {
    id: "sauge",
    name: "Sage",
    bg: "#eef2ea",
    fg: "#22301f",
    accent: "#5f8f5a",
  },
  {
    id: "sable",
    name: "Sand",
    bg: "#f4ecdd",
    fg: "#3a2f1e",
    accent: "#b8722d",
  },
  {
    id: "encre",
    name: "Ink",
    bg: "#0e1220",
    fg: "#e6e9f5",
    accent: "#7aa2ff",
  },
  {
    id: "corail",
    name: "Coral",
    bg: "#1b1214",
    fg: "#f4e9ea",
    accent: "#ff7a6e",
  },
];
export interface Watch {
  id: string;
  path: string;
  mode: "sync" | "hoover";
  enabled: number;
  recursive: number;
  created: number;
  last_scan: number | null;
  last_error: string | null;
  interval: number | null;
  total: number;
  ready: number;
  errors: number;
  pending: number;
}
export type Provider =
  | "ollama"
  | "openai"
  | "openai_cloud"
  | "deepseek"
  | "anthropic"
  | "custom"
  | "embedded";
export const PROVIDERS: {
  id: Provider;
  name: string;
  endpoint: string;
  cloud: boolean;
  models: string[];
}[] = [
  {
    id: "ollama",
    name: "Ollama (local)",
    endpoint: "http://127.0.0.1:11434",
    cloud: false,
    // Tested on strict grounding: cites, abstains, resists injection.
    models: ["qwen3:8b", "qwen3:4b-instruct", "qwen3:1.7b"],
  },
  {
    id: "openai",
    name: "LM Studio / llama.cpp (local OpenAI)",
    endpoint: "http://127.0.0.1:1234/v1",
    cloud: false,
    models: [],
  },
  {
    id: "openai_cloud",
    name: "OpenAI (cloud)",
    endpoint: "https://api.openai.com/v1",
    cloud: true,
    models: ["gpt-5", "gpt-5-mini", "gpt-4.1"],
  },
  {
    id: "deepseek",
    name: "DeepSeek (cloud)",
    endpoint: "https://api.deepseek.com/v1",
    cloud: true,
    models: ["deepseek-chat", "deepseek-reasoner"],
  },
  {
    id: "anthropic",
    name: "Anthropic Claude (cloud)",
    endpoint: "https://api.anthropic.com",
    cloud: true,
    models: ["claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5"],
  },
  {
    id: "custom",
    name: "Other OpenAI-compatible server (Mistral, Groq, OpenRouter…)",
    endpoint: "https://",
    cloud: true,
    models: [],
  },
  {
    id: "embedded",
    name: "Embedded engine (llama.cpp, GGUF file)",
    endpoint: "embedded",
    cloud: false,
    models: [],
  },
];
/// Ollama chat models proven here, with their trade-offs.
export const CHAT_MODELS: { tag: string; note: string }[] = [
  { tag: "qwen3:8b", note: "5.2 GB · safest, ~17 tok/s" },
  {
    tag: "qwen3:4b-instruct",
    note: "2.5 GB · recommended for a light kit, ~30 tok/s",
  },
  {
    tag: "qwen3:1.7b",
    note: "1.4 GB · very fast (~60 tok/s), may drift to English",
  },
];
export const ABSTAIN_TEXT = "I do not have that information.";
export interface Doc {
  id: string;
  name: string;
  source: string;
  kind: string;
  status: string;
  stage: string;
  error: string | null;
  language: string | null;
  bytes: number;
  chunks: number;
  embedded: number;
  created: number;
  watch_id: string | null;
}
export interface Chunk {
  id: number;
  text: string;
  locator: string;
}
export interface Source {
  id: number;
  doc_id: string;
  name: string;
  text: string;
  locator: string;
  score: number;
}
export interface Message {
  id: string;
  role: string;
  content: string;
  sources: Source[];
  feedback?: number;
}
export interface Run {
  id: string;
  kind: string;
  model: string;
  status: string;
  latency_ms: number;
  tokens: number;
  tps: number | null;
  details: string;
  created: number;
}
export interface Evaluation {
  id: string;
  question: string;
  expected_document: string;
}
export interface Report {
  mode: string;
  k: number;
  n: number;
  hit_at_k: number;
  mrr: number;
  errors: number;
  latency_ms: number;
  cases: {
    question: string;
    rank?: number;
    error?: string;
    warning?: string;
  }[];
}
export interface Snapshot {
  settings: Settings;
  documents: Doc[];
  conversations: {
    id: string;
    title: string;
    created: number;
    assistant_id: string;
  }[];
  watches: Watch[];
  assistants: Assistant[];
  runs: Run[];
  evaluations: Evaluation[];
  totals: {
    requests: number;
    successful: number | null;
    latency_ms: number | null;
    tps: number | null;
    tokens: number | null;
  }[];
  feedback: { rated: number; positive: number | null }[];
  data_dir: string;
}
export interface Health {
  engine_ok: boolean;
  models: { name: string; size: number }[] | null;
  error: string | null;
  dependencies: Record<string, boolean>;
  whisper_model_exists: boolean;
  memory_total: number;
  memory_used: number;
  platform: string;
  arch: string;
}
