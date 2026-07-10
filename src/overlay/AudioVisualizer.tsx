export function AudioVisualizer({ bars, processing = false }: { bars: number[]; processing?: boolean }) {
  const values = bars.length > 0 ? bars : Array.from({ length: 12 }, () => 0.08);
  return (
    <div className={`audio-bars ${processing ? "audio-bars--processing" : ""}`} aria-label="Microphone input level" role="meter">
      {values.map((value, index) => (
        <i key={index} style={{ height: `${Math.round(5 + value * 19)}px`, animationDelay: `${index * 45}ms` }} />
      ))}
    </div>
  );
}
