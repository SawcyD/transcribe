use std::path::Path;

use crate::errors::AppError;

use super::CapturedAudio;

pub fn write_pcm16(path: &Path, audio: &CapturedAudio) -> Result<(), AppError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.format.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|error| AppError::Microphone(error.to_string()))?;
    for sample in &audio.samples {
        writer
            .write_sample(*sample)
            .map_err(|error| AppError::Microphone(error.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|error| AppError::Microphone(error.to_string()))
}
