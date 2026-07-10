import { Braces, Bug, Check, ClipboardCopy, FileText, List, MessageSquareText, Sparkles, WandSparkles } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useAppStore } from "../app/useAppStore";
import { Button } from "../components/common/Button";
import { native } from "../lib/native";

type Transform = {
  id: string;
  name: string;
  description: string;
  icon: typeof WandSparkles;
  accent: string;
};

const transforms: Transform[] = [
  { id: "polish", name: "Polish", description: "Clear, natural writing that keeps your meaning and tone.", icon: WandSparkles, accent: "mint" },
  { id: "prompt_engineer", name: "Prompt Engineer", description: "Turn rough intent into an executable prompt for an AI agent.", icon: Sparkles, accent: "blue" },
  { id: "developer_task", name: "Developer Task", description: "Shape technical speech into requirements and acceptance criteria.", icon: Braces, accent: "violet" },
  { id: "bug_report", name: "Bug Report", description: "Extract expected behavior, actual behavior, and reproduction steps.", icon: Bug, accent: "orange" },
  { id: "commit_message", name: "Commit Message", description: "Create a concise conventional commit with an optional body.", icon: MessageSquareText, accent: "rose" },
  { id: "documentation", name: "Documentation", description: "Organize dictation into readable technical documentation.", icon: FileText, accent: "cyan" },
  { id: "turn_into_list", name: "Turn Into List", description: "Convert prose into a clean, ordered set of points.", icon: List, accent: "lime" },
];

export function TransformsPage() {
  const { lastTranscript } = useAppStore();
  const [selectedId, setSelectedId] = useState("polish");
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [provider, setProvider] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const selected = useMemo(() => transforms.find((item) => item.id === selectedId) ?? transforms[0], [selectedId]);

  useEffect(() => {
    if (!input && lastTranscript) setInput(lastTranscript.finalTranscript);
  }, [input, lastTranscript]);

  const run = async () => {
    if (!input.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const result = await native.transform(input, selected.id);
      setOutput(result.transformedText);
      setProvider(result.provider);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Transform failed. Your original text is unchanged.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page page--transforms">
      <header className="page-header"><div><span className="eyebrow">NEXT-STAGE OUTPUT</span><h1>Transforms</h1><p>Preview an improved version before anything is copied or inserted. Provider failures fall back to deterministic local transforms.</p></div></header>
      <div className="transform-layout">
        <section className="transform-grid">
          {transforms.map(({ id, name, description, icon: Icon, accent }) => <button className={`transform-card ${id === selected.id ? "transform-card--selected" : ""}`} key={id} onClick={() => { setSelectedId(id); setOutput(""); }}><span className={`transform-icon transform-icon--${accent}`}><Icon size={20} /></span><h2>{name}</h2><p>{description}</p><small>{id === "polish" || id === "prompt_engineer" ? "Ready to run" : "Preset scaffold"}</small></button>)}
        </section>
        <section className="transform-editor">
          <div className="editor-heading"><div><span className="eyebrow">{selected.name.toUpperCase()}</span><h2>Preview workspace</h2></div><span className="editor-provider">{provider ? `via ${provider}` : "No changes applied"}</span></div>
          <label className="editor-field"><span>Original text</span><textarea value={input} onChange={(event) => setInput(event.target.value)} placeholder="Paste or type rough text here…" /></label>
          <div className="editor-actions"><Button variant="primary" disabled={!input.trim() || busy} onClick={() => void run()} icon={busy ? <span className="button-spinner" /> : <Sparkles size={15} />}>{busy ? "Transforming…" : `Run ${selected.name}`}</Button>{lastTranscript && <Button variant="ghost" onClick={() => { setInput(lastTranscript.finalTranscript); setOutput(""); }}>Use last transcript</Button>}</div>
          {error && <p className="transform-error">{error}</p>}
          <div className="result-panel"><div className="result-heading"><span>Transformed text</span>{output && <div><Button variant="ghost" icon={<ClipboardCopy size={14} />} onClick={() => void native.copyText(output)}>Copy</Button><Button variant="ghost" icon={<Check size={14} />} onClick={() => { setInput(output); setOutput(""); }}>Replace input</Button></div>}</div>{output ? <><div className="transform-result">{output}</div><div className="transform-diff"><span className="diff-removed">− {input}</span><span className="diff-added">＋ {output}</span></div></> : <p className="empty-copy">Your preview will appear here. The original stays intact until you choose to replace it.</p>}</div>
        </section>
      </div>
    </div>
  );
}
