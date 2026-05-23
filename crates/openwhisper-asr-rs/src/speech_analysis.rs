use crate::vad::{rms_level, VadConfig};

/// Minimum peak/noise ratio for dictation gate and transcript fallback without token probs.
pub(crate) const MIN_PEAK_TO_FLOOR_RATIO: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpeechSpan {
    start_sample: usize,
    end_sample: usize,
}

/// Summary of how much real speech energy is present in a PCM buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeechAnalysis {
    pub sample_rate_hz: u32,
    pub noise_floor_rms: f32,
    pub peak_rms: f32,
    pub effective_threshold: f32,
    pub speech_sample_count: usize,
    pub total_samples: usize,
}

impl SpeechAnalysis {
    /// Full recordings must show both enough voiced audio and clear dynamics above the noise floor.
    pub fn passes_dictation_gate(self, config: VadConfig) -> bool {
        if self.total_samples < config.min_samples {
            return false;
        }

        let required = config.min_speech_samples(self.sample_rate_hz);
        if self.speech_sample_count < required {
            return false;
        }

        // Stationary fan/hum keeps a narrow RMS band; real speech spans quiet + loud windows.
        if self.peak_to_noise_ratio() < MIN_PEAK_TO_FLOOR_RATIO {
            return false;
        }

        if self.peak_rms < self.effective_threshold {
            return false;
        }

        true
    }

    /// Short streaming windows only use the adaptive threshold, not the dynamics check.
    pub fn passes_streaming_window_gate(self, config: VadConfig) -> bool {
        self.speech_sample_count >= config.min_speech_samples(self.sample_rate_hz)
    }

    pub fn speech_ratio(self) -> f32 {
        if self.total_samples == 0 {
            return 0.0;
        }
        self.speech_sample_count as f32 / self.total_samples as f32
    }

    pub fn peak_to_noise_ratio(self) -> f32 {
        self.peak_rms / self.noise_floor_rms.max(1e-7)
    }
}

/// Returns PCM trimmed to the voiced region (first→last adaptive-threshold window + padding).
pub fn trim_samples_to_speech_region(
    samples: &[f32],
    config: VadConfig,
    sample_rate_hz: u32,
) -> Vec<f32> {
    let Some(span) = find_speech_span(samples, config, sample_rate_hz) else {
        return Vec::new();
    };

    let max_samples = (sample_rate_hz as usize * 30).max(config.min_samples);
    let mut start = span.start_sample;
    let end = span.end_sample;
    if end.saturating_sub(start) > max_samples {
        start = end.saturating_sub(max_samples);
    }

    samples.get(start..end).unwrap_or(&[]).to_vec()
}

fn find_speech_span(
    samples: &[f32],
    config: VadConfig,
    sample_rate_hz: u32,
) -> Option<SpeechSpan> {
    let window = config.min_samples.max(1);
    let levels = rms_windows(samples, window);
    if levels.is_empty() {
        return None;
    }

    let threshold = config.rms_threshold;
    let mut start_sample = None;
    let mut end_sample = 0usize;

    for (index, level) in levels.iter().enumerate() {
        if *level >= threshold {
            let window_start = index * window;
            start_sample.get_or_insert(window_start);
            end_sample = window_start + window;
        }
    }

    let start_sample = start_sample?;

    const PAD_BEFORE_MS: u32 = 150;
    const PAD_AFTER_MS: u32 = 400;
    let pad_before = speech_pad_samples(sample_rate_hz, PAD_BEFORE_MS).min(start_sample);
    let pad_after = speech_pad_samples(sample_rate_hz, PAD_AFTER_MS);
    let start_sample = start_sample.saturating_sub(pad_before);
    let end_sample = (end_sample + pad_after).min(samples.len());

    Some(SpeechSpan {
        start_sample,
        end_sample,
    })
}

pub fn analyze_speech(samples: &[f32], config: VadConfig, sample_rate_hz: u32) -> SpeechAnalysis {
    let window = config.min_samples.max(1);
    let levels = rms_windows(samples, window);
    let (noise_floor_rms, peak_rms) = noise_and_peak_rms(&levels);
    let effective_threshold = adaptive_threshold(config, noise_floor_rms);
    let speech_sample_count = levels
        .iter()
        .filter(|level| **level >= config.rms_threshold)
        .map(|_| window)
        .sum();

    SpeechAnalysis {
        sample_rate_hz,
        noise_floor_rms,
        peak_rms,
        effective_threshold,
        speech_sample_count,
        total_samples: samples.len(),
    }
}

fn speech_pad_samples(sample_rate_hz: u32, ms: u32) -> usize {
    sample_rate_hz as usize * ms as usize / 1_000
}

fn adaptive_threshold(config: VadConfig, noise_floor_rms: f32) -> f32 {
    const NOISE_MARGIN: f32 = 4.5;
    config.rms_threshold.max(noise_floor_rms * NOISE_MARGIN)
}

fn rms_windows(samples: &[f32], window: usize) -> Vec<f32> {
    samples
        .chunks(window)
        .filter(|chunk| chunk.len() == window)
        .map(rms_level)
        .collect()
}

fn noise_and_peak_rms(levels: &[f32]) -> (f32, f32) {
    if levels.is_empty() {
        return (0.0, 0.0);
    }
    let mut sorted = levels.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    (
        percentile_from_sorted(&sorted, 0.10),
        percentile_from_sorted(&sorted, 0.95),
    )
}

fn percentile_from_sorted(sorted: &[f32], ratio: f32) -> f32 {
    let index = ((sorted.len() - 1) as f32 * ratio.clamp(0.0, 1.0)).round() as usize;
    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::{analyze_speech, trim_samples_to_speech_region};
    use crate::vad::VadConfig;

    fn test_config() -> VadConfig {
        VadConfig::default()
    }

    #[test]
    fn steady_room_hum_does_not_pass_dictation_gate() {
        let samples = vec![0.02; 32_000];
        let analysis = analyze_speech(&samples, test_config(), 16_000);
        assert!(!analysis.passes_dictation_gate(test_config()));
    }

    #[test]
    fn real_speech_with_quiet_gaps_passes_dictation_gate() {
        let mut samples = vec![0.008; 24_000];
        samples[8_000..16_000].fill(0.07);
        let analysis = analyze_speech(&samples, test_config(), 16_000);
        assert!(analysis.passes_dictation_gate(test_config()));
    }

    #[test]
    fn trim_keeps_only_voiced_region_from_long_silence_padding() {
        let mut samples = vec![0.0; 80_000];
        samples[40_000..41_000].fill(0.08);
        let trimmed = trim_samples_to_speech_region(&samples, test_config(), 16_000);
        assert!(trimmed.len() < 10_000);
        assert!(trimmed.len() > 500);
    }
}
