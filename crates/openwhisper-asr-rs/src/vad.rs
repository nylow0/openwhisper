#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VadConfig {
    pub rms_threshold: f32,
    pub min_samples: usize,
    pub silence_ms: u32,
    /// Minimum voiced audio required before calling the ASR decoder.
    pub min_speech_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            rms_threshold: 0.015,
            min_samples: 160,
            silence_ms: 900,
            min_speech_ms: 250,
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
        if let Ok(value) = std::env::var("OPENWHISPER_ASR_MIN_SPEECH_MS") {
            match value.parse::<u32>() {
                Ok(min_speech_ms) if min_speech_ms > 0 => self.min_speech_ms = min_speech_ms,
                _ => log::warn!("Ignoring invalid OPENWHISPER_ASR_MIN_SPEECH_MS={value}"),
            }
        }
        self
    }

    pub fn min_speech_samples(self, sample_rate_hz: u32) -> usize {
        (sample_rate_hz as usize * self.min_speech_ms as usize / 1_000).max(self.min_samples)
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

pub fn has_sufficient_speech(samples: &[f32], config: VadConfig, sample_rate_hz: u32) -> bool {
    crate::speech_analysis::analyze_speech(samples, config, sample_rate_hz)
        .passes_streaming_window_gate(config)
}

#[cfg(test)]
mod tests {
    use super::{has_sufficient_speech, is_speech, rms_level, VadConfig};
    use crate::speech_analysis::analyze_speech;

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
                min_speech_ms: 250,
            }
        ));
        assert!(!is_speech(
            &samples[..80],
            VadConfig {
                rms_threshold: 0.01,
                min_samples: 160,
                silence_ms: 900,
                min_speech_ms: 250,
            }
        ));
    }

    #[test]
    fn silence_timeout_uses_worker_sample_rate_to_finalize_after_quiet_audio() {
        assert_eq!(VadConfig::default().silence_samples(16_000), 14_400);
    }

    #[test]
    fn speech_gate_requires_enough_voiced_windows_not_just_one_spike() {
        let config = VadConfig {
            rms_threshold: 0.03,
            min_samples: 160,
            silence_ms: 900,
            min_speech_ms: 250,
        };
        let quiet = vec![0.01; 16_000];
        let brief_spike = {
            let mut samples = vec![0.01; 16_000];
            samples[800..960].fill(0.05);
            samples
        };
        let sustained = vec![0.05; 8_000];

        assert!(!has_sufficient_speech(&quiet, config, 16_000));
        assert!(!has_sufficient_speech(&brief_spike, config, 16_000));
        assert!(has_sufficient_speech(&sustained, config, 16_000));
        assert!(!analyze_speech(&vec![0.02; 16_000], config, 16_000).passes_dictation_gate(config));
    }
}
