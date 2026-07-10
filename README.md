# VoiceFlow Dev

VoiceFlow Dev is a local-first Windows voice dictation utility for developers. It is a Tauri v2 desktop app with a React/Vite interface, Rust audio pipeline, Deepgram Nova-3 streaming transcription, OpenAI-compatible cleanup/transforms, SQLite history, and Windows Credential Manager secrets.

## Run locally

Requirements: Windows 10/11, Node.js 20+, pnpm, Rust stable with the MSVC toolchain, and WebView2.

```powershell
pnpm install
pnpm tauri dev
```

For browser-only UI work (native commands use safe fallbacks):

```powershell
pnpm dev
```

Validation commands:

```powershell
pnpm lint
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

## First launch

Open Settings and enter a Deepgram API key. It is stored in Windows Credential Manager under `dev.voiceflow.desktop`; it is never written to SQLite or exposed to React. A cleanup key is optional: deterministic cleanup and local transforms still work without it.

Hold `Ctrl + Win` to dictate into the previously focused Windows field. Press `Ctrl + Win + Space` to toggle hands-free mode; press it again, click the checkmark, or say `finish dictation` to finish. `Escape` cancels. The overlay is a separate borderless always-on-top Tauri window.

Hold `Ctrl + Win + Alt` for Command Mode. It records with `command` mode metadata so the same voice-action and developer transforms can be used for explicit commands.

## Delivery status

Complete in this slice:

- Push-to-talk capture with immediate CPAL microphone start and 30 FPS RMS/peak visualizer.
- Deepgram Nova-3 streaming with interim results, keyterms, confidence, and word timings.
- Four preserved transcript stages, deterministic dictionary/backtracking cleanup, optional structured AI cleanup, and “press enter” safety gate.
- Focus restoration, clipboard paste fallback, terminal Shift+Insert handling, clipboard restoration, SQLite history, metrics, tray, secure credentials, session limit, and recovery copy behavior.
- Hands-free toggle, dictionary add/edit/delete workflow, Polish and Prompt Engineer previews with copy/replace actions and local fallbacks.
- A quiet glass recording pill with tap-to-finish hands-free behavior, cleanup styles (balanced, casual, developer, code literal), repeated false-start removal, and offline-safe developer task, bug report, commit message, and documentation transforms.

Staged for the next slice: application-specific insertion adapters, selected-text context extraction, full transform preset persistence/shortcuts, local Whisper, multi-monitor overlay position persistence, and installer signing.

## Windows permissions

The app needs microphone permission, access to the foreground window for focus restoration, and permission to register a low-level keyboard hook. Windows Defender or enterprise policy may prompt before allowing the global shortcut hook. No account system or cloud sync is required.
