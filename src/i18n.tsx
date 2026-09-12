import { createContext, useContext, useState, type ReactNode } from "react";
import { FR } from "./locales/fr";

export type Lang = "en" | "fr";
const KEY = "langolier.lang";

function initial(): Lang {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === "en" || saved === "fr") return saved;
  } catch {
    /* storage blocked */
  }
  return navigator.language?.toLowerCase().startsWith("fr") ? "fr" : "en";
}

let current: Lang = initial();
export const getLang = () => current;

// English source text is the key
export function t(s: string, vars?: Record<string, string | number>): string {
  let out = current === "fr" ? FR[s] || s : s;
  if (vars)
    for (const [k, v] of Object.entries(vars))
      out = out.split(`{${k}}`).join(String(v));
  return out;
}

// Backend messages are English; translate the ones we know, pass the rest through
export const message = (e: unknown) =>
  t(e instanceof Error ? e.message : String(e));

const Ctx = createContext<{ lang: Lang; setLang: (l: Lang) => void }>({
  lang: "en",
  setLang: () => {},
});
export const useLang = () => useContext(Ctx);

// Switching remounts the tree, so plain t() calls need no hook
export function LangProvider({
  children,
}: {
  children: (lang: Lang) => ReactNode;
}) {
  const [lang, set] = useState<Lang>(current);
  const setLang = (l: Lang) => {
    current = l;
    document.documentElement.lang = l;
    try {
      localStorage.setItem(KEY, l);
    } catch {
      /* nothing to persist to */
    }
    set(l);
  };
  return (
    <Ctx.Provider value={{ lang, setLang }}>{children(lang)}</Ctx.Provider>
  );
}
