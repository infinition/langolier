import { useMemo, useState } from "react";
import { t } from "../i18n";
import {
  ChevronRight,
  ChevronDown,
  Folder,
  FolderOpen,
  Video,
  Clipboard,
  FileText,
  Code2,
  Table,
  BookOpen,
  Captions,
  File,
  Trash2,
  Eye,
} from "lucide-react";
import type { Doc } from "../types";
import { locale } from "../api";

const MEDIA = [
  "mp4",
  "mkv",
  "mov",
  "webm",
  "avi",
  "m4v",
  "mp3",
  "wav",
  "m4a",
  "flac",
  "ogg",
  "url",
];
const CODE = [
  "py",
  "rs",
  "js",
  "ts",
  "tsx",
  "jsx",
  "r",
  "jl",
  "sql",
  "css",
  "toml",
  "yaml",
  "yml",
  "html",
];
const DATA = ["json", "csv"];
const NOTES = ["md", "txt"];

interface Category {
  id: string;
  name: string;
  icon: typeof Folder;
}
export const CATEGORIES: Category[] = [
  { id: "media", name: t("Videos & transcripts"), icon: Video },
  { id: "paste", name: "Presse-papiers", icon: Clipboard },
  { id: "notes", name: "Notes", icon: FileText },
  { id: "pdf", name: t("PDF documents"), icon: BookOpen },
  { id: "notebook", name: "Notebooks", icon: BookOpen },
  { id: "code", name: t("Code & configuration"), icon: Code2 },
  { id: "data", name: t("Data"), icon: Table },
  { id: "subs", name: "Sous-titres", icon: Captions },
  { id: "other", name: "Autres", icon: File },
];
export function categoryOf(d: Doc, dataDir: string): string {
  if (
    dataDir &&
    d.source.startsWith(`${dataDir}/media/`) &&
    NOTES.includes(d.kind)
  )
    return "paste";
  if (MEDIA.includes(d.kind)) return "media";
  if (NOTES.includes(d.kind)) return "notes";
  if (d.kind === "pdf") return "pdf";
  if (d.kind === "ipynb") return "notebook";
  if (CODE.includes(d.kind)) return "code";
  if (DATA.includes(d.kind)) return "data";
  if (["srt", "vtt"].includes(d.kind)) return "subs";
  return "other";
}

interface Node {
  key: string;
  name: string;
  folders: Map<string, Node>;
  docs: Doc[];
}
function newNode(key: string, name: string): Node {
  return { key, name, folders: new Map(), docs: [] };
}
/// A source's folder segments, without the file name.
function segments(d: Doc, category: string): string[] {
  if (category === "media" && d.kind === "url") return [];
  if (category === "paste") return [];
  if (!d.source.startsWith("/")) return [];
  const parts = d.source.split("/").filter(Boolean);
  return parts.slice(0, -1);
}
/// Common path prefix of a category, trimmed so the tree starts at the first branch.
function commonPrefix(paths: string[][]): number {
  if (!paths.length) return 0;
  let n = 0;
  while (paths.every((p) => p.length > n && p[n] === paths[0][n])) n++;
  return n;
}
function build(
  docs: Doc[],
  dataDir: string,
): { category: Category; root: Node; prefix: string }[] {
  const out: { category: Category; root: Node; prefix: string }[] = [];
  for (const category of CATEGORIES) {
    const mine = docs.filter((d) => categoryOf(d, dataDir) === category.id);
    if (!mine.length) continue;
    const segs = mine.map((d) => segments(d, category.id));
    const withPath = segs.filter((s) => s.length);
    const cut =
      withPath.length > 1
        ? commonPrefix(withPath)
        : withPath.length === 1
          ? withPath[0].length
          : 0;
    const prefix = withPath.length
      ? "/" + withPath[0].slice(0, cut).join("/")
      : "";
    const root = newNode(category.id, category.name);
    mine.forEach((d, i) => {
      let node = root;
      for (const s of segs[i].slice(cut)) {
        let child = node.folders.get(s);
        if (!child) {
          child = newNode(`${node.key}/${s}`, s);
          node.folders.set(s, child);
        }
        node = child;
      }
      node.docs.push(d);
    });
    out.push({ category, root, prefix });
  }
  return out;
}
/// Ids of every source under a node, subfolders included.
function idsOf(n: Node): string[] {
  const out = n.docs.map((d) => d.id);
  n.folders.forEach((f) => out.push(...idsOf(f)));
  return out;
}

// Tree view of the sources: category, folders, files.
export default function LibraryTree({
  docs,
  dataDir,
  sort,
  onOpen,
  selected,
  onSelect,
  onDelete,
  onPreview,
}: {
  docs: Doc[];
  dataDir: string;
  sort: "name" | "date";
  onOpen?: (d: Doc) => void;
  selected?: Set<string>;
  onSelect?: (ids: string[], checked: boolean) => void;
  onDelete?: (ids: string[], label: string) => void;
  onPreview?: (d: Doc) => void;
}) {
  const [closed, setClosed] = useState<Set<string>>(new Set());
  const tree = useMemo(() => build(docs, dataDir), [docs, dataDir]);
  const toggle = (key: string) =>
    setClosed((s) => {
      const n = new Set(s);
      if (n.has(key)) n.delete(key);
      else n.add(key);
      return n;
    });
  const sortDocs = (list: Doc[]) =>
    [...list].sort((a, b) =>
      sort === "date"
        ? b.created - a.created
        : a.name.localeCompare(b.name, "fr", { sensitivity: "base" }),
    );
  const sortFolders = (m: Map<string, Node>) =>
    [...m.values()].sort((a, b) =>
      a.name.localeCompare(b.name, "fr", { sensitivity: "base" }),
    );

  /// Folder checkbox: checked when all is taken, indeterminate when partial.
  function GroupCheckbox({ node, label }: { node: Node; label: string }) {
    if (!selected || !onSelect) return null;
    const ids = idsOf(node);
    const taken = ids.filter((id) => selected.has(id)).length;
    const all = taken === ids.length && ids.length > 0;
    return (
      <input
        type="checkbox"
        aria-label={t("Select everything in {label}", { label })}
        checked={all}
        ref={(el) => {
          if (el) el.indeterminate = taken > 0 && !all;
        }}
        onClick={(e) => e.stopPropagation()}
        onChange={(e) => onSelect(ids, e.target.checked)}
      />
    );
  }
  function Trash({ ids, label }: { ids: string[]; label: string }) {
    if (!onDelete || !ids.length) return null;
    return (
      <button
        className="tree-trash"
        title={
          ids.length > 1
            ? t('Remove the {n} sources under "{label}"', {
                n: ids.length,
                label,
              })
            : t("Remove from the index")
        }
        onClick={(e) => {
          e.stopPropagation();
          onDelete(ids, label);
        }}
      >
        <Trash2 size={13} />
      </button>
    );
  }
  function DocRow({ d, depth }: { d: Doc; depth: number }) {
    return (
      <li>
        <div
          className={`tree-row tree-doc ${d.status}`}
          style={{ paddingLeft: 10 + depth * 18 + 13 }}
        >
          {selected && onSelect && (
            <input
              type="checkbox"
              aria-label={d.name}
              checked={selected.has(d.id)}
              onChange={(e) => onSelect([d.id], e.target.checked)}
            />
          )}
          <button
            className="tree-label"
            onClick={() =>
              onOpen ? onOpen(d) : onSelect?.([d.id], !selected?.has(d.id))
            }
            title={`${d.source}\n${new Date(d.created * 1000).toLocaleString(locale())}`}
          >
            <span
              className={`status-dot ${d.status === "ready" ? "online" : d.status === "error" ? "warning" : "pulse"}`}
            />
            <span className="truncate">{d.name}</span>
          </button>
          <small>
            {d.chunks ? `${d.chunks} p.` : t(d.stage)}
            {" · "}
            {new Date(d.created * 1000).toLocaleDateString(locale())}
          </small>
          {onPreview && (
            <button
              className="tree-eye"
              title={t("View indexed content")}
              onClick={(e) => {
                e.stopPropagation();
                onPreview(d);
              }}
            >
              <Eye size={13} />
            </button>
          )}
          <Trash ids={[d.id]} label={d.name} />
        </div>
      </li>
    );
  }
  function FolderRow({
    node,
    depth,
    icon,
    className = "",
    suffix,
  }: {
    node: Node;
    depth: number;
    icon: React.ReactNode;
    className?: string;
    suffix?: React.ReactNode;
  }) {
    const isClosed = closed.has(node.key);
    const ids = idsOf(node);
    return (
      <>
        <div
          className={`tree-row ${className}`}
          style={{ paddingLeft: 10 + depth * 18 }}
        >
          <GroupCheckbox node={node} label={node.name} />
          <button className="tree-label" onClick={() => toggle(node.key)}>
            {isClosed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
            {icon}
            <span className="truncate">{node.name}</span>
          </button>
          <small>{ids.length}</small>
          {suffix}
          <Trash ids={ids} label={node.name} />
        </div>
        {!isClosed && (
          <ul>
            {sortFolders(node.folders).map((f) => (
              <li key={f.key} className="tree-folder">
                <FolderRow
                  node={f}
                  depth={depth + 1}
                  icon={
                    closed.has(f.key) ? (
                      <Folder size={14} />
                    ) : (
                      <FolderOpen size={14} />
                    )
                  }
                />
              </li>
            ))}
            {sortDocs(node.docs).map((d) => (
              <DocRow key={d.id} d={d} depth={depth + 1} />
            ))}
          </ul>
        )}
      </>
    );
  }

  if (!tree.length) return null;
  return (
    <ul className="tree">
      {tree.map(({ category, root, prefix }) => {
        const Icon = category.icon;
        return (
          <li key={root.key} className="tree-category">
            <FolderRow
              node={root}
              depth={0}
              className="tree-category-row"
              icon={<Icon size={15} className="lime" />}
              suffix={
                prefix ? (
                  <code className="tree-prefix">{prefix}</code>
                ) : undefined
              }
            />
          </li>
        );
      })}
    </ul>
  );
}
