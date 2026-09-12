import { useState } from "react";
import { t } from "../i18n";
import {
  Upload,
  Clipboard,
  Link2,
  Plus,
  FolderOpen,
  Download,
  Video,
  Library,
  ArrowRight,
  LoaderCircle,
} from "lucide-react";
import { Modal, Button } from "./Common";
import { number } from "../api";
export default function ImportModal({
  tab,
  setTab,
  close,
  pickFiles,
  imported,
}: {
  tab: "files" | "text" | "url";
  setTab: (tab: "files" | "text" | "url") => void;
  close: () => void;
  pickFiles: (directory?: boolean) => Promise<void>;
  imported: (cmd: string, args: Record<string, unknown>) => Promise<void>;
}) {
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  async function submit() {
    setBusy(true);
    try {
      await imported(
        tab === "text" ? "import_text" : "import_url",
        tab === "text" ? { title, text } : { url },
      );
    } catch {
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal title={t("A little more material.")} onClose={close}>
      <p className="modal-intro">
        {t("Add what you want to find again, understand and connect.")}
      </p>
      <div className="import-tabs">
        {(
          [
            { id: "files", label: "Fichiers", icon: Upload },
            { id: "text", label: t("Pasted text"), icon: Clipboard },
            { id: "url", label: t("Video link"), icon: Link2 },
          ] as const
        ).map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            className={tab === id ? "selected" : ""}
            onClick={() => setTab(id)}
          >
            <Icon size={16} />
            {label}
          </button>
        ))}
      </div>
      {tab === "files" ? (
        <>
          <button className="import-drop" onClick={() => void pickFiles()}>
            <Upload size={27} />
            <strong>{t("Drop your files here")}</strong>
            <span>ou cliquez pour les choisir sur votre machine</span>
            <small>
              {t("PDF, DOCX, MD, PY, IPYNB, MP4, MP3, SRT and more")}
            </small>
          </button>
          <Button className="full" onClick={() => void pickFiles(true)}>
            <FolderOpen size={16} /> {t("Import a whole folder")}
          </Button>
          <p className="field-help">
            {t(
              "Subfolders included. .git, node_modules, target and Python environments are skipped.",
            )}
          </p>
        </>
      ) : tab === "text" ? (
        <div className="form-stack">
          <label>
            {t("Source title")}
            <input
              autoFocus
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder={t("e.g. Lecture notes on transformers")}
            />
          </label>
          <label>
            {t("Your text")}
            <textarea
              aria-label={t("Your text")}
              rows={10}
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={t("Paste an article, your notes, a code snippet…")}
            />
          </label>
          <div className="form-footer">
            <span>
              {number(text.length)} {t("characters")}
            </span>
            <Button
              primary
              disabled={busy || !text.trim()}
              onClick={() => void submit()}
            >
              {busy ? (
                <LoaderCircle className="spin" size={15} />
              ) : (
                <Plus size={15} />
              )}{" "}
              {t("Add to my memory")}
            </Button>
          </div>
        </div>
      ) : (
        <div className="form-stack">
          <label>
            {t("Video address")}
            <input
              autoFocus
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://www.youtube.com/watch?v=…"
              type="url"
            />
          </label>
          <div className="pipeline-mini">
            <span>
              <Download size={16} />
              {t("Download")}
            </span>
            <ArrowRight size={12} />
            <span>
              <Video size={16} />
              {t("Transcription")}
            </span>
            <ArrowRight size={12} />
            <span>
              <Library size={16} />
              {t("Indexing")}
            </span>
          </div>
          <p className="field-help">
            {t(
              "Language detected automatically. Audio is transcribed with timestamps. Availability depends on the site and on access rights; protected content is not supported.",
            )}
          </p>
          <Button
            primary
            disabled={busy || !url.trim()}
            onClick={() => void submit()}
          >
            {busy ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <Link2 size={15} />
            )}{" "}
            {t("Add this video")}
          </Button>
        </div>
      )}
    </Modal>
  );
}
