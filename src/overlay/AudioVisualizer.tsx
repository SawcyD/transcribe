import type { CSSProperties } from "react";

export function AudioVisualizer({ bars, processing = false }: { bars: number[]; processing?: boolean }) {
  const source = bars.length > 0 ? bars : Array.from({ length: 12 }, () => 0.08);
  const level = source.reduce((sum, value) => sum + value, 0) / source.length;
  const barCount = 18;
  const halfCount = barCount / 2;
  const left = Array.from({ length: halfCount }, (_, index) => {
    const sourceIndex = Math.min(source.length - 1, Math.floor((index / (halfCount - 1)) * (source.length - 1)));
    return source[sourceIndex] ?? level;
  });
  const values = [...left, ...left.slice().reverse()];

  return (
    <div
      className={`audio-wave ${processing ? "audio-wave--processing" : ""}`}
      aria-label="Microphone input level"
      aria-valuemax={1}
      aria-valuemin={0}
      aria-valuenow={Number(level.toFixed(2))}
      role="meter"
    >
      <span className="audio-wave-line" aria-hidden="true" />
      {values.map((value, index) => {
        const scale = 0.1 + Math.max(0, Math.min(1, value)) * 1.1;
        const opacity = 0.22 + value * 0.78;
        const style = {
          opacity,
          "--wave-scale": scale.toFixed(3),
          transitionDelay: `${index * 8}ms`,
        } as CSSProperties;
        return (
          <i
            className="audio-wave-bar"
            key={index}
            style={style}
          />
        );
      })}
    </div>
  );
}
