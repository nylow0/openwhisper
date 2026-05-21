use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::audio_capture::{write_debug_wav, AudioChunkReceiver, WORKER_SAMPLE_RATE_HZ};
use crate::buffering::{RingPcmBuffer, WindowPolicy};
use crate::engine::{AsrEngine, Transcription};
use crate::protocol::WorkerEvent;
use crate::vad::{is_speech, VadConfig};

pub struct StreamingSession {
    events: Receiver<WorkerEvent>,
    stop_tx: Sender<()>,
    worker: JoinHandle<Box<dyn AsrEngine>>,
}

impl StreamingSession {
    pub fn spawn(engine: Box<dyn AsrEngine>, samples: AudioChunkReceiver) -> Self {
        let (event_tx, events) = channel();
        let (stop_tx, stop_rx) = channel();
        let vad = engine.vad_config();
        let worker = thread::spawn(move || {
            run_streaming_decoder(
                engine,
                samples,
                stop_rx,
                event_tx,
                WindowPolicy::dictation_default(),
                vad,
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
    vad: VadConfig,
) -> Box<dyn AsrEngine> {
    let mut ring = RingPcmBuffer::new(policy.ring_capacity_samples());
    let mut samples_since_decode = 0;
    let mut silence_samples = 0;
    let mut has_active_speech = false;

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        match samples.recv_timeout(Duration::from_millis(50)) {
            Ok(chunk) => {
                let chunk_is_speech = is_speech(&chunk, vad);
                ring.push_chunk(&chunk);

                if chunk_is_speech {
                    has_active_speech = true;
                    silence_samples = 0;
                    samples_since_decode += chunk.len();
                    if samples_since_decode >= policy.step_samples() {
                        samples_since_decode = 0;
                        decode_window(&mut *engine, &ring, policy, &event_tx, DecodeKind::Partial);
                    }
                } else if has_active_speech {
                    silence_samples += chunk.len();
                    if silence_samples >= vad.silence_samples(policy.sample_rate_hz) {
                        decode_window(&mut *engine, &ring, policy, &event_tx, DecodeKind::Final);
                        has_active_speech = false;
                        samples_since_decode = 0;
                        silence_samples = 0;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    engine
}

#[derive(Clone, Copy)]
enum DecodeKind {
    Partial,
    Final,
}

fn decode_window(
    engine: &mut dyn AsrEngine,
    ring: &RingPcmBuffer,
    policy: WindowPolicy,
    event_tx: &Sender<WorkerEvent>,
    kind: DecodeKind,
) {
    if ring.is_empty() {
        return;
    }

    let samples = ring.tail_window(policy.window_samples());
    let path = streaming_window_path();
    let event = match write_debug_wav(&path, &samples, WORKER_SAMPLE_RATE_HZ)
        .and_then(|_| engine.transcribe_file(&path))
    {
        Ok(transcription) => transcript_event(transcription, kind),
        Err(error) => WorkerEvent::TranscriptError {
            error: error.to_string(),
            chunk_timestamp: unix_secs(),
        },
    };
    let _ = std::fs::remove_file(path);
    let _ = event_tx.send(event);
}

fn transcript_event(transcription: Transcription, kind: DecodeKind) -> WorkerEvent {
    match kind {
        DecodeKind::Partial => WorkerEvent::TranscriptPartial {
            text: transcription.text,
            is_final: false,
            processing_latency_ms: transcription.processing_latency_ms,
        },
        DecodeKind::Final => WorkerEvent::TranscriptFinal {
            text: transcription.text,
            words: transcription.words,
            language: transcription.language,
            processing_latency_ms: transcription.processing_latency_ms,
        },
    }
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
    use crate::vad::VadConfig;

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
            run_streaming_decoder(
                Box::new(WindowEngine),
                sample_rx,
                stop_rx,
                event_tx,
                policy,
                vad_config(30),
            )
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
            run_streaming_decoder(
                Box::new(WindowEngine),
                sample_rx,
                stop_rx,
                event_tx,
                policy,
                vad_config(30),
            )
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

    #[test]
    fn decoder_finalizes_once_after_spoken_audio_turns_silent() {
        let (sample_tx, sample_rx) = sync_channel(4);
        let (stop_tx, stop_rx) = channel();
        let (event_tx, event_rx) = channel();
        let policy = test_policy();
        let worker = std::thread::spawn(move || {
            run_streaming_decoder(
                Box::new(WindowEngine),
                sample_rx,
                stop_rx,
                event_tx,
                policy,
                vad_config(20),
            )
        });

        sample_tx.send(vec![0.2; policy.step_samples()]).unwrap();
        sample_tx.send(vec![0.0; policy.step_samples()]).unwrap();
        sample_tx.send(vec![0.0; policy.step_samples()]).unwrap();
        let partial = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let final_event = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(event_rx.recv_timeout(Duration::from_millis(100)).is_err());
        stop_tx.send(()).unwrap();
        worker.join().unwrap();

        assert_eq!(
            serde_json::to_value(partial).unwrap()["type"],
            "transcript.partial"
        );
        assert_eq!(
            serde_json::to_value(final_event).unwrap()["type"],
            "transcript.final"
        );
    }

    #[test]
    fn noisy_room_floor_below_threshold_does_not_spam_decode_events() {
        let (sample_tx, sample_rx) = sync_channel(2);
        let (stop_tx, stop_rx) = channel();
        let (event_tx, event_rx) = channel();
        let policy = test_policy();
        let worker = std::thread::spawn(move || {
            run_streaming_decoder(
                Box::new(WindowEngine),
                sample_rx,
                stop_rx,
                event_tx,
                policy,
                VadConfig {
                    rms_threshold: 0.03,
                    min_samples: 1,
                    silence_ms: 20,
                },
            )
        });

        sample_tx.send(vec![0.01; policy.step_samples()]).unwrap();
        sample_tx.send(vec![0.02; policy.step_samples()]).unwrap();
        assert!(event_rx.recv_timeout(Duration::from_millis(100)).is_err());
        stop_tx.send(()).unwrap();
        worker.join().unwrap();
    }

    fn test_policy() -> WindowPolicy {
        WindowPolicy {
            sample_rate_hz: 16_000,
            step_ms: 10,
            length_ms: 20,
            keep_ms: 10,
        }
    }

    fn vad_config(silence_ms: u32) -> VadConfig {
        VadConfig {
            rms_threshold: 0.03,
            min_samples: 1,
            silence_ms,
        }
    }
}
