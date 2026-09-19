import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
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

// Backend messages are English; translate the ones we know, pass the rest
// through. Some carry a count, which would make the sentence impossible to
// look up, so the numbers are lifted out and put back after the lookup.
export function message(e: unknown): string {
  const text = e instanceof Error ? e.message : String(e);
  const counted = text.match(/^(.*?)(\d+)\/(\d+)(.*)$/s);
  if (!counted) return t(text);
  return t(`${counted[1]}{done}/{total}${counted[4]}`, {
    done: counted[2],
    total: counted[3],
  });
}

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
  // The palette window lives on, hidden, so it must follow a switch made in
  // the main window: storage events cover shared storage, focus covers the rest.
  useEffect(() => {
    const resync = () => {
      const saved = initial();
      if (saved !== current) {
        current = saved;
        document.documentElement.lang = saved;
        set(saved);
      }
    };
    window.addEventListener("storage", resync);
    window.addEventListener("focus", resync);
    return () => {
      window.removeEventListener("storage", resync);
      window.removeEventListener("focus", resync);
    };
  }, []);
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
