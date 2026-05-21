use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::audio_capture::list_input_devices;
use crate::buffering::{RingPcmBuffer, WindowPolicy};
use crate::engine::{mock_words, AsrEngine, MockEngine};
use crate::protocol::{protocol_error, WorkerCommand, WorkerEvent};
use crate::vad::{is_speech, VadConfig};

const MOCK_SENTENCE: &str = "Hello world this is a test of the OpenWhisper dictation system";

pub fn run_stdio() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_worker(stdin.lock(), stdout.lock())
}

pub fn run_worker<R, W>(reader: R, mut writer: W) -> anyhow::Result<()>
where
    R: BufRead,
    W: Write,
{
    let mut state = WorkerState::default();

    for line in reader.lines() {
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

        if should_stop {
            break;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct WorkerState {
    engine: MockEngine,
    is_model_loaded: bool,
    is_recording: bool,
}

impl Default for WorkerState {
    fn default() -> Self {
        Self {
            engine: MockEngine::default(),
            is_model_loaded: false,
            is_recording: false,
        }
    }
}

impl WorkerState {
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
            WorkerCommand::Shutdown => (vec![], true),
        }
    }

    fn load_model(&mut self, model: Option<String>, device: Option<String>) -> WorkerEvent {
        match self.engine.load_model(
            model.as_deref().unwrap_or("mock"),
            device.as_deref().unwrap_or("cpu"),
        ) {
            Ok(info) => {
                self.is_model_loaded = true;
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
        if self.is_recording {
            return vec![WorkerEvent::Error {
                code: Some("DICTATION_ALREADY_RUNNING".to_string()),
                message: "Dictation is already recording".to_string(),
                recoverable: true,
            }];
        }

        self.is_recording = true;

        let mut events = Vec::new();
        if !self.is_model_loaded {
            events.push(self.load_model(Some("mock".to_string()), Some("cpu".to_string())));
        }
        events.push(WorkerEvent::TranscriptPartial {
            text: "Hello world".to_string(),
            is_final: false,
            processing_latency_ms: 1,
        });
        events
    }

    fn stop_dictation(&mut self) -> Vec<WorkerEvent> {
        if !self.is_recording {
            return vec![WorkerEvent::Error {
                code: Some("DICTATION_NOT_RUNNING".to_string()),
                message: "Dictation is not recording".to_string(),
                recoverable: true,
            }];
        }

        self.is_recording = false;
        let policy = WindowPolicy::dictation_default();
        let mut ring = RingPcmBuffer::new(policy.window_samples());
        ring.push_chunk(&vec![0.03; policy.samples_for_ms(250)]);
        let speech_detected = is_speech(
            &ring.tail_window(policy.samples_for_ms(250)),
            VadConfig::default(),
        );

        let text = if speech_detected { MOCK_SENTENCE } else { "" };
        vec![WorkerEvent::TranscriptFinal {
            text: text.to_string(),
            words: mock_words(text),
            language: Some("en".to_string()),
            processing_latency_ms: 1,
        }]
    }

    fn transcribe_file(&mut self, audio_path: &Path) -> WorkerEvent {
        match self.engine.transcribe_file(audio_path) {
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

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use serde_json::Value;

    use super::run_worker;

    const V1_COMMANDS: &str = include_str!("../../../asr/test_data/protocol/v1_commands.ndjson");

    fn run(input: &str) -> Vec<Value> {
        let mut output = Vec::new();
        run_worker(BufReader::new(Cursor::new(input.as_bytes())), &mut output).unwrap();
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
    fn dictation_start_stop_emit_desktop_compatible_transcript_events() {
        let lines = run("{\"type\":\"dictation.start\"}\n{\"type\":\"dictation.stop\"}");

        assert_eq!(lines[0]["type"], "model.loaded");
        assert_eq!(lines[1]["type"], "transcript.partial");
        assert_eq!(lines[2]["type"], "transcript.final");
        assert!(lines[2]["text"].as_str().unwrap().contains("OpenWhisper"));
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
            vec![
                "health.ok",
                "model.loaded",
                "transcript.partial",
                "transcript.final"
            ]
        );
    }
}
