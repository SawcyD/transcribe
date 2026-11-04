//! Streams 16-bit PCM (the voice agent's speech) to the default speakers as it
//! arrives. Audio is queued at the source rate and resampled on the fly to the
//! device's rate and channel count inside the output callback.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};

use crate::errors::AppError;

pub struct Player {
    _stream: Stream,
    queue: Arc<Mutex<VecDeque<f32>>>,
    level: Arc<AtomicU32>,
}

/// Linear-interpolating resampler state, advanced once per output frame.
pub(crate) struct Resampler {
    step: f64,
    position: f64,
    current: f32,
    next: f32,
}

impl Resampler {
    pub(crate) fn new(source_rate: u32, target_rate: u32) -> Self {
        Self {
            step: source_rate as f64 / target_rate.max(1) as f64,
            position: 0.0,
            current: 0.0,
            next: 0.0,
        }
    }

    /// Produces one output sample, pulling source samples as needed. Returns
    /// silence (and holds position) when the queue runs dry.
    pub(crate) fn sample(&mut self, queue: &mut VecDeque<f32>) -> f32 {
        while self.position >= 1.0 {
            self.position -= 1.0;
            self.current = self.next;
            match queue.pop_front() {
                Some(value) => self.next = value,
                None => {
                    self.next = 0.0;
                    self.current = 0.0;
                    self.position = 0.0;
                    return 0.0;
                }
            }
        }
        let value = self.current + (self.next - self.current) * self.position as f32;
        self.position += self.step;
        value
    }
}

/// Names of every output device Windows offers (speakers, headphones, and
/// virtual devices such as SteelSeries Sonar's Game/Chat/Media channels).
pub fn list_output_devices() -> Result<Vec<String>, AppError> {
    let host = cpal::default_host();
    let devices = host
        .output_devices()
        .map_err(|error| AppError::Microphone(error.to_string()))?;
    let mut names = devices.filter_map(|device| device.name().ok()).collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    Ok(names)
}

/// The chosen output device, or the system default when none is chosen or the
/// chosen one is gone (unplugged, Sonar closed).
fn output_device(name: Option<&str>) -> Result<cpal::Device, AppError> {
    let host = cpal::default_host();
    if let Some(name) = name.filter(|name| !name.is_empty()) {
        if let Some(device) = host
            .output_devices()
            .ok()
            .and_then(|mut devices| devices.find(|device| device.name().ok().as_deref() == Some(name)))
        {
            return Ok(device);
        }
        log::warn!("output device '{name}' is unavailable; using the system default");
    }
    host.default_output_device()
        .ok_or_else(|| AppError::Microphone("no speakers or headphones are available".into()))
}

impl Player {
    pub fn start(source_rate: u32, device_name: Option<&str>) -> Result<Self, AppError> {
        let device = output_device(device_name)?;
        let supported = device
            .default_output_config()
            .map_err(|error| AppError::Microphone(error.to_string()))?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let channels = config.channels as usize;
        let queue = Arc::new(Mutex::new(VecDeque::<f32>::with_capacity(source_rate as usize * 10)));
        let level = Arc::new(AtomicU32::new(0f32.to_bits()));
        let mut resampler = Resampler::new(source_rate, config.sample_rate.0);
        let callback_queue = Arc::clone(&queue);
        let callback_level = Arc::clone(&level);
        let mut fill = move |frames: usize, mut write: Box<dyn FnMut(usize, f32) + '_>| {
            let Ok(mut queue) = callback_queue.lock() else {
                return;
            };
            let mut energy = 0f32;
            for frame in 0..frames {
                let value = resampler.sample(&mut queue);
                energy += value * value;
                for channel in 0..channels {
                    write(frame * channels + channel, value);
                }
            }
            let rms = (energy / frames.max(1) as f32).sqrt();
            callback_level.store(rms.to_bits(), Ordering::Relaxed);
        };
        let error = |error| log::error!("speaker stream error: {error}");
        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    let frames = data.len() / channels;
                    fill(frames, Box::new(|index, value| data[index] = value));
                },
                error,
                None,
            ),
            SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data: &mut [i16], _| {
                    let frames = data.len() / channels;
                    fill(frames, Box::new(|index, value| data[index] = (value.clamp(-1.0, 1.0) * i16::MAX as f32) as i16));
                },
                error,
                None,
            ),
            SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data: &mut [u16], _| {
                    let frames = data.len() / channels;
                    fill(frames, Box::new(|index, value| data[index] = ((value.clamp(-1.0, 1.0) + 1.0) * 32_767.5) as u16));
                },
                error,
                None,
            ),
            other => {
                return Err(AppError::Microphone(format!("unsupported speaker sample format: {other:?}")))
            }
        }
        .map_err(|error| AppError::Microphone(error.to_string()))?;
        stream.play().map_err(|error| AppError::Microphone(error.to_string()))?;
        Ok(Self {
            _stream: stream,
            queue,
            level,
        })
    }

    /// Queues little-endian 16-bit mono PCM.
    pub fn push_pcm16(&self, bytes: &[u8]) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.extend(
                bytes
                    .chunks_exact(2)
                    .map(|pair| i16::from_le_bytes([pair[0], pair[1]]) as f32 / i16::MAX as f32),
            );
        }
    }

    /// Drops anything not yet played (the user interrupted).
    pub fn clear(&self) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.clear();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.queue.lock().map(|queue| !queue.is_empty()).unwrap_or(false)
    }

    /// Recent output loudness, 0–1, for the agent's waveform.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::Resampler;
    use std::collections::VecDeque;

    #[test]
    fn upsamples_24k_to_48k_with_interpolation() {
        let mut queue: VecDeque<f32> = vec![0.0, 1.0, 1.0, 0.0].into();
        let mut resampler = Resampler::new(24_000, 48_000);
        let out: Vec<f32> = (0..8).map(|_| resampler.sample(&mut queue)).collect();
        // Two output samples per input sample, halfway points interpolated.
        assert!(out.windows(2).all(|pair| (pair[0] - pair[1]).abs() <= 0.5 + 1e-6));
        assert!(out.iter().any(|value| (*value - 1.0).abs() < 1e-6));
        assert!(out.iter().any(|value| (*value - 0.5).abs() < 1e-6));
    }

    #[test]
    fn outputs_silence_when_starved() {
        let mut queue = VecDeque::new();
        let mut resampler = Resampler::new(24_000, 48_000);
        assert_eq!(resampler.sample(&mut queue), 0.0);
        queue.push_back(0.8);
        // Recovers as soon as audio arrives again.
        let samples: Vec<f32> = (0..4).map(|_| resampler.sample(&mut queue)).collect();
        assert!(samples.iter().any(|value| *value > 0.0));
    }
}
