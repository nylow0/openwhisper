use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::audio_capture::{write_debug_wav, AudioChunkReceiver, WORKER_SAMPLE_RATE_HZ};
use crate::buffering::{RingPcmBuffer, WindowPolicy};
use crate::engine::AsrEngine;
use crate::protocol::WorkerEvent;

pub struct StreamingSession {
    events: Receiver<WorkerEvent>,
    stop_tx: Sender<()>,
    worker: JoinHandle<Box<dyn AsrEngine>>,
}

impl StreamingSession {
    pub fn spawn(engine: Box<dyn AsrEngine>, samples: AudioChunkReceiver) -> Self {
        let (event_tx, events) = channel();
        let (stop_tx, stop_rx) = channel();
        let worker = thread::spawn(move || {
            run_streaming_decoder(
                engine,
                samples,
                stop_rx,
                event_tx,
                WindowPolicy::dictation_default(),
            )
        });

        Self {
            events,
            stop_tx,
            worker,
        }
    }

    pub fn try_recv_event(&self) -> Option<WorkerEvent> {
        self.events.try_recv().ok()
    }

    pub fn stop(self) -> anyhow::Result<Box<dyn AsrEngine>> {
        let _ = self.stop_tx.send(());
        self.worker
            .join()
            .map_err(|_| anyhow::anyhow!("streaming decoder thread panicked"))
    }
}

fn run_streaming_decoder(
    mut engine: Box<dyn AsrEngine>,
    samples: AudioChunkReceiver,
    stop_rx: Receiver<()>,
    event_tx: Sender<WorkerEvent>,
    policy: WindowPolicy,
) -> Box<dyn AsrEngine> {
    let mut ring = RingPcmBuffer::new(policy.ring_capacity_samples());
    let mut samples_since_decode = 0;

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        match samples.recv_timeout(Duration::from_millis(50)) {
            Ok(chunk) => {
                samples_since_decode += chunk.len();
                ring.push_chunk(&chunk);
                if samples_since_decode >= policy.step_samples() {
                    samples_since_decode = 0;
                    decode_partial(&mut *engine, &ring, policy, &event_tx);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    engine
}

fn decode_partial(
    engine: &mut dyn AsrEngine,
    ring: &RingPcmBuffer,
    policy: WindowPolicy,
    event_tx: &Sender<WorkerEvent>,
) {
    if ring.is_empty() {
        return;
    }

    let samples = ring.tail_window(policy.window_samples());
    let path = streaming_window_path();
    let event = match write_debug_wav(&path, &samples, WORKER_SAMPLE_RATE_HZ)
        .and_then(|_| engine.transcribe_file(&path))
    {
        Ok(transcription) => WorkerEvent::TranscriptPartial {
            text: transcription.text,
            is_final: false,
            processing_latency_ms: transcription.processing_latency_ms,
        },
        Err(error) => WorkerEvent::TranscriptError {
            error: error.to_string(),
            chunk_timestamp: unix_secs(),
        },
    };
    let _ = std::fs::remove_file(path);
    let _ = event_tx.send(event);
}

fn streaming_window_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "openwhisper-asr-rs-window-{}-{}.wav",
        std::process::id(),
        unix_timestamp_nanos()
    ))
}

fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unix_timestamp_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::mpsc::{channel, sync_channel};
    use std::time::Duration;

    use crate::buffering::WindowPolicy;
    use crate::engine::{AsrEngine, ModelLoadInfo, Transcription};

    use super::run_streaming_decoder;

    #[derive(Debug)]
    struct WindowEngine;

    impl AsrEngine for WindowEngine {
        fn load_model(&mut self, _model: &str, _device: &str) -> anyhow::Result<ModelLoadInfo> {
            Ok(ModelLoadInfo {
                device: "cpu".to_string(),
                memory_mb: 0.0,
            })
        }

        fn transcribe_file(&mut self, audio_path: &Path) -> anyhow::Result<Transcription> {
            let sample_count = hound::WavReader::open(audio_path)?.samples::<i16>().count();
            Ok(Transcription {
                text: format!("window {sample_count}"),
                words: Vec::new(),
                language: Some("en".to_string()),
                processing_latency_ms: 7,
            })
        }
    }

    #[test]
    fn decoder_emits_partial_after_one_step_because_live_dictation_needs_feedback() {
        let (sample_tx, sample_rx) = sync_channel(1);
        let (stop_tx, stop_rx) = channel();
        let (event_tx, event_rx) = channel();
        let policy = WindowPolicy {
            sample_rate_hz: 16_000,
            step_ms: 10,
            length_ms: 20,
            keep_ms: 10,
        };
        let worker = std::thread::spawn(move || {
            run_streaming_decoder(Box::new(WindowEngine), sample_rx, stop_rx, event_tx, policy)
        });

        sample_tx.send(vec![0.2; policy.step_samples()]).unwrap();
        let partial = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        stop_tx.send(()).unwrap();
        worker.join().unwrap();

        let value = serde_json::to_value(partial).unwrap();
        assert_eq!(value["type"], "transcript.partial");
        assert_eq!(value["is_final"], false);
        assert_eq!(value["processing_latency_ms"], 7);
    }

    #[test]
    fn decoder_keeps_windows_bounded_when_live_audio_outlasts_window() {
        let (sample_tx, sample_rx) = sync_channel(2);
        let (stop_tx, stop_rx) = channel();
        let (event_tx, event_rx) = channel();
        let policy = WindowPolicy {
            sample_rate_hz: 16_000,
            step_ms: 10,
            length_ms: 20,
            keep_ms: 10,
        };
        let worker = std::thread::spawn(move || {
            run_streaming_decoder(Box::new(WindowEngine), sample_rx, stop_rx, event_tx, policy)
        });

        sample_tx.send(vec![0.2; policy.step_samples()]).unwrap();
        sample_tx
            .send(vec![0.2; policy.step_samples() * 3])
            .unwrap();
        let _ = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let latest = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        stop_tx.send(()).unwrap();
        worker.join().unwrap();

        let value = serde_json::to_value(latest).unwrap();
        assert_eq!(value["text"], "window 320");
    }
}
