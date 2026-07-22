import { useEffect, useState, type CSSProperties } from "react";

const BAR_COUNT = 24;

/**
 * Renders the level stream published by the Rust capture thread.
 *
 * The component owns no audio: it only maps the `bars` it is handed. When the
 * user prefers reduced motion it collapses to a single static level bar rather
 * than animating 24 independent elements.
 */
export function AudioVisualizer({ bars }: { bars: number[] }) {
  const [reducedMotion, setReducedMotion] = useState(
    () => window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false,
  );

  useEffect(() => {
    const query = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    if (!query) return;
    const onChange = (event: MediaQueryListEvent) => setReducedMotion(event.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  const source = bars.length > 0 ? bars : [];
  const level = source.length > 0 ? source.reduce((sum, value) => sum + value, 0) / source.length : 0;

  if (reducedMotion) {
    return (
      <div
        className="audio-wave audio-wave--static"
        role="meter"
        aria-label="Microphone input level"
        aria-valuemin={0}
        aria-valuemax={1}
        aria-valuenow={Number(level.toFixed(2))}
      >
        <i style={{ width: `${Math.min(100, Math.round(level * 140))}%` }} />
      </div>
    );
  }

  const values = Array.from({ length: BAR_COUNT }, (_, index) => {
    if (source.length === 0) return 0;
    const sourceIndex = Math.min(
      source.length - 1,
      Math.round((index / (BAR_COUNT - 1)) * (source.length - 1)),
    );
    return source[sourceIndex] ?? level;
  });

  return (
    <div
      className="audio-wave"
      role="meter"
      aria-label="Microphone input level"
      aria-valuemin={0}
      aria-valuemax={1}
      aria-valuenow={Number(level.toFixed(2))}
    >
      {values.map((value, index) => {
        // Silence renders as a flat baseline, never as invented motion.
        const scale = 0.08 + Math.max(0, Math.min(1, value)) * 0.92;
        const style = { "--wave-scale": scale.toFixed(3) } as CSSProperties;
        return <i className="audio-wave-bar" key={index} style={style} />;
      })}
    </div>
  );
}
