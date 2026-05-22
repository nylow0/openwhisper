#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VadConfig {
    pub rms_threshold: f32,
    pub min_samples: usize,
    pub silence_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            rms_threshold: 0.015,
            min_samples: 160,
            silence_ms: 900,
        }
    }
}

impl VadConfig {
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(value) = std::env::var("OPENWHISPER_ASR_VAD_RMS_THRESHOLD") {
            match value.parse::<f32>() {
                Ok(threshold) if threshold >= 0.0 => self.rms_threshold = threshold,
                _ => log::warn!("Ignoring invalid OPENWHISPER_ASR_VAD_RMS_THRESHOLD={value}"),
            }
        }
        if let Ok(value) = std::env::var("OPENWHISPER_ASR_VAD_SILENCE_MS") {
            match value.parse::<u32>() {
                Ok(silence_ms) if silence_ms > 0 => self.silence_ms = silence_ms,
                _ => log::warn!("Ignoring invalid OPENWHISPER_ASR_VAD_SILENCE_MS={value}"),
            }
        }
        self
    }

    pub fn silence_samples(self, sample_rate_hz: u32) -> usize {
        sample_rate_hz as usize * self.silence_ms as usize / 1_000
    }
}

pub fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_sq: f32 = samples.iter().map(|sample| sample * sample).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

pub fn is_speech(samples: &[f32], config: VadConfig) -> bool {
    samples.len() >= config.min_samples && rms_level(samples) >= config.rms_threshold
}

#[cfg(test)]
mod tests {
    use super::{is_speech, rms_level, VadConfig};

    #[test]
    fn rms_empty_is_zero_because_silence_windows_must_not_trip_vad() {
        assert_eq!(rms_level(&[]), 0.0);
    }

    #[test]
    fn detects_speech_only_after_enough_samples_clear_threshold() {
        let samples = vec![0.05; 160];

        assert!(is_speech(
            &samples,
            VadConfig {
                rms_threshold: 0.01,
                min_samples: 160,
                silence_ms: 900,
            }
        ));
        assert!(!is_speech(
            &samples[..80],
            VadConfig {
                rms_threshold: 0.01,
                min_samples: 160,
                silence_ms: 900,
            }
        ));
    }

    #[test]
    fn silence_timeout_uses_worker_sample_rate_to_finalize_after_quiet_audio() {
        assert_eq!(VadConfig::default().silence_samples(16_000), 14_400);
    }
}
