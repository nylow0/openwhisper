use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::audio_capture::{
    list_input_devices, start_default_streaming_recording_session, AudioChunkReceiver,
    RecordingSession,
};
use crate::engine::{AsrEngine, MockEngine, WhisperCppEngine};
use crate::protocol::{protocol_error, WorkerCommand, WorkerEvent};
use crate::streaming::StreamingSession;

pub fn run_stdio() -> anyhow::Result<()> {
    let (line_tx, line_rx) = channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            if line_tx.send(line).is_err() {
                break;
            }
        }
    });

    let stdout = io::stdout();
    run_live_worker(
        line_rx,
        stdout.lock(),
        default_recorder_factory,
        worker_engine_from_env(),
    )
}

#[cfg(test)]
fn run_worker_with_dependencies<R, W, F>(
    reader: R,
    mut writer: W,
    recorder_factory: F,
    engine: Box<dyn AsrEngine>,
) -> anyhow::Result<()>
where
    R: BufRead,
    W: Write,
    F: FnMut() -> Result<Box<dyn ActiveRecording>>,
{
    let mut state = WorkerState::new(recorder_factory, engine);

    for line in reader.lines() {
        write_events(&mut writer, state.drain_stream_events())?;
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let command = serde_json::from_str::<WorkerCommand>(&line);
        let (events, should_stop) = match command {
            Ok(command) => state.handle(command),
            Err(error) => (
                vec![protocol_error(format!("Invalid JSON: {error}"))],
                false,
            ),
        };

        for event in events {
            write_event(&mut writer, &event)?;
        }
        write_events(&mut writer, state.drain_stream_events())?;

        if should_stop {
            break;
        }
    }

    Ok(())
}

fn run_live_worker<W, F>(
    lines: std::sync::mpsc::Receiver<io::Result<String>>,
    mut writer: W,
    recorder_factory: F,
    engine: Box<dyn AsrEngine>,
) -> anyhow::Result<()>
where
    W: Write,
    F: FnMut() -> Result<Box<dyn ActiveRecording>>,
{
    let mut state = WorkerState::new(recorder_factory, engine);

    loop {
        write_events(&mut writer, state.drain_stream_events())?;

        match lines.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(line)) => {
                if line.trim().is_empty() {
                    continue;
                }

                let command = serde_json::from_str::<WorkerCommand>(&line);
                let (events, should_stop) = match command {
                    Ok(command) => state.handle(command),
                    Err(error) => (
                        vec![protocol_error(format!("Invalid JSON: {error}"))],
                        false,
                    ),
                };
                write_events(&mut writer, events)?;
                if should_stop {
                    break;
                }
            }
            Ok(Err(error)) => return Err(error.into()),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

trait ActiveRecording {
    fn stop(self: Box<Self>) -> Result<PathBuf>;
    fn cancel(self: Box<Self>);
    fn take_stream_samples(&mut self) -> Option<AudioChunkReceiver> {
        None
    }
}

impl ActiveRecording for RecordingSession {
    fn stop(self: Box<Self>) -> Result<PathBuf> {
        RecordingSession::stop(*self)
    }

    fn cancel(self: Box<Self>) {
        RecordingSession::cancel(*self);
    }

    fn take_stream_samples(&mut self) -> Option<AudioChunkReceiver> {
        RecordingSession::take_stream_samples(self)
    }
}

struct WorkerState<F>
where
    F: FnMut() -> Result<Box<dyn ActiveRecording>>,
{
    engine: Option<Box<dyn AsrEngine>>,
    is_model_loaded: bool,
    recording: Option<Box<dyn ActiveRecording>>,
    recorder_factory: F,
    stream_tail_finalized: bool,
    streaming: Option<StreamingSession>,
}

impl<F> WorkerState<F>
where
    F: FnMut() -> Result<Box<dyn ActiveRecording>>,
{
    fn new(recorder_factory: F, engine: Box<dyn AsrEngine>) -> Self {
        Self {
            engine: Some(engine),
            is_model_loaded: false,
            recording: None,
            recorder_factory,
            stream_tail_finalized: false,
            streaming: None,
        }
    }

    fn handle(&mut self, command: WorkerCommand) -> (Vec<WorkerEvent>, bool) {
        match command {
            WorkerCommand::HealthCheck { timestamp } => (
                vec![WorkerEvent::HealthOk {
                    timestamp: timestamp.unwrap_or_else(unix_secs),
                    status: "ready",
                }],
                false,
            ),
            WorkerCommand::ModelLoad { device, model } => {
                (vec![self.load_model(model, device)], false)
            }
            WorkerCommand::DictationStart => (self.start_dictation(), false),
            WorkerCommand::DictationStop => (self.stop_dictation(), false),
            WorkerCommand::TranscribeFile { audio_path } => {
                (vec![self.transcribe_file(Path::new(&audio_path))], false)
            }
            WorkerCommand::DevicesList => (
                vec![WorkerEvent::DevicesResult {
                    devices: list_input_devices(),
                }],
                false,
            ),
            WorkerCommand::Shutdown => {
                self.cancel_recording();
                (vec![], true)
            }
        }
    }

    fn load_model(&mut self, model: Option<String>, device: Option<String>) -> WorkerEvent {
        let default_model =
            std::env::var("OPENWHISPER_ASR_MODEL").unwrap_or_else(|_| "medium_en_q8".to_string());
        let default_device =
            std::env::var("OPENWHISPER_ASR_DEVICE").unwrap_or_else(|_| "cpu".to_string());
        let Some(engine) = self.engine.as_mut() else {
            return WorkerEvent::Error {
                code: Some("DICTATION_RUNNING".to_string()),
                message: "Cannot load a model while streaming dictation".to_string(),
                recoverable: true,
            };
        };
        let model_name = model.as_deref().unwrap_or(&default_model);
        let device_name = device.as_deref().unwrap_or(&default_device);
        let load_start = Instant::now();
        match engine.load_model(model_name, device_name) {
            Ok(info) => {
                self.is_model_loaded = true;
                log::info!(
                    "ASR model setup complete: model={} requested_device={} selected_device={} setup_ms={} profile_memory_mb={}",
                    model_name,
                    device_name,
                    info.device,
                    load_start.elapsed().as_millis(),
                    info.memory_mb
                );
                WorkerEvent::ModelLoaded {
                    device: info.device,
                    memory_mb: info.memory_mb,
                }
            }
            Err(error) => WorkerEvent::ModelError {
                error: error.to_string(),
                recoverable: true,
            },
        }
    }

    fn start_dictation(&mut self) -> Vec<WorkerEvent> {
        if self.recording.is_some() {
            return vec![WorkerEvent::Error {
                code: Some("DICTATION_ALREADY_RUNNING".to_string()),
                message: "Dictation is already recording".to_string(),
                recoverable: true,
            }];
        }

        let mut events = Vec::new();
        if !self.is_model_loaded {
            events.push(self.load_model(None, None));
        }

        match (self.recorder_factory)() {
            Ok(mut recording) => {
                if let Some(samples) = recording.take_stream_samples() {
                    let engine = self
                        .engine
                        .take()
                        .expect("ASR engine must be idle when dictation starts");
                    self.streaming = Some(StreamingSession::spawn(engine, samples));
                    self.stream_tail_finalized = false;
                }
                self.recording = Some(recording);
            }
            Err(error) => events.push(WorkerEvent::AudioError {
                error: error.to_string(),
                code: "AUDIO_RECORDING_FAILED".to_string(),
            }),
        }

        events
    }

    fn stop_dictation(&mut self) -> Vec<WorkerEvent> {
        let Some(recording) = self.recording.take() else {
            return vec![WorkerEvent::Error {
                code: Some("DICTATION_NOT_RUNNING".to_string()),
                message: "Dictation is not recording".to_string(),
                recoverable: true,
            }];
        };

        let recording_path = match recording.stop() {
            Ok(path) => path,
            Err(error) => {
                return vec![WorkerEvent::AudioError {
                    error: error.to_string(),
                    code: "AUDIO_RECORDING_FAILED".to_string(),
                }];
            }
        };

        let mut events = self.stop_streaming();
        if !self.stream_tail_finalized {
            events.push(self.transcribe_file(&recording_path));
        }
        self.stream_tail_finalized = false;
        let _ = std::fs::remove_file(recording_path);
        events
    }

    fn transcribe_file(&mut self, audio_path: &Path) -> WorkerEvent {
        let Some(engine) = self.engine.as_mut() else {
            return WorkerEvent::Error {
                code: Some("DICTATION_RUNNING".to_string()),
                message: "Cannot transcribe a file while streaming dictation".to_string(),
                recoverable: true,
            };
        };
        match engine.transcribe_file(audio_path) {
            Ok(transcription) => WorkerEvent::TranscriptFinal {
                text: transcription.text,
                words: transcription.words,
                language: transcription.language,
                processing_latency_ms: transcription.processing_latency_ms,
            },
            Err(error) => WorkerEvent::TranscriptError {
                error: error.to_string(),
                chunk_timestamp: unix_secs(),
            },
        }
    }

    fn cancel_recording(&mut self) {
        if let Some(recording) = self.recording.take() {
            recording.cancel();
        }
        let _ = self.stop_streaming();
    }

    fn drain_stream_events(&mut self) -> Vec<WorkerEvent> {
        let mut events = Vec::new();
        if let Some(streaming) = &self.streaming {
            while let Some(event) = streaming.try_recv_event() {
                match &event {
                    WorkerEvent::TranscriptFinal { .. } => self.stream_tail_finalized = true,
                    WorkerEvent::TranscriptPartial { .. } => self.stream_tail_finalized = false,
                    _ => {}
                }
                events.push(event);
            }
        }
        events
    }

    fn stop_streaming(&mut self) -> Vec<WorkerEvent> {
        let mut events = self.drain_stream_events();
        if let Some(streaming) = self.streaming.take() {
            match streaming.stop() {
                Ok(engine) => self.engine = Some(engine),
                Err(error) => {
                    self.engine = Some(worker_engine_from_env());
                    events.push(WorkerEvent::TranscriptError {
                        error: error.to_string(),
                        chunk_timestamp: unix_secs(),
                    });
                }
            }
        }
        events
    }
}

fn write_events(writer: &mut impl Write, events: Vec<WorkerEvent>) -> anyhow::Result<()> {
    for event in events {
        write_event(writer, &event)?;
    }
    Ok(())
}

fn write_event(writer: &mut impl Write, event: &WorkerEvent) -> anyhow::Result<()> {
    serde_json::to_writer(&mut *writer, event)?;
    writeln!(writer)?;
    writer.flush()?;
    Ok(())
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn default_recorder_factory() -> Result<Box<dyn ActiveRecording>> {
    let path = default_recording_path();
    let session = start_default_streaming_recording_session(path)?;
    Ok(Box::new(session))
}

fn worker_engine_from_env() -> Box<dyn AsrEngine> {
    match std::env::var("OPENWHISPER_ASR_ENGINE")
        .unwrap_or_else(|_| "mock".to_string())
        .to_lowercase()
        .as_str()
    {
        "whispercpp" => Box::new(WhisperCppEngine::default()),
        _ => Box::new(MockEngine::default()),
    }
}

fn default_recording_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "openwhisper-asr-rs-{}-{}.wav",
        std::process::id(),
        unix_timestamp_nanos()
    ))
}

fn unix_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{channel, sync_channel};
    use std::thread;
    use std::time::Duration;

    use serde_json::Value;

    use crate::engine::{AsrEngine, ModelLoadInfo, Transcription};

    use super::{
        run_live_worker, run_worker_with_dependencies, unix_timestamp_nanos, ActiveRecording,
    };

    const V1_COMMANDS: &str = include_str!("../../../asr/test_data/protocol/v1_commands.ndjson");

    struct FakeRecording {
        path: PathBuf,
    }

    struct StreamingFakeRecording {
        path: PathBuf,
        samples: Option<crate::audio_capture::AudioChunkReceiver>,
    }

    #[derive(Debug)]
    struct FailingEngine;

    impl ActiveRecording for FakeRecording {
        fn stop(self: Box<Self>) -> anyhow::Result<PathBuf> {
            std::fs::write(&self.path, b"fake wav")?;
            Ok(self.path)
        }

        fn cancel(self: Box<Self>) {}
    }

    impl ActiveRecording for StreamingFakeRecording {
        fn stop(self: Box<Self>) -> anyhow::Result<PathBuf> {
            std::fs::write(&self.path, b"fake wav")?;
            Ok(self.path)
        }

        fn cancel(self: Box<Self>) {}

        fn take_stream_samples(&mut self) -> Option<crate::audio_capture::AudioChunkReceiver> {
            self.samples.take()
        }
    }

    impl AsrEngine for FailingEngine {
        fn load_model(&mut self, _model: &str, _device: &str) -> anyhow::Result<ModelLoadInfo> {
            Ok(ModelLoadInfo {
                device: "cpu".to_string(),
                memory_mb: 0.0,
            })
        }

        fn transcribe_file(&mut self, _audio_path: &Path) -> anyhow::Result<Transcription> {
            anyhow::bail!("decode failed")
        }
    }

    fn run(input: &str) -> Vec<Value> {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-test-{}.wav",
            unix_timestamp_nanos()
        ));
        run_with_recording_path(input, path)
    }

    fn run_with_recording_path(input: &str, path: PathBuf) -> Vec<Value> {
        run_with_recording_path_and_engine(
            input,
            path,
            Box::new(crate::engine::MockEngine::default()),
        )
    }

    fn run_with_recording_path_and_engine(
        input: &str,
        path: PathBuf,
        engine: Box<dyn AsrEngine>,
    ) -> Vec<Value> {
        let mut output = Vec::new();
        run_worker_with_dependencies(
            BufReader::new(Cursor::new(input.as_bytes())),
            &mut output,
            || Ok(Box::new(FakeRecording { path: path.clone() })),
            engine,
        )
        .unwrap();
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[test]
    fn health_check_matches_python_worker_contract() {
        let lines = run(r#"{"type":"health.check","timestamp":7}"#);

        assert_eq!(lines[0]["type"], "health.ok");
        assert_eq!(lines[0]["timestamp"], 7);
        assert_eq!(lines[0]["status"], "ready");
    }

    #[test]
    fn dictation_start_stop_records_then_emits_desktop_compatible_final() {
        let lines = run("{\"type\":\"dictation.start\"}\n{\"type\":\"dictation.stop\"}");

        assert_eq!(lines[0]["type"], "model.loaded");
        assert_eq!(lines[1]["type"], "transcript.final");
        assert!(lines[1]["text"]
            .as_str()
            .unwrap()
            .contains("Mock transcript for openwhisper-asr-rs-test"));
    }

    #[test]
    fn live_worker_flushes_partial_before_stop_so_desktop_can_show_streaming_text() {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-streaming-test-{}.wav",
            unix_timestamp_nanos()
        ));
        let (sample_tx, sample_rx) = sync_channel(1);
        sample_tx.send(vec![0.2; 8_000]).unwrap();
        let (line_tx, line_rx) = channel();
        let feeder = thread::spawn(move || {
            line_tx
                .send(Ok("{\"type\":\"dictation.start\"}".to_string()))
                .unwrap();
            thread::sleep(Duration::from_millis(250));
            line_tx
                .send(Ok("{\"type\":\"dictation.stop\"}".to_string()))
                .unwrap();
            line_tx
                .send(Ok("{\"type\":\"shutdown\"}".to_string()))
                .unwrap();
        });
        let mut output = Vec::new();
        let mut samples = Some(sample_rx);

        run_live_worker(
            line_rx,
            &mut output,
            || {
                Ok(Box::new(StreamingFakeRecording {
                    path: path.clone(),
                    samples: samples.take(),
                }))
            },
            Box::new(crate::engine::MockEngine::default()),
        )
        .unwrap();
        feeder.join().unwrap();
        let event_types = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| {
                serde_json::from_str::<Value>(line).unwrap()["type"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            event_types,
            vec!["model.loaded", "transcript.partial", "transcript.final"]
        );
        assert!(!path.exists());
    }

    #[test]
    fn stop_skips_duplicate_final_after_silence_already_finalized_stream_tail() {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-silence-final-test-{}.wav",
            unix_timestamp_nanos()
        ));
        let (sample_tx, sample_rx) = sync_channel(2);
        sample_tx.send(vec![0.2; 8_000]).unwrap();
        sample_tx.send(vec![0.0; 16_000]).unwrap();
        let (line_tx, line_rx) = channel();
        let feeder = thread::spawn(move || {
            line_tx
                .send(Ok("{\"type\":\"dictation.start\"}".to_string()))
                .unwrap();
            thread::sleep(Duration::from_millis(250));
            line_tx
                .send(Ok("{\"type\":\"dictation.stop\"}".to_string()))
                .unwrap();
            line_tx
                .send(Ok("{\"type\":\"shutdown\"}".to_string()))
                .unwrap();
        });
        let mut output = Vec::new();
        let mut samples = Some(sample_rx);

        run_live_worker(
            line_rx,
            &mut output,
            || {
                Ok(Box::new(StreamingFakeRecording {
                    path: path.clone(),
                    samples: samples.take(),
                }))
            },
            Box::new(crate::engine::MockEngine::default()),
        )
        .unwrap();
        feeder.join().unwrap();
        let event_types = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| {
                serde_json::from_str::<Value>(line).unwrap()["type"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            event_types,
            vec!["model.loaded", "transcript.partial", "transcript.final"]
        );
        assert!(!path.exists());
    }

    #[test]
    fn dictation_stop_removes_temp_recording_after_mock_transcript() {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-cleanup-test-{}.wav",
            unix_timestamp_nanos()
        ));

        let lines = run_with_recording_path(
            "{\"type\":\"dictation.start\"}\n{\"type\":\"dictation.stop\"}",
            path.clone(),
        );

        assert_eq!(lines[1]["type"], "transcript.final");
        assert_eq!(
            lines[1]["text"],
            format!(
                "Mock transcript for {}",
                path.file_name().unwrap().to_string_lossy()
            )
        );
        assert!(!path.exists());
    }

    #[test]
    fn dictation_stop_removes_temp_recording_when_transcription_fails() {
        let path = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-error-cleanup-test-{}.wav",
            unix_timestamp_nanos()
        ));

        let lines = run_with_recording_path_and_engine(
            "{\"type\":\"dictation.start\"}\n{\"type\":\"dictation.stop\"}",
            path.clone(),
            Box::new(FailingEngine),
        );

        assert_eq!(lines[1]["type"], "transcript.error");
        assert_eq!(lines[1]["error"], "decode failed");
        assert!(!path.exists());
    }

    #[test]
    fn invalid_json_reports_recoverable_protocol_error() {
        let lines = run("not-json");

        assert_eq!(lines[0]["type"], "error");
        assert_eq!(lines[0]["code"], "PROTOCOL_ERROR");
        assert_eq!(lines[0]["recoverable"], true);
    }

    #[test]
    fn replays_v1_command_fixture_without_protocol_drift() {
        let lines = run(V1_COMMANDS);
        let event_types: Vec<&str> = lines
            .iter()
            .map(|line| line["type"].as_str().unwrap())
            .collect();

        assert_eq!(
            event_types,
            vec!["health.ok", "model.loaded", "transcript.final"]
        );
    }
}
