import { useEffect, useState } from "react";
import { t } from "../i18n";
import { KeyRound, Save, Trash2, Check, X } from "lucide-react";
import { Button } from "./Common";
import { api, desktop, errorText } from "./../api";

export interface SavedKey {
  id: string;
  label: string;
  provider: string;
  hint: string;
}

// API key field backed by the vault.
export default function ApiKeyField({
  value,
  onChange,
  provider,
  placeholder,
  label = t("API key"),
  only,
}: {
  value: string;
  onChange: (key: string) => void;
  provider: string;
  placeholder?: string;
  label?: string;
  /// Shows only this provider's entries.
  only?: string;
}) {
  const [keys, setKeys] = useState<SavedKey[]>([]);
  const [picked, setPicked] = useState("");
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState("");
  const [note, setNote] = useState("");

  const reload = () =>
    desktop
      ? api<SavedKey[]>("api_keys")
          .then((all) =>
            setKeys(only ? all.filter((k) => k.provider === only) : all),
          )
          .catch(() => {})
      : Promise.resolve();
  useEffect(() => {
    void reload();
  }, []);

  async function pick(id: string) {
    setPicked(id);
    setNote("");
    if (!id) return;
    try {
      onChange(await api<string>("use_api_key", { id }));
      const k = keys.find((x) => x.id === id);
      setNote(t('Key "{label}" is in place.', { label: k?.label || "" }));
    } catch (e) {
      setNote(errorText(e));
    }
  }
  async function save() {
    try {
      const r = await api<{ label: string; hint: string }>("save_api_key", {
        label: name,
        provider,
        key: value,
      });
      setNaming(false);
      setName("");
      await reload();
      setNote(
        t('Saved as "{label}" ({hint}).', { label: r.label, hint: r.hint }),
      );
    } catch (e) {
      setNote(errorText(e));
    }
  }
  async function remove() {
    const k = keys.find((x) => x.id === picked);
    if (
      !k ||
      !window.confirm(t('Remove "{label}" from the vault?', { label: k.label }))
    )
      return;
    try {
      await api("delete_api_key", { id: picked });
      setPicked("");
      await reload();
      setNote(t('"{label}" removed from the vault.', { label: k.label }));
    } catch (e) {
      setNote(errorText(e));
    }
  }

  return (
    <label className="api-key-field">
      {label}
      <div className="api-key-row">
        <KeyRound size={14} />
        <select
          value={picked}
          onChange={(e) => void pick(e.target.value)}
          aria-label={t("Saved key")}
        >
          <option value="">
            {keys.length ? t("Pick a saved entry…") : t("Vault empty")}
          </option>
          {keys.map((k) => (
            <option key={k.id} value={k.id}>
              {k.label} · {k.hint}
              {k.provider ? ` · ${k.provider}` : ""}
            </option>
          ))}
        </select>
        {picked && (
          <button
            type="button"
            title={t("Remove from the vault")}
            onClick={() => void remove()}
          >
            <Trash2 size={14} />
          </button>
        )}
      </div>
      <div className="api-key-row">
        <input
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={value}
          onChange={(e) => {
            onChange(e.target.value);
            setPicked("");
          }}
          placeholder={placeholder || t("Paste a key…")}
        />
        {naming ? (
          <>
            <input
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t("Key name")}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void save();
                }
                if (e.key === "Escape") setNaming(false);
              }}
            />
            <button
              type="button"
              title={t("Save")}
              disabled={!name.trim()}
              onClick={() => void save()}
            >
              <Check size={14} />
            </button>
            <button
              type="button"
              title={t("Cancel")}
              onClick={() => setNaming(false)}
            >
              <X size={14} />
            </button>
          </>
        ) : (
          <Button disabled={!value.trim()} onClick={() => setNaming(true)}>
            <Save size={14} /> {t("Save as…")}
          </Button>
        )}
      </div>
      <small className="muted">
        {note ||
          t(
            "The vault saves you retyping keys. They sit in Langolier's local database in clear text, like the rest of your settings. Protect your session.",
          )}
      </small>
    </label>
  );
}
