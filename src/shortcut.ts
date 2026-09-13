import { t } from "./i18n";
// Global shortcuts: Tauri accelerator to readable form and back.
const MAC = navigator.platform.toUpperCase().includes("MAC");
const LABELS: Record<string, string> = {
  CommandOrControl: MAC ? "⌘" : "Ctrl",
  Command: "⌘",
  Super: MAC ? "⌘" : "Win",
  Control: MAC ? "⌃" : "Ctrl",
  Alt: MAC ? "⌥" : "Alt",
  Shift: MAC ? "⇧" : "Maj",
  Space: "Espace",
  Enter: t("Enter"),
  Escape: t("Esc"),
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};
export function formatShortcut(accel: string): string {
  if (!accel) return t("None");
  return accel
    .split("+")
    .map((part) => {
      if (LABELS[part]) return LABELS[part];
      if (part.startsWith("Key")) return part.slice(3);
      if (part.startsWith("Digit")) return part.slice(5);
      return part;
    })
    .join(MAC ? "" : "+");
}
/// Builds the accelerator from a keydown.
export function captureShortcut(e: KeyboardEvent): string | null {
  if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return null;
  const mods: string[] = [];
  if (e.metaKey) mods.push(MAC ? "Command" : "Super");
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (!mods.length) return "";
  const code = e.code;
  const ok =
    /^(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-9]|2[0-4])|Space|Enter|Tab|Escape|Backspace|Delete|Home|End|PageUp|PageDown|Arrow(Up|Down|Left|Right)|Minus|Equal|Comma|Period|Slash|Semicolon|Quote|Backquote|BracketLeft|BracketRight|Backslash)$/.test(
      code,
    );
  if (!ok) return "";
  return [...mods, code].join("+");
}
