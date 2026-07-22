import {
  Braces,
  Bug,
  ClipboardCopy,
  FileText,
  List,
  MessageSquareText,
  Search,
  Sparkles,
  WandSparkles,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useLocation } from "react-router-dom";
import { useAppStore } from "../app/useAppStore";
import { Button } from "../components/common/Button";
import { InfoBar } from "../components/fluent/InfoBar";
import { SettingsRow, SettingsSection } from "../components/fluent/SettingsRow";
import { ToggleSwitch } from "../components/fluent/ToggleSwitch";
import { defaultSettings, native } from "../lib/native";
import type { AppSettings } from "../types/models";
import { diffWords } from "./textDiff";

interface Transform {
  id: string;
  name: string;
  description: string;
  icon: typeof WandSparkles;
  /** Only some transforms are safe to run unattended after every dictation. */
  autoApplicable: boolean;
}

const TRANSFORMS: Transform[] = [
  { id: "polish", name: "Polish", description: "Improve grammar and readability while preserving meaning.", icon: WandSparkles, autoApplicable: true },
  { id: "prompt_engineer", name: "Prompt Engineer", description: "Turn rough dictation into a structured AI prompt.", icon: Sparkles, autoApplicable: true },
  { id: "developer_task", name: "Developer Task", description: "Convert speech into a clear implementation task.", icon: Braces, autoApplicable: false },
  { id: "bug_report", name: "Bug Report", description: "Create steps, expected behaviour, and actual behaviour.", icon: Bug, autoApplicable: false },
  { id: "commit_message", name: "Commit Message", description: "Generate a concise source-control commit message.", icon: MessageSquareText, autoApplicable: false },
  { id: "documentation", name: "Documentation", description: "Format the text as technical documentation.", icon: FileText, autoApplicable: false },
  { id: "turn_into_list", name: "Turn Into List", description: "Convert the transcript into a structured list.", icon: List, autoApplicable: true },
];

export function TransformsPage() {
  const location = useLocation();
  const { lastTranscript } = useAppStore();
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [provider, setProvider] = useState("");
  const [showDiff, setShowDiff] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void native.settings().then(setSettings);
  }, []);

  // History can hand a transcript straight to this page.
  useEffect(() => {
    const handed = (location.state as { text?: string } | null)?.text;
    if (handed) setInput(handed);
  }, [location.state]);

  useEffect(() => {
    if (!input && lastTranscript) setInput(lastTranscript.finalTranscript);
  }, [input, lastTranscript]);

  const selected = TRANSFORMS.find((entry) => entry.id === selectedId) ?? null;

  const visible = useMemo(() => {
    const needle = search.trim().toLocaleLowerCase();
    if (!needle) return TRANSFORMS;
    return TRANSFORMS.filter((entry) =>
      `${entry.name} ${entry.description}`.toLocaleLowerCase().includes(needle),
    );
  }, [search]);

  const diff = useMemo(() => (output ? diffWords(input, output) : []), [input, output]);

  const run = async (transformId: string) => {
    if (!input.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const result = await native.transform(input, transformId);
      setOutput(result.transformedText);
      setProvider(result.provider);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The transform failed. Your original text is unchanged.");
    } finally {
      setBusy(false);
    }
  };

  const setAutoApply = async (id: string, enabled: boolean) => {
    const next = { ...settings, autoApplyTransform: enabled ? id : null };
    setSettings(next);
    await native.saveSettings(next);
  };

  return (
    <div className="page">
      <header className="page-header">
        <h1>Transforms</h1>
        <p className="page-header__meta">
          <span>Apply structured formatting to the latest transcript or a selected history item.</span>
        </p>
      </header>

      {error && <InfoBar severity="error" title="Transform failed" message={error} />}
      {provider.startsWith("local") && output && (
        <InfoBar
          severity="warning"
          title="AI cleanup was unavailable"
          message="VoiceFlow applied its local transform rules instead. The result is more conservative than the AI version."
        />
      )}

      {selected ? (
        <>
          <div className="workspace-header">
            <div>
              <h2>{selected.name}</h2>
              <p>{selected.description}</p>
            </div>
            <Button variant="secondary" onClick={() => { setSelectedId(null); setOutput(""); }}>
              Back to all transforms
            </Button>
          </div>

          <div className="workspace">
            <label className="workspace__pane">
              <span className="workspace__label">Original</span>
              <textarea
                value={input}
                placeholder="Paste or type text here…"
                onChange={(event) => {
                  setInput(event.target.value);
                  setOutput("");
                }}
              />
            </label>
            <div className="workspace__pane">
              <span className="workspace__label">
                Preview
                {provider && <em className="workspace__provider">via {provider}</em>}
              </span>
              <div className="workspace__preview">
                {output || <span className="transcript-paper__empty">Run the transform to see the result. The original stays intact.</span>}
              </div>
            </div>
          </div>

          <div className="command-row">
            <Button variant="primary" disabled={!input.trim() || busy} onClick={() => void run(selected.id)}>
              {busy ? "Applying…" : "Apply"}
            </Button>
            <Button variant="secondary" disabled={!output} icon={<ClipboardCopy size={15} />} onClick={() => void native.copyText(output)}>
              Copy
            </Button>
            <Button
              variant="secondary"
              disabled={!output}
              onClick={() => {
                setInput(output);
                setOutput("");
              }}
            >
              Replace original
            </Button>
            {lastTranscript && (
              <Button variant="secondary" onClick={() => { setInput(lastTranscript.finalTranscript); setOutput(""); }}>
                Use last transcript
              </Button>
            )}
          </div>

          {output && (
            <SettingsSection title="Changes">
              <div className="settings-row">
                <span className="settings-row__text">
                  <strong>Show differences</strong>
                  <small>Insertions and deletions between the original and the preview.</small>
                </span>
                <span className="settings-row__action">
                  <ToggleSwitch label="Show differences" checked={showDiff} onChange={setShowDiff} />
                </span>
              </div>
              {showDiff && (
                <div className="diff-view">
                  {diff.map((segment, index) => (
                    <span key={index} className={`diff-${segment.type}`}>
                      {segment.text}
                    </span>
                  ))}
                </div>
              )}
            </SettingsSection>
          )}
        </>
      ) : (
        <>
          <div className="command-bar">
            <label className="search-field">
              <Search size={15} aria-hidden="true" />
              <input
                value={search}
                aria-label="Search transforms"
                placeholder="Search transforms"
                onChange={(event) => setSearch(event.target.value)}
              />
            </label>
          </div>

          <div className="settings-group__rows">
            {visible.map((entry) => (
              <div className="settings-row" key={entry.id}>
                <span className="settings-row__icon">
                  <entry.icon size={17} aria-hidden="true" />
                </span>
                <span className="settings-row__text">
                  <strong>{entry.name}</strong>
                  <small>{entry.description}</small>
                </span>
                <span className="settings-row__action">
                  <Button variant="secondary" onClick={() => { setSelectedId(entry.id); setOutput(""); }}>
                    Run
                  </Button>
                </span>
              </div>
            ))}
          </div>

          <SettingsSection
            title="Automatically apply after dictation"
            description="Runs after cleanup and before the text is inserted. Only transforms that are safe to run unattended can be selected."
          >
            {TRANSFORMS.filter((entry) => entry.autoApplicable).map((entry) => (
              <SettingsRow
                key={entry.id}
                label={entry.name}
                description={entry.description}
                action={
                  <ToggleSwitch
                    label={`Automatically apply ${entry.name}`}
                    checked={settings.autoApplyTransform === entry.id}
                    onChange={(enabled) => void setAutoApply(entry.id, enabled)}
                  />
                }
              />
            ))}
          </SettingsSection>
        </>
      )}
    </div>
  );
}
