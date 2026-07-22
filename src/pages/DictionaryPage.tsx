import { Download, Pencil, Plus, Search, Trash2, Upload } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "../components/common/Button";
import { ContentDialog } from "../components/fluent/ContentDialog";
import { InfoBar } from "../components/fluent/InfoBar";
import { ToggleSwitch } from "../components/fluent/ToggleSwitch";
import { native } from "../lib/native";
import type { DictionaryCategory, DictionaryEntry, DictionaryEntryInput } from "../types/models";

const TABS: Array<{ id: DictionaryCategory; label: string; blurb: string }> = [
  { id: "vocabulary", label: "Vocabulary", blurb: "Terms sent to Deepgram as keyterm hints so they transcribe correctly." },
  { id: "replacement", label: "Replacements", blurb: "Spoken phrases rewritten to their intended form after transcription." },
  {
    id: "protected_identifier",
    label: "Protected identifiers",
    blurb: "Names that AI cleanup must reproduce verbatim and never rewrite.",
  },
];

const blankEntry = (category: DictionaryCategory): DictionaryEntryInput => ({
  displayTerm: "",
  spokenForms: [""],
  replacement: null,
  category,
  priority: 100,
  caseSensitive: false,
  wholeWordOnly: true,
  enabled: true,
});

export function DictionaryPage() {
  const [entries, setEntries] = useState<DictionaryEntry[]>([]);
  const [tab, setTab] = useState<DictionaryCategory>("vocabulary");
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<{ id: string | null; value: DictionaryEntryInput } | null>(null);
  const [deleting, setDeleting] = useState<DictionaryEntry | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const importInput = useRef<HTMLInputElement>(null);

  const load = () => void native.dictionary().then(setEntries);
  useEffect(load, []);

  const visible = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    return entries
      .filter((entry) => entry.category === tab)
      .filter((entry) =>
        `${entry.displayTerm} ${entry.spokenForms.join(" ")} ${entry.replacement ?? ""}`
          .toLocaleLowerCase()
          .includes(needle),
      );
  }, [entries, tab, query]);

  const active = TABS.find((entry) => entry.id === tab) ?? TABS[0];

  const save = async () => {
    if (!editing) return;
    const value = editing.value;
    if (!value.displayTerm.trim() || value.spokenForms.every((form) => !form.trim())) {
      setError("A written term and at least one spoken form are required.");
      return;
    }
    setSaving(true);
    try {
      await native.saveDictionaryEntry(editing.id, {
        ...value,
        spokenForms: value.spokenForms.map((form) => form.trim()).filter(Boolean),
      });
      setEditing(null);
      setError(null);
      load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not save the dictionary entry.");
    } finally {
      setSaving(false);
    }
  };

  /** Exports the visible category as JSON via a browser download. */
  const exportEntries = () => {
    const payload = JSON.stringify(
      entries.map(({ id: _id, usageCount: _usage, createdAt: _created, updatedAt: _updated, ...rest }) => rest),
      null,
      2,
    );
    const url = URL.createObjectURL(new Blob([payload], { type: "application/json" }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "voiceflow-dictionary.json";
    anchor.click();
    URL.revokeObjectURL(url);
  };

  const importEntries = async (file: File) => {
    try {
      const parsed = JSON.parse(await file.text());
      if (!Array.isArray(parsed)) throw new Error("The file must contain a list of entries.");
      // Imported one at a time so a single malformed entry does not discard the rest.
      let failures = 0;
      for (const candidate of parsed as DictionaryEntryInput[]) {
        try {
          await native.saveDictionaryEntry(null, candidate);
        } catch {
          failures += 1;
        }
      }
      setError(failures > 0 ? `${failures} entries could not be imported.` : null);
      load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not read the file.");
    }
  };

  return (
    <div className="page">
      <header className="page-header">
        <h1>Dictionary</h1>
        <p className="page-header__meta">
          <span>Vocabulary, replacements, and protected identifiers used during cleanup.</span>
        </p>
      </header>

      {error && <InfoBar severity="error" title="Dictionary" message={error} />}

      <div className="segmented" role="tablist" aria-label="Dictionary category">
        {TABS.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="tab"
            aria-selected={tab === entry.id}
            className={`segmented__item${tab === entry.id ? " segmented__item--selected" : ""}`}
            onClick={() => setTab(entry.id)}
          >
            {entry.label}
          </button>
        ))}
      </div>
      <p className="settings-group__description mode-explanation">{active.blurb}</p>

      <div className="command-bar">
        <label className="search-field">
          <Search size={15} aria-hidden="true" />
          <input
            value={query}
            aria-label="Search the dictionary"
            placeholder={`Search ${active.label.toLowerCase()}`}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <Button variant="primary" icon={<Plus size={15} />} onClick={() => setEditing({ id: null, value: blankEntry(tab) })}>
          Add
        </Button>
        <Button variant="secondary" icon={<Upload size={15} />} onClick={() => importInput.current?.click()}>
          Import
        </Button>
        <Button variant="secondary" icon={<Download size={15} />} onClick={exportEntries}>
          Export
        </Button>
        <input
          ref={importInput}
          type="file"
          accept="application/json"
          className="visually-hidden"
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) void importEntries(file);
            event.target.value = "";
          }}
        />
        <small className="command-bar__count">
          {visible.length} {visible.length === 1 ? "entry" : "entries"}
        </small>
      </div>

      {visible.length === 0 ? (
        <div className="empty-state">
          <strong>Nothing here yet</strong>
          <p>Add a term so VoiceFlow recognises it during transcription.</p>
        </div>
      ) : (
        <div className="settings-group__rows">
          {visible.map((entry) => (
            <div className="settings-row" key={entry.id}>
              <span className="settings-row__text">
                <strong>{entry.displayTerm}</strong>
                <small>
                  {tab === "replacement" && entry.replacement
                    ? `${entry.spokenForms.join(", ")} → ${entry.replacement}`
                    : entry.spokenForms.join(", ")}
                </small>
              </span>
              <span className="settings-row__action">
                {entry.id.startsWith("builtin:") && <span className="settings-row__value">Built-in</span>}
                <button
                  type="button"
                  className="icon-button"
                  aria-label={`Edit ${entry.displayTerm}`}
                  onClick={() =>
                    setEditing({
                      id: entry.id,
                      value: {
                        displayTerm: entry.displayTerm,
                        spokenForms: entry.spokenForms.length > 0 ? entry.spokenForms : [""],
                        replacement: entry.replacement ?? null,
                        category: entry.category,
                        priority: entry.priority,
                        caseSensitive: entry.caseSensitive,
                        wholeWordOnly: entry.wholeWordOnly,
                        enabled: entry.enabled,
                      },
                    })
                  }
                >
                  <Pencil size={15} />
                </button>
                <button
                  type="button"
                  className="icon-button"
                  aria-label={`Delete ${entry.displayTerm}`}
                  disabled={entry.id.startsWith("builtin:")}
                  onClick={() => setDeleting(entry)}
                >
                  <Trash2 size={15} />
                </button>
              </span>
            </div>
          ))}
        </div>
      )}

      <ContentDialog
        open={editing !== null}
        title={editing?.id ? "Edit dictionary entry" : "Add dictionary entry"}
        primaryText={saving ? "Saving…" : "Save"}
        primaryDisabled={saving}
        onPrimary={() => void save()}
        onClose={() => {
          setEditing(null);
          setError(null);
        }}
      >
        {editing && (
          <div className="dialog-form">
            <label>
              <span>Written term</span>
              <input
                className="settings-input"
                value={editing.value.displayTerm}
                onChange={(event) => setEditing({ ...editing, value: { ...editing.value, displayTerm: event.target.value } })}
              />
            </label>
            <label>
              <span>Spoken forms</span>
              <input
                className="settings-input"
                placeholder="Comma separated"
                value={editing.value.spokenForms.join(", ")}
                onChange={(event) =>
                  setEditing({ ...editing, value: { ...editing.value, spokenForms: event.target.value.split(",") } })
                }
              />
            </label>
            <label>
              <span>Category</span>
              <select
                className="settings-combo"
                value={editing.value.category}
                onChange={(event) =>
                  setEditing({ ...editing, value: { ...editing.value, category: event.target.value as DictionaryCategory } })
                }
              >
                {TABS.map((entry) => (
                  <option key={entry.id} value={entry.id}>
                    {entry.label}
                  </option>
                ))}
              </select>
            </label>
            {editing.value.category === "replacement" && (
              <label>
                <span>Replace with</span>
                <input
                  className="settings-input"
                  value={editing.value.replacement ?? ""}
                  onChange={(event) =>
                    setEditing({ ...editing, value: { ...editing.value, replacement: event.target.value || null } })
                  }
                />
              </label>
            )}
            <label className="dialog-form__toggle">
              <span>Match whole words only</span>
              <ToggleSwitch
                label="Match whole words only"
                checked={editing.value.wholeWordOnly}
                onChange={(value) => setEditing({ ...editing, value: { ...editing.value, wholeWordOnly: value } })}
              />
            </label>
            <label className="dialog-form__toggle">
              <span>Enabled</span>
              <ToggleSwitch
                label="Enabled"
                checked={editing.value.enabled}
                onChange={(value) => setEditing({ ...editing, value: { ...editing.value, enabled: value } })}
              />
            </label>
          </div>
        )}
      </ContentDialog>

      <ContentDialog
        open={deleting !== null}
        title={`Delete "${deleting?.displayTerm ?? ""}"?`}
        primaryText="Delete"
        destructive
        onPrimary={() => {
          if (!deleting) return;
          void native.deleteDictionaryEntry(deleting.id).then(() => {
            setDeleting(null);
            load();
          });
        }}
        onClose={() => setDeleting(null)}
      >
        <p>This entry will no longer be applied during transcription cleanup.</p>
      </ContentDialog>
    </div>
  );
}
