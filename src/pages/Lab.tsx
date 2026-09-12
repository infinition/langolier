import { useState } from "react";
import { t } from "../i18n";
import {
  Search,
  LoaderCircle,
  ArrowRight,
  FlaskConical,
  Plus,
  X,
  Sparkles,
  Download,
} from "lucide-react";
import type { Snapshot, Source, Report } from "../types";
import { Button, PageHeading } from "../components/Common";
import { api } from "../api";
export default function Lab({
  data,
  refresh,
  onError,
  onExport,
  onSource,
}: {
  data: Snapshot;
  refresh: () => Promise<void>;
  onError: (e: unknown) => void;
  onExport: () => void;
  onSource: (s: Source) => void;
}) {
  const [query, setQuery] = useState("");
  const [searchMode, setSearchMode] = useState("hybrid");
  const [result, setResult] = useState<{
    sources: Source[];
    latency_ms: number;
    warning?: string;
  } | null>(null);
  const [question, setQuestion] = useState("");
  const [expected, setExpected] = useState("");
  const [reports, setReports] = useState<Report[]>([]);
  const [busy, setBusy] = useState("");
  async function search() {
    setBusy("search");
    try {
      setResult(await api("search_sources", { query, mode: searchMode }));
    } catch (e) {
      onError(e);
    } finally {
      setBusy("");
    }
  }
  async function add() {
    try {
      await api("add_evaluation", { question, expectedDocument: expected });
      setQuestion("");
      await refresh();
    } catch (e) {
      onError(e);
    }
  }
  async function evaluate() {
    setBusy("eval");
    try {
      setReports(await api("run_evaluations"));
      await refresh();
    } catch (e) {
      onError(e);
    } finally {
      setBusy("");
    }
  }
  return (
    <main className="content-page">
      <PageHeading
        eyebrow={t("INTUITION, TESTED AGAINST FACTS")}
        title={t("The laboratory.")}
        description={t(
          "Inspect retrieval. Compare strategies. Build your reference set.",
        )}
      />
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h3>{t("Retrieval microscope")}</h3>
            <p>
              {t(
                "The passages retrieved before any generation, with their fusion score.",
              )}
            </p>
          </div>
          <Search size={20} />
        </div>
        <div className="lab-search">
          <input
            aria-label={t("Search question")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("What are you looking for?")}
          />
          <select
            value={searchMode}
            onChange={(e) => setSearchMode(e.target.value)}
          >
            <option value="hybrid">{t("Hybrid")}</option>
            <option value="lexical">{t("Lexical BM25")}</option>
            <option value="semantic">{t("Semantic")}</option>
          </select>
          <Button
            primary
            disabled={!!busy || !query.trim()}
            onClick={() => void search()}
          >
            {busy === "search" ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <ArrowRight size={15} />
            )}{" "}
            Rechercher
          </Button>
        </div>
        {result && (
          <div className="lab-results">
            <div className="muted">
              {result.sources.length} passages · {result.latency_ms} ms{" "}
              {result.warning && `· ${result.warning}`}
            </div>
            {result.sources.map((s, i) => (
              <button
                key={s.id}
                className="lab-result"
                onClick={() => onSource(s)}
              >
                <span className="rank">{i + 1}</span>
                <div>
                  <strong>
                    {s.name}
                    <small>{s.locator}</small>
                  </strong>
                  <p>{s.text.slice(0, 240)}…</p>
                </div>
                <code>{s.score.toFixed(4)}</code>
              </button>
            ))}
          </div>
        )}
      </section>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h3>{t("Benchmark your corpus")}</h3>
            <p>
              {t(
                "One question, one expected document. Compare BM25, semantic and hybrid.",
              )}
            </p>
          </div>
          <Button
            onClick={() => void evaluate()}
            disabled={!!busy || !data.evaluations.length}
          >
            {busy === "eval" ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <FlaskConical size={15} />
            )}{" "}
            Lancer la comparaison
          </Button>
        </div>
        <div className="evaluation-form">
          <input
            aria-label={t("Reference question")}
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            placeholder={t("e.g. How do I limit overfitting?")}
          />
          <select
            aria-label={t("Expected source")}
            value={expected}
            onChange={(e) => setExpected(e.target.value)}
          >
            <option value="">{t("Expected source")}</option>
            {data.documents
              .filter((d) => d.status === "ready")
              .map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
          </select>
          <Button
            disabled={!question.trim() || !expected}
            onClick={() => void add()}
          >
            <Plus size={15} /> {t("Add")}
          </Button>
        </div>
        <div className="eval-cases">
          {data.evaluations.map((e) => (
            <div key={e.id}>
              <span>{e.question}</span>
              <small>
                {data.documents.find((d) => d.id === e.expected_document)
                  ?.name || t("Source removed")}
              </small>
              <button
                aria-label={t("Delete this case")}
                onClick={() =>
                  void api("delete_evaluation", { id: e.id })
                    .then(refresh)
                    .catch(onError)
                }
              >
                <X size={13} />
              </button>
            </div>
          ))}
        </div>
        {reports.length > 0 && (
          <div className="benchmark-results">
            {reports.map((r) => (
              <div key={r.mode}>
                <span className="eyebrow">{r.mode}</span>
                <strong>
                  {Math.round(r.hit_at_k * 100)}
                  <small>%</small>
                </strong>
                <p>
                  Hit@{r.k} sur {r.n} questions
                </p>
                <div>
                  <span>MRR</span>
                  <b>{r.mrr.toFixed(3)}</b>
                </div>
                <div>
                  <span>{t("Total time")}</span>
                  <b>{(r.latency_ms / 1000).toFixed(1)} s</b>
                </div>
                <div>
                  <span>{t("Errors / fallbacks")}</span>
                  <b>{r.errors}</b>
                </div>
              </div>
            ))}
          </div>
        )}
        <p className="field-help">
          {t(
            "Hit@K: expected document present in the K passages. MRR: mean reciprocal rank. These measure retrieval, not the truth of an answer. Results are kept in the log and in the metrics export.",
          )}
        </p>
      </section>
      <section className="training-card">
        <div className="training-icon">
          <Sparkles size={23} />
        </div>
        <div>
          <span className="eyebrow">{t("FROM CONVERSATION TO DATASET")}</span>
          <h3>{t("Learn from what works.")}</h3>
          <p>
            {t(
              "Export the answers you approved as JSONL to prepare a LoRA fine-tune. The MLX recipe and the data separation protocol ship with the project.",
            )}
          </p>
          <span className="badge">
            {t("EXPORT READY · TRAINING OUTSIDE THE APP")}
          </span>
        </div>
        <Button onClick={onExport}>
          <Download size={16} /> {t("Export the dataset")}
        </Button>
      </section>
    </main>
  );
}
