import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { useAppStore } from "../app/useAppStore";
import { isTauri, native } from "../lib/native";
import { buddyFrames, buddyMoodFor, type CaptureMood } from "./BuddyState";

const REST_AFTER_MS = 45_000;
const STROLL_DURATION_MS = 4_500;
const STROLL_PAUSE_MS = 7_500;
const STROLL_TICK_MS = 50;
const ACTION_RESULT_MS = 2_200;
const DRAG_THRESHOLD = 7;
const BUDDY_MARGIN = 24;

type Point = { x: number; y: number };
type StrollPhase = "idle" | "walking_out" | "sleeping" | "walking_home";
type StrollRun = { cancelled: boolean; home: Point; destination: Point };

function wait(ms: number) {
  return new Promise<void>((resolve) => window.setTimeout(resolve, ms));
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), Math.max(min, max));
}

export function BuddyOverlay() {
  const { audio, dictation } = useAppStore();
  const [captureMood, setCaptureMood] = useState<CaptureMood>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [resting, setResting] = useState(false);
  const [greeting, setGreeting] = useState(true);
  const strollEnabled = false;
  const [strollPhase, setStrollPhase] = useState<StrollPhase>("idle");
  const [walkDirection, setWalkDirection] = useState<"left" | "right">("left");
  const [frameStep, setFrameStep] = useState(0);
  const pointerStart = useRef<{ x: number; y: number } | null>(null);
  const dragged = useRef(false);
  const strollRun = useRef<StrollRun | null>(null);
  const restartTimer = useRef<number | null>(null);
  const canStrollRef = useRef(false);

  canStrollRef.current = strollEnabled && dictation.state === "idle" && captureMood === null;

  async function setWindowPosition(point: Point) {
    await getCurrentWindow().setPosition(new PhysicalPosition(Math.round(point.x), Math.round(point.y)));
  }

  async function animateWindow(run: StrollRun, from: Point, to: Point) {
    const steps = Math.max(1, Math.round(STROLL_DURATION_MS / STROLL_TICK_MS));
    for (let step = 1; step <= steps; step += 1) {
      if (run.cancelled || strollRun.current !== run) return false;
      const progress = step / steps;
      const eased = progress * progress * (3 - 2 * progress);
      await setWindowPosition({
        x: from.x + (to.x - from.x) * eased,
        y: from.y + (to.y - from.y) * eased,
      });
      await wait(STROLL_TICK_MS);
    }
    return !run.cancelled && strollRun.current === run;
  }

  function stopStroll(returnHome: boolean) {
    if (restartTimer.current !== null) {
      window.clearTimeout(restartTimer.current);
      restartTimer.current = null;
    }
    const run = strollRun.current;
    if (!run) return;
    run.cancelled = true;
    strollRun.current = null;
    setStrollPhase("idle");
    setResting(false);
    if (returnHome) void setWindowPosition(run.home);
  }

  async function startStroll() {
    if (!isTauri() || !canStrollRef.current || strollRun.current) return;
    const buddyWindow = getCurrentWindow();
    const monitor = await currentMonitor();
    if (!monitor || !canStrollRef.current) return;

    const position = await buddyWindow.outerPosition();
    const size = await buddyWindow.outerSize();
    const width = Math.max(144, size.width);
    const height = Math.max(144, size.height);
    const workLeft = monitor.workArea.position.x + BUDDY_MARGIN;
    const workTop = monitor.workArea.position.y + BUDDY_MARGIN;
    const workRight = monitor.workArea.position.x + monitor.workArea.size.width - width - BUDDY_MARGIN;
    const workBottom = monitor.workArea.position.y + monitor.workArea.size.height - height - BUDDY_MARGIN;
    if (workRight - workLeft < width * 2) return;

    const home = {
      x: clamp(position.x, workLeft, workRight),
      y: clamp(position.y, workTop, workBottom),
    };
    const monitorCenter = monitor.workArea.position.x + monitor.workArea.size.width / 2;
    const destination = {
      x: home.x + width / 2 <= monitorCenter ? workRight : workLeft,
      y: home.y,
    };
    const run: StrollRun = { cancelled: false, home, destination };
    strollRun.current = run;
    setWalkDirection(destination.x < home.x ? "left" : "right");
    setResting(false);
    setStrollPhase("walking_out");

    if (!(await animateWindow(run, home, destination))) return;
    if (run.cancelled || strollRun.current !== run) return;
    setStrollPhase("sleeping");
    setResting(true);
    await wait(STROLL_PAUSE_MS);
    if (run.cancelled || strollRun.current !== run) return;

    setResting(false);
    setStrollPhase("walking_home");
    setWalkDirection(destination.x < home.x ? "right" : "left");
    if (!(await animateWindow(run, destination, home))) return;
    if (run.cancelled || strollRun.current !== run) return;
    strollRun.current = null;
    setStrollPhase("idle");
    if (canStrollRef.current) {
      restartTimer.current = window.setTimeout(() => {
        restartTimer.current = null;
        void startStroll();
      }, REST_AFTER_MS);
    }
  }

  useEffect(() => {
    if (dictation.state !== "idle" || captureMood !== null || !strollEnabled) {
      stopStroll(true);
      setResting(false);
      return;
    }
    const timer = window.setTimeout(() => {
      if (strollEnabled) void startStroll();
      else setResting(true);
    }, REST_AFTER_MS);
    return () => window.clearTimeout(timer);
  }, [captureMood, dictation.state, strollEnabled]);

  useEffect(() => () => stopStroll(false), []);

  const baseMood = buddyMoodFor(dictation.state, captureMood, resting, dictation.mode);
  const ambientMood = strollPhase === "walking_out" || strollPhase === "walking_home"
    ? "walking"
    : strollPhase === "sleeping"
      ? "sleeping"
      : baseMood;
  const mood = greeting && dictation.state === "idle" && captureMood === null && strollPhase === "idle" ? "waving" : ambientMood;
  const frames = buddyFrames[mood];
  const frame = useMemo(() => frames[frameStep % frames.length] ?? frames[0], [frameStep, frames]);
  const isIdle = mood === "idle";
  const isSleeping = mood === "sleeping";
  const isWaving = mood === "waving";
  const isCallRecording = mood === "call_recording";
  const isWalking = mood === "walking";
  const idleFrame = frame % 30;
  const spriteAsset = isIdle
    ? idleFrame < 16 ? "/voiceflow-buddy-idle-01.png" : "/voiceflow-buddy-idle-02.png"
    : isSleeping
    ? "/voiceflow-buddy-sleeping-32.png"
    : isWaving
      ? "/voiceflow-buddy-wave.png"
      : isWalking
        ? "/voiceflow-buddy-walk.png"
        : isCallRecording
          ? "/voiceflow-buddy-sheet.png"
        : null;

  useEffect(() => {
    const timer = window.setTimeout(() => setGreeting(false), 2_200);
    return () => window.clearTimeout(timer);
  }, []);

  useEffect(() => {
    setFrameStep(0);
    if (frames.length < 2) return;
    const interval = mood === "idle" ? 180 : mood === "push_to_talk" ? 150 : mood === "hands_free" ? 420 : mood === "call_recording" ? 240 : mood === "sleeping" ? 180 : mood === "walking" ? 120 : mood === "waving" ? 180 : 300;
    const timer = window.setInterval(() => setFrameStep((step) => step + 1), interval);
    return () => window.clearInterval(timer);
  }, [frames, mood]);

  async function captureContext() {
    if (captureMood === "capturing") return;
    stopStroll(true);
    setResting(false);
    setCaptureMood("capturing");
    try {
      const context = await native.captureScreenContext();
      await native.openAssistantDrawer(context);
      setCaptureMood("captured");
    } catch {
      setCaptureMood("failed");
    }
    window.setTimeout(() => setCaptureMood(null), ACTION_RESULT_MS);
  }

  function handlePointerDown(event: PointerEvent<HTMLElement>) {
    // Secondary and middle buttons open the menu instead of starting a drag.
    if (event.button !== 0) return;
    stopStroll(false);
    setMenuOpen(false);
    pointerStart.current = { x: event.screenX, y: event.screenY };
    dragged.current = false;
  }

  function handlePointerMove(event: PointerEvent<HTMLElement>) {
    const start = pointerStart.current;
    if (!start || dragged.current) return;
    if (Math.hypot(event.screenX - start.x, event.screenY - start.y) < DRAG_THRESHOLD) return;
    dragged.current = true;
    void getCurrentWindow().startDragging();
  }

  function handlePointerUp(event: PointerEvent<HTMLElement>) {
    if (event.button !== 0 || menuOpen) {
      pointerStart.current = null;
      return;
    }
    if (!dragged.current) {
      if (mood === "warning") {
        if (dictation.state === "error") void native.cancel();
        else setCaptureMood(null);
      } else {
        void captureContext();
      }
    }
    pointerStart.current = null;
  }

  function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    if (mood === "warning") {
      if (dictation.state === "error") void native.cancel();
      else setCaptureMood(null);
    } else {
      void captureContext();
    }
  }

  const menuItems: Array<{ label: string; run: () => void }> = [
    { label: "Open Assistant", run: () => void captureContext() },
    { label: "Start dictation", run: () => void native.start("hands_free") },
    { label: "Hide Buddy", run: () => void native.hideBuddy() },
    { label: "Buddy settings", run: () => void native.showBuddySettings() },
  ];

  const showLiveWave = mood === "push_to_talk" || mood === "hands_free" || mood === "call_recording";
  const waveValues = audio?.bars.length ? audio.bars : Array.from({ length: 12 }, () => 0.08);

  return (
    <main
      aria-label={mood === "warning" ? "VoiceFlow Buddy error; click to dismiss" : "VoiceFlow Buddy; click to capture screen context"}
      className={`buddy-pet buddy-pet--${mood} buddy-pet--walk-${walkDirection}`}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onKeyDown={handleKeyDown}
      onContextMenu={(event) => {
        event.preventDefault();
        setMenuOpen(true);
      }}
      role="button"
      tabIndex={0}
    >
      {menuOpen && (
        <>
          {/* Clicking anywhere else dismisses the menu, as a native flyout does. */}
          <div className="buddy-menu__scrim" onPointerDown={() => setMenuOpen(false)} />
          <ul className="buddy-menu" role="menu" aria-label="Buddy actions">
            {menuItems.map((item) => (
              <li key={item.label}>
                <button
                  type="button"
                  role="menuitem"
                  onPointerUp={(event) => {
                    event.stopPropagation();
                    setMenuOpen(false);
                    item.run();
                  }}
                >
                  {item.label}
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      <span
        aria-hidden="true"
        className="buddy-sprite"
        style={{
          backgroundImage: spriteAsset ? `url('${spriteAsset}')` : undefined,
          backgroundSize: isIdle ? "400% 400%" : isSleeping ? "800% 400%" : isWaving ? "200% 200%" : isWalking ? "400% 200%" : "400% 300%",
          backgroundPosition: isSleeping
            ? `${(frame % 8) * 14.2857}% ${Math.floor(frame / 8) * 33.333}%`
            : isIdle
              ? `${(idleFrame < 16 ? idleFrame : idleFrame - 16) % 4 * 33.333}% ${Math.floor((idleFrame < 16 ? idleFrame : idleFrame - 16) / 4) * 33.333}%`
              : isWaving
              ? `${(frame % 2) * 100}% ${Math.floor(frame / 2) * 100}%`
              : isWalking
                ? `${(frame % 4) * 33.333}% ${Math.floor(frame / 4) * 100}%`
                : isCallRecording
                  ? "100% 0%"
                : `${(frame % 4) * 33.333}% ${Math.floor(frame / 4) * 50}%`,
        }}
      />
      {showLiveWave && (
        <span className={`buddy-face-wave buddy-face-wave--${mood}`} aria-hidden="true">
          {waveValues.slice(0, 12).map((value, index) => (
            <i key={index} style={{ height: `${Math.max(3, Math.round(3 + Math.min(1, value) * 17))}px` }} />
          ))}
        </span>
      )}
      {mood === "capturing" && <span className="buddy-scan-overlay" aria-hidden="true" />}
      {mood === "thinking" && <span className="buddy-thought-spark" aria-hidden="true" />}
      {mood === "analyzing" && <span className="buddy-analysis-particles" aria-hidden="true"><i /><i /><i /></span>}
      {isCallRecording && <span className="buddy-recording-dot" aria-label="Call recording active" />}
    </main>
  );
}
