# VoiceFlow Dev — audit & brainstorm (Nov 2026)

Frame: a one-day budget. The goal is "dictation feels instant and never lies to me",
not "more features". Anything that doesn't make the hold-talk-release loop faster or
more trustworthy is parked.

## Audit: what is actually broken

| # | Problem | Why it matters | Status |
|---|---------|----------------|--------|
| 1 | The dock maps **every** error to "Could not paste automatically · Text copied to clipboard" (`OverlayState.ts`). An empty transcript, a bad Deepgram key, a missing mic, or a history-save failure *after a successful paste* all show the same lie. | This is the bug in the screenshot. | Fixed |
| 2 | Errors are sticky. The hotkey dispatcher only starts a session from `Idle`, and nothing moves `Error → Idle` except Escape. The dock stays on screen and the next hold of Ctrl+Win silently does nothing. | Feels like the app "froze". | Fixed: auto-dismiss after a few seconds |
| 3 | Several `?` early-returns in `dictation::finish` (dictionary load, auto-transform failure) skip `set_error`, leaving the state machine stuck in `Cleaning` with a spinner forever. | Same "frozen" symptom. | Fixed |
| 4 | Focus restore treats `SetForegroundWindow == false` as fatal even when the target is already the foreground window, and gives up after one try. | False "paste failed". | Fixed |
| 5 | A history/database write failure after a good paste reported the dictation as failed. | False "paste failed". | Fixed: logged, dictation still succeeds |
| 6 | The mic → Deepgram channel holds 256 chunks with `try_send`. The keyring lookup + TLS + WebSocket handshake happens *after* the mic starts, so on a slow connection the first words are silently dropped. | "It cut off my first word." | Fixed: bigger buffer |
| 7 | Audio is streamed at the device's native rate (usually 48 kHz). Deepgram is tuned for 16 kHz speech; 48 kHz triples upload bytes for zero accuracy gain. | Latency on weak Wi-Fi. | Fixed: resample to 16 kHz before upload |
| 8 | Live interim text only held Deepgram's *current* segment, so the preview forgot everything you said before the last pause. The dock didn't show live text at all. | No "it's hearing me" feedback. | Fixed: running transcript (finals + interim) streamed into the dock |
| 9 | Spoken-formatting cues matched inside words: "bulletin" → "\n- in", "newline**s**" etc. | Mangled text. | Fixed: word-boundary, case-insensitive |
| 10 | Clipboard is restored 80 ms after Ctrl+V. Slow (Electron) apps read the clipboard late and paste the *old* contents. | Wrong text pasted, intermittently. | Fixed: 250 ms |
| 11 | Overlay re-fetches all settings on every state change. | Minor waste. | Left as is |

## Diverge: ideas on the table

**Voice / latency**
- Resample to 16 kHz (done) · bigger pre-connect buffer (done)
- Pre-warm the Deepgram socket on modifier-down, before the full chord is confirmed
- Send Deepgram `Finalize` on release instead of `CloseStream` and keep the socket for the next utterance (saves the ~300–600 ms handshake every time)
- "Can't hear you" hint when the mic is below the noise floor for 2 s
- Silence auto-stop in hands-free mode

**Streaming**
- Live caption in the dock (done)
- *Stream-typing*: type finalized words straight into the app as you speak. Tempting, but it fights the core design: AI cleanup, backtracking ("scratch that") and dictionary fixes all need the whole utterance. **Parked** — if built, make it an opt-in "raw live mode" that skips cleanup.

**Dock UI**
- Wider pill that grows to show the live caption (done)
- Specific error copy per category (done), success shows word count + app (done)
- Remember dragged position per monitor

**Quality of life**
- "Paste last transcript again" hotkey (tray already has the logic)
- Undo last insertion (send Ctrl+Z once, only if nothing else was typed)
- Retry button on the dock that re-transcribes the kept recovery audio
- Snippets: "insert my email" → expands from the dictionary
- Per-app dictionaries, WPM/time-saved stats on Home

## Provoke

- *Is AI cleanup worth its latency?* Every dictation waits on a second network round trip. For short utterances (< 8 words) deterministic cleanup is usually identical. Skipping the LLM for short text is probably the single biggest perceived-speed win left.
- *Is the clipboard the right transport?* It's the reason for issues 4 and 10. `SendInput` with `KEYEVENTF_UNICODE` types text without touching the clipboard and works in terminals; it's slower for long text. A hybrid (type < 200 chars, paste longer) is worth testing on a real Windows box.

## Converge — strongest directions

1. **Trust**: the dock must only say what actually happened (done).
2. **Perceived speed**: live caption + 16 kHz + no dropped first words (done); short dictations (< 8 words, configurable) now skip the AI cleanup round trip unless they contain spoken code (done). Next is the persistent socket.
3. **Recovery**: a Retry action on errors using the audio that's already kept.

Riskiest assumption: that Deepgram accuracy is unchanged at 16 kHz on the user's mic. Check by
dictating the same sentence on both builds and comparing History.

## Parked
Stream-typing into apps · local Whisper fallback · snippets · per-app dictionaries · stats.
