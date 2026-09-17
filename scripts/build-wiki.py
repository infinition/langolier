#!/usr/bin/env python3
"""Turns wiki/*.md into docs/wiki.html, one page with its own navigation.

The markdown in wiki/ stays the single source: it feeds the GitHub wiki through
AcidWiki and this page through here. Run it after editing any wiki file.
"""
import html
import io
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WIKI = ROOT / "wiki"
OUT = ROOT / "docs" / "wiki.html"

# Order of the sections, and the order of pages inside them.
ORDER = [
    ("", ["Home"]),
    ("getting-started", ["Installation", "First-Steps", "Engines"]),
    ("guides", ["Assistants", "Watches", "Telegram", "Local-API", "Export"]),
    ("reference", ["Settings", "Retrieval"]),
]
SECTION_TITLES = {
    "": "",
    "getting-started": "Getting started",
    "guides": "Guides",
    "reference": "Reference",
}


def inline(text):
    """Bold, italics, code and links, in that order so code wins."""
    out = []
    for i, part in enumerate(re.split(r"(`[^`]+`)", text)):
        if i % 2:
            out.append(f"<code>{html.escape(part[1:-1])}</code>")
            continue
        part = html.escape(part)
        part = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", part)
        part = re.sub(
            r"\[([^\]]+)\]\(([^)]+)\)",
            lambda m: f'<a href="{link(m.group(2))}">{m.group(1)}</a>',
            part,
        )
        out.append(part)
    return "".join(out)


def link(target):
    """A wiki link points at an anchor here; anything else is left alone."""
    if target.startswith(("http://", "https://", "#", "mailto:")):
        return target
    return "#" + slug(target)


def slug(name):
    return name.strip().lower().replace(" ", "-").replace("_", "-")


def render(md):
    """Enough markdown for what the wiki actually uses."""
    lines = md.split("\n")
    out, i = [], 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("```"):
            i += 1
            block = []
            while i < len(lines) and not lines[i].startswith("```"):
                block.append(html.escape(lines[i]))
                i += 1
            out.append("<pre>" + "\n".join(block) + "</pre>")
        elif line.startswith("|") and i + 1 < len(lines) and set(lines[i + 1].replace("|", "").strip()) <= set("-: "):
            head = [c.strip() for c in line.strip("|").split("|")]
            i += 2
            rows = []
            while i < len(lines) and lines[i].startswith("|"):
                rows.append([c.strip() for c in lines[i].strip("|").split("|")])
                i += 1
            i -= 1
            cells = "".join(f"<th>{inline(c)}</th>" for c in head)
            body = "".join(
                "<tr>" + "".join(f"<td>{inline(c)}</td>" for c in r) + "</tr>" for r in rows
            )
            out.append(f'<div class="tablewrap"><table><thead><tr>{cells}</tr></thead><tbody>{body}</tbody></table></div>')
        elif re.match(r"^#{1,4} ", line):
            level = len(line) - len(line.lstrip("#"))
            out.append(f"<h{level + 1}>{inline(line[level:].strip())}</h{level + 1}>")
        elif re.match(r"^[-*] ", line):
            items = []
            while i < len(lines) and re.match(r"^[-*] ", lines[i]):
                items.append(f"<li>{inline(lines[i][2:])}</li>")
                i += 1
            i -= 1
            out.append("<ul>" + "".join(items) + "</ul>")
        elif re.match(r"^\d+\. ", line):
            items = []
            while i < len(lines) and re.match(r"^\d+\. ", lines[i]):
                items.append(f"<li>{inline(re.sub(r'^\d+\. ', '', lines[i]))}</li>")
                i += 1
            i -= 1
            out.append("<ol>" + "".join(items) + "</ol>")
        elif line.strip():
            para = [line]
            while i + 1 < len(lines) and lines[i + 1].strip() and not re.match(r"^([-*#>|]|\d+\.|```)", lines[i + 1]):
                i += 1
                para.append(lines[i])
            out.append("<p>" + inline(" ".join(para)) + "</p>")
        i += 1
    return "\n".join(out)


def main():
    pages, nav = [], []
    for folder, names in ORDER:
        entries = []
        for name in names:
            path = (WIKI / folder / f"{name}.md") if folder else (WIKI / f"{name}.md")
            if not path.exists():
                raise SystemExit(f"missing wiki page: {path}")
            md = io.open(path, encoding="utf-8").read()
            title = md.split("\n", 1)[0].lstrip("# ").strip()
            body = render(md.split("\n", 1)[1])
            pages.append(f'<article id="{slug(name)}" class="wiki-page"><h1>{html.escape(title)}</h1>\n{body}</article>')
            entries.append(f'<a href="#{slug(name)}">{html.escape(title)}</a>')
        label = SECTION_TITLES[folder]
        nav.append(
            (f'<span class="nav-label">{label}</span>' if label else "") + "".join(entries)
        )

    template = io.open(ROOT / "docs" / "index.html", encoding="utf-8").read()
    style = template[template.index("<style>") : template.index("</style>") + 8]
    head = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover, interactive-widget=resizes-content">
<title>Langolier Wiki</title>
<meta name="description" content="How to install Langolier, feed it, build assistants, watch folders, bridge Telegram and query it from Shortcuts or Siri.">
<meta name="color-scheme" content="dark">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; connect-src 'self'; font-src 'self'; base-uri 'none'; form-action 'none'">
<meta name="referrer" content="strict-origin-when-cross-origin">
<meta name="theme-color" content="#121213">
<link rel="icon" type="image/png" href="langolier.png">
<link rel="apple-touch-icon" href="langolier.png">
{style}
<style>
/* Two columns on a desktop, one on a phone. Every page is in the document,
   the anchors just move you through it. */
.wiki-shell {{ display: grid; grid-template-columns: 236px minmax(0, 1fr); gap: 44px; align-items: start; padding-block: 34px; }}
.wiki-nav {{ position: sticky; top: 112px; display: flex; flex-direction: column; gap: 2px; font-size: 13.5px; }}
.wiki-nav a {{ color: var(--muted); padding: 5px 10px; border-radius: 8px; }}
.wiki-nav a:hover {{ background: rgba(230,160,107,.08); color: var(--lime); text-decoration: none; }}
.wiki-nav .nav-label {{ font-size: 10px; letter-spacing: .14em; text-transform: uppercase; color: var(--dim); margin: 16px 0 5px 10px; }}
.wiki-nav .nav-label:first-child {{ margin-top: 0; }}
.wiki-page {{ scroll-margin-top: 116px; padding-bottom: 42px; border-bottom: 1px solid var(--line); margin-bottom: 42px; }}
.wiki-page:last-child {{ border-bottom: 0; }}
.wiki-page h1 {{ font-size: clamp(26px, 3.6vw, 34px); margin-bottom: 18px; }}
.wiki-page h2 {{ font-size: 21px; margin: 32px 0 12px; }}
.wiki-page h3 {{ font-size: 16px; margin: 24px 0 10px; color: var(--cream); }}
.wiki-page p, .wiki-page li {{ color: var(--muted); }}
.wiki-page ul, .wiki-page ol {{ padding-left: 20px; margin: 0 0 1em; }}
.wiki-page li {{ margin-bottom: 5px; }}
.wiki-page .tablewrap {{ margin-bottom: 1em; }}
.wiki-page strong {{ color: var(--cream); }}
.wiki-page code {{ color: var(--lime); }}
@media (max-width: 900px) {{
  .wiki-shell {{ grid-template-columns: minmax(0, 1fr); gap: 20px; }}
  .wiki-nav {{ position: static; flex-direction: row; flex-wrap: wrap; border: 1px solid var(--line); border-radius: 12px; padding: 12px; }}
  .wiki-nav .nav-label {{ flex-basis: 100%; margin: 8px 0 2px 4px; }}
  .wiki-nav .nav-label:first-child {{ margin-top: 0; }}
}}
</style>
</head>
<body>
<header class="topbar">
  <div class="shell">
    <a class="brand" href="index.html">
      <img class="brand-mark" src="langolier.png" alt="">
      <span>langolier<span class="dot">.</span></span>
    </a>
    <nav class="tabs">
      <a href="index.html">Home</a>
      <a href="index.html#install">Install</a>
      <a href="https://github.com/infinition/langolier">GitHub</a>
    </nav>
  </div>
</header>
<main class="shell wiki-shell">
  <nav class="wiki-nav">{"".join(nav)}</nav>
  <div>
{chr(10).join(pages)}
  </div>
</main>
<footer>
  <div class="shell">
    <span>Langolier · MPL-2.0 · <a href="https://github.com/infinition/langolier">github.com/infinition/langolier</a></span>
    <a class="support" href="https://www.buymeacoffee.com/infinition">Support the project</a>
  </div>
</footer>
</body>
</html>
"""
    io.open(OUT, "w", encoding="utf-8").write(head)
    print(f"{OUT.relative_to(ROOT)}: {len(pages)} pages, {len(head) // 1024} Ko")


if __name__ == "__main__":
    main()
