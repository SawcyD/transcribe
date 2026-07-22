import { ClipboardCopy, RotateCcw, Trash2, Wand2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { ComboBox, CommandBar, CommandGroup, DataGrid, SearchBox, type DataGridColumn } from "@memora/ui";
import { Button } from "../components/common/Button";
import { ContentDialog } from "../components/fluent/ContentDialog";
import { native } from "../lib/native";
import type { AppSettings, TranscriptRecord } from "../types/models";
import { filterHistory } from "./historyFilter";

type Stage = "finalTranscript" | "cleanedTranscript" | "normalizedTranscript" | "rawTranscript";
type ModeFilter = "all" | TranscriptRecord["mode"];
type DateFilter = "any" | "today" | "week" | "month";
type StatusFilter = "all" | TranscriptRecord["insertionStatus"];

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
          (statusFilter === "all" || record.insertionStatus === statusFilter) &&
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
  const gridColumns: DataGridColumn<TranscriptRecord>[] = [
    { id: "created", header: "Date and time", width: 136, render: (record) => new Date(record.createdAt).toLocaleString([], { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" }) },
    { id: "application", header: "Application", width: 120, render: (record) => record.applicationName ?? "Unknown app" },
    { id: "mode", header: "Mode", width: 108, render: (record) => MODE_LABELS[record.mode] },
    { id: "duration", header: "Duration", width: 78, align: "end", render: (record) => `${(record.durationMs / 1000).toFixed(1)}s` },
    { id: "final", header: "Final text", render: (record) => record.finalTranscript || "—" },
  ];

  return (
    <div className="page">
      <header className="page-header">
        <h1>History</h1>
        <p className="page-header__meta">
          <span>Every locally stored transcript and its processing stages.</span>
        </p>
      </header>

      <CommandBar role="search" className="voiceflow-command-bar">
        <CommandGroup><SearchBox value={query} label="Search transcripts" placeholder="Search transcripts" onChange={setQuery} /></CommandGroup>
        <CommandGroup>
        <select className="settings-combo" aria-label="Filter by application" value={appFilter} onChange={(event) => setAppFilter(event.target.value)}>
          <option value="all">All applications</option>
          {applications.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        <select className="settings-combo" aria-label="Filter by mode" value={modeFilter} onChange={(event) => setModeFilter(event.target.value as ModeFilter)}>
          <option value="all">All modes</option>
          {Object.entries(MODE_LABELS).map(([id, label]) => (
            <option key={id} value={id}>
              {label}
            </option>
          ))}
        </select>
        <ComboBox label="Filter by date" value={dateFilter} onChange={setDateFilter} options={[{ value: "any", label: "Any time" }, { value: "today", label: "Today" }, { value: "week", label: "Past 7 days" }, { value: "month", label: "Past 30 days" }]} />
        <select className="settings-combo" aria-label="Filter by status" value={statusFilter} onChange={(event) => setStatusFilter(event.target.value as StatusFilter)}>
          <option value="all">All statuses</option>
          <option value="inserted">Inserted</option>
          <option value="copied">Copied</option>
          <option value="failed">Failed</option>
          <option value="cancelled">Cancelled</option>
        </select>
        <small className="command-bar__count">
          {filtered.length} {filtered.length === 1 ? "item" : "items"}
        </small>
        </CommandGroup>
      </CommandBar>

      <section className="history-shell">
        <aside className="history-list">
          {!loading && <DataGrid className="voiceflow-history-grid" rows={filtered} columns={gridColumns} rowKey={(record) => record.id} ariaLabel="Transcript history" selectedKeys={selectedId ? [selectedId] : []} onSelectionChange={(keys) => setSelectedId(typeof keys[0] === "string" ? keys[0] : null)} emptyMessage={records.length === 0 ? "No dictations yet. Hold Ctrl + Win and start speaking." : "No matching transcripts. Try clearing a filter."} />}
          <div className="history-scroll">
            {loading ? (
              <p className="list-message">Loading history…</p>
            ) : grouped.length === 0 ? (
              <div className="empty-state">
                <strong>{records.length === 0 ? "No dictations yet" : "No matching transcripts"}</strong>
                <p>{records.length === 0 ? "Hold Ctrl + Win and start speaking." : "Try clearing a filter."}</p>
              </div>
            ) : (
              grouped.map(([label, group]) => (
                <div key={label}>
                  <h2 className="history-group">{label}</h2>
                  {group.map((record) => (
                    <button
                      key={record.id}
                      type="button"
                      aria-current={record.id === selectedId}
                      className={`history-item${record.id === selectedId ? " history-item--active" : ""}`}
                      onClick={() => setSelectedId(record.id)}
                    >
                      <span className="history-item__meta">
                        <time>{new Date(record.createdAt).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</time>
                        <b>{record.applicationName ?? "Unknown app"}</b>
                        <em>{MODE_LABELS[record.mode]}</em>
                        <i>{record.finalTranscript.trim().split(/\s+/).filter(Boolean).length} words</i>
                      </span>
                      <span className="history-item__excerpt">{record.finalTranscript}</span>
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
                  <dt>Application</dt>
                  <dd>{selected.applicationName ?? "Unknown"}</dd>
                </div>
                <div>
                  <dt>Mode</dt>
                  <dd>{MODE_LABELS[selected.mode]}</dd>
                </div>
                <div>
                  <dt>Duration</dt>
                  <dd>{(selected.durationMs / 1000).toFixed(1)} seconds</dd>
                </div>
                <div>
                  <dt>Created</dt>
                  <dd>{new Date(selected.createdAt).toLocaleString()}</dd>
                </div>
                <div>
                  <dt>Insertion</dt>
                  <dd>{selected.insertionStatus}</dd>
                </div>
                <div>
                  <dt>Provider</dt>
                  <dd>
                    {selected.provider} · {selected.model}
                  </dd>
                </div>
                <div>
                  <dt>Confidence</dt>
                  <dd>{selected.confidence == null ? "—" : `${Math.round(selected.confidence * 100)}%`}</dd>
                </div>
                <div>
                  <dt>Transform</dt>
                  <dd>{selected.transformId ?? "None"}</dd>
                </div>
              </dl>

              <div className="command-row">
                <Button variant="primary" icon={<ClipboardCopy size={15} />} onClick={() => void native.copyText(selected.finalTranscript)}>
                  Copy
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
                <Button variant="secondary" icon={<Trash2 size={15} />} onClick={() => setConfirmingDelete(true)}>
                  Delete
                </Button>
              </div>
            </>
          ) : (
            <div className="empty-state">
              <strong>No transcript selected</strong>
              <p>Choose an entry to see its stages and metadata.</p>
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
        <p>This permanently removes the transcript and all four of its stages. This cannot be undone.</p>
      </ContentDialog>
    </div>
  );
}
