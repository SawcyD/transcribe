use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::{errors::AppError, models::AudioLevelPayload};

use super::audio_level::{decibels, peak, rms, spectral_bars, BandSmoother, LevelSmoother};

#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    pub sample_rate: u32,
}

pub struct CaptureSession {
    stream: Stream,
    started: Instant,
    stop: Arc<AtomicBool>,
    visualizer_thread: Option<thread::JoinHandle<()>>,
    samples: Arc<Mutex<Vec<i16>>>,
    pub format: AudioFormat,
    /// Rate of the PCM sent to the transcription provider. Speech models are
    /// tuned for 16 kHz, so higher device rates are downsampled before upload.
    pub stream_sample_rate: u32,
}

/// Target rate for streamed audio. 48 kHz devices would otherwise upload 3x
/// the bytes with no accuracy gain, which shows up as latency on weak Wi-Fi.
pub const STREAM_SAMPLE_RATE: u32 = 16_000;

/// Averaging decimator: each output sample is the mean of the input samples
/// in its window, which doubles as a cheap anti-aliasing low-pass for speech.
/// State carries across callbacks so non-integer ratios (44.1k -> 16k) stay
/// phase-correct.
pub(crate) struct Downsampler {
    step: f64,
    phase: f64,
    sum: i64,
    count: i64,
}

impl Downsampler {
    pub(crate) fn new(source_rate: u32, target_rate: u32) -> Self {
        Self {
            step: (source_rate as f64 / target_rate.max(1) as f64).max(1.0),
            phase: 0.0,
            sum: 0,
            count: 0,
        }
    }

    pub(crate) fn process(&mut self, input: &[i16]) -> Vec<i16> {
        if self.step <= 1.0 {
            return input.to_vec();
        }
        let mut output = Vec::with_capacity((input.len() as f64 / self.step) as usize + 1);
        for &sample in input {
            self.sum += sample as i64;
            self.count += 1;
            self.phase += 1.0;
            if self.phase >= self.step {
                output.push((self.sum / self.count) as i16);
                self.sum = 0;
                self.count = 0;
                self.phase -= self.step;
            }
        }
        output
    }
}

pub struct CapturedAudio {
    pub samples: Vec<i16>,
    pub format: AudioFormat,
    pub duration_ms: i64,
}

pub fn list_input_devices() -> Result<Vec<String>, AppError> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|error| AppError::Microphone(error.to_string()))?;
    let mut names = devices
        .filter_map(|device| device.name().ok())
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    Ok(names)
}

impl CaptureSession {
    pub fn start(
        app: AppHandle,
        session_id: String,
        requested_device: Option<&str>,
        noise_floor_db: f32,
    ) -> Result<(Self, mpsc::Receiver<Vec<u8>>), AppError> {
        let host = cpal::default_host();
        let device = match requested_device {
            Some(name) => host
                .input_devices()
                .map_err(|error| AppError::Microphone(error.to_string()))?
                .find(|device| device.name().ok().as_deref() == Some(name))
                .ok_or_else(|| {
                    AppError::Microphone(format!("configured input device '{name}' is unavailable"))
                })?,
            None => host.default_input_device().ok_or_else(|| {
                AppError::Microphone("no default input device is available".into())
            })?,
        };
        let supported = device
            .default_input_config()
            .map_err(|error| AppError::Microphone(error.to_string()))?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let format = AudioFormat {
            sample_rate: config.sample_rate.0,
        };
        let source_channels = config.channels as usize;
        let stream_sample_rate = format.sample_rate.min(STREAM_SAMPLE_RATE);
        // The provider socket connects *after* the mic starts. Callbacks arrive
        // roughly every 10 ms, so 256 slots (~2.5 s) could overflow on a slow
        // handshake and silently drop the first words. 4096 is ~40 s of slack.
        let (sender, receiver) = mpsc::channel::<Vec<u8>>(4096);
        let samples = Arc::new(Mutex::new(Vec::with_capacity(
            format.sample_rate as usize * 60,
        )));
        let latest_rms = Arc::new(AtomicU32::new(0.0f32.to_bits()));
        let latest_peak = Arc::new(AtomicU32::new(0.0f32.to_bits()));
        let stop = Arc::new(AtomicBool::new(false));

        let stream = match sample_format {
            SampleFormat::F32 => build_f32_stream(
                &device,
                &config,
                source_channels,
                Downsampler::new(format.sample_rate, stream_sample_rate),
                sender,
                &samples,
                &latest_rms,
                &latest_peak,
            )?,
            SampleFormat::I16 => build_i16_stream(
                &device,
                &config,
                source_channels,
                Downsampler::new(format.sample_rate, stream_sample_rate),
                sender,
                &samples,
                &latest_rms,
                &latest_peak,
            )?,
            SampleFormat::U16 => build_u16_stream(
                &device,
                &config,
                source_channels,
                Downsampler::new(format.sample_rate, stream_sample_rate),
                sender,
                &samples,
                &latest_rms,
                &latest_peak,
            )?,
            format => {
                return Err(AppError::Microphone(format!(
                    "unsupported input sample format: {format:?}"
                )))
            }
        };
        stream
            .play()
            .map_err(|error| AppError::Microphone(error.to_string()))?;

        let visualizer_stop = Arc::clone(&stop);
        let visualizer_samples = Arc::clone(&samples);
        let visualizer_sample_rate = format.sample_rate;
        let visualizer = thread::Builder::new()
            .name("voiceflow-audio-level".into())
            .spawn(move || {
                let mut smoother = LevelSmoother::new(noise_floor_db);
                let mut band_smoother = BandSmoother::new(12);
                while !visualizer_stop.load(Ordering::Relaxed) {
                    let raw_rms = f32::from_bits(latest_rms.load(Ordering::Relaxed));
                    let raw_peak = f32::from_bits(latest_peak.load(Ordering::Relaxed));
                    let level = smoother.update(raw_rms);
                    let recent_samples = visualizer_samples
                        .lock()
                        .ok()
                        .map(|samples| {
                            let start = samples.len().saturating_sub(512);
                            samples[start..].to_vec()
                        })
                        .unwrap_or_default();
                    let spectrum =
                        spectral_bars(&recent_samples, visualizer_sample_rate, 12, level);
                    let payload = AudioLevelPayload {
                        session_id: session_id.clone(),
                        rms: raw_rms,
                        peak: raw_peak,
                        decibels: decibels(raw_rms),
                        bars: band_smoother.update(&spectrum),
                    };
                    let _ = app.emit("audio-level", payload);
                    thread::sleep(Duration::from_millis(33));
                }
            })
            .map_err(|error| AppError::Microphone(error.to_string()))?;

        Ok((
            Self {
                stream,
                started: Instant::now(),
                stop,
                visualizer_thread: Some(visualizer),
                samples,
                format,
                stream_sample_rate,
            },
            receiver,
        ))
    }

    pub fn stop(mut self) -> CapturedAudio {
        let _ = self.stream.pause();
        self.stop.store(true, Ordering::Relaxed);
        drop(self.stream);
        if let Some(handle) = self.visualizer_thread.take() {
            let _ = handle.join();
        }
        let samples = self
            .samples
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default();
        CapturedAudio {
            samples,
            format: self.format,
            duration_ms: self.started.elapsed().as_millis() as i64,
        }
    }
}

fn process_samples(
    mono: Vec<i16>,
    downsampler: &mut Downsampler,
    sender: &mpsc::Sender<Vec<u8>>,
    stored: &Arc<Mutex<Vec<i16>>>,
    latest_rms: &Arc<AtomicU32>,
    latest_peak: &Arc<AtomicU32>,
) {
    if mono.is_empty() {
        return;
    }
    latest_rms.store(rms(&mono).to_bits(), Ordering::Relaxed);
    latest_peak.store(peak(&mono).to_bits(), Ordering::Relaxed);
    if let Ok(mut buffer) = stored.lock() {
        buffer.extend_from_slice(&mono);
    }
    let streamed = downsampler.process(&mono);
    if streamed.is_empty() {
        return;
    }
    let mut bytes = Vec::with_capacity(streamed.len() * 2);
    for sample in streamed {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    if sender.try_send(bytes).is_err() {
        log::warn!("audio stream buffer full; dropping a chunk");
    }
}

fn downmix_f32(data: &[f32], channels: usize) -> Vec<i16> {
    data.chunks(channels)
        .map(|frame| {
            let average = frame.iter().copied().sum::<f32>() / frame.len().max(1) as f32;
            (average.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
        })
        .collect()
}
fn downmix_i16(data: &[i16], channels: usize) -> Vec<i16> {
    data.chunks(channels)
        .map(|frame| {
            (frame.iter().map(|sample| *sample as i32).sum::<i32>() / frame.len().max(1) as i32)
                as i16
        })
        .collect()
}
fn downmix_u16(data: &[u16], channels: usize) -> Vec<i16> {
    data.chunks(channels)
        .map(|frame| {
            let average =
                frame.iter().map(|sample| *sample as i64).sum::<i64>() / frame.len().max(1) as i64;
            (average - 32_768).clamp(i16::MIN as i64, i16::MAX as i64) as i16
        })
        .collect()
}

macro_rules! build_stream {
    ($name:ident, $sample:ty, $downmix:ident) => {
        fn $name(
            device: &cpal::Device,
            config: &StreamConfig,
            channels: usize,
            mut downsampler: Downsampler,
            sender: mpsc::Sender<Vec<u8>>,
            stored: &Arc<Mutex<Vec<i16>>>,
            latest_rms: &Arc<AtomicU32>,
            latest_peak: &Arc<AtomicU32>,
        ) -> Result<Stream, AppError> {
            let stored = Arc::clone(stored);
            let latest_rms = Arc::clone(latest_rms);
            let latest_peak = Arc::clone(latest_peak);
            device
                .build_input_stream(
                    config,
                    move |data: &[$sample], _| {
                        process_samples(
                            $downmix(data, channels),
                            &mut downsampler,
                            &sender,
                            &stored,
                            &latest_rms,
                            &latest_peak,
                        )
                    },
                    |error| log::error!("microphone stream error: {error}"),
                    None,
                )
                .map_err(|error| AppError::Microphone(error.to_string()))
        }
    };
}

build_stream!(build_f32_stream, f32, downmix_f32);
build_stream!(build_i16_stream, i16, downmix_i16);
build_stream!(build_u16_stream, u16, downmix_u16);

#[cfg(test)]
mod tests {
    use super::Downsampler;

    #[test]
    fn downsamples_48k_to_16k_by_averaging() {
        let mut downsampler = Downsampler::new(48_000, 16_000);
        let output = downsampler.process(&[3, 6, 9, 30, 30, 30, -3, -3, -3]);
        assert_eq!(output, vec![6, 30, -3]);
    }

    #[test]
    fn keeps_phase_across_callbacks_for_fractional_ratios() {
        let mut downsampler = Downsampler::new(44_100, 16_000);
        let total: usize = (0..100)
            .map(|_| downsampler.process(&[0; 441]).len())
            .sum();
        // 44_100 input samples must become ~16_000 output samples.
        assert!((15_999..=16_000).contains(&total), "{total}");
    }

    #[test]
    fn passes_through_rates_at_or_below_target() {
        let mut downsampler = Downsampler::new(16_000, 16_000);
        assert_eq!(downsampler.process(&[1, 2, 3]), vec![1, 2, 3]);
    }
}
