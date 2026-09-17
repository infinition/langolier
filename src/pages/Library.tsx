/// The sources page: the library, its filters, and the bulk actions on it.
import { useEffect, useState } from "react";
import { api, bytes, docError, number, patchSettings } from "../api";
import { Button, Empty, PageHeading } from "../components/Common";
import { confirm } from "../components/SourceEditor";
import LibraryTree from "../components/LibraryTree";
import MemorySearch from "../components/MemorySearch";
import { t } from "../i18n";
import type { Doc, Settings, Source } from "../types";
import {
  CircleAlert,
  CircleCheck,
  Code2,
  Copy,
  FileText,
  List,
  ListTree,
  LoaderCircle,
  Plus,
  RefreshCw,
  Search,
  ShieldCheck,
  Trash2,
  Video,
  Pause,
  Play,
} from "lucide-react";

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

export default function LibraryPage({
  docs,
  dataDir,
  settings,
  onImport,
  onOpenDoc,
  onOpenPassage,
  onError,
  onNotify,
  onRefresh,
  onShowDetail,
}: {
  docs: Doc[];
  dataDir: string;
  settings: Settings;
  onImport: () => void;
  onOpenDoc: (doc: Doc) => void;
  onOpenPassage: (source: Source, highlight: string) => void;
  onError: (e: unknown) => void;
  onNotify: (text: string) => void;
  onRefresh: () => Promise<void>;
  onShowDetail: (text: string) => void;
}) {
  const [filter, setFilter] = useState("");
  const [typeFilter, setTypeFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState<
    "all" | "pending" | "error" | "duplicate"
  >("all");
  const [libraryView, setLibraryView] = useState<"list" | "tree" | "search">(
    "list",
  );
  const [treeSort, setTreeSort] = useState<"name" | "date">("date");
  const queued = docs.filter((d) =>
    ["queued", "processing"].includes(d.status),
  ).length;
  const chunks = docs.reduce((a, d) => a + d.chunks, 0);
  const indexed = docs.reduce((a, d) => a + d.embedded, 0);
  const failed = docs.filter((d) => d.status === "error").length;
  const duplicates = docs.filter((d) => d.status === "duplicate").length;
  const visibleDocs = docs.filter(
    (d) =>
      d.name.toLowerCase().includes(filter.toLowerCase()) &&
      (typeFilter === "all" ||
        (typeFilter === "video"
          ? kindIcon(d.kind) === Video
          : typeFilter === "code"
            ? kindIcon(d.kind) === Code2
            : kindIcon(d.kind) === FileText)) &&
      (statusFilter === "all" ||
        (statusFilter === "pending"
          ? ["queued", "processing"].includes(d.status)
          : d.status === statusFilter)),
  );
  const retriable = visibleDocs.filter((d) => d.status === "error");
  const duplicated = visibleDocs.filter((d) => d.status === "duplicate");
  // A chip carries the filter, so it must not stay on once its sources are gone.
  useEffect(() => {
    if (
      (!failed && statusFilter === "error") ||
      (!duplicates && statusFilter === "duplicate")
    )
      setStatusFilter("all");
  }, [failed, duplicates, statusFilter]);
  async function removeDocs(ids: string[], label: string, prompt?: string) {
    const many = ids.length > 1;
    if (
      !(await confirm(
        prompt ??
          (many
            ? t(
                'Remove the {n} sources under "{label}" from the index? The original files are untouched.',
                { n: ids.length, label },
              )
            : t(
                'Remove "{label}" from the index? The original file is untouched.',
                { label },
              )),
        prompt ? label : many ? t("Remove a folder") : t("Remove a source"),
      ))
    )
      return;
    try {
      const r = await api<{ removed: number; busy: number }>(
        "delete_documents",
        { ids },
      );
      if (r.busy)
        onNotify(
          t("{n} source(s) removed; {busy} still processing, try again after.", {
            n: r.removed,
            busy: r.busy,
          }),
        );
      await onRefresh();
    } catch (e) {
      onError(e);
    }
  }
  /// Queues every failed source of the current view again.
  async function retryDocs(ids: string[]) {
    if (!ids.length) return;
    if (
      !(await confirm(
        t(
          "Process the {n} source(s) needing a check again? Processing runs in the background.",
          { n: ids.length },
        ),
        t("Process again"),
      ))
    )
      return;
    try {
      const r = await api<{ queued: number; busy: number }>("retry_documents", {
        ids,
      });
      onNotify(
        r.busy
          ? t("{n} source(s) queued again; {busy} still processing.", {
              n: r.queued,
              busy: r.busy,
            })
          : t("{n} source(s) queued again.", { n: r.queued }),
      );
      await onRefresh();
    } catch (e) {
      onError(e);
    }
  }
  return (
      <main className="content-page">
        <PageHeading
          eyebrow={t("YOUR KNOWLEDGE, NO SILOS")}
          title={t("The raw material.")}
          description={t(
            "Every source becomes a starting point. Your memory grows with you.",
          )}
          action={
            <Button primary onClick={onImport}>
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
          <button
            type="button"
            className={`stat-filter ${queued ? "lime" : ""} ${statusFilter === "pending" ? "selected" : ""}`}
            aria-pressed={statusFilter === "pending"}
            onClick={() =>
              setStatusFilter(statusFilter === "pending" ? "all" : "pending")
            }
          >
            <span
              className={`status-dot ${queued ? "online pulse" : ""}`}
            />
            {queued
              ? t("{n} queued or running", { n: queued })
              : t("Queue up to date")}
          </button>
          {failed > 0 && (
            <button
              type="button"
              className={`stat-filter amber ${statusFilter === "error" ? "selected" : ""}`}
              aria-pressed={statusFilter === "error"}
              onClick={() =>
                setStatusFilter(statusFilter === "error" ? "all" : "error")
              }
            >
              <CircleAlert size={13} />
              {t("{n} to check", { n: failed })}
            </button>
          )}
          {duplicates > 0 && (
            <button
              type="button"
              className={`stat-filter ${statusFilter === "duplicate" ? "selected" : ""}`}
              aria-pressed={statusFilter === "duplicate"}
              onClick={() =>
                setStatusFilter(
                  statusFilter === "duplicate" ? "all" : "duplicate",
                )
              }
            >
              <Copy size={12} />
              {t("{n} duplicate(s)", { n: duplicates })}
            </button>
          )}
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
          {libraryView !== "search" &&
            (statusFilter === "duplicate" && duplicated.length > 0 ? (
              <Button
                onClick={() =>
                  void removeDocs(
                    duplicated.map((d) => d.id),
                    t("Remove the duplicates"),
                    t(
                      "Remove the {n} duplicate(s) from the index? The source already holding those bytes stays, and the original files are untouched.",
                      { n: duplicated.length },
                    ),
                  )
                }
              >
                <Trash2 size={14} />{" "}
                {t("Remove {n} duplicate(s)", { n: duplicated.length })}
              </Button>
            ) : retriable.length > 0 ? (
              <Button
                onClick={() => void retryDocs(retriable.map((d) => d.id))}
              >
                <RefreshCw size={14} />{" "}
                {t("Process {n} again", { n: retriable.length })}
              </Button>
            ) : null)}
        </div>
        {libraryView === "search" ? (
          <MemorySearch
            onOpen={onOpenPassage}
            onError={onError}
            onChanged={() => void onRefresh()}
          />
        ) : libraryView === "tree" && docs.length ? (
          <LibraryTree
            docs={visibleDocs}
            dataDir={dataDir}
            sort={treeSort}
            onOpen={onOpenDoc}
            onDelete={(ids, label) => void removeDocs(ids, label)}
          />
        ) : !docs.length ? (
          <Empty
            title={t("A whole memory to build.")}
            text={t(
              "Drop a folder, paste text or add a video. Processing continues in the background.",
            )}
            action={
              <Button onClick={onImport}>
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
                    onClick={() => onOpenDoc(d)}
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
                      ) : d.status === "duplicate" ? (
                        <Copy size={12} />
                      ) : (
                        <span className="status-dot" />
                      )}
                      {t(d.stage)}
                    </span>
                    {d.error && (
                      <button
                        className="error-detail"
                        onClick={() => onShowDetail(docError(d.error!))}
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
                          .then(onRefresh)
                          .catch(onError)
                      }
                    >
                      <RefreshCw size={14} />
                    </button>
                    <button
                      title={t("Remove from the index")}
                      disabled={d.status === "processing"}
                      onClick={() =>
                        void api("delete_document", { id: d.id })
                          .then(onRefresh)
                          .catch(onError)
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
                action={
                  statusFilter !== "all" ? (
                    <Button onClick={() => setStatusFilter("all")}>
                      {t("Show every source")}
                    </Button>
                  ) : undefined
                }
              />
            )}
          </div>
        )}
        <section className="panel form-stack ingestion-settings">
          <div className="panel-heading">
            <div>
              <h3>{t("Processing")}</h3>
              <p>{t("How sources become passages and vectors")}</p>
            </div>
            <Button
              onClick={() =>
                void patchSettings(settings, {
                  ingestion_paused: !settings.ingestion_paused,
                })
                  .then(onRefresh)
                  .catch(onError)
              }
            >
              {settings.ingestion_paused ? (
                <Play size={15} />
              ) : (
                <Pause size={15} />
              )}{" "}
              {settings.ingestion_paused
                ? t("Resume processing")
                : t("Pause processing")}
            </Button>
          </div>
          {settings.ingestion_paused && (
            <div className="setting-note">
              <Pause size={16} />
              <p>
                {t(
                  "Processing paused: watches keep spotting files and the queue grows, but nothing is ingested. Nothing is lost, everything resumes on restart.",
                )}
              </p>
            </div>
          )}
          <label className="check inline-check">
            <input
              type="checkbox"
              checked={settings.ingest_in_background}
              onChange={(e) =>
                void patchSettings(settings, {
                  ingest_in_background: e.target.checked,
                })
                  .then(onRefresh)
                  .catch(onError)
              }
            />
            {t("Keep processing sources in the menu bar")}
            <small>
              {t(
                "Off: queued sources wait. OCR, transcription and vectors are what heat the machine.",
              )}
            </small>
          </label>
          <label>
            {t("Passage length: {n} characters", { n: settings.chunk_size })}
            <input
              type="range"
              min="400"
              max="4000"
              step="100"
              value={settings.chunk_size}
              onChange={(e) =>
                void patchSettings(settings, {
                  chunk_size: Number(e.target.value),
                })
                  .then(onRefresh)
                  .catch(onError)
              }
            />
            <small>
              {t(
                "Applies to sources indexed from now on. Existing passages keep the length they were cut with, until a reindex.",
              )}
            </small>
          </label>
          <label>
            {t("Overlap between passages: {n} characters", {
              n: settings.chunk_overlap,
            })}
            <input
              type="range"
              min="0"
              max="800"
              step="50"
              value={settings.chunk_overlap}
              onChange={(e) =>
                void patchSettings(settings, {
                  chunk_overlap: Number(e.target.value),
                })
                  .then(onRefresh)
                  .catch(onError)
              }
            />
          </label>
        </section>
        <div className="info-note">
          <ShieldCheck size={15} />
          <p>
            {t(
              "Originals stay where they are. Removing a source drops its passages and vectors from the index. Older messages and their citations stay in the history.",
            )}
          </p>
        </div>
      </main>
  );
}
