import type { ReactNode } from "react";
import { t } from "../i18n";
import { Library, X, FileText, ArrowUpRight } from "lucide-react";
import type { Source } from "../types";
export function Logo({ small = false }: { small?: boolean }) {
  return (
    <img
      className={`logo-symbol ${small ? "small" : ""}`}
      src="/icon.png"
      alt=""
      draggable={false}
    />
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
