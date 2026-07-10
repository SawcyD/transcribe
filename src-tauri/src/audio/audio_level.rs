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

pub fn bar_values(level: f32, count: usize, frame: u64) -> Vec<f32> {
    let center = (count.saturating_sub(1)) as f32 / 2.0;
    (0..count)
        .map(|index| {
            let distance = if center == 0.0 {
                0.0
            } else {
                (index as f32 - center).abs() / center
            };
            let envelope = 1.0 - distance * 0.38;
            let phase = (frame as f32 * 0.11 + index as f32 * 1.73).sin() * 0.07;
            (level * (envelope + phase)).clamp(0.03, 1.0)
        })
        .collect()
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
    fn center_bars_have_a_larger_envelope() {
        let bars = bar_values(0.8, 12, 0);
        assert!(bars[5] > bars[0]);
    }
}
