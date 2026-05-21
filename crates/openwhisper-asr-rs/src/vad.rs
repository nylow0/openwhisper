#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    pub rms_threshold: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            rms_threshold: 0.015,
        }
    }
}

pub fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_sq: f32 = samples.iter().map(|v| v * v).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

pub fn is_speech(samples: &[f32], config: VadConfig) -> bool {
    rms_level(samples) >= config.rms_threshold
}

#[cfg(test)]
mod tests {
    use super::{is_speech, rms_level, VadConfig};

    #[test]
    fn rms_empty_is_zero() {
        assert_eq!(rms_level(&[]), 0.0);
    }

    #[test]
    fn detects_speech_over_threshold() {
        let samples = vec![0.0, 0.05, -0.05, 0.02, -0.02];
        assert!(is_speech(
            &samples,
            VadConfig {
                rms_threshold: 0.01
            }
        ));
    }
}
