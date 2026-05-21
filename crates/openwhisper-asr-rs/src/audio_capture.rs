use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaptureDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Vec<CaptureDeviceInfo> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok())
        .unwrap_or_default();

    let Ok(devices) = host.input_devices() else {
        return vec![];
    };

    devices
        .enumerate()
        .map(|(index, device)| {
            let name = device
                .name()
                .unwrap_or_else(|_| format!("Unknown input device #{index}"));
            CaptureDeviceInfo {
                id: format!("input-{index}"),
                is_default: !default_name.is_empty() && name == default_name,
                name,
            }
        })
        .collect()
}

#[allow(dead_code)]
pub fn mixdown_to_mono_f32(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let channel_count = channels.max(1) as usize;
    interleaved
        .chunks(channel_count)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

#[allow(dead_code)]
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| *sample as f32 / i16::MAX as f32)
        .collect()
}

#[allow(dead_code)]
pub fn u16_to_f32(samples: &[u16]) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| (*sample as f32 - 32768.0) / 32768.0)
        .collect()
}

#[allow(dead_code)]
pub fn write_debug_wav(path: &Path, samples: &[f32], sample_rate_hz: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate_hz,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("failed to create debug wav at {}", path.display()))?;
    for sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        writer.write_sample((clamped * i16::MAX as f32) as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{i16_to_f32, mixdown_to_mono_f32, u16_to_f32, write_debug_wav};

    #[test]
    fn stereo_mixdown_preserves_timing_while_normalizing_to_worker_mono_format() {
        let mono = mixdown_to_mono_f32(&[1.0, -1.0, 0.5, 0.25], 2);

        assert_eq!(mono, vec![0.0, 0.375]);
    }

    #[test]
    fn integer_pcm_conversion_maps_to_f32_audio_range() {
        assert_eq!(i16_to_f32(&[0]), vec![0.0]);
        assert_eq!(u16_to_f32(&[32768]), vec![0.0]);
    }

    #[test]
    fn debug_wav_is_written_for_replayable_audio_reports() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("debug.wav");

        write_debug_wav(&path, &[0.0, 0.5, -0.5], 16_000).unwrap();

        assert!(path.exists());
        assert!(path.metadata().unwrap().len() > 44);
    }
}
