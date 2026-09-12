import { useState } from "react";
import { t } from "../i18n";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Eye,
  FolderOpen,
  Plus,
  RefreshCw,
  Trash2,
  CircleAlert,
  LoaderCircle,
  ArrowDownToLine,
  Repeat,
  FolderTree,
  Pause,
  Play,
  ListChecks,
  CircleCheck,
} from "lucide-react";
import type { Doc, Watch } from "../types";
import { Button, PageHeading, Empty, Modal } from "../components/Common";
import { confirm } from "../components/SourceEditor";
import { api, desktop, number } from "../api";

// Offered intervals, in seconds.
export const INTERVALS: [number, string][] = [
  [0, t("Global setting")],
  [30, "30 seconds"],
  [60, "1 minute"],
  [300, "5 minutes"],
  [900, "15 minutes"],
  [3600, "1 hour"],
  [6 * 3600, "6 hours"],
  [86400, "1 day"],
  [7 * 86400, "7 days"],
];
function intervalLabel(secs: number | null, fallback: number): string {
  const v = secs || 0;
  const found = INTERVALS.find(([s]) => s === v);
  if (found && v) return t(found[1] as string);
  if (!v) return t("Global ({n} s)", { n: fallback });
  return v % 86400 === 0
    ? t("{n} day(s)", { n: v / 86400 })
    : v % 3600 === 0
      ? t("{n} hour(s)", { n: v / 3600 })
      : v % 60 === 0
        ? t("{n} minute(s)", { n: v / 60 })
        : `${v} s`;
}
const ago = (at: number | null) => {
  if (!at) return t("never");
  const d = Math.max(0, Math.floor(Date.now() / 1000 - at));
  return d < 60
    ? t("{n} s ago", { n: d })
    : d < 3600
      ? t("{n} min ago", { n: Math.floor(d / 60) })
      : d < 86400
        ? t("{n} h ago", { n: Math.floor(d / 3600) })
        : t("{n} d ago", { n: Math.floor(d / 86400) });
};

// Watched folders.
export default function Watches({
  watches,
  docs,
  interval,
  paused,
  onError,
  onChanged,
}: {
  watches: Watch[];
  docs: Doc[];
  interval: number;
  paused: boolean;
  onError: (e: unknown) => void;
  onChanged: () => Promise<void>;
}) {
  const [detail, setDetail] = useState<Watch | null>(null);
  const [path, setPath] = useState("");
  const [mode, setMode] = useState<"sync" | "hoover">("sync");
  const [recursive, setRecursive] = useState(true);
  const [every, setEvery] = useState(0);
  const [busy, setBusy] = useState<string | null>(null);

  async function pick() {
    try {
      const dir = await open({
        directory: true,
        multiple: false,
        title: t("Folder to watch"),
      });
      if (typeof dir === "string") setPath(dir);
    } catch (e) {
      onError(e);
    }
  }
  async function add() {
    if (!path.trim()) return;
    setBusy("add");
    try {
      await api("add_watch", {
        path,
        mode,
        recursive,
        interval: every || null,
      });
      setPath("");
      await onChanged();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(null);
    }
  }
  async function patch(w: Watch, fields: Record<string, unknown>) {
    setBusy(w.id);
    try {
      await api("update_watch", { id: w.id, ...fields });
      await onChanged();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(null);
    }
  }
  async function scan(w: Watch) {
    setBusy(w.id);
    try {
      const r = await api<{
        added: number;
        updated: number;
        moved: number;
        errors: string[];
      }>("scan_watch", { id: w.id });
      await onChanged();
      if (r.errors.length) onError(new Error(r.errors.slice(0, 3).join(" · ")));
    } catch (e) {
      onError(e);
    } finally {
      setBusy(null);
    }
  }
  async function remove(w: Watch) {
    const forget = await confirm(
      t(
        'Remove the watch "{path}"?\n\nOK: drop the watch and keep the {n} source(s) already ingested.\nCancel: do nothing.',
        { path: w.path, n: w.total },
      ),
      t("Remove the watch"),
    );
    if (!forget) return;
    setBusy(w.id);
    try {
      await api("delete_watch", { id: w.id, forget: false });
      await onChanged();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(null);
    }
  }
  async function removeAll(w: Watch) {
    if (
      !(await confirm(
        t(
          "Remove the watch AND forget its {n} source(s) (passages and vectors dropped from memory)? Files on disk are untouched.",
          { n: w.total },
        ),
        t("Forget the watch and its knowledge"),
      ))
    )
      return;
    setBusy(w.id);
    try {
      await api("delete_watch", { id: w.id, forget: true });
      await onChanged();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(null);
    }
  }

  return (
    <main className="content-page">
      <PageHeading
        eyebrow={t("YOUR MEMORY KEEPS WATCH")}
        title={t("The watches.")}
        description={t(
          "Folders watched continuously, local or on a mounted NAS. Whatever lands there enters your memory on its own.",
        )}
        action={
          <Button
            primary={paused}
            onClick={() =>
              void api("set_ingestion_paused", { paused: !paused })
                .then(onChanged)
                .catch(onError)
            }
            title={t(
              "Pause processing of queued files without losing anything",
            )}
          >
            {paused ? <Play size={15} /> : <Pause size={15} />}{" "}
            {paused ? t("Resume processing") : t("Pause processing")}
          </Button>
        }
      />
      {paused && (
        <div className="setting-note">
          <Pause size={16} />
          <p>
            {t(
              "Processing paused: watches keep spotting files and the queue grows, but nothing is ingested. Nothing is lost, everything resumes on restart.",
            )}
          </p>
        </div>
      )}
      <section className="panel watch-form">
        <div className="watch-form-row">
          <div className="searchbox grow">
            <FolderOpen size={16} />
            <input
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder="/Volumes/NAS/Documents or /Users/me/Notes"
              aria-label={t("Folder to watch")}
            />
          </div>
          {desktop && (
            <Button onClick={() => void pick()}>
              <FolderOpen size={15} /> {t("Choose")}
            </Button>
          )}
        </div>
        <div className="watch-form-row">
          <div className="segmented">
            <button
              className={mode === "sync" ? "selected" : ""}
              onClick={() => setMode("sync")}
            >
              <Repeat size={13} /> {t("Sync")}
            </button>
            <button
              className={mode === "hoover" ? "selected" : ""}
              onClick={() => setMode("hoover")}
            >
              <ArrowDownToLine size={13} /> {t("Hoover")}
            </button>
          </div>
          <label className="inline-check">
            <input
              type="checkbox"
              checked={recursive}
              onChange={(e) => setRecursive(e.target.checked)}
            />
            {t("Subfolders")}
          </label>
          <select
            value={every}
            onChange={(e) => setEvery(Number(e.target.value))}
            aria-label="Intervalle"
          >
            {INTERVALS.map(([s, l]) => (
              <option key={s} value={s}>
                {s ? l : t("Global setting ({n} s)", { n: interval })}
              </option>
            ))}
          </select>
          <Button
            primary
            disabled={!path.trim() || busy === "add"}
            onClick={() => void add()}
          >
            {busy === "add" ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <Plus size={15} />
            )}{" "}
            {t("Add the watch")}
          </Button>
        </div>
        <p className="field-help">
          <b>{t("Sync")}</b>{" "}
          {t(
            "leaves files where they are and reprocesses the ones that change.",
          )}{" "}
          <b>{t("Hoover")}</b>{" "}
          {t(
            "moves each file into Langolier's data folder then indexes it: the watched folder empties, the original is kept for a future reindex.",
          )}
        </p>
      </section>
      {watches.length ? (
        <div className="watch-grid">
          {watches.map((w) => {
            const pct = w.total ? Math.round((w.ready / w.total) * 100) : 0;
            const on = !!w.enabled;
            const working = busy === w.id;
            return (
              <article key={w.id} className={`watch-card${on ? "" : " off"}`}>
                <header>
                  <span className={`watch-mode ${w.mode}`}>
                    {w.mode === "hoover" ? (
                      <ArrowDownToLine size={12} />
                    ) : (
                      <Repeat size={12} />
                    )}
                    {w.mode === "hoover" ? t("Hoover") : t("Sync")}
                  </span>
                  <button
                    className={`toggle${on ? " on" : ""}`}
                    role="switch"
                    aria-checked={on}
                    aria-label={
                      on ? t("Disable the watch") : t("Enable the watch")
                    }
                    disabled={working}
                    onClick={() => void patch(w, { enabled: !on })}
                  >
                    <span />
                  </button>
                </header>
                <h3 title={w.path}>
                  <FolderTree size={15} />{" "}
                  <span className="truncate">{w.path}</span>
                </h3>
                <div
                  className="watch-progress"
                  aria-label={t("{pct}% indexed", { pct })}
                >
                  <div
                    style={{ width: `${pct}%` }}
                    className={w.pending ? "pulse" : ""}
                  />
                </div>
                <div className="watch-stats">
                  <span>
                    <b>{number(w.ready)}</b>/{number(w.total)} {t("ready")}
                  </span>
                  {w.pending > 0 && (
                    <span className="lime">
                      {t("{n} running", { n: w.pending })}
                    </span>
                  )}
                  {w.errors > 0 && (
                    <span className="warn">
                      {t("{n} failed", { n: w.errors })}
                    </span>
                  )}
                  <span className="muted">
                    <Eye size={11} /> {ago(w.last_scan)}
                  </span>
                </div>
                {w.last_error && (
                  <p className="watch-error">
                    <CircleAlert size={13} /> {w.last_error}
                  </p>
                )}
                <footer>
                  <select
                    value={w.mode}
                    disabled={working}
                    onChange={(e) => void patch(w, { mode: e.target.value })}
                    aria-label="Mode"
                  >
                    <option value="sync">{t("Sync")}</option>
                    <option value="hoover">{t("Hoover")}</option>
                  </select>
                  <select
                    value={w.interval || 0}
                    disabled={working}
                    onChange={(e) =>
                      void patch(w, { interval: Number(e.target.value) })
                    }
                    aria-label="Intervalle"
                    title={intervalLabel(w.interval, interval)}
                  >
                    {INTERVALS.map(([s, l]) => (
                      <option key={s} value={s}>
                        {s ? l : `Global (${interval} s)`}
                      </option>
                    ))}
                  </select>
                  <label
                    className="inline-check"
                    title={t("Include subfolders")}
                  >
                    <input
                      type="checkbox"
                      checked={!!w.recursive}
                      disabled={working}
                      onChange={(e) =>
                        void patch(w, { recursive: e.target.checked })
                      }
                    />
                    {t("Subfolders")}
                  </label>
                  <span className="spacer" />
                  <button
                    title={t("See this watch's files")}
                    onClick={() => setDetail(w)}
                  >
                    <ListChecks size={14} />
                  </button>
                  <button
                    title={t("Scan now")}
                    disabled={working || !on}
                    onClick={() => void scan(w)}
                  >
                    {working ? (
                      <LoaderCircle className="spin" size={14} />
                    ) : (
                      <RefreshCw size={14} />
                    )}
                  </button>
                  <button
                    title={t("Remove the watch (keep the knowledge)")}
                    disabled={working}
                    onClick={() => void remove(w)}
                  >
                    <Trash2 size={14} />
                  </button>
                  <button
                    className="danger"
                    title={t("Remove the watch and forget its sources")}
                    disabled={working}
                    onClick={() => void removeAll(w)}
                  >
                    <Trash2 size={14} /> +
                  </button>
                </footer>
              </article>
            );
          })}
        </div>
      ) : (
        <Empty
          title={t("No watch yet.")}
          text={t(
            "Add a folder: every supported file that appears there gets indexed, and changed files are reprocessed.",
          )}
        />
      )}
      {detail && (
        <WatchDetail
          watch={detail}
          docs={docs.filter((d) => d.watch_id === detail.id)}
          onClose={() => setDetail(null)}
          onError={onError}
          onChanged={onChanged}
        />
      )}
      <div className="info-note">
        <CircleAlert size={15} />
        <p>
          {t(
            "Turning a card off pauses watching without erasing anything. Network folders are polled rather than notified: an unmounted volume is flagged on the card and picked up again as soon as it reappears.",
          )}
        </p>
      </div>
    </main>
  );
}

/// What the watch ingested, and above all what is stuck.
function WatchDetail({
  watch,
  docs,
  onClose,
  onError,
  onChanged,
}: {
  watch: Watch;
  docs: Doc[];
  onClose: () => void;
  onError: (e: unknown) => void;
  onChanged: () => Promise<void>;
}) {
  const errors = docs.filter((d) => d.status === "error");
  const running = docs.filter(
    (d) => d.status === "queued" || d.status === "processing",
  );
  const ready = docs.filter((d) => d.status === "ready");
  async function retry(ids: string[]) {
    try {
      await api("reindex_documents", { ids });
      await onChanged();
    } catch (e) {
      onError(e);
    }
  }
  return (
    <Modal title={watch.path} wide onClose={onClose}>
      <div className="watch-detail">
        {running.length > 0 && (
          <section>
            <h4>
              <LoaderCircle className="spin" size={14} /> En traitement (
              {running.length})
            </h4>
            <ul>
              {running.slice(0, 200).map((d) => (
                <li key={d.id}>
                  <span className="truncate">{d.name}</span>
                  <em>{d.status === "processing" ? d.stage : "en attente"}</em>
                </li>
              ))}
            </ul>
            {running.length > 200 && (
              <p className="muted">…et {running.length - 200} autres.</p>
            )}
          </section>
        )}
        {errors.length > 0 && (
          <section className="watch-detail-errors">
            <h4>
              <CircleAlert size={14} /> En erreur ({errors.length})
              <Button onClick={() => void retry(errors.map((d) => d.id))}>
                <RefreshCw size={13} /> {t("Retry everything")}
              </Button>
            </h4>
            <ul>
              {errors.map((d) => (
                <li key={d.id}>
                  <span className="truncate" title={d.source}>
                    {d.name}
                  </span>
                  <em title={d.error || ""}>{d.error || "raison inconnue"}</em>
                  <button title={t("Retry")} onClick={() => void retry([d.id])}>
                    <RefreshCw size={13} />
                  </button>
                </li>
              ))}
            </ul>
          </section>
        )}
        <section>
          <h4>
            <CircleCheck size={14} /> {t("Indexed")} ({ready.length})
          </h4>
          {ready.length === 0 && (
            <p className="muted">{t("No indexed source yet.")}</p>
          )}
        </section>
        {!docs.length && (
          <Empty
            title={t("No tracked file")}
            text={t(
              "This watch has found nothing yet, or its sources were detached.",
            )}
          />
        )}
      </div>
    </Modal>
  );
}
