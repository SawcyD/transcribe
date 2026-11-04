import { BookPlus, Check, ClipboardCopy, Download, RotateCcw, Star, Trash2, Wand2, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { SearchBox } from "@memora/ui";
import { Button } from "../components/common/Button";
import { ContentDialog } from "../components/fluent/ContentDialog";
import { native } from "../lib/native";
import type { AppSettings, TranscriptRecord } from "../types/models";
import { filterHistory, historyToMarkdown } from "./historyFilter";

type Stage = "finalTranscript" | "cleanedTranscript" | "normalizedTranscript" | "rawTranscript";
type ModeFilter = "all" | TranscriptRecord["mode"];
type DateFilter = "any" | "today" | "week" | "month";
type StatusFilter = "all" | "starred" | TranscriptRecord["insertionStatus"];

const STAGES: Array<{ id: Stage; label: string }> = [
  { id: "finalTranscript", label: "Final" },
  { id: "cleanedTranscript", label: "Cleaned" },
  { id: "normalizedTranscript", label: "Normalized" },
  { id: "rawTranscript", label: "Raw" },
];

const MODE_LABELS: Record<TranscriptRecord["mode"], string> = {
  push_to_talk: "Push to talk",
  hands_free: "Hands-free",
  call: "Call",
  command: "Command Mode",
};

const STATUS_LABELS: Record<TranscriptRecord["insertionStatus"], string> = {
  inserted: "Inserted",
  copied: "Copied only",
  failed: "Failed",
  cancelled: "Cancelled",
};

function wordCount(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

function formatDuration(ms: number): string {
  const seconds = Math.round(ms / 1000);
  return seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
}

/** Groups records under Today / Yesterday / an absolute date. */
function dayLabel(iso: string): string {
  const date = new Date(iso);
  const today = new Date();
  const isSameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();
  if (isSameDay(date, today)) return "Today";
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (isSameDay(date, yesterday)) return "Yesterday";
  return date.toLocaleDateString([], { weekday: "long", month: "long", day: "numeric" });
}

function withinRange(iso: string, range: DateFilter): boolean {
  if (range === "any") return true;
  const age = Date.now() - new Date(iso).getTime();
  const day = 24 * 60 * 60 * 1000;
  if (range === "today") return new Date(iso).toDateString() === new Date().toDateString();
  if (range === "week") return age <= 7 * day;
  return age <= 30 * day;
}

export function HistoryPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const [records, setRecords] = useState<TranscriptRecord[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [stage, setStage] = useState<Stage>("finalTranscript");
  const [modeFilter, setModeFilter] = useState<ModeFilter>("all");
  const [appFilter, setAppFilter] = useState("all");
  const [dateFilter, setDateFilter] = useState<DateFilter>("any");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [loading, setLoading] = useState(true);
  const [confirmingPaste, setConfirmingPaste] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [suppressPasteConfirm, setSuppressPasteConfirm] = useState(false);
  const [copied, setCopied] = useState(false);
  const [dictionaryTerm, setDictionaryTerm] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const pageRef = useRef<HTMLDivElement>(null);
  const termInputRef = useRef<HTMLInputElement>(null);
  const dictionaryOpen = dictionaryTerm !== null;

  // The dialog mounts in a portal, so autoFocus is unreliable; focus and
  // select the prefilled term once it opens so Enter saves straight away.
  useEffect(() => {
    if (!dictionaryOpen) return;
    const id = window.setTimeout(() => {
      termInputRef.current?.focus();
      termInputRef.current?.select();
    }, 30);
    return () => window.clearTimeout(id);
  }, [dictionaryOpen]);

  const load = async () => {
    const [next, stored] = await Promise.all([native.history(), native.settings()]);
    setRecords(next);
    setSettings(stored);
    setSelectedId((current) =>
      current && next.some((item) => item.id === current) ? current : next.at(0)?.id ?? null,
    );
    setLoading(false);
  };

  useEffect(() => {
    void load();
  }, []);

  // Home and the overlay can deep-link to a specific transcript.
  useEffect(() => {
    const requested = (location.state as { transcriptId?: string } | null)?.transcriptId;
    if (requested) setSelectedId(requested);
  }, [location.state]);

  const applications = useMemo(
    () => Array.from(new Set(records.map((record) => record.applicationName ?? "Unknown app"))).sort(),
    [records],
  );

  const filtered = useMemo(
    () =>
      filterHistory(records, query).filter(
        (record) =>
          (modeFilter === "all" || record.mode === modeFilter) &&
          (appFilter === "all" || (record.applicationName ?? "Unknown app") === appFilter) &&
          (statusFilter === "all" ||
            (statusFilter === "starred" ? record.isFavorite : record.insertionStatus === statusFilter)) &&
          withinRange(record.createdAt, dateFilter),
      ),
    [records, query, modeFilter, appFilter, statusFilter, dateFilter],
  );

  const grouped = useMemo(() => {
    const groups = new Map<string, TranscriptRecord[]>();
    for (const record of filtered) {
      const label = dayLabel(record.createdAt);
      groups.set(label, [...(groups.get(label) ?? []), record]);
    }
    return Array.from(groups.entries());
  }, [filtered]);

  const selected = records.find((record) => record.id === selectedId) ?? null;

  const runPaste = async () => {
    if (!selected) return;
    setConfirmingPaste(false);
    if (suppressPasteConfirm && settings) {
      await native.saveSettings({ ...settings, confirmPasteAgain: false });
      setSettings({ ...settings, confirmPasteAgain: false });
    }
    await native.pasteTranscript(selected.id);
  };

  const stageValue = selected?.[stage] ?? "";
  const filtersActive = query !== "" || modeFilter !== "all" || appFilter !== "all" || dateFilter !== "any" || statusFilter !== "all";
  const clearFilters = () => {
    setQuery("");
    setModeFilter("all");
    setAppFilter("all");
    setDateFilter("any");
    setStatusFilter("all");
  };

  const flash = (message: string) => {
    setNotice(message);
    window.setTimeout(() => setNotice((current) => (current === message ? null : current)), 1800);
  };

  const toggleFavorite = async (record: TranscriptRecord) => {
    const favorite = !record.isFavorite;
    // Optimistic: the star flips immediately; a failure reloads the truth.
    setRecords((current) => current.map((item) => (item.id === record.id ? { ...item, isFavorite: favorite } : item)));
    try {
      await native.setTranscriptFavorite(record.id, favorite);
    } catch {
      void load();
    }
  };

  const exportMarkdown = () => {
    const url = URL.createObjectURL(new Blob([historyToMarkdown(filtered)], { type: "text/markdown" }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `voiceflow-dictations-${new Date().toISOString().slice(0, 10)}.md`;
    anchor.click();
    URL.revokeObjectURL(url);
  };

  /** Pre-fills the dictionary dialog with whatever the user highlighted. */
  const openAddToDictionary = () => {
    const selection = window.getSelection()?.toString().trim() ?? "";
    setDictionaryTerm(selection.split(/\s+/).length <= 4 ? selection : "");
  };

  const saveDictionaryTerm = async () => {
    const term = dictionaryTerm?.trim();
    if (!term) return;
    // The backend requires at least one spoken form; the lowercase term is
    // what the dictionary page itself defaults to.
    try {
      await native.saveDictionaryEntry(null, {
        displayTerm: term,
        spokenForms: [term.toLowerCase()],
        replacement: null,
        category: "vocabulary",
        priority: 100,
        caseSensitive: false,
        wholeWordOnly: true,
        enabled: true,
      });
    } catch (cause) {
      flash(cause instanceof Error ? cause.message : "Couldn't add that word");
      return;
    }
    setDictionaryTerm(null);
    flash(`Added “${term}” to your dictionary`);
  };

  // Page shortcuts: Ctrl+F search, Ctrl+C copy (when no text is highlighted),
  // S star, Delete remove. Ignored while typing in a field.
  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const typing = target?.closest("input, textarea, select, [contenteditable=true]");
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
        event.preventDefault();
        pageRef.current?.querySelector<HTMLInputElement>(".history-toolbar input")?.focus();
        return;
      }
      if (typing || !selected || confirmingDelete || confirmingPaste || dictionaryTerm !== null) return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c" && !window.getSelection()?.toString()) {
        event.preventDefault();
        void copySelected();
      } else if (event.key === "Delete") {
        setConfirmingDelete(true);
      } else if (event.key.toLowerCase() === "s" && !event.ctrlKey && !event.metaKey && !event.altKey) {
        void toggleFavorite(selected);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const copySelected = async () => {
    if (!selected) return;
    await native.copyText(selected.finalTranscript);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  // Up/Down walk the visible list like a mail client; the selection follows.
  const onListKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const index = filtered.findIndex((record) => record.id === selectedId);
    const next = filtered[Math.min(filtered.length - 1, Math.max(0, index + (event.key === "ArrowDown" ? 1 : -1)))];
    if (!next) return;
    setSelectedId(next.id);
    listRef.current?.querySelector<HTMLButtonElement>(`[data-id="${next.id}"]`)?.focus();
  };

  return (
    <div className="page" ref={pageRef}>
      <header className="page-header">
        <h1>History</h1>
        <p className="page-header__meta">
          <span>Everything you've dictated, stored only on this PC. Star the ones you want to keep forever.</span>
        </p>
      </header>

      <div className="history-toolbar" role="search">
        <div className="history-toolbar__search">
          <SearchBox value={query} label="Search transcripts" placeholder="Search what you said…" onChange={setQuery} />
        </div>
        <div className="history-toolbar__filters">
          <select className="filter-pill" aria-label="Filter by application" value={appFilter} data-active={appFilter !== "all"} onChange={(event) => setAppFilter(event.target.value)}>
            <option value="all">All apps</option>
            {applications.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
          <select className="filter-pill" aria-label="Filter by mode" value={modeFilter} data-active={modeFilter !== "all"} onChange={(event) => setModeFilter(event.target.value as ModeFilter)}>
            <option value="all">All modes</option>
            {Object.entries(MODE_LABELS).map(([id, label]) => (
              <option key={id} value={id}>
                {label}
              </option>
            ))}
          </select>
          <select className="filter-pill" aria-label="Filter by date" value={dateFilter} data-active={dateFilter !== "any"} onChange={(event) => setDateFilter(event.target.value as DateFilter)}>
            <option value="any">Any time</option>
            <option value="today">Today</option>
            <option value="week">Past 7 days</option>
            <option value="month">Past 30 days</option>
          </select>
          <select className="filter-pill" aria-label="Filter by status" value={statusFilter} data-active={statusFilter !== "all"} onChange={(event) => setStatusFilter(event.target.value as StatusFilter)}>
            <option value="all">Any status</option>
            <option value="starred">★ Starred</option>
            {Object.entries(STATUS_LABELS).map(([id, label]) => (
              <option key={id} value={id}>
                {label}
              </option>
            ))}
          </select>
          {filtersActive && (
            <button type="button" className="text-button" onClick={clearFilters}>
              <X size={13} aria-hidden="true" /> Clear
            </button>
          )}
        </div>
        <small className="history-toolbar__count">
          {filtered.length === records.length ? `${records.length} total` : `${filtered.length} of ${records.length}`}
        </small>
        <button type="button" className="text-button" disabled={filtered.length === 0} onClick={exportMarkdown} title="Export the transcripts shown as a Markdown file">
          <Download size={13} aria-hidden="true" /> Export
        </button>
      </div>

      <section className="history-shell">
        <aside className="history-list">
          <div className="history-scroll" ref={listRef} onKeyDown={onListKeyDown} aria-label="Transcripts">
            {loading ? (
              <p className="list-message">Loading history…</p>
            ) : grouped.length === 0 ? (
              <div className="empty-state">
                <strong>{records.length === 0 ? "No dictations yet" : "No matching transcripts"}</strong>
                <p>{records.length === 0 ? "Hold Ctrl + Win and start speaking." : "Nothing matches these filters."}</p>
                {records.length > 0 && (
                  <button type="button" className="text-button" onClick={clearFilters}>
                    Clear filters
                  </button>
                )}
              </div>
            ) : (
              grouped.map(([label, group]) => (
                <div key={label}>
                  <h2 className="history-group">{label}</h2>
                  {group.map((record) => (
                    <button
                      key={record.id}
                      data-id={record.id}
                      type="button"
                      aria-current={record.id === selectedId}
                      tabIndex={record.id === selectedId || (!selectedId && record === filtered[0]) ? 0 : -1}
                      className={`history-item${record.id === selectedId ? " history-item--active" : ""}`}
                      onClick={() => setSelectedId(record.id)}
                    >
                      <span className="history-item__top">
                        <b>{record.applicationName ?? "Unknown app"}</b>
                        {record.isFavorite && <Star size={12} className="history-item__star" aria-label="Starred" />}
                        {record.insertionStatus !== "inserted" && (
                          <span className={`status-chip status-chip--${record.insertionStatus}`}>{STATUS_LABELS[record.insertionStatus]}</span>
                        )}
                        <time>{new Date(record.createdAt).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</time>
                      </span>
                      <span className="history-item__excerpt">{record.finalTranscript || "—"}</span>
                      <span className="history-item__meta">
                        {MODE_LABELS[record.mode]} · {wordCount(record.finalTranscript)} words · {formatDuration(record.durationMs)}
                      </span>
                    </button>
                  ))}
                </div>
              ))
            )}
          </div>
        </aside>

        <article className="history-detail">
          {selected ? (
            <>
              <header className="history-detail__header">
                <div>
                  <h2>{selected.applicationName ?? "Unknown app"}</h2>
                  <p>
                    {new Date(selected.createdAt).toLocaleString([], { weekday: "short", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" })}
                    {" · "}
                    {MODE_LABELS[selected.mode]}
                  </p>
                </div>
                <div className="history-detail__badges">
                  <span className={`status-chip status-chip--${selected.insertionStatus}`}>{STATUS_LABELS[selected.insertionStatus]}</span>
                  <button
                    type="button"
                    className={`icon-button star-button${selected.isFavorite ? " star-button--on" : ""}`}
                    aria-pressed={selected.isFavorite}
                    aria-label={selected.isFavorite ? "Unstar (S)" : "Star (S)"}
                    title={selected.isFavorite ? "Unstar · S" : "Star — starred dictations are never auto-deleted · S"}
                    onClick={() => void toggleFavorite(selected)}
                  >
                    <Star size={16} />
                  </button>
                </div>
              </header>

              <div className="history-detail__actions">
                <Button variant="primary" title="Copy · Ctrl+C" icon={copied ? <Check size={15} /> : <ClipboardCopy size={15} />} onClick={() => void copySelected()}>
                  {copied ? "Copied" : "Copy"}
                </Button>
                <Button
                  variant="secondary"
                  icon={<RotateCcw size={15} />}
                  onClick={() => {
                    if (settings?.confirmPasteAgain === false) void native.pasteTranscript(selected.id);
                    else setConfirmingPaste(true);
                  }}
                >
                  Paste again
                </Button>
                <Button
                  variant="secondary"
                  icon={<Wand2 size={15} />}
                  onClick={() => navigate("/transforms", { state: { text: selected.finalTranscript } })}
                >
                  Transform
                </Button>
                <button type="button" className="icon-button icon-button--push" aria-label="Add a word to the dictionary" title="Add a word to the dictionary (highlight it first)" onMouseDown={(event) => event.preventDefault()} onClick={openAddToDictionary}>
                  <BookPlus size={16} />
                </button>
                <button type="button" className="icon-button icon-button--danger" aria-label="Delete transcript" title="Delete transcript · Del" onClick={() => setConfirmingDelete(true)}>
                  <Trash2 size={16} />
                </button>
              </div>

              <div className="segmented" role="tablist" aria-label="Transcript stage">
                {STAGES.map((entry) => (
                  <button
                    key={entry.id}
                    type="button"
                    role="tab"
                    aria-selected={stage === entry.id}
                    className={`segmented__item${stage === entry.id ? " segmented__item--selected" : ""}`}
                    onClick={() => setStage(entry.id)}
                  >
                    {entry.label}
                  </button>
                ))}
              </div>

              <div className="transcript-paper">
                {stageValue || <span className="transcript-paper__empty">This stage was not stored for this transcript.</span>}
              </div>

              <dl className="metadata-list">
                <div>
                  <dt>Length</dt>
                  <dd>
                    {wordCount(selected.finalTranscript)} words · {formatDuration(selected.durationMs)}
                  </dd>
                </div>
                <div>
                  <dt>Confidence</dt>
                  <dd>{selected.confidence == null ? "—" : `${Math.round(selected.confidence * 100)}%`}</dd>
                </div>
                <div>
                  <dt>Provider</dt>
                  <dd>
                    {selected.provider} · {selected.model}
                  </dd>
                </div>
                <div>
                  <dt>Transform</dt>
                  <dd>{selected.transformId ?? "None"}</dd>
                </div>
              </dl>
            </>
          ) : (
            <div className="empty-state">
              <strong>No transcript selected</strong>
              <p>Pick a dictation on the left to read, copy, or paste it again.</p>
            </div>
          )}
        </article>
      </section>

      <ContentDialog
        open={confirmingPaste}
        title="Paste this transcript?"
        primaryText="Paste"
        onPrimary={() => void runPaste()}
        onClose={() => setConfirmingPaste(false)}
      >
        <p>The transcript will be pasted into the currently focused application.</p>
        <label className="checkbox-row">
          <input type="checkbox" checked={suppressPasteConfirm} onChange={(event) => setSuppressPasteConfirm(event.target.checked)} />
          <span>Do not ask again</span>
        </label>
      </ContentDialog>

      <ContentDialog
        open={dictionaryTerm !== null}
        title="Add to dictionary"
        primaryText="Add"
        primaryDisabled={!dictionaryTerm?.trim()}
        onPrimary={() => void saveDictionaryTerm()}
        onClose={() => setDictionaryTerm(null)}
      >
        <p>Deepgram will be primed to recognise this word or name in future dictations.</p>
        <input
          className="settings-input dictionary-quick-add"
          aria-label="Word or name"
          placeholder="e.g. Kubernetes, Tauri, useEffect"
          ref={termInputRef}
          value={dictionaryTerm ?? ""}
          onChange={(event) => setDictionaryTerm(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void saveDictionaryTerm();
            }
          }}
        />
      </ContentDialog>

      {notice && (
        <div className="save-indicator save-indicator--success save-indicator--visible" role="status">
          <Check size={14} />
          <span>{notice}</span>
        </div>
      )}

      <ContentDialog
        open={confirmingDelete}
        title="Delete this transcript?"
        primaryText="Delete"
        destructive
        onPrimary={() => {
          if (!selected) return;
          setConfirmingDelete(false);
          void native.deleteTranscript(selected.id).then(load);
        }}
        onClose={() => setConfirmingDelete(false)}
      >
        <p>
          This permanently removes the transcript and all four of its stages. This cannot be undone.
          {selected?.isFavorite ? " It is starred." : ""}
        </p>
      </ContentDialog>
    </div>
  );
}
