//! VoiceFlow's own sounds (read-aloud answers, start/stop tones, the output
//! test chime), played through the output device chosen in Settings › Audio.
//! They used to play inside the WebView, which always uses the Windows default
//! device and bypasses mixers like SteelSeries Sonar.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use tauri::{AppHandle, Manager};

use crate::{app_state::AppState, audio::playback::Player, errors::AppError};

pub const RATE: u32 = 24_000;
static GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy)]
pub enum Channel {
    /// Read-aloud answers. A new one replaces the old one.
    Speech,
    /// Short UI sounds. Independent of speech so a tone never cuts it off.
    Tone,
}

fn slot(state: &AppState, channel: Channel) -> &tokio::sync::Mutex<Option<(u64, Player)>> {
    match channel {
        Channel::Speech => &state.speech_out,
        Channel::Tone => &state.tone_out,
    }
}

/// Plays 16-bit mono PCM at `rate` on the chosen output device.
pub async fn play(app: &AppHandle, channel: Channel, pcm: Vec<u8>, rate: u32) -> Result<(), AppError> {
    let device = app.state::<AppState>().settings.read().await.output_device_name.clone();
    play_on(app, channel, pcm, rate, device).await
}

/// Like `play`, on a specific device (None = Windows default).
pub async fn play_on(
    app: &AppHandle,
    channel: Channel,
    pcm: Vec<u8>,
    rate: u32,
    device: Option<String>,
) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let player = tokio::task::spawn_blocking(move || Player::start(rate, device.as_deref()))
        .await
        .map_err(|error| AppError::Microphone(error.to_string()))??;
    player.push_pcm16(&pcm);
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
    *slot(&state, channel).lock().await = Some((generation, player));
    // Release the device once this sound has finished (unless replaced).
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(150)).await;
            let state = app.state::<AppState>();
            let guard = slot(&state, channel).lock().await;
            match guard.as_ref() {
                Some((current, player)) if *current == generation => {
                    if !player.is_playing() {
                        // Let the device drain its last buffer before closing.
                        drop(guard);
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        let mut guard = slot(&state, channel).lock().await;
                        if guard.as_ref().is_some_and(|(current, _)| *current == generation) {
                            *guard = None;
                        }
                        return;
                    }
                }
                _ => return,
            }
        }
    });
    Ok(())
}

pub async fn stop(app: &AppHandle, channel: Channel) {
    *slot(&app.state::<AppState>(), channel).lock().await = None;
}

/// A soft sine blip with short fades (no clicks), as 16-bit PCM at RATE.
pub fn tone_pcm(notes: &[(f32, u32)], gain: f32) -> Vec<u8> {
    let mut out = Vec::new();
    for &(frequency, duration_ms) in notes {
        let samples = (RATE as u64 * duration_ms as u64 / 1000) as usize;
        let fade = (RATE as usize / 100).min(samples / 2).max(1); // 10 ms
        for index in 0..samples {
            let envelope = (index.min(samples - 1 - index) as f32 / fade as f32).min(1.0);
            let value = (2.0 * std::f32::consts::PI * frequency * index as f32 / RATE as f32).sin();
            let sample = (value * envelope * gain * i16::MAX as f32) as i16;
            out.extend_from_slice(&sample.to_le_bytes());
        }
    }
    out
}

/// The dictation tones, matching the old WebView versions.
pub fn named_tone(name: &str) -> Option<Vec<u8>> {
    let notes: &[(f32, u32)] = match name {
        "start" => &[(660.0, 90)],
        "stop" => &[(520.0, 90)],
        "success" => &[(880.0, 110)],
        "error" => &[(300.0, 220)],
        // Settings › Audio "Test": a friendly rising pair.
        "test" => &[(660.0, 140), (880.0, 200)],
        _ => return None,
    };
    Some(tone_pcm(notes, if name == "test" { 0.18 } else { 0.06 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tones_have_the_right_length_and_no_click_at_the_edges() {
        let pcm = tone_pcm(&[(440.0, 100)], 0.5);
        assert_eq!(pcm.len(), 2_400 * 2);
        let first = i16::from_le_bytes([pcm[0], pcm[1]]);
        let last = i16::from_le_bytes([pcm[pcm.len() - 2], pcm[pcm.len() - 1]]);
        assert_eq!(first, 0);
        assert!(last.abs() < 200, "{last}");
        assert!(named_tone("start").is_some() && named_tone("nope").is_none());
    }
}
