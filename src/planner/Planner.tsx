import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowRight, Check, ChevronLeft, ChevronRight, Plus, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { events, isTauri, native, type PlannerTask } from "../lib/native";

/** YYYY-MM-DD in local time. */
function isoDay(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function shiftDay(day: string, by: number): string {
  const [year, month, date] = day.split("-").map(Number);
  return isoDay(new Date(year, month - 1, date + by));
}

function dayLabel(day: string): string {
  const today = isoDay(new Date());
  const name = day === today ? "Today" : day === shiftDay(today, 1) ? "Tomorrow" : day === shiftDay(today, -1) ? "Yesterday" : null;
  const [year, month, date] = day.split("-").map(Number);
  const formatted = new Date(year, month - 1, date).toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" });
  return name ? `${name} · ${formatted}` : formatted;
}

/** "14:30" -> "2:30 PM", in the user's locale. */
function timeLabel(time: string): string {
  const [hours, minutes] = time.split(":").map(Number);
  return new Date(2000, 0, 1, hours, minutes).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

/** Chains of tasks linked by "after", for the flow chart: each starts at a task nothing on this day comes before. */
function chains(tasks: PlannerTask[]): PlannerTask[][] {
  const byId = new Map(tasks.map((task) => [task.id, task]));
  const linked = tasks.filter((task) => task.after && byId.has(task.after));
  if (linked.length === 0) return [];
  const next = (id: string) => tasks.filter((task) => task.after === id);
  const roots = tasks.filter((task) => !(task.after && byId.has(task.after)) && next(task.id).length > 0);
  return roots.map((root) => {
    const chain = [root];
    const seen = new Set([root.id]);
    for (let current = next(root.id)[0]; current && !seen.has(current.id); current = next(current.id)[0]) {
      chain.push(current);
      seen.add(current.id);
    }
    return chain;
  });
}

/**
 * The day planner: a glass card with the day's timeline on the left and the
 * order tasks depend on each other on the right. Flow edits the same plan
 * by voice (see planner.rs); this window follows its changes live.
 */
export function Planner() {
  const [day, setDay] = useState(() => isoDay(new Date()));
  const [tasks, setTasks] = useState<PlannerTask[]>([]);
  const [title, setTitle] = useState("");
  const [time, setTime] = useState("");
  // Tasks on their way out (done moving to another day), so they can fade first.
  const [leaving, setLeaving] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => void native.plannerTasks().then(setTasks), []);

  useEffect(() => {
    load();
    const subscriptions = Promise.all([events.plannerChanged(load), events.plannerShowDay(setDay)]);
    return () => void subscriptions.then((items) => items.forEach((unlisten) => unlisten()));
  }, [load]);

  const dayTasks = useMemo(() => tasks.filter((task) => task.day === day), [tasks, day]);
  const timed = dayTasks.filter((task) => task.time).sort((a, b) => (a.time ?? "").localeCompare(b.time ?? ""));
  const anytime = dayTasks.filter((task) => !task.time);
  const flows = useMemo(() => chains(dayTasks), [dayTasks]);
  const remaining = dayTasks.filter((task) => !task.done).length;

  const save = async (task: PlannerTask) => {
    setError(null);
    try {
      await native.plannerSaveTask(task);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
    load();
  };

  const leaveThen = (id: string, action: () => Promise<unknown>) => {
    setLeaving((current) => new Set(current).add(id));
    window.setTimeout(() => {
      void action().finally(() => {
        setLeaving((current) => {
          const rest = new Set(current);
          rest.delete(id);
          return rest;
        });
        load();
      });
    }, 220);
  };

  const add = async () => {
    if (!title.trim()) return;
    await save({ id: "", title: title.trim(), day, time: time || null, done: false, after: null });
    setTitle("");
    setTime("");
  };

  const chip = (task: PlannerTask) => (
    <li key={task.id} className={`planner-task${task.done ? " planner-task--done" : ""}${leaving.has(task.id) ? " planner-task--leaving" : ""}`}>
      <button type="button" className="planner-task__check" aria-label={task.done ? `Mark "${task.title}" not done` : `Mark "${task.title}" done`} onClick={() => void save({ ...task, done: !task.done })}>
        {task.done && <Check size={11} strokeWidth={3} />}
      </button>
      <span className="planner-task__title">{task.title}</span>
      <span className="planner-task__actions">
        <button type="button" title="Move to the next day" aria-label={`Move "${task.title}" to the next day`} onClick={() => leaveThen(task.id, () => native.plannerSaveTask({ ...task, day: shiftDay(task.day, 1) }))}>
          <ArrowRight size={13} />
        </button>
        <button type="button" title="Remove" aria-label={`Remove "${task.title}"`} onClick={() => leaveThen(task.id, () => native.plannerRemoveTask(task.id))}>
          <X size={13} />
        </button>
      </span>
    </li>
  );

  return (
    <div className="planner">
      <header className="planner__header" data-tauri-drag-region>
        <div data-tauri-drag-region>
          <h1 data-tauri-drag-region>Plan</h1>
          <p data-tauri-drag-region>{dayTasks.length === 0 ? "Nothing planned" : remaining === 0 ? "All done" : `${remaining} to go`}</p>
        </div>
        <nav className="planner__days" aria-label="Day">
          <button type="button" aria-label="Previous day" onClick={() => setDay(shiftDay(day, -1))}>
            <ChevronLeft size={15} />
          </button>
          <button type="button" className="planner__day" title="Back to today" onClick={() => setDay(isoDay(new Date()))}>
            {dayLabel(day)}
          </button>
          <button type="button" aria-label="Next day" onClick={() => setDay(shiftDay(day, 1))}>
            <ChevronRight size={15} />
          </button>
        </nav>
        <button type="button" className="planner__close" aria-label="Close planner" onClick={() => isTauri() && void getCurrentWindow().hide()}>
          <X size={15} />
        </button>
      </header>

      <div className="planner__body" key={day}>
        <section className="planner__timeline" aria-label="Timeline">
          {dayTasks.length === 0 ? (
            <p className="planner__empty">Add a task below, or ask Flow: “add study for my algorithms exam to tomorrow morning”.</p>
          ) : (
            <>
              {timed.length > 0 && (
                <ol className="planner__track">
                  {timed.map((task) => (
                    <div key={task.id} className="planner__slot">
                      <time>{timeLabel(task.time ?? "")}</time>
                      {chip(task)}
                    </div>
                  ))}
                </ol>
              )}
              {anytime.length > 0 && (
                <>
                  <h2>Any time</h2>
                  <ul className="planner__anytime">{anytime.map(chip)}</ul>
                </>
              )}
            </>
          )}
        </section>

        <section className="planner__flow" aria-label="Order">
          <h2>Flow</h2>
          {flows.length === 0 ? (
            <p className="planner__empty">When one task has to happen before another, it shows here. Try: “after I draft the outline, review the PR”.</p>
          ) : (
            flows.map((chain) => (
              <ol key={chain[0].id} className="planner__chain">
                {chain.map((task, index) => (
                  <li key={task.id} className={`planner__node${task.done ? " planner__node--done" : ""}`} style={{ animationDelay: `${index * 70}ms` }}>
                    {task.done && <Check size={11} strokeWidth={3} aria-label="done" />}
                    {task.title}
                  </li>
                ))}
              </ol>
            ))
          )}
        </section>
      </div>

      <form
        className="planner__add"
        onSubmit={(event) => {
          event.preventDefault();
          void add();
        }}
      >
        <input aria-label="New task" placeholder={`Add a task for ${dayLabel(day).split(" · ")[0].toLowerCase()}…`} value={title} maxLength={200} onChange={(event) => setTitle(event.target.value)} />
        <input aria-label="Time (optional)" type="time" value={time} onChange={(event) => setTime(event.target.value)} />
        <button type="submit" aria-label="Add task" disabled={!title.trim()}>
          <Plus size={15} />
        </button>
      </form>
      {error && <p className="planner__error">{error}</p>}
    </div>
  );
}
