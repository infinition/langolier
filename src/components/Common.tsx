import { useEffect, useRef, useState, type ReactNode } from "react";
import { mountLangolier } from "../langolier-alive";
import { t } from "../i18n";
import { api, desktop } from "../api";
import { captureShortcut, formatShortcut } from "../shortcut";
import { Library, X, FileText, ArrowUpRight, Copy, Check } from "lucide-react";
import type { Source } from "../types";
/// The creature, alive in the sidebar. It breathes on its own; `busy` makes it
/// swallow documents, which is what ingestion looks like from the outside.
export function Logo({
  small = false,
  busy = false,
}: {
  small?: boolean;
  busy?: boolean;
}) {
  const host = useRef<HTMLSpanElement>(null);
  const alive = useRef<ReturnType<typeof mountLangolier>>(null);
  useEffect(() => {
    alive.current = mountLangolier(host.current);
    const creature = alive.current;
    return () => creature?.destroy();
  }, []);
  useEffect(() => {
    alive.current?.setFeeding(busy);
  }, [busy]);
  return (
    <span
      ref={host}
      className={`logo-symbol logo-alive ${small ? "small" : ""}`}
      onClick={() => alive.current?.feed(9)}
      role="img"
      aria-label="Langolier"
    >
      <span className="langolier__body">
        <img
          className="langolier__image"
          src="/langolier-alive.png"
          alt=""
          draggable={false}
        />
        <span className="eye-glow eye-glow--left" aria-hidden="true" />
        <span className="eye-glow eye-glow--right" aria-hidden="true" />
        <span className="mouth-depth" aria-hidden="true" />
      </span>
      <span className="knowledge-layer" aria-hidden="true">
        <span className="mouth-ingest-flash" />
      </span>
    </span>
  );
}
export function Button({
  children,
  onClick,
  primary = false,
  disabled = false,
  className = "",
  title,
}: {
  children: ReactNode;
  onClick?: () => void;
  primary?: boolean;
  disabled?: boolean;
  className?: string;
  title?: string;
}) {
  return (
    <button
      disabled={disabled}
      className={`button ${primary ? "primary" : ""} ${className}`}
      onClick={onClick}
      title={title}
    >
      {children}
    </button>
  );
}
export function Empty({
  icon: Icon = Library,
  title,
  text,
  action,
}: {
  icon?: typeof Library;
  title: string;
  text: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty">
      <div className="empty-icon">
        <Icon size={26} />
      </div>
      <h3>{title}</h3>
      <p>{text}</p>
      {action}
    </div>
  );
}
export function Modal({
  title,
  children,
  onClose,
  wide = false,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={`modal ${wide ? "wide" : ""}`}
      >
        <div className="modal-header">
          <h2>{title}</h2>
          <button
            aria-label={t("Close")}
            className="icon-button"
            onClick={onClose}
          >
            <X size={20} />
          </button>
        </div>
        {children}
      </section>
    </div>
  );
}
export function SourceButton({
  source,
  index,
  onClick,
}: {
  source: Source;
  index: number;
  onClick: () => void;
}) {
  return (
    <button className="source-chip" onClick={onClick}>
      <span>{index + 1}</span>
      <FileText size={12} />
      <span className="truncate">{source.name}</span>
      <ArrowUpRight size={12} />
    </button>
  );
}

export function PageHeading({
  eyebrow,
  title,
  description,
  action,
}: {
  eyebrow: string;
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="page-heading">
      <div>
        <div className="eyebrow">{eyebrow}</div>
        <h1>{title}</h1>
        <p>{description}</p>
      </div>
      {action}
    </div>
  );
}

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Webviews without the async clipboard: the old selection trick.
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    const ok = document.execCommand("copy");
    area.remove();
    return ok;
  }
}
// Copy button for an answer: shown on hover after a beat, pops on click.
export function CopyButton({
  text,
  className = "",
}: {
  text: string;
  className?: string;
}) {
  const [state, setState] = useState<"idle" | "done" | "failed">("idle");
  return (
    <button
      type="button"
      className={`copy-btn ${state} ${className}`}
      title={t("Copy the answer")}
      aria-label={t("Copy the answer")}
      onClick={async (e) => {
        e.stopPropagation();
        setState((await copyText(text)) ? "done" : "failed");
        setTimeout(() => setState("idle"), 1600);
      }}
    >
      {state === "done" ? <Check size={13} /> : <Copy size={13} />}
      <span>
        {state === "done"
          ? t("Copied")
          : state === "failed"
            ? t("Copy failed")
            : t("Copy")}
      </span>
    </button>
  );
}

// Key recorder for a global shortcut. Takes focus itself, since WebKit does
// not focus a clicked button, and pauses the live shortcut while recording.
export function ShortcutCapture({
  value,
  onChange,
  empty,
}: {
  value: string;
  onChange: (accel: string) => void;
  empty?: string;
}) {
  const [capturing, setCapturing] = useState(false);
  const pause = (paused: boolean) => {
    if (desktop) void api("pause_shortcut", { paused }).catch(() => {});
  };
  return (
    <button
      type="button"
      className={`shortcut-capture${capturing ? " capturing" : ""}`}
      onClick={(e) => {
        e.currentTarget.focus();
        setCapturing(true);
        pause(true);
      }}
      onBlur={() => {
        setCapturing(false);
        pause(false);
      }}
      onKeyDown={(e) => {
        if (!capturing) return;
        e.preventDefault();
        e.stopPropagation();
        if (e.key === "Escape") {
          e.currentTarget.blur();
          return;
        }
        const accel = captureShortcut(e.nativeEvent);
        if (accel) {
          onChange(accel);
          e.currentTarget.blur();
        }
      }}
    >
      {capturing
        ? t("Press the combination…")
        : value
          ? formatShortcut(value)
          : empty || t("None")}
    </button>
  );
}
