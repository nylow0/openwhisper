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
        for &sample in chunk {
            if self.samples.len() == self.capacity_samples {
                self.samples.pop_front();
            }
            self.samples.push_back(sample);
        }
    }

    pub fn tail_window(&self, n_samples: usize) -> Vec<f32> {
        let keep = n_samples.min(self.samples.len());
        self.samples
            .iter()
            .skip(self.samples.len() - keep)
            .copied()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

#[cfg(test)]
mod tests {
    use super::RingPcmBuffer;

    #[test]
    fn keeps_only_tail_when_over_capacity() {
        let mut ring = RingPcmBuffer::new(5);
        ring.push_chunk(&[1.0, 2.0, 3.0]);
        ring.push_chunk(&[4.0, 5.0, 6.0]);

        assert_eq!(ring.len(), 5);
        assert_eq!(ring.tail_window(5), vec![2.0, 3.0, 4.0, 5.0, 6.0]);
    }
}
