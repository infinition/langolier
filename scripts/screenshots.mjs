// Captures the README screenshots against a stubbed backend, so the images
// hold demo data only and never someone's real library.
//
//   npm run dev
//   node scripts/screenshots.mjs [outDir]
import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";

const OUT = process.argv[2] || "docs/screenshots";
const URL = process.env.LANGOLIER_URL || "http://127.0.0.1:1420";
const DAY = 86400;
const now = Math.floor(Date.now() / 1000);

const doc = (id, name, source, kind, chunks, days, watch_id = null) => ({
  id, name, source, kind, status: "ready", stage: "", error: null,
  language: "en", bytes: chunks * 4200, chunks, embedded: chunks,
  created: now - days * DAY, watch_id,
});

const SNAPSHOT = {
  settings: {
    provider: "ollama", endpoint: "http://127.0.0.1:11434", model: "qwen3:8b",
    embedding_endpoint: "http://127.0.0.1:11434", embedding_model: "embeddinggemma",
    whisper_model: "/opt/models/ggml-base.bin", top_k: 6, temperature: 0.2,
    context_size: 8192, api_key: "", max_tokens: 2048, min_dense_score: 0.35,
    min_lexical_score: 0, rerank_enabled: true, rerank_model: "",
    rerank_candidates: 20, rerank_threshold: 0.35, strict_grounding: true,
    abstain_text: "I do not have that information.", verify_answer: false,
    shortcut: "CommandOrControl+Shift+Space", watch_interval: 30,
    active_assistant: "a1", ingestion_paused: false,
  },
  documents: [
    doc("d1", "onboarding-handbook.pdf", "/srv/knowledge/hr/onboarding-handbook.pdf", "pdf", 84, 2, "w1"),
    doc("d2", "expense-policy.docx", "/srv/knowledge/hr/expense-policy.docx", "docx", 31, 2, "w1"),
    doc("d3", "q3-forecast.xlsx", "/srv/knowledge/finance/q3-forecast.xlsx", "xlsx", 12, 4, "w1"),
    doc("d4", "architecture-review.pptx", "/srv/knowledge/engineering/architecture-review.pptx", "pptx", 27, 5, "w1"),
    doc("d5", "retrieval.rs", "/srv/knowledge/engineering/src/retrieval.rs", "rs", 19, 6),
    doc("d6", "ingest.py", "/srv/knowledge/engineering/src/ingest.py", "py", 23, 6),
    doc("d7", "embeddings-benchmark.ipynb", "/srv/knowledge/research/embeddings-benchmark.ipynb", "ipynb", 41, 9),
    doc("d8", "All-hands, October", "/srv/knowledge/meetings/all-hands-october.mp4", "mp4", 156, 11, "w2"),
    doc("d9", "Design review recording", "/srv/knowledge/meetings/design-review.mp4", "mp4", 98, 13, "w2"),
    doc("d10", "Field notes, voice memo", "/srv/knowledge/meetings/field-notes.m4a", "m4a", 34, 14, "w2"),
    doc("d11", "release-notes.md", "/srv/knowledge/engineering/release-notes.md", "md", 16, 15),
    doc("d12", "support-tickets.csv", "/srv/knowledge/support/support-tickets.csv", "csv", 63, 18),
  ],
  conversations: [
    { id: "c1", title: "Expense limits for contractors", created: now - 3600, assistant_id: "a1" },
    { id: "c2", title: "What changed in the retrieval path", created: now - 2 * DAY, assistant_id: "" },
    { id: "c3", title: "Summarise the October all-hands", created: now - 4 * DAY, assistant_id: "a1" },
  ],
  watches: [
    { id: "w1", path: "/srv/knowledge", mode: "sync", enabled: 1, recursive: 1,
      created: now - 30 * DAY, last_scan: now - 120, last_error: null, interval: null,
      total: 154, ready: 151, errors: 1, pending: 2 },
    { id: "w2", path: "/Volumes/media/recordings", mode: "hoover", enabled: 1, recursive: 0,
      created: now - 12 * DAY, last_scan: now - 900, last_error: null, interval: 3600,
      total: 38, ready: 38, errors: 0, pending: 0 },
  ],
  assistants: [
    { id: "a1", name: "Handbook", mission: "Answer staff questions from the HR handbook and the expense policy. Quote the clause. Never guess.",
      welcome: "Hello, ask me anything about the handbook.", avatar: "", theme: "nuit",
      scope: { watches: ["w1"], documents: [] }, overrides: {}, show_sources: true,
      hide_source_names: false, max_conversation_tokens: 0, daily_token_budget: 200000,
      telegram_token: "", telegram_allowed: "", created: now - 20 * DAY },
    { id: "a2", name: "Release notes", mission: "Explain what shipped, when, and what broke. Engineering audience.",
      welcome: "Which release do you want to know about?", avatar: "", theme: "encre",
      scope: { all: true }, overrides: { model: "qwen3:4b-instruct" }, show_sources: false,
      hide_source_names: true, max_conversation_tokens: 24000, daily_token_budget: 0,
      telegram_token: "", telegram_allowed: "", created: now - 6 * DAY },
  ],
  runs: Array.from({ length: 24 }, (_, i) => ({
    id: `r${i}`, kind: i % 5 === 0 ? "ingest" : "chat", model: "qwen3:8b",
    status: i === 7 ? "error" : "ok", latency_ms: 1800 + ((i * 733) % 2600),
    tokens: 140 + ((i * 97) % 500), tps: 16 + ((i * 7) % 9),
    details: "hybrid", created: now - (24 - i) * 1800,
  })),
  evaluations: [
    { id: "e1", question: "What is the per diem for travel?", expected_document: "d2" },
    { id: "e2", question: "Which service owns reranking?", expected_document: "d4" },
  ],
  totals: [{ requests: 318, successful: 311, latency_ms: 2940, tps: 18.4, tokens: 91400 }],
  feedback: [{ rated: 46, positive: 41 }],
  data_dir: "/home/demo/.local/share/local.langolier.studio",
};

const HEALTH = {
  engine_ok: true,
  models: [
    { name: "qwen3:8b", size: 5_200_000_000 },
    { name: "qwen3:4b-instruct", size: 2_500_000_000 },
    { name: "embeddinggemma", size: 330_000_000 },
  ],
  error: null,
  dependencies: { ffmpeg: true, "whisper-cli": true, "yt-dlp": true, tesseract: true, pdftoppm: true },
  whisper_model_exists: true,
  memory_total: 25_769_803_776,
  memory_used: 11_811_160_064,
  platform: "linux",
  arch: "x86_64",
};

const MESSAGES = [
  { id: "m1", role: "user", content: "What is the expense limit for a contractor travelling abroad?", sources: [] },
  { id: "m2", role: "assistant", feedback: 1,
    content:
      "Contractors travelling outside the country claim a flat per diem of 65 EUR, capped at fourteen consecutive days [1]. Anything above that needs written approval from the budget owner before the trip, not after [1].\n\nAccommodation is reimbursed on receipts up to 180 EUR a night in listed cities and 120 EUR elsewhere [2]. Flights must be booked in economy unless the leg is over eight hours [2].",
    sources: [
      { id: 1, doc_id: "d2", name: "expense-policy.docx", locator: "Heading 4 . line 112", score: 0.81,
        text: "Contractors travelling outside the country claim a flat per diem of 65 EUR, capped at fourteen consecutive days. Amounts above the cap require written approval from the budget owner in advance." },
      { id: 2, doc_id: "d2", name: "expense-policy.docx", locator: "Heading 5 . line 148", score: 0.74,
        text: "Accommodation is reimbursed on receipts up to 180 EUR per night in listed cities and 120 EUR elsewhere. Flights are booked in economy unless a single leg exceeds eight hours." },
    ] },
];

const CHUNKS = [
  { id: 1, locator: "Page 3 . excerpt 1", text: "Every new joiner is assigned a buddy for the first four weeks. The buddy is not the line manager and is not responsible for performance." },
  { id: 2, locator: "Page 4 . excerpt 1", text: "Equipment is ordered before the start date. A replacement machine is available on request within two working days." },
  { id: 3, locator: "Page 7 . excerpt 2", text: "Probation lasts three months and ends with a written review. Extensions are possible once, for a further month." },
];

const STUB = {
  snapshot: SNAPSHOT,
  health: HEALTH,
  messages: MESSAGES,
  document_chunks: CHUNKS,
  api_keys: [{ id: "k1", label: "Work OpenAI", provider: "openai_cloud", hint: "sk-...4f2a" }],
  embed_models: [
    { tag: "embeddinggemma", label: "EmbeddingGemma 300M (multilingual, light)", dims: 768, gguf_file: "embeddinggemma.gguf" },
    { tag: "qwen3-embedding:0.6b", label: "Qwen3-Embedding 0.6B (multilingual, more accurate, heavier)", dims: 1024, gguf_file: "qwen3-embedding.gguf" },
  ],
  search_sources: [],
  new_conversation: "c1",
};

const PAGES = [
  ["home", "Conversation"],
  ["sources", "Sources"],
  ["watches", "Watches"],
  ["assistants", "Assistants"],
  ["engines", "Engines"],
  ["observatory", "Observatory"],
];

await mkdir(OUT, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });

await page.addInitScript((stub) => {
  localStorage.setItem("langolier.lang", "en");
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { windowLabel: "main", label: "main" },
    },
    plugins: {},
    invoke: async (cmd) => {
      if (cmd.startsWith("plugin:")) return cmd.endsWith("|listen") ? 0 : null;
      return cmd in stub ? stub[cmd] : null;
    },
    transformCallback: (cb) => {
      const id = Math.floor(Math.random() * 1e9);
      window[`_${id}`] = cb;
      return id;
    },
  };
}, STUB);

await page.goto(URL, { waitUntil: "networkidle" });
for (const [file, nav] of PAGES) {
  await page.locator("nav button").filter({ hasText: nav }).first().click();
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${OUT}/${file}.png` });
  console.log(`${OUT}/${file}.png`);
}

// The assistant editor, where a profile is actually configured.
await page.locator("nav button").filter({ hasText: "Assistants" }).first().click();
await page.waitForTimeout(400);
await page.locator(".assistant-list button, .assistant-row").filter({ hasText: "Handbook" }).first().click();
await page.waitForTimeout(800);
await page.screenshot({ path: `${OUT}/assistant-editor.png` });
console.log(`${OUT}/assistant-editor.png`);

// A grounded answer with its citations, the view that matters most.
await page.locator("nav button").filter({ hasText: "Conversation" }).first().click();
await page.locator(".history-item button").first().click();
await page.waitForTimeout(900);
await page.screenshot({ path: `${OUT}/conversation.png` });
console.log(`${OUT}/conversation.png`);
await browser.close();
