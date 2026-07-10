import { ClipboardCopy, RotateCcw, Search, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "../components/common/Button";
import { native } from "../lib/native";
import type { TranscriptRecord } from "../types/models";
import { filterHistory } from "./historyFilter";

type Stage = "finalTranscript" | "rawTranscript" | "normalizedTranscript" | "cleanedTranscript";

const stageLabels: Record<Stage, string> = {
  finalTranscript: "Final",
  rawTranscript: "Raw",
  normalizedTranscript: "Normalized",
  cleanedTranscript: "Cleaned",
};

export function HistoryPage() {
  const [records, setRecords] = useState<TranscriptRecord[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [stage, setStage] = useState<Stage>("finalTranscript");
  const [loading, setLoading] = useState(true);

  const load = async () => {
    const next = await native.history();
    setRecords(next);
    setSelectedId((current) => current && next.some((item) => item.id === current) ? current : next.at(0)?.id ?? null);
    setLoading(false);
  };

  useEffect(() => { void load(); }, []);
  const filtered = useMemo(() => filterHistory(records, query), [records, query]);
  const selected = records.find((record) => record.id === selectedId) ?? null;

  return (
    <div className="page page--history">
      <header className="page-header"><div><span className="eyebrow">LOCAL ARCHIVE</span><h1>Transcript history</h1><p>Every processing stage stays available so edits and provider behavior remain inspectable.</p></div></header>
      <section className="history-shell">
        <aside className="history-list">
          <label className="search-field"><Search size={16} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search transcripts" /></label>
          <div className="history-scroll">
            {loading ? <p className="empty-copy">Loading history…</p> : filtered.length === 0 ? <p className="empty-copy">No matching transcripts.</p> : filtered.map((record) => (
              <button key={record.id} className={record.id === selectedId ? "history-item history-item--active" : "history-item"} onClick={() => setSelectedId(record.id)}>
                <span>{record.finalTranscript}</span>
                <small>{record.applicationName ?? "Unknown app"}<time>{new Date(record.createdAt).toLocaleDateString()}</time></small>
              </button>
            ))}
          </div>
        </aside>
        <article className="history-detail">
          {selected ? (
            <>
              <div className="detail-header"><div><span className={`result-dot result-dot--${selected.insertionStatus}`} />{selected.applicationName ?? "Unknown application"}</div><time>{new Date(selected.createdAt).toLocaleString()}</time></div>
              <div className="stage-tabs" role="tablist">
                {(Object.keys(stageLabels) as Stage[]).map((key) => <button role="tab" aria-selected={stage === key} key={key} onClick={() => setStage(key)}>{stageLabels[key]}</button>)}
              </div>
              <div className="transcript-paper">{selected[stage]}</div>
              <dl className="metadata-grid">
                <div><dt>Duration</dt><dd>{(selected.durationMs / 1000).toFixed(1)}s</dd></div>
                <div><dt>Processing</dt><dd>{selected.processingMs}ms</dd></div>
                <div><dt>Confidence</dt><dd>{selected.confidence === undefined ? "—" : `${Math.round(selected.confidence * 100)}%`}</dd></div>
                <div><dt>Provider</dt><dd>{selected.provider} · {selected.model}</dd></div>
              </dl>
              <div className="detail-actions">
                <Button variant="primary" icon={<ClipboardCopy size={16} />} onClick={() => void native.copyText(selected.finalTranscript)}>Copy final</Button>
                <Button icon={<RotateCcw size={16} />} onClick={() => void native.pasteTranscript(selected.id)}>Paste again</Button>
                <Button variant="ghost" icon={<Trash2 size={16} />} onClick={() => void native.deleteTranscript(selected.id).then(load)}>Delete</Button>
              </div>
            </>
          ) : <div className="detail-empty"><HistoryGlyph /><h2>No transcript selected</h2><p>Complete a dictation to create your first local record.</p></div>}
        </article>
      </section>
    </div>
  );
}

function HistoryGlyph() {
  return <span className="history-glyph" aria-hidden="true"><i /><i /><i /></span>;
}
