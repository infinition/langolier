import {
  MessageCircle,
  CircleCheck,
  Activity,
  ThumbsUp,
  Download,
  Cpu,
  FlaskConical,
} from "lucide-react";
import { t } from "../i18n";
import type { Snapshot, Health } from "../types";
import { Button, PageHeading } from "../components/Common";
import { number, bytes, locale } from "../api";
export default function Metrics({
  data,
  health,
  onExport,
}: {
  data: Snapshot;
  health: Health | null;
  onExport: () => void;
}) {
  const total = data.totals[0];
  const chatRuns = data.runs
    .filter((r) => r.kind === "chat")
    .slice(0, 24)
    .reverse();
  const rated = data.feedback[0];
  const success = total.requests
    ? `${Math.round(((total.successful || 0) / total.requests) * 100)} %`
    : "N/D";
  const max = Math.max(...chatRuns.map((r) => r.latency_ms), 1);
  const stats = [
    {
      label: t("COMPLETED QUERIES"),
      value: number(total.requests),
      detail: t("Every conversation attempt"),
      icon: MessageCircle,
    },
    {
      label: t("TECHNICAL SUCCESS"),
      value: success,
      detail: t("Completed generations / attempts"),
      icon: CircleCheck,
    },
    {
      label: t("MODEL SPEED"),
      value: total.tps ? `${total.tps.toFixed(1)}` : "N/D",
      detail: t("Tokens per second, measured at the engine"),
      icon: Activity,
    },
    {
      label: t("POSITIVE FEEDBACK"),
      value: rated.rated
        ? `${Math.round(((rated.positive || 0) / rated.rated) * 100)} %`
        : "N/D",
      detail: t("{n} answer(s) rated by hand", { n: rated.rated }),
      icon: ThumbsUp,
    },
  ];
  return (
    <main className="content-page">
      <PageHeading
        eyebrow={t("MEASURE TO UNDERSTAND")}
        title={t("Under the hood.")}
        description={t("What works, what drags, what deserves your attention.")}
        action={
          <Button onClick={onExport}>
            <Download size={15} /> {t("Export the metrics")}
          </Button>
        }
      />
      <div className="stats-grid">
        {stats.map(({ label, value, detail, icon: Icon }) => (
          <div className="stat-card" key={label}>
            <div>
              <span>{label}</span>
              <Icon size={17} />
            </div>
            <strong>{value}</strong>
            <p>{detail}</p>
          </div>
        ))}
      </div>
      <div className="metrics-grid">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <h3>{t("The rhythm of your exchanges")}</h3>
              <p>{t("Total latency of the last 24 attempts")}</p>
            </div>
            <span className="badge">{t("REAL MEASUREMENTS")}</span>
          </div>
          {chatRuns.length ? (
            <>
              <div className="bar-chart">
                {chatRuns.map((r) => (
                  <div
                    key={r.id}
                    title={`${(r.latency_ms / 1000).toFixed(1)} s · ${r.status}`}
                    className={r.status === "ok" ? "" : "failed"}
                    style={{
                      height: `${Math.max(4, (r.latency_ms / max) * 100)}%`,
                    }}
                  />
                ))}
              </div>
              <div className="chart-axis">
                <span>{t("Oldest")}</span>
                <span>Maximum : {(max / 1000).toFixed(1)} s</span>
                <span>{t("Now")}</span>
              </div>
            </>
          ) : (
            <div className="chart-empty">
              <Activity size={30} />
              <p>{t("The first exchange will draw the curve.")}</p>
              <span>{t("No simulated data.")}</span>
            </div>
          )}
        </section>
        <section className="panel machine-panel">
          <div className="panel-heading">
            <div>
              <h3>{t("The local footprint")}</h3>
              <p>{t("Your machine's resources")}</p>
            </div>
            <Cpu size={20} />
          </div>
          <div className="memory-gauge">
            <strong>{health ? bytes(health.memory_used) : "N/D"}</strong>
            <span>{t("system memory used")}</span>
            <div className="progress">
              <i
                style={{
                  width: health
                    ? `${(health.memory_used / health.memory_total) * 100}%`
                    : "0%",
                }}
              />
            </div>
            <small>
              {health
                ? `${bytes(health.memory_total)} disponibles au total`
                : t("Open the native app to measure")}
            </small>
          </div>
          <div className="machine-row">
            <span>{t("Indexed text")}</span>
            <b>
              {number(data.documents.reduce((a, d) => a + d.chunks, 0))}{" "}
              passages
            </b>
          </div>
          <div className="machine-row">
            <span>{t("Tokens generated")}</span>
            <b>{number(total.tokens || 0)}</b>
          </div>
          <div className="machine-row">
            <span>{t("Average latency")}</span>
            <b>
              {total.latency_ms
                ? `${(total.latency_ms / 1000).toFixed(1)} s`
                : "N/D"}
            </b>
          </div>
        </section>
      </div>
      <section className="panel event-panel">
        <div className="panel-heading">
          <div>
            <h3>{t("Activity log")}</h3>
            <p>{t("Ingestion, conversations and evaluations")}</p>
          </div>
          <span className="badge">{t("LAST 100 EVENTS")}</span>
        </div>
        {data.runs.length ? (
          data.runs.slice(0, 20).map((r) => (
            <div key={r.id} className="event-row">
              <span
                className={`status-dot ${r.status === "ok" ? "online" : "warning"}`}
              />
              <b>
                {r.kind === "chat"
                  ? "Conversation"
                  : r.kind === "evaluation"
                    ? t("Evaluation")
                    : "Ingestion"}
              </b>
              <span className="truncate">
                {r.model || t("Document pipeline")}
              </span>
              <span>{r.status}</span>
              <span>{(r.latency_ms / 1000).toFixed(1)} s</span>
              <time>
                {new Date(r.created * 1000).toLocaleTimeString(locale())}
              </time>
            </div>
          ))
        ) : (
          <p className="muted no-events">
            {t("Your activity will show up here.")}
          </p>
        )}
      </section>
      <div className="info-note">
        <FlaskConical size={16} />
        <p>
          {t(
            "Technical success does not measure accuracy. Rate retrieval relevance in the Laboratory and check answers against their sources.",
          )}
        </p>
      </div>
    </main>
  );
}
