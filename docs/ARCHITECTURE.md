# VoiceFlow Dev architecture

VoiceFlow Dev is a local-first Tauri v2 application. React owns presentation and user preferences; Rust owns microphone access, provider credentials, dictation state, focused-window targeting, insertion, and persistence. A webview never receives an API key.

## Runtime flow

1. A Windows low-level keyboard hook observes the modifier-only push-to-talk gesture without blocking the hook thread.
2. Rust captures the foreground window and focused control, transitions `Idle -> Starting`, and starts CPAL before opening a provider connection.
3. The audio callback downmixes native input to mono PCM16, writes to a bounded stream, and updates an atomic RMS value. A separate 30 FPS emitter drives the overlay.
4. Deepgram receives buffered and live chunks over a Nova-3 WebSocket. Interim and final segments are reconciled separately.
5. On release, capture stops and the provider is finalized. The raw transcript is normalized with dictionary rules, explicit backtracking is parsed, optional AI cleanup returns validated structured JSON, and deterministic rules run again.
6. Rust restores the captured target, pastes with an application-aware shortcut, optionally performs a validated post-paste action, and restores text clipboard contents where practical.
7. SQLite stores `raw`, `normalized`, `cleaned`, and `final` stages independently. A failed paste retains the final text on the clipboard and in History.

## Trust boundaries

- Deepgram receives mono audio and vocabulary hints only during an active session.
- The configured OpenAI-compatible cleanup endpoint receives normalized transcript text and protected identifiers only when cleanup is enabled.
- API keys are stored in Windows Credential Manager and are never returned to React, written to SQLite, or logged.
- SQLite queries are parameterized. Full transcript text and audio bytes are excluded from default logs.
- Password and secure controls are rejected when Windows exposes that information; context reading is off by default in this slice.

## Concurrency

- The CPAL callback performs bounded, non-blocking work.
- Audio streaming, cleanup, database writes, and insertion run off the Tauri main thread.
- A single native mutex guards the active session. Invalid or overlapping state transitions return typed errors.
- Audio levels use an atomic handoff so network latency cannot stall the visualizer.

## Provider boundaries

`TranscriptionProvider` exposes start, send, and finish operations. `DeepgramProvider` is the production implementation and `MockTranscriptionProvider` is used by tests. Cleanup uses a separate `CleanupProvider` boundary so another OpenAI-compatible endpoint can be substituted without touching dictation orchestration.

## Persistence

The first migration creates all requested product tables plus FTS5 indexing for transcript search. Settings store non-secret configuration only. Credentials use the service name centralized in `src-tauri/src/brand.rs`.

## Current delivery boundary

The working slice covers push-to-talk, hands-free toggle, tap-to-finish behavior, Command Mode, a compact glass recording pill, streaming transcription, deterministic and optional AI cleanup, cleanup style modes, focus restoration, paste fallback, persistent History, tray behavior, secure provider setup, dictionary CRUD, repeated false-start removal, and previewable developer transforms. Auto-apply transforms run after cleanup and before insertion when selected in Settings. Per-application insertion adapters, selected-text context extraction, full transform shortcut persistence, local Whisper, and multi-monitor overlay position persistence remain intentionally staged for the next implementation phase.
