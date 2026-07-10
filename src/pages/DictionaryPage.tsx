import { BookOpenText, Plus, Search, ShieldCheck, TextCursorInput, Trash2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "../components/common/Button";
import { native } from "../lib/native";
import type { DictionaryCategory, DictionaryEntry, DictionaryEntryInput } from "../types/models";

const categoryMeta: Record<DictionaryCategory, { label: string; icon: typeof BookOpenText }> = {
  vocabulary: { label: "Vocabulary", icon: BookOpenText },
  replacement: { label: "Replacements", icon: TextCursorInput },
  protected_identifier: { label: "Protected identifiers", icon: ShieldCheck },
};

const blankEntry: DictionaryEntryInput = {
  displayTerm: "",
  spokenForms: [""],
  replacement: null,
  category: "vocabulary",
  priority: 100,
  caseSensitive: false,
  wholeWordOnly: true,
  enabled: true,
};

export function DictionaryPage() {
  const [entries, setEntries] = useState<DictionaryEntry[]>([]);
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<{ id: string | null; value: DictionaryEntryInput } | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const load = () => void native.dictionary().then(setEntries);
  useEffect(load, []);
  const filtered = useMemo(() => entries.filter((entry) => `${entry.displayTerm} ${entry.spokenForms.join(" ")} ${entry.replacement ?? ""}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())), [entries, query]);

  const save = async () => {
    if (!editing || !editing.value.displayTerm.trim() || editing.value.spokenForms.every((form) => !form.trim())) return;
    setSaving(true);
    try {
      await native.saveDictionaryEntry(editing.id, { ...editing.value, spokenForms: editing.value.spokenForms.map((form) => form.trim()).filter(Boolean) });
      setEditing(null);
      setMessage("Dictionary updated");
      load();
      window.setTimeout(() => setMessage(null), 1600);
    } catch (cause) {
      setMessage(cause instanceof Error ? cause.message : "Could not save dictionary entry");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="page">
      <header className="page-header"><div><span className="eyebrow">RECOGNITION MEMORY</span><h1>Developer dictionary</h1><p>Vocabulary hints reach transcription before audio is decoded; deterministic rules protect the final spelling afterward.</p></div><Button variant="primary" icon={<Plus size={16} />} onClick={() => setEditing({ id: null, value: blankEntry })}>Add entry</Button></header>
      <label className="search-field dictionary-search"><Search size={16} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Find a term or spoken form" /></label>
      {message && <p className="inline-feedback">{message}</p>}
      <div className="dictionary-groups">
        {(Object.keys(categoryMeta) as DictionaryCategory[]).map((category) => {
          const meta = categoryMeta[category];
          const Icon = meta.icon;
          const group = filtered.filter((entry) => entry.category === category);
          return <section className="dictionary-card" key={category}><header><span className="icon-well"><Icon size={18} /></span><div><h2>{meta.label}</h2><p>{group.length} enabled terms</p></div></header><div className="term-cloud">{group.map((entry) => <button className="term-chip" key={entry.id} title={`${entry.spokenForms.join(", ")} · click to edit`} onClick={() => setEditing({ id: entry.id, value: { displayTerm: entry.displayTerm, spokenForms: entry.spokenForms, replacement: entry.replacement ?? null, category: entry.category, priority: entry.priority, caseSensitive: entry.caseSensitive, wholeWordOnly: entry.wholeWordOnly, enabled: entry.enabled } })}>{entry.replacement ?? entry.displayTerm}</button>)}</div></section>;
        })}
      </div>
      {editing && <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setEditing(null); }}><section className="dictionary-editor" role="dialog" aria-modal="true" aria-label="Edit dictionary entry"><header><div><span className="eyebrow">DICTIONARY ENTRY</span><h2>{editing.id ? "Edit term" : "Add term"}</h2></div><button className="icon-button" aria-label="Close" onClick={() => setEditing(null)}><X size={17} /></button></header><div className="editor-form"><label className="field"><span>Written term</span><input value={editing.value.displayTerm} onChange={(event) => setEditing({ ...editing, value: { ...editing.value, displayTerm: event.target.value } })} autoFocus /></label><label className="field"><span>Spoken forms</span><input value={editing.value.spokenForms.join(", ")} onChange={(event) => setEditing({ ...editing, value: { ...editing.value, spokenForms: event.target.value.split(",") } })} placeholder="spoken form, alternate form" /></label><label className="field"><span>Category</span><select value={editing.value.category} onChange={(event) => setEditing({ ...editing, value: { ...editing.value, category: event.target.value as DictionaryCategory } })}>{Object.entries(categoryMeta).map(([id, item]) => <option value={id} key={id}>{item.label}</option>)}</select></label>{editing.value.category === "replacement" && <label className="field"><span>Replacement text</span><input value={editing.value.replacement ?? ""} onChange={(event) => setEditing({ ...editing, value: { ...editing.value, replacement: event.target.value } })} /></label>}</div><div className="editor-footer">{editing.id && !editing.id.startsWith("builtin:") && <Button variant="ghost" icon={<Trash2 size={14} />} onClick={() => void native.deleteDictionaryEntry(editing.id!).then(() => { setEditing(null); load(); })}>Delete</Button>}<span /><Button variant="ghost" onClick={() => setEditing(null)}>Cancel</Button><Button variant="primary" disabled={saving} onClick={() => void save()}>{saving ? "Saving…" : "Save entry"}</Button></div></section></div>}
    </div>
  );
}
