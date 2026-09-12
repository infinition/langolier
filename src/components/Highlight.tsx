import { Fragment } from "react";

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/// Terms to highlight for a query.
export function termsOf(query: string, exact: boolean): string[] {
  const q = query.trim();
  if (!q) return [];
  if (exact) return [q];
  return [...new Set(q.split(/[^\p{L}\p{N}_-]+/u).filter((w) => w.length > 2))];
}
export function countMatches(text: string, terms: string[]): number {
  if (!terms.length) return 0;
  const re = new RegExp(terms.map(escape).join("|"), "giu");
  return (text.match(re) || []).length;
}

// Text with matches wrapped in <mark>.
export default function Highlight({
  text,
  terms,
  offset = 0,
  current,
}: {
  text: string;
  terms: string[];
  offset?: number;
  current?: number;
}) {
  if (!terms.length) return <>{text}</>;
  const re = new RegExp(`(${terms.map(escape).join("|")})`, "giu");
  const parts = text.split(re);
  let hit = offset;
  return (
    <>
      {parts.map((part, i) =>
        i % 2 === 1 ? (
          <mark
            key={i}
            data-hit={hit}
            className={hit++ === current ? "current" : undefined}
          >
            {part}
          </mark>
        ) : (
          <Fragment key={i}>{part}</Fragment>
        ),
      )}
    </>
  );
}
