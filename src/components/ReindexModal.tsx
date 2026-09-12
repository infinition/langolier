import { useMemo, useState } from "react";
import { t } from "../i18n";
import { RefreshCw, LoaderCircle } from "lucide-react";
import type { Doc } from "../types";
import { Button, Modal } from "./Common";
import LibraryTree, { CATEGORIES, categoryOf } from "./LibraryTree";
import { api, number } from "../api";

// Pick what to reindex: by type, or by folder in the tree.
export default function ReindexModal({
  docs,
  dataDir,
  onClose,
  onError,
  onDone,
}: {
  docs: Doc[];
  dataDir: string;
  onClose: () => void;
  onError: (e: unknown) => void;
  onDone: (message: string) => void;
}) {
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);

  // Types actually present, with their counts.
  const groups = useMemo(() => {
    const byId = new Map<string, string[]>();
    for (const d of docs) {
      const c = categoryOf(d, dataDir);
      byId.set(c, [...(byId.get(c) || []), d.id]);
    }
    return CATEGORIES.filter((c) => byId.has(c.id)).map((c) => ({
      ...c,
      ids: byId.get(c.id)!,
    }));
  }, [docs, dataDir]);

  const setMany = (ids: string[], checked: boolean) =>
    setPicked((s) => {
      const n = new Set(s);
      ids.forEach((id) => (checked ? n.add(id) : n.delete(id)));
      return n;
    });

  async function run() {
    setBusy(true);
    try {
      const ids = [...picked];
      const n = await api<number>("reindex_documents", { ids });
      onDone(
        t("{n} source(s) put back in the queue.", { n: number(n) }) +
          (n < ids.length
            ? t(" {n} still processing, try again after.", {
                n: ids.length - n,
              })
            : t(" The originals must stay reachable where they are.")),
      );
      onClose();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal title={t("Reindex sources")} wide onClose={onClose}>
      <div className="reindex-modal">
        <p className="field-help">
          {t(
            "Reindexing rereads the original files and recomputes passages and vectors. Required after an embedding model change, pointless otherwise.",
          )}
        </p>
        <div>
          <small className="reindex-legend">{t("By type")}</small>
          <div className="reindex-types">
            {groups.map((g) => {
              const taken = g.ids.filter((id) => picked.has(id)).length;
              const Icon = g.icon;
              return (
                <button
                  key={g.id}
                  type="button"
                  className={`reindex-chip${taken === g.ids.length ? " selected" : taken ? " partial" : ""}`}
                  onClick={() => setMany(g.ids, taken !== g.ids.length)}
                >
                  <Icon size={14} /> {g.name} <b>{g.ids.length}</b>
                </button>
              );
            })}
          </div>
        </div>
        <div className="scope-toolbar">
          <strong>
            {t("{n} selected out of {total}", {
              n: number(picked.size),
              total: number(docs.length),
            })}
          </strong>
          <span className="spacer" />
          <Button
            onClick={() =>
              setMany(
                docs.map((d) => d.id),
                true,
              )
            }
          >
            {t("Select all")}
          </Button>
          <Button onClick={() => setPicked(new Set())}>{t("Clear all")}</Button>
        </div>
        <div className="scope-tree">
          <LibraryTree
            docs={docs}
            dataDir={dataDir}
            sort="name"
            selected={picked}
            onSelect={setMany}
          />
        </div>
        <div className="assistant-actions">
          <Button
            primary
            disabled={busy || !picked.size}
            onClick={() => void run()}
          >
            {busy ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <RefreshCw size={15} />
            )}{" "}
            {t("Reindex {n} source(s)", { n: number(picked.size) })}
          </Button>
          <Button onClick={onClose}>{t("Cancel")}</Button>
        </div>
      </div>
    </Modal>
  );
}
