#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VadConfig {
    pub rms_threshold: f32,
    pub min_samples: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            rms_threshold: 0.015,
            min_samples: 160,
        }
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
            }
        ));
        assert!(!is_speech(
            &samples[..80],
            VadConfig {
                rms_threshold: 0.01,
                min_samples: 160,
            }
        ));
    }
}
