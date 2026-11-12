//! "Hey Flow": starts the voice agent hands-free. A small on-device keyword
//! spotter (sherpa-onnx) listens on the microphone chosen in Settings, so no
//! audio leaves the machine until the agent itself starts.
//!
//! The phrases live in `resources/kws/keywords.txt`, written as the model's
//! sound pieces. To change them, re-encode the new phrase with the model's
//! `bpe.model` (sherpa-onnx's `text2token` tool does this).

use std::{
    ffi::CString,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use sherpa_rs::sherpa_rs_sys as sys;
use tauri::{AppHandle, Manager};

use crate::{app_state::AppState, voice_agent};

/// How sure the spotter must be (0-1). Raise it if Flow wakes up when nobody
/// called it; lower it if it misses you.
const THRESHOLD: f32 = 0.2;
/// Extra weight given to the wake phrases. Higher = easier to trigger.
const BOOST: f32 = 1.5;

/// A streaming keyword spotter over the bundled model.
struct Spotter {
    spotter: *const sys::SherpaOnnxKeywordSpotter,
    stream: *const sys::SherpaOnnxOnlineStream,
}

impl Spotter {
    fn new(model: &Path) -> Result<Self, String> {
        let path = |name: &str| {
            let file = model.join(name);
            if !file.exists() {
                return Err(format!("wake word model file is missing: {}", file.display()));
            }
            CString::new(file.to_string_lossy().as_bytes()).map_err(|error| error.to_string())
        };
        let (encoder, decoder, joiner) = (path("encoder.int8.onnx")?, path("decoder.int8.onnx")?, path("joiner.int8.onnx")?);
        let (tokens, keywords) = (path("tokens.txt")?, path("keywords.txt")?);
        let provider = CString::new("cpu").expect("static string");
        // SAFETY: an all-zero config is the C API's "unset" state (null strings,
        // zero numbers). Every pointer set below outlives the create call,
        // which copies what it needs.
        unsafe {
            let mut config: sys::SherpaOnnxKeywordSpotterConfig = std::mem::zeroed();
            config.feat_config.sample_rate = 16_000;
            config.feat_config.feature_dim = 80;
            config.model_config.transducer.encoder = encoder.as_ptr();
            config.model_config.transducer.decoder = decoder.as_ptr();
            config.model_config.transducer.joiner = joiner.as_ptr();
            config.model_config.tokens = tokens.as_ptr();
            config.model_config.provider = provider.as_ptr();
            config.model_config.num_threads = 1;
            config.max_active_paths = 4;
            config.num_trailing_blanks = 1;
            config.keywords_score = BOOST;
            config.keywords_threshold = THRESHOLD;
            config.keywords_file = keywords.as_ptr();
            let spotter = sys::SherpaOnnxCreateKeywordSpotter(&config);
            if spotter.is_null() {
                return Err("the wake word model could not be loaded".into());
            }
            let stream = sys::SherpaOnnxCreateKeywordStream(spotter);
            if stream.is_null() {
                sys::SherpaOnnxDestroyKeywordSpotter(spotter);
                return Err("the wake word stream could not be created".into());
            }
            Ok(Self { spotter, stream })
        }
    }

    /// Feeds the next stretch of mono audio. True when a wake phrase just ended.
    fn heard(&mut self, samples: &[f32], sample_rate: u32) -> bool {
        let mut hit = false;
        // SAFETY: both handles are valid until drop; the sample pointer and
        // length describe `samples`, which the call copies.
        unsafe {
            sys::SherpaOnnxOnlineStreamAcceptWaveform(self.stream, sample_rate as i32, samples.as_ptr(), samples.len() as i32);
            while sys::SherpaOnnxIsKeywordStreamReady(self.spotter, self.stream) == 1 {
                sys::SherpaOnnxDecodeKeywordStream(self.spotter, self.stream);
                let result = sys::SherpaOnnxGetKeywordResult(self.spotter, self.stream);
                if result.is_null() {
                    continue;
                }
                if !(*result).keyword.is_null() && *(*result).keyword != 0 {
                    hit = true;
                    // Start fresh, or the same phrase would be reported again.
                    sys::SherpaOnnxResetKeywordStream(self.spotter, self.stream);
                }
                sys::SherpaOnnxDestroyKeywordResult(result);
            }
        }
        hit
    }
}

impl Drop for Spotter {
    fn drop(&mut self) {
        // SAFETY: created in `new`, destroyed exactly once here.
        unsafe {
            sys::SherpaOnnxDestroyOnlineStream(self.stream);
            sys::SherpaOnnxDestroyKeywordSpotter(self.spotter);
        }
    }
}

/// An open microphone delivering mono audio at the device's own rate (the
/// spotter resamples).
struct Microphone {
    _stream: cpal::Stream,
    audio: mpsc::Receiver<Vec<f32>>,
    sample_rate: u32,
}

fn open_microphone(name: Option<&str>) -> Result<Microphone, String> {
    let host = cpal::default_host();
    let device = match name {
        Some(name) => host
            .input_devices()
            .map_err(|error| error.to_string())?
            .find(|device| device.name().ok().as_deref() == Some(name))
            .ok_or_else(|| format!("microphone '{name}' is unavailable"))?,
        None => host.default_input_device().ok_or("no microphone is available")?,
    };
    let supported = device.default_input_config().map_err(|error| error.to_string())?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let channels = (config.channels as usize).max(1);
    let (sender, audio) = mpsc::channel::<Vec<f32>>();
    let on_error = |error| log::warn!("wake word microphone error: {error}");
    let stream = match format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _: &_| {
                let _ = sender.send(data.chunks(channels).map(|frame| frame.iter().sum::<f32>() / frame.len() as f32).collect());
            },
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _: &_| {
                let _ = sender.send(
                    data.chunks(channels)
                        .map(|frame| frame.iter().map(|sample| *sample as f32).sum::<f32>() / frame.len() as f32 / 32_768.0)
                        .collect(),
                );
            },
            on_error,
            None,
        ),
        other => return Err(format!("unsupported microphone format: {other:?}")),
    }
    .map_err(|error| error.to_string())?;
    stream.play().map_err(|error| error.to_string())?;
    Ok(Microphone { _stream: stream, audio, sample_rate: config.sample_rate.0 })
}

/// The bundled model: next to the app when installed, in the source tree in
/// development.
fn model_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .resolve("resources/kws", tauri::path::BaseDirectory::Resource)
        .ok()
        .filter(|dir| dir.join("keywords.txt").exists())
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/kws"))
}

/// Follows the "wake word" setting for the life of the app: listens on the
/// chosen microphone while it is on, and releases the microphone when off.
pub fn spawn(app: AppHandle) {
    let spawned = thread::Builder::new().name("voiceflow-wake-word".into()).spawn(move || {
        // (which microphone, the open microphone, the spotter)
        let mut listening: Option<(Option<String>, Microphone, Spotter)> = None;
        let mut failed = false;
        loop {
            let state = app.state::<AppState>();
            let (enabled, wanted) = {
                let settings = tauri::async_runtime::block_on(state.settings.read());
                (settings.wake_word_enabled, settings.microphone_name.clone())
            };
            if !enabled || listening.as_ref().is_some_and(|(current, ..)| *current != wanted) {
                listening = None;
                failed = false;
            }
            if enabled && listening.is_none() && !failed {
                match open_microphone(wanted.as_deref()).and_then(|microphone| Ok((microphone, Spotter::new(&model_dir(&app))?))) {
                    Ok((microphone, spotter)) => {
                        log::info!("wake word listening on {}", wanted.as_deref().unwrap_or("the default microphone"));
                        listening = Some((wanted, microphone, spotter));
                    }
                    Err(error) => {
                        log::warn!("wake word unavailable: {error}");
                        failed = true;
                    }
                }
            }
            let Some((_, microphone, spotter)) = listening.as_mut() else {
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            // Listen for about a second, then look at the settings again.
            // While a conversation is running the audio is thrown away, so
            // Flow can't wake itself up.
            let busy = tauri::async_runtime::block_on(voice_agent::is_running(&state));
            let until = Instant::now() + Duration::from_secs(1);
            let mut lost = false;
            while Instant::now() < until {
                match microphone.audio.recv_timeout(Duration::from_millis(200)) {
                    Ok(samples) if !busy && spotter.heard(&samples, microphone.sample_rate) => {
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
                        break;
                    }
                    Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        lost = true;
                        break;
                    }
                }
            }
            if lost {
                log::warn!("wake word microphone stopped; reopening");
                listening = None;
            }
        }
    });
    if let Err(error) = spawned {
        log::warn!("wake word thread did not start: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 16 kHz mono 16-bit WAV, as written by tests/fixtures.
    fn wav(name: &str) -> Vec<f32> {
        let bytes = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap();
        let data = bytes.windows(4).position(|window| window == b"data").unwrap() + 8;
        bytes[data..].chunks_exact(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]]) as f32 / 32_768.0).collect()
    }

    /// Streams a clip in 10 ms chunks, like the microphone does, then a second of silence.
    fn wakes_on(name: &str) -> bool {
        let mut spotter = Spotter::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/kws")).unwrap();
        let mut samples = wav(name);
        samples.extend(std::iter::repeat(0.0).take(16_000));
        samples.chunks(160).fold(false, |hit, chunk| spotter.heard(chunk, 16_000) || hit)
    }

    #[test]
    fn wakes_on_hey_flow_but_not_on_other_speech() {
        assert!(wakes_on("hey_flow.wav"), "\"Hey Flow\" should wake it");
        assert!(!wakes_on("other_speech.wav"), "ordinary speech should not wake it");
    }
}
