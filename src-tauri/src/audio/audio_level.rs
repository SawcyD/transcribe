pub fn rms(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum = samples
        .iter()
        .map(|sample| {
            let value = *sample as f64 / i16::MAX as f64;
            value * value
        })
        .sum::<f64>();
    (sum / samples.len() as f64).sqrt() as f32
}

pub fn peak(samples: &[i16]) -> f32 {
    samples
        .iter()
        .map(|sample| sample.unsigned_abs() as f32 / i16::MAX as f32)
        .fold(0.0, f32::max)
}

pub fn decibels(value: f32) -> f32 {
    20.0 * value.max(0.000_01).log10()
}

#[derive(Debug, Clone)]
pub struct LevelSmoother {
    current: f32,
    attack: f32,
    release: f32,
    noise_floor_db: f32,
}

impl LevelSmoother {
    pub fn new(noise_floor_db: f32) -> Self {
        Self {
            current: 0.0,
            attack: 0.58,
            release: 0.16,
            noise_floor_db,
        }
    }

    pub fn update(&mut self, raw_rms: f32) -> f32 {
        let db = decibels(raw_rms);
        let normalized = if db <= self.noise_floor_db {
            0.0
        } else {
            ((db - self.noise_floor_db) / (0.0 - self.noise_floor_db))
                .clamp(0.0, 1.0)
                .powf(0.72)
        };
        let coefficient = if normalized > self.current {
            self.attack
        } else {
            self.release
        };
        self.current += (normalized - self.current) * coefficient;
        if self.current < 0.012 {
            self.current = 0.0;
        }
        self.current
    }
}

/// Estimates energy in evenly spaced voice-frequency bands for the overlay.
/// This intentionally stays lightweight: it samples the most recent window and
/// runs on the visualizer thread, never on the realtime microphone callback.
pub fn spectral_bars(samples: &[i16], sample_rate: u32, count: usize, level: f32) -> Vec<f32> {
    if count == 0 {
        return Vec::new();
    }
    if samples.len() < 32 || sample_rate == 0 {
        return vec![level.clamp(0.03, 1.0); count];
    }

    let window = samples.len().min(512);
    let samples = &samples[samples.len() - window..];
    let min_frequency = 110.0f32;
    let max_frequency = (sample_rate as f32 * 0.45).min(7_000.0).max(min_frequency);
    let ratio = if count > 1 {
        (max_frequency / min_frequency).powf(1.0 / (count - 1) as f32)
    } else {
        1.0
    };

    (0..count)
        .map(|index| {
            let frequency = min_frequency * ratio.powi(index as i32);
            let phase_step = 2.0 * std::f32::consts::PI * frequency / sample_rate as f32;
            let mut real = 0.0f32;
            let mut imaginary = 0.0f32;
            for (sample_index, sample) in samples.iter().enumerate() {
                let normalized = *sample as f32 / i16::MAX as f32;
                let window_position =
                    sample_index as f32 / (window.saturating_sub(1).max(1)) as f32;
                let window_weight =
                    0.5 - 0.5 * (2.0 * std::f32::consts::PI * window_position).cos();
                let phase = phase_step * sample_index as f32;
                real += normalized * window_weight * phase.cos();
                imaginary -= normalized * window_weight * phase.sin();
            }
            let magnitude = (real * real + imaginary * imaginary).sqrt() * 2.0 / window as f32;
            (magnitude * 9.0 + level * 0.18).clamp(0.03, 1.0)
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct BandSmoother {
    current: Vec<f32>,
    attack: f32,
    release: f32,
}

impl BandSmoother {
    pub fn new(count: usize) -> Self {
        Self {
            current: vec![0.03; count],
            attack: 0.42,
            release: 0.18,
        }
    }

    pub fn update(&mut self, target: &[f32]) -> Vec<f32> {
        if self.current.len() != target.len() {
            self.current = vec![0.03; target.len()];
        }
        for (current, next) in self.current.iter_mut().zip(target.iter().copied()) {
            let coefficient = if next > *current {
                self.attack
            } else {
                self.release
            };
            *current += (next - *current) * coefficient;
        }
        self.current.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_is_zero_for_silence() {
        assert_eq!(rms(&[0; 128]), 0.0);
    }

    #[test]
    fn rms_tracks_a_known_signal() {
        let level = rms(&[i16::MAX, i16::MIN + 1]);
        assert!((level - 1.0).abs() < 0.001);
    }

    #[test]
    fn smoothing_attacks_faster_than_it_releases() {
        let mut smoother = LevelSmoother::new(-52.0);
        let attacked = smoother.update(0.8);
        let released = smoother.update(0.0);
        assert!(attacked > 0.5);
        assert!(released > attacked * 0.7);
    }

    #[test]
    fn spectral_bars_keep_a_stable_count() {
        let bars = spectral_bars(&[0; 512], 48_000, 12, 0.2);
        assert_eq!(bars.len(), 12);
        assert!(bars.iter().all(|value| *value >= 0.03));
    }

    #[test]
    fn band_smoother_attacks_and_releases() {
        let mut smoother = BandSmoother::new(1);
        let rising = smoother.update(&[1.0])[0];
        let falling = smoother.update(&[0.03])[0];
        assert!(rising > 0.03);
        assert!(falling > 0.03);
        assert!(falling < rising);
    }
}
