use std::collections::VecDeque;

#[derive(Debug)]
pub struct RingPcmBuffer {
    samples: VecDeque<f32>,
    capacity_samples: usize,
}

impl RingPcmBuffer {
    pub fn new(capacity_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity_samples),
            capacity_samples,
        }
    }

    pub fn push_chunk(&mut self, chunk: &[f32]) {
        if self.capacity_samples == 0 {
            return;
        }

        for sample in chunk {
            if self.samples.len() == self.capacity_samples {
                self.samples.pop_front();
            }
            self.samples.push_back(*sample);
        }
    }

    pub fn tail_window(&self, sample_count: usize) -> Vec<f32> {
        let keep = sample_count.min(self.samples.len());
        self.samples
            .iter()
            .skip(self.samples.len() - keep)
            .copied()
            .collect()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPolicy {
    pub sample_rate_hz: u32,
    pub step_ms: u32,
    pub length_ms: u32,
    pub keep_ms: u32,
}

impl WindowPolicy {
    pub fn dictation_default() -> Self {
        Self {
            sample_rate_hz: 16_000,
            step_ms: 500,
            length_ms: 5_000,
            keep_ms: 500,
        }
    }

    pub fn samples_for_ms(&self, ms: u32) -> usize {
        (self.sample_rate_hz as usize * ms as usize) / 1_000
    }

    pub fn window_samples(&self) -> usize {
        self.samples_for_ms(self.length_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::{RingPcmBuffer, WindowPolicy};

    #[test]
    fn ring_buffer_keeps_only_recent_audio_because_streaming_must_bound_memory() {
        let mut ring = RingPcmBuffer::new(5);
        assert!(ring.is_empty());

        ring.push_chunk(&[1.0, 2.0, 3.0]);
        ring.push_chunk(&[4.0, 5.0, 6.0]);

        assert_eq!(ring.len(), 5);
        assert_eq!(ring.tail_window(5), vec![2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn window_policy_converts_ms_to_samples_for_fixed_internal_rate() {
        let policy = WindowPolicy::dictation_default();

        assert_eq!(policy.samples_for_ms(500), 8_000);
        assert_eq!(policy.window_samples(), 80_000);
    }
}
