# VoiceFlow UI style

A portable copy of VoiceFlow's look: a calm Windows 11–style dark UI with soft, rounded cards, pill-shaped controls, hairline borders, and colour used only for state. Everything you need is in this file:

1. **[Principles](#principles)**: the rules that make it look like this.
2. **[The stylesheet](#the-stylesheet)**: one CSS block. Paste it into your project as `vf.css`.
3. **[Components](#components)**: copy-paste HTML for each piece.

It has no dependencies and works with plain HTML, React (change `class` to `className`), Vue, Svelte, Tauri or Electron. All classes start with `vf-` so they won't clash with your own. Dark is the default; light mode follows the OS, or you can force it with `<html data-theme="light">` / `"dark"`.

---

## Principles

- **Colour means state.** The neutral UI is greys on near-black. Blue (accent) = active or you. Purple = the assistant or AI. Green = done. Amber = warning. Red = error or destructive. If something is coloured, it is telling you something.
- **Cards, not lines.** Related settings sit in one rounded card (14px), with hairline dividers between rows. Each group has a small uppercase label *above* the card, never inside it.
- **Pills for anything you press or filter.** Buttons, filters, tabs, chips and floating docks are fully rounded (`999px`). Inputs and cards are rounded rectangles (9–14px).
- **One readable column.** Content sits in a centred column about 860px wide (data-heavy pages go up to 1180px), never stretched edge to edge.
- **Hairlines, not shadows.** Use 1px borders at about 8% white. Shadows are only for things that float (docks, bubbles, dialogs).
- **Quiet text hierarchy.** Titles in white, descriptions at 79% opacity, metadata at 55%. Use font weight (600 to 650) for emphasis, not colour.
- **Small, calm motion.** 100–180ms fades and colour transitions. Use opacity only for page entrances, no sliding. Respect `prefers-reduced-motion`.
- **Every state has words.** Empty states say what to do next. Errors say what happened and what to do. Never show a blank panel or a raw error code.

---

## The stylesheet

```css
/* ===== VoiceFlow UI: tokens ===== */
:root {
  --vf-font: "Segoe UI Variable Text", "Segoe UI Variable", "Segoe UI", system-ui, -apple-system, sans-serif;
  --vf-font-display: "Segoe UI Variable Display", "Segoe UI Variable", "Segoe UI", system-ui, -apple-system, sans-serif;
  --vf-font-mono: "Cascadia Code", Consolas, ui-monospace, monospace;

  /* Surfaces (dark) */
  --vf-bg: #1b1c1f;               /* window / page */
  --vf-sidebar: #18191c;
  --vf-card: rgb(255 255 255 / .05); /* cards sit on the page */
  --vf-control: rgb(255 255 255 / .06);
  --vf-control-hover: rgb(255 255 255 / .09);
  --vf-hover: rgb(255 255 255 / .06);  /* row / list hover */
  --vf-stroke: rgb(255 255 255 / .083);
  --vf-stroke-strong: rgb(255 255 255 / .54); /* toggle outlines */
  --vf-float: rgb(32 33 37 / .94);     /* docks, bubbles, toasts */

  /* Text */
  --vf-text: #fff;
  --vf-text-2: rgb(255 255 255 / .79);
  --vf-text-3: rgb(255 255 255 / .55);
  --vf-text-on-accent: #000;

  /* State colours */
  --vf-accent: #4ea3e8;           /* you / active / primary */
  --vf-assistant: #a78bfa;        /* the AI */
  --vf-success: #6ccb5f;
  --vf-warning: #fce100;
  --vf-danger: #ff99a4;
  --vf-star: #f5c542;
  --vf-accent-soft: color-mix(in srgb, var(--vf-accent) 16%, transparent);
  --vf-assistant-soft: color-mix(in srgb, var(--vf-assistant) 16%, transparent);

  /* Shape and depth */
  --vf-radius-card: 14px;
  --vf-radius-control: 9px;
  --vf-radius-pill: 999px;
  --vf-shadow-float: 0 1px 2px rgb(0 0 0 / .18), 0 8px 24px rgb(0 0 0 / .32);
  --vf-ease: cubic-bezier(.2, 0, 0, 1);

  color-scheme: dark;
}

/* Light theme: follows the OS unless forced with data-theme. */
@media (prefers-color-scheme: light) {
  :root:not([data-theme="dark"]) {
    --vf-bg: #f3f3f3; --vf-sidebar: #ebebeb;
    --vf-card: rgb(255 255 255 / .75); --vf-control: rgb(255 255 255 / .8); --vf-control-hover: rgb(249 249 249 / .9);
    --vf-hover: rgb(0 0 0 / .04); --vf-stroke: rgb(0 0 0 / .08); --vf-stroke-strong: rgb(0 0 0 / .45); --vf-float: rgb(252 252 252 / .94);
    --vf-text: #1a1a1a; --vf-text-2: rgb(0 0 0 / .62); --vf-text-3: rgb(0 0 0 / .45); --vf-text-on-accent: #fff;
    --vf-accent: #0067c0; --vf-assistant: #7c5cd6; --vf-success: #0f7b0f; --vf-warning: #9d5d00; --vf-danger: #c42b1c; --vf-star: #c98a00;
    --vf-shadow-float: 0 1px 2px rgb(0 0 0 / .08), 0 8px 24px rgb(0 0 0 / .14);
    color-scheme: light;
  }
}
:root[data-theme="light"] {
  --vf-bg: #f3f3f3; --vf-sidebar: #ebebeb;
  --vf-card: rgb(255 255 255 / .75); --vf-control: rgb(255 255 255 / .8); --vf-control-hover: rgb(249 249 249 / .9);
  --vf-hover: rgb(0 0 0 / .04); --vf-stroke: rgb(0 0 0 / .08); --vf-stroke-strong: rgb(0 0 0 / .45); --vf-float: rgb(252 252 252 / .94);
  --vf-text: #1a1a1a; --vf-text-2: rgb(0 0 0 / .62); --vf-text-3: rgb(0 0 0 / .45); --vf-text-on-accent: #fff;
  --vf-accent: #0067c0; --vf-assistant: #7c5cd6; --vf-success: #0f7b0f; --vf-warning: #9d5d00; --vf-danger: #c42b1c; --vf-star: #c98a00;
  --vf-shadow-float: 0 1px 2px rgb(0 0 0 / .08), 0 8px 24px rgb(0 0 0 / .14);
  color-scheme: light;
}

/* ===== Base ===== */
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; color: var(--vf-text); background: var(--vf-bg); font: 14px/1.45 var(--vf-font); -webkit-font-smoothing: antialiased; }
h1, h2, h3, p { margin: 0; }
button, input, select, textarea { font: inherit; color: inherit; }
:focus-visible { outline: 2px solid var(--vf-accent); outline-offset: 2px; }

/* ===== Layout ===== */
.vf-shell { display: grid; grid-template-columns: 236px minmax(0, 1fr); height: 100vh; }
.vf-shell--compact { grid-template-columns: 60px minmax(0, 1fr); }
.vf-sidebar { display: flex; flex-direction: column; gap: 2px; padding: 0 10px 12px; background: var(--vf-sidebar); border-right: 1px solid var(--vf-stroke); }
.vf-brand { display: flex; align-items: center; gap: 10px; min-height: 56px; padding: 0 8px; font-weight: 650; }
.vf-brand__mark { display: grid; place-items: center; width: 28px; height: 28px; color: var(--vf-accent); border-radius: 9px; background: var(--vf-accent-soft); }
.vf-brand__toggle { margin-left: auto; }
.vf-nav-item { display: flex; align-items: center; gap: 12px; min-height: 38px; padding: 0 12px; color: var(--vf-text); text-decoration: none; border-radius: 10px; font-size: 13.5px; transition: background-color 120ms; }
.vf-nav-item svg { color: var(--vf-text-2); }
.vf-nav-item:hover { background: var(--vf-hover); }
.vf-nav-item[aria-current="page"] { background: var(--vf-accent-soft); font-weight: 600; }
.vf-nav-item[aria-current="page"] svg { color: var(--vf-accent); }
.vf-sidebar__footer { margin-top: auto; padding-top: 8px; border-top: 1px solid var(--vf-stroke); }
.vf-content { overflow-y: auto; }
.vf-page { width: min(860px, calc(100% - 64px)); margin: 0 auto; padding: 36px 0 64px; animation: vf-fade 180ms var(--vf-ease) both; }
.vf-page--wide { width: min(1180px, calc(100% - 64px)); }
.vf-page-header { margin-bottom: 26px; }
.vf-page-header h1 { margin-bottom: 6px; font: 650 26px/1.2 var(--vf-font-display); letter-spacing: -.02em; }
.vf-page-header p { color: var(--vf-text-2); font-size: 13px; }
@keyframes vf-fade { from { opacity: 0; } to { opacity: 1; } }

/* ===== Groups: label above, one card of rows ===== */
.vf-group { margin: 0 0 28px; }
.vf-group__label { margin: 0 0 8px 4px; color: var(--vf-text-2); font-size: 11.5px; font-weight: 600; letter-spacing: .06em; text-transform: uppercase; }
.vf-group__hint { margin: -2px 0 10px 4px; color: var(--vf-text-2); font-size: 12.5px; }
.vf-card { overflow: hidden; border: 1px solid var(--vf-stroke); border-radius: var(--vf-radius-card); background: var(--vf-card); box-shadow: 0 1px 2px rgb(0 0 0 / .12); }
.vf-card--padded { padding: 20px 22px; }
.vf-row { display: flex; align-items: center; gap: 20px; min-height: 62px; padding: 13px 18px; border-bottom: 1px solid var(--vf-stroke); transition: background-color 120ms; }
.vf-row:last-child { border-bottom: 0; }
.vf-row:hover { background: var(--vf-hover); }
.vf-row__text { flex: 1; min-width: 0; }
.vf-row__title { font-size: 13.5px; font-weight: 500; }
.vf-row__desc { margin-top: 2px; color: var(--vf-text-2); font-size: 12.5px; }

/* ===== Buttons ===== */
.vf-btn { display: inline-flex; align-items: center; gap: 8px; min-height: 32px; padding: 0 14px; color: var(--vf-text); background: var(--vf-control); border: 1px solid var(--vf-stroke); border-radius: var(--vf-radius-pill); font-size: 13px; font-weight: 550; cursor: pointer; transition: background-color 100ms, transform 100ms; }
.vf-btn:hover:not(:disabled) { background: var(--vf-control-hover); }
.vf-btn:active:not(:disabled) { transform: scale(.97); }
.vf-btn:disabled { opacity: .45; cursor: default; }
.vf-btn--primary { color: var(--vf-text-on-accent); background: var(--vf-accent); border-color: transparent; box-shadow: inset 0 1px 0 rgb(255 255 255 / .12), 0 4px 14px color-mix(in srgb, var(--vf-accent) 25%, transparent); }
.vf-btn--primary:hover:not(:disabled) { background: color-mix(in srgb, var(--vf-accent) 90%, black); }
.vf-btn--danger { color: #fff; background: color-mix(in srgb, var(--vf-danger) 80%, black); border-color: transparent; }
.vf-btn--assistant { background: var(--vf-assistant-soft); border-color: transparent; }
.vf-text-btn { display: inline-flex; align-items: center; gap: 4px; min-height: 28px; padding: 0 10px; color: var(--vf-accent); background: transparent; border: 0; border-radius: var(--vf-radius-pill); font-size: 12.5px; font-weight: 550; cursor: pointer; }
.vf-text-btn:hover { background: var(--vf-accent-soft); }
.vf-icon-btn { display: inline-grid; place-items: center; width: 32px; height: 32px; padding: 0; color: var(--vf-text-2); background: transparent; border: 0; border-radius: 50%; cursor: pointer; }
.vf-icon-btn:hover { color: var(--vf-text); background: var(--vf-hover); }
.vf-icon-btn--danger:hover { color: var(--vf-danger); background: color-mix(in srgb, var(--vf-danger) 12%, transparent); }

/* ===== Inputs ===== */
.vf-input, .vf-select { height: 32px; padding: 0 12px; color: var(--vf-text); background: var(--vf-control); border: 1px solid var(--vf-stroke); border-radius: var(--vf-radius-control); font-size: 13px; }
.vf-input:focus, .vf-select:focus { border-color: color-mix(in srgb, var(--vf-accent) 60%, transparent); outline: none; }
.vf-search { width: 100%; max-width: 380px; padding-left: 34px; border-radius: var(--vf-radius-pill); background: var(--vf-control) url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='14' height='14' fill='none' stroke='%23999' stroke-width='1.6'%3E%3Ccircle cx='6' cy='6' r='4.5'/%3E%3Cpath d='M9.5 9.5L13 13'/%3E%3C/svg%3E") no-repeat 12px center; }
.vf-textarea { width: 100%; min-height: 44px; padding: 8px 12px; color: var(--vf-text); background: var(--vf-control); border: 1px solid var(--vf-stroke); border-radius: 14px; resize: vertical; }

/* Toggle switch: <label class="vf-toggle"><input type="checkbox"><span></span> On</label> */
.vf-toggle { display: inline-flex; align-items: center; gap: 8px; color: var(--vf-text-2); font-size: 12px; cursor: pointer; }
.vf-toggle input { position: absolute; opacity: 0; pointer-events: none; }
.vf-toggle span { position: relative; width: 40px; height: 20px; border: 1px solid var(--vf-stroke-strong); border-radius: 10px; transition: background-color 150ms, border-color 150ms; }
.vf-toggle span::after { position: absolute; top: 3px; left: 4px; width: 12px; height: 12px; content: ""; border-radius: 50%; background: var(--vf-text-2); transition: left 150ms, background-color 150ms; }
.vf-toggle input:checked + span { background: var(--vf-accent); border-color: transparent; }
.vf-toggle input:checked + span::after { left: 22px; background: var(--vf-text-on-accent); }
.vf-toggle input:focus-visible + span { outline: 2px solid var(--vf-accent); outline-offset: 2px; }

/* Segmented tabs */
.vf-segmented { display: inline-flex; gap: 2px; padding: 3px; border: 1px solid var(--vf-stroke); border-radius: var(--vf-radius-pill); background: var(--vf-control); }
.vf-segmented button { min-height: 28px; padding: 0 14px; color: var(--vf-text); background: transparent; border: 0; border-radius: var(--vf-radius-pill); font-size: 12.5px; cursor: pointer; }
.vf-segmented button:hover { background: var(--vf-hover); }
.vf-segmented button[aria-selected="true"] { background: var(--vf-accent-soft); font-weight: 600; }

/* Filter pill (a <select> that lights up when set) */
.vf-filter { height: 30px; padding: 0 28px 0 12px; color: var(--vf-text); background: var(--vf-control) url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' fill='none' stroke='%23999' stroke-width='1.5'%3E%3Cpath d='M1 1l4 4 4-4'/%3E%3C/svg%3E") no-repeat right 11px center; border: 1px solid var(--vf-stroke); border-radius: var(--vf-radius-pill); font-size: 12.5px; appearance: none; cursor: pointer; }
.vf-filter[data-active="true"] { background-color: var(--vf-accent-soft); border-color: color-mix(in srgb, var(--vf-accent) 45%, transparent); }

/* ===== Small pieces ===== */
.vf-chip { display: inline-flex; align-items: center; gap: 5px; padding: 3px 10px; color: var(--vf-text-2); border-radius: var(--vf-radius-pill); background: var(--vf-control); font-size: 12px; }
.vf-status { padding: 1px 8px; border-radius: var(--vf-radius-pill); font-size: 11px; font-weight: 600; line-height: 18px; }
.vf-status--success { color: var(--vf-success); background: color-mix(in srgb, var(--vf-success) 14%, transparent); }
.vf-status--warning { color: var(--vf-warning); background: color-mix(in srgb, var(--vf-warning) 14%, transparent); }
.vf-status--danger { color: var(--vf-danger); background: color-mix(in srgb, var(--vf-danger) 14%, transparent); }
.vf-status--assistant { color: var(--vf-assistant); background: var(--vf-assistant-soft); }
.vf-kbd { display: inline-flex; align-items: center; justify-content: center; min-width: 26px; height: 24px; padding: 0 8px; border-radius: 7px; background: var(--vf-control); box-shadow: inset 0 -1px 0 rgb(0 0 0 / .25), inset 0 0 0 1px var(--vf-stroke); font: 11.5px var(--vf-font); }
.vf-keys { display: inline-flex; gap: 5px; }
.vf-stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 10px; }
.vf-stat { display: grid; gap: 2px; padding: 14px 16px; border: 1px solid var(--vf-stroke); border-radius: var(--vf-radius-card); background: var(--vf-card); }
.vf-stat b { font: 650 22px var(--vf-font-display); font-variant-numeric: tabular-nums; }
.vf-stat span { color: var(--vf-text-2); font-size: 12px; }
.vf-empty { display: grid; gap: 4px; justify-items: center; padding: 28px 16px; color: var(--vf-text-2); text-align: center; border: 1px dashed var(--vf-stroke); border-radius: var(--vf-radius-card); font-size: 13px; }
.vf-empty strong { color: var(--vf-text); }
.vf-info { display: flex; align-items: flex-start; gap: 10px; padding: 12px 14px; border: 1px solid var(--vf-stroke); border-left: 3px solid var(--vf-warning); border-radius: var(--vf-radius-card); background: var(--vf-card); font-size: 13px; }
.vf-info--error { border-left-color: var(--vf-danger); }
.vf-info strong { display: block; }
.vf-info p { color: var(--vf-text-2); }

/* Hero card (a page's main status) */
.vf-hero { padding: 22px 24px; border: 1px solid var(--vf-stroke); border-radius: 18px; background: linear-gradient(135deg, color-mix(in srgb, var(--vf-accent) 9%, var(--vf-card)), var(--vf-card) 60%); }
.vf-hero__eyebrow { color: var(--vf-text-2); font-size: 11.5px; font-weight: 600; letter-spacing: .06em; text-transform: uppercase; }
.vf-hero__title { margin-top: 4px; font: 600 28px/34px var(--vf-font-display); letter-spacing: -.02em; }

/* List with selectable items (e.g. history) */
.vf-list { display: grid; gap: 2px; padding: 4px; border: 1px solid var(--vf-stroke); border-radius: var(--vf-radius-card); background: var(--vf-card); }
.vf-list__group { padding: 12px 12px 6px; color: var(--vf-text-2); font-size: 11px; font-weight: 600; letter-spacing: .06em; text-transform: uppercase; }
.vf-item { display: grid; gap: 4px; width: 100%; padding: 10px 12px; color: var(--vf-text); text-align: left; background: transparent; border: 0; border-radius: 10px; cursor: pointer; transition: background-color 100ms; }
.vf-item:hover { background: var(--vf-hover); }
.vf-item[aria-current="true"] { background: var(--vf-accent-soft); }
.vf-item__top { display: flex; align-items: center; gap: 8px; }
.vf-item__top b { font-size: 13px; font-weight: 600; }
.vf-item__top time { margin-left: auto; color: var(--vf-text-2); font-size: 12px; font-variant-numeric: tabular-nums; }
.vf-item__body { display: -webkit-box; overflow: hidden; font-size: 12.5px; opacity: .86; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.vf-item__meta { color: var(--vf-text-3); font-size: 11.5px; }

/* Chat */
.vf-chat { display: flex; flex-direction: column; gap: 10px; padding: 16px; }
.vf-msg { max-width: 82%; padding: 10px 14px; border-radius: 16px; font-size: 13.5px; line-height: 1.55; }
.vf-msg--you { align-self: flex-end; background: var(--vf-accent-soft); border-bottom-right-radius: 6px; }
.vf-msg--ai { align-self: flex-start; background: color-mix(in srgb, var(--vf-assistant) 10%, transparent); border: 1px solid color-mix(in srgb, var(--vf-assistant) 22%, transparent); border-bottom-left-radius: 6px; }
.vf-msg code { padding: 1px 5px; border-radius: 5px; background: rgb(127 127 127 / .16); font: 12.5px var(--vf-font-mono); }

/* ===== Floating pieces ===== */
/* Dock: the floating pill (status + waveform + actions) */
.vf-dock { display: flex; align-items: center; gap: 10px; width: 360px; height: 48px; padding: 0 8px 0 16px; border-radius: var(--vf-radius-pill); background: var(--vf-float); box-shadow: 0 0 0 1px color-mix(in srgb, var(--vf-text) 14%, transparent), var(--vf-shadow-float); backdrop-filter: blur(30px) saturate(1.1); }
.vf-dock--success { box-shadow: 0 0 0 1px color-mix(in srgb, var(--vf-success) 45%, transparent), var(--vf-shadow-float); }
.vf-dock--error { box-shadow: 0 0 0 1px color-mix(in srgb, var(--vf-danger) 55%, transparent), var(--vf-shadow-float); }
.vf-dock--assistant { box-shadow: 0 0 0 1px color-mix(in srgb, var(--vf-assistant) 55%, transparent), 0 0 24px color-mix(in srgb, var(--vf-assistant) 22%, transparent); }
.vf-dock__dot { flex: 0 0 auto; width: 8px; height: 8px; border-radius: 50%; background: var(--vf-accent); animation: vf-pulse 1.6s ease-out infinite; }
.vf-dock--assistant .vf-dock__dot { width: 10px; height: 10px; background: var(--vf-assistant); box-shadow: 0 0 12px var(--vf-assistant); }
.vf-dock__text { flex: 1; min-width: 0; overflow: hidden; font-size: 12.5px; text-overflow: ellipsis; white-space: nowrap; }
.vf-dock__text b { font-weight: 600; }
.vf-dock__text small { color: var(--vf-text-2); font-size: 12px; }
.vf-dock__time { color: var(--vf-text-2); font-size: 11.5px; font-variant-numeric: tabular-nums; }
/* Caption that keeps the newest words visible: <span class="vf-caption"><bdi>…text…</bdi></span> */
.vf-caption { display: block; overflow: hidden; direction: rtl; text-align: left; text-overflow: ellipsis; white-space: nowrap; }
.vf-caption bdi { direction: ltr; unicode-bidi: isolate; }
.vf-wave { display: flex; align-items: center; justify-content: center; gap: 2px; width: 56px; height: 14px; }
.vf-wave i { flex: 0 0 3px; height: 100%; border-radius: 1px; background: var(--vf-accent); transform: scaleY(var(--h, .1)); transition: transform 90ms var(--vf-ease); }
.vf-dock--assistant .vf-wave i { background: var(--vf-assistant); }
@keyframes vf-pulse { 0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--vf-accent) 55%, transparent); } 100% { box-shadow: 0 0 0 7px transparent; } }

/* Speech bubble (AI answer), tail points down at a dock */
.vf-bubble { position: relative; width: min(420px, 100%); border-radius: 18px; background: var(--vf-float); box-shadow: 0 0 0 1px color-mix(in srgb, var(--vf-assistant) 45%, transparent), var(--vf-shadow-float); backdrop-filter: blur(28px); }
.vf-bubble::after { position: absolute; bottom: -6px; left: 50%; width: 12px; height: 12px; content: ""; background: inherit; box-shadow: 1px 1px 0 color-mix(in srgb, var(--vf-assistant) 45%, transparent); transform: translateX(-50%) rotate(45deg); }
.vf-bubble__head { display: flex; align-items: center; gap: 8px; min-height: 44px; padding: 0 8px 0 14px; font-size: 13px; }
.vf-bubble__body { padding: 2px 14px 12px; font-size: 13.5px; line-height: 1.55; }
.vf-bubble__foot { display: flex; align-items: center; gap: 4px; padding: 8px 10px 10px; border-top: 1px solid var(--vf-stroke); }

/* Toast: bottom-centre confirmation ("All changes saved") */
.vf-toast { position: fixed; bottom: 22px; left: 50%; display: inline-flex; align-items: center; gap: 8px; padding: 8px 14px; border: 1px solid var(--vf-stroke); border-radius: var(--vf-radius-pill); background: var(--vf-float); box-shadow: var(--vf-shadow-float); font-size: 12.5px; font-weight: 550; transform: translateX(-50%); }

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation-duration: .01ms !important; animation-iteration-count: 1 !important; transition-duration: .01ms !important; }
}
@media (max-width: 760px) {
  .vf-shell { grid-template-columns: 60px minmax(0, 1fr); }
  .vf-page { width: calc(100% - 32px); }
}
```

---

## Components

Icons in these examples are [Lucide](https://lucide.dev) (`lucide-react` or the plain SVGs) at 16px with a 1.75 stroke. Any outline icon set at the same size works.

### App shell with sidebar

```html
<div class="vf-shell">
  <aside class="vf-sidebar">
    <div class="vf-brand">
      <span class="vf-brand__mark">◆</span> MyApp
      <button class="vf-icon-btn vf-brand__toggle" aria-label="Collapse sidebar">⟨</button>
    </div>
    <a class="vf-nav-item" href="#" aria-current="page">🏠 Home</a>
    <a class="vf-nav-item" href="#">🕘 History</a>
    <div class="vf-sidebar__footer">
      <a class="vf-nav-item" href="#">⚙️ Settings</a>
    </div>
  </aside>
  <main class="vf-content">
    <div class="vf-page">
      <header class="vf-page-header">
        <h1>Settings</h1>
        <p>Changes save automatically.</p>
      </header>
      <!-- groups go here -->
    </div>
  </main>
</div>
```

### Settings group: label, then a card of rows

```html
<section class="vf-group">
  <h2 class="vf-group__label">General</h2>
  <div class="vf-card">
    <div class="vf-row">
      <div class="vf-row__text">
        <div class="vf-row__title">Start with Windows</div>
        <div class="vf-row__desc">Keep shortcuts available after you sign in.</div>
      </div>
      <label class="vf-toggle"><input type="checkbox" checked><span></span> On</label>
    </div>
    <div class="vf-row">
      <div class="vf-row__text"><div class="vf-row__title">Theme</div></div>
      <select class="vf-select"><option>Use system setting</option></select>
    </div>
  </div>
</section>
```

### Buttons

```html
<button class="vf-btn vf-btn--primary">Start dictation</button>
<button class="vf-btn">Paste again</button>
<button class="vf-btn vf-btn--assistant">Follow up</button>
<button class="vf-text-btn">View all</button>
<button class="vf-icon-btn vf-icon-btn--danger" aria-label="Delete">🗑</button>
```

Use one primary button per view. Put destructive actions behind an icon button plus a confirm dialog, never a big red button.

### Tabs, filters, search

```html
<div class="vf-segmented" role="tablist">
  <button role="tab" aria-selected="true">Chat</button>
  <button role="tab" aria-selected="false">Saved chats · 2</button>
</div>

<input class="vf-input vf-search" placeholder="Search what you said…">
<select class="vf-filter" data-active="true"><option>★ Starred</option></select>
```

### Chips, status, keyboard keys, stats

```html
<span class="vf-chip">● Ready</span>
<span class="vf-status vf-status--success">Inserted</span>
<span class="vf-status vf-status--warning">Copied only</span>
<span class="vf-keys"><kbd class="vf-kbd">Ctrl</kbd><kbd class="vf-kbd">Win</kbd></span>

<div class="vf-stats">
  <div class="vf-stat"><b>1,240</b><span>words today</span></div>
  <div class="vf-stat"><b>18 min</b><span>saved vs typing</span></div>
</div>
```

### Hero card, info bar, empty state

```html
<section class="vf-hero">
  <div class="vf-hero__eyebrow">Dictation</div>
  <div class="vf-hero__title">Ready</div>
</section>

<div class="vf-info"><div><strong>Not configured</strong><p>Add your API key to start.</p></div></div>

<div class="vf-empty"><strong>No dictations yet</strong><p>Hold Ctrl + Win and start speaking.</p></div>
```

### Selectable list (grouped by day)

```html
<div class="vf-list">
  <div class="vf-list__group">Today</div>
  <button class="vf-item" aria-current="true">
    <span class="vf-item__top"><b>Visual Studio Code</b><time>6:15 PM</time></span>
    <span class="vf-item__body">Refactor the session limit so it reads the configured minutes.</span>
    <span class="vf-item__meta">Push to talk · 14 words · 6s</span>
  </button>
</div>
```

### Chat

```html
<div class="vf-card vf-chat">
  <div class="vf-msg vf-msg--you">What does this error mean?</div>
  <div class="vf-msg vf-msg--ai">The value was <code>moved</code> before it was used again.</div>
</div>
```

### Floating dock, bubble, toast

```html
<!-- You / recording -->
<div class="vf-dock">
  <span class="vf-dock__dot"></span>
  <span class="vf-dock__text"><span class="vf-caption"><bdi>…the newest words stay visible</bdi></span></span>
  <span class="vf-wave"><i style="--h:.3"></i><i style="--h:.8"></i><i style="--h:.5"></i><i style="--h:.9"></i><i style="--h:.4"></i></span>
  <span class="vf-dock__time">0:14</span>
</div>

<!-- States: add vf-dock--success / vf-dock--error / vf-dock--assistant -->
<div class="vf-dock vf-dock--error">
  <span class="vf-dock__dot" style="background:var(--vf-danger);animation:none"></span>
  <span class="vf-dock__text"><b>Couldn't paste</b> <small>· Copied — press Ctrl+V</small></span>
</div>

<!-- AI answer, sits above the dock -->
<div class="vf-bubble">
  <div class="vf-bubble__head"><span class="vf-status vf-status--assistant">Assistant</span></div>
  <div class="vf-bubble__body">Here's what's going on…</div>
  <div class="vf-bubble__foot"><button class="vf-btn vf-btn--assistant">Follow up</button></div>
</div>

<div class="vf-toast">✓ All changes saved</div>
```

Drive the waveform bars by setting `--h` (0 to 1) on each `<i>` from your audio level. Silence should be flat, so never animate fake motion.

---

## Copy rules (how text sounds)

- Say what happened and what to do: "Couldn't paste · Copied, press Ctrl+V", not "Insertion error 0x5".
- Name things by what they do for the user: "Ask about your screen", not "Command Mode".
- Use sentence case everywhere ("Start dictation", not "Start Dictation").
- Settings save automatically. Confirm quietly with a toast, never with a dialog.
- Empty states tell you the next step: "Hold Ctrl + Win and start speaking."
