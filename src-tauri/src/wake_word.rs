//! "Hey Flow": starts the voice agent hands-free. It uses Windows' built-in
//! speech recognizer with a fixed phrase list, so listening is local and
//! free: no audio leaves the machine until the agent itself starts.

use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};
use windows_collections::IIterable;
use windows::{
    core::HSTRING,
    Foundation::TypedEventHandler,
    Media::SpeechRecognition::{
        SpeechContinuousRecognitionResultGeneratedEventArgs, SpeechContinuousRecognitionSession,
        SpeechRecognitionConfidence, SpeechRecognitionListConstraint, SpeechRecognizer, SpeechRecognizerState,
    },
};

use crate::{app_state::AppState, voice_agent};

/// What the agent answers to. Changing the name means changing these.
// Longer phrases are recognized more reliably than short ones, so the full
// product name is accepted too.
const PHRASES: [&str; 5] = ["hey flow", "okay flow", "hi flow", "hey voiceflow", "hey voice flow"];

struct Listener {
    recognizer: SpeechRecognizer,
}

impl Listener {
    fn start(app: &AppHandle) -> windows::core::Result<Self> {
        let recognizer = SpeechRecognizer::new()?;
        let phrases: IIterable<HSTRING> = PHRASES.iter().map(|phrase| HSTRING::from(*phrase)).collect::<Vec<_>>().into();
        recognizer
            .Constraints()?
            .Append(&SpeechRecognitionListConstraint::Create(&phrases)?)?;
        recognizer.CompileConstraintsAsync()?.join()?;
        let session = recognizer.ContinuousRecognitionSession()?;
        // By default the session stops itself after a stretch of silence, and
        // the first phrase after that is missed while it restarts.
        session.SetAutoStopSilenceTimeout(Duration::from_secs(24 * 60 * 60).into())?;
        let app = app.clone();
        session.ResultGenerated(&TypedEventHandler::<
            SpeechContinuousRecognitionSession,
            SpeechContinuousRecognitionResultGeneratedEventArgs,
        >::new(move |_, args| {
            let Some(args) = args.as_ref() else { return Ok(()) };
            // With a phrase list the recognizer tries to fit everything it
            // hears to one of the phrases, so weak matches are other speech.
            let confidence = args.Result()?.Confidence()?;
            if confidence == SpeechRecognitionConfidence::High || confidence == SpeechRecognitionConfidence::Medium {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    if voice_agent::is_running(&state).await {
                        return;
                    }
                    if let Err(error) = voice_agent::start(&app, &state).await {
                        log::warn!("wake word could not start the voice agent: {error}");
                    }
                });
            }
            Ok(())
        }))?;
        session.StartAsync()?.join()?;
        Ok(Self { recognizer })
    }

    /// Windows ends a continuous session after long silences; start it again.
    fn keep_alive(&self) {
        if self.recognizer.State().is_ok_and(|state| state == SpeechRecognizerState::Idle) {
            if let Ok(session) = self.recognizer.ContinuousRecognitionSession() {
                let _ = session.StartAsync().and_then(|start| start.join());
            }
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        if let Ok(session) = self.recognizer.ContinuousRecognitionSession() {
            let _ = session.StopAsync();
        }
    }
}

/// Follows the "wake word" setting for the life of the app: listens while it
/// is on, releases the microphone when it is turned off.
pub fn spawn(app: AppHandle) {
    let spawned = thread::Builder::new().name("voiceflow-wake-word".into()).spawn(move || {
        let mut listener: Option<Listener> = None;
        let mut failed = false;
        loop {
            let enabled = tauri::async_runtime::block_on(app.state::<AppState>().settings.read()).wake_word_enabled;
            match (&listener, enabled) {
                (None, true) if !failed => match Listener::start(&app) {
                    Ok(started) => {
                        log::info!("wake word listening");
                        listener = Some(started);
                    }
                    Err(error) => {
                        // Usually: speech recognition or microphone access is off in Windows.
                        log::warn!("wake word unavailable: {error}");
                        failed = true;
                    }
                },
                (Some(running), true) => running.keep_alive(),
                (_, false) => {
                    listener = None;
                    failed = false;
                }
                _ => {}
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
    if let Err(error) = spawned {
        log::warn!("wake word thread did not start: {error}");
    }
}
