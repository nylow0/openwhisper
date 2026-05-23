use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use hound::SampleFormat;

use crate::audio_capture::{i16_to_f32, mixdown_to_mono_f32, write_debug_wav};
use crate::speech_analysis::{analyze_speech, trim_samples_to_speech_region, SpeechAnalysis};
use crate::vad::VadConfig;

pub fn analyze_audio_file(path: &Path, config: VadConfig) -> Result<SpeechAnalysis> {
    let (samples, sample_rate_hz) = load_wav_mono_f32(path)?;
    Ok(analyze_speech(&samples, config, sample_rate_hz))
}

/// Loads audio once, optionally trims to the voiced region, and returns speech metrics for that decode input.
pub fn prepare_decode_audio(
    source: &Path,
    config: VadConfig,
) -> Result<(PathBuf, Option<PathBuf>, SpeechAnalysis)> {
    let (samples, sample_rate_hz) = load_wav_mono_f32(source)?;
    let trimmed = trim_samples_to_speech_region(&samples, config, sample_rate_hz);

    if trimmed.is_empty() || trimmed.len() == samples.len() {
        let speech = analyze_speech(&samples, config, sample_rate_hz);
        return Ok((source.to_path_buf(), None, speech));
    }

    let speech = analyze_speech(&trimmed, config, sample_rate_hz);
    let path = std::env::temp_dir().join(format!(
        "openwhisper-asr-rs-trimmed-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    write_debug_wav(&path, &trimmed, sample_rate_hz)?;
    log::info!(
        "Trimmed dictation audio for decode: {:.1}s -> {:.1}s",
        samples.len() as f32 / sample_rate_hz as f32,
        trimmed.len() as f32 / sample_rate_hz as f32
    );
    Ok((path.clone(), Some(path), speech))
}

fn load_wav_mono_f32(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("failed to open wav for speech gate: {}", path.display()))?;
    let spec = reader.spec();
    let sample_rate_hz = spec.sample_rate;
    let channels = spec.channels.max(1);

    let mono = match spec.sample_format {
        SampleFormat::Float => {
            let interleaved = reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .context("failed to read float wav samples")?;
            mixdown_to_mono_f32(&interleaved, channels)
        }
        SampleFormat::Int if spec.bits_per_sample <= 16 => {
            let interleaved = reader
                .samples::<i16>()
                .collect::<Result<Vec<_>, _>>()
                .context("failed to read i16 wav samples")?;
            mixdown_to_mono_f32(&i16_to_f32(&interleaved), channels)
        }
        SampleFormat::Int => {
            let interleaved = reader
                .samples::<i32>()
                .collect::<Result<Vec<_>, _>>()
                .context("failed to read i32 wav samples")?;
            let normalized = interleaved
                .iter()
                .map(|sample| *sample as f32 / i32::MAX as f32)
                .collect::<Vec<_>>();
            mixdown_to_mono_f32(&normalized, channels)
        }
    };

    Ok((mono, sample_rate_hz))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use hound::{SampleFormat, WavSpec, WavWriter};

    use super::analyze_audio_file;
    use crate::transcript_quality::{should_discard_transcript, DecodeQuality};
    use crate::vad::VadConfig;

    fn write_test_wav(path: &PathBuf, samples: &[f32], sample_rate: u32) {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        for sample in samples {
            writer
                .write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .unwrap();
        }
        writer.finalize().unwrap();
    }

    fn test_config() -> VadConfig {
        VadConfig::default()
    }

    #[test]
    fn silent_wav_fails_speech_gate() {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-speech-gate-silent-{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_test_wav(&path, &vec![0.0; 16_000], 16_000);
        let speech = analyze_audio_file(&path, test_config()).unwrap();
        assert!(!speech.passes_dictation_gate(test_config()));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn steady_hum_wav_fails_speech_gate() {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-speech-gate-hum-{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_test_wav(&path, &vec![0.02; 32_000], 16_000);
        let speech = analyze_audio_file(&path, test_config()).unwrap();
        assert!(!speech.passes_dictation_gate(test_config()));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn voiced_wav_passes_speech_gate() {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-speech-gate-voiced-{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut samples = vec![0.008; 24_000];
        samples[8_000..16_000].fill(0.07);
        write_test_wav(&path, &samples, 16_000);
        let speech = analyze_audio_file(&path, test_config()).unwrap();
        assert!(speech.passes_dictation_gate(test_config()));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn you_on_hum_wav_is_discarded_after_decode() {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-speech-gate-you-{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_test_wav(&path, &vec![0.02; 32_000], 16_000);
        let speech = analyze_audio_file(&path, test_config()).unwrap();
        let quality = DecodeQuality {
            audio_duration_ms: 2_000,
            content_token_count: 1,
            avg_content_token_probability: Some(0.17),
        };
        assert!(should_discard_transcript("you", quality, speech));
        let _ = std::fs::remove_file(path);
    }
}
