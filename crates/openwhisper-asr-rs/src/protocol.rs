use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WorkerCommand {
    #[serde(rename = "health.check")]
    HealthCheck { timestamp: Option<u64> },

    #[serde(rename = "dictation.start")]
    DictationStart,

    #[serde(rename = "dictation.stop")]
    DictationStop,

    #[serde(rename = "model.load")]
    ModelLoad {
        device: Option<String>,
        model: Option<String>,
    },

    #[serde(rename = "transcribe.file")]
    TranscribeFile { audio_path: String },

    #[serde(rename = "devices.list")]
    DevicesList,

    #[serde(rename = "shutdown")]
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WordResult {
    pub text: String,
    pub start_ms: u32,
    pub end_ms: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WorkerEvent {
    #[serde(rename = "health.ok")]
    HealthOk {
        timestamp: u64,
        status: &'static str,
    },

    #[serde(rename = "model.loaded")]
    ModelLoaded { device: String, memory_mb: f64 },

    #[serde(rename = "model.error")]
    ModelError { error: String, recoverable: bool },

    #[serde(rename = "transcript.partial")]
    #[allow(dead_code)]
    TranscriptPartial {
        text: String,
        is_final: bool,
        processing_latency_ms: u32,
    },

    #[serde(rename = "transcript.final")]
    TranscriptFinal {
        text: String,
        words: Vec<WordResult>,
        language: Option<String>,
        processing_latency_ms: u32,
    },

    #[serde(rename = "transcript.error")]
    TranscriptError { error: String, chunk_timestamp: u64 },

    #[serde(rename = "audio.error")]
    #[allow(dead_code)]
    AudioError { error: String, code: String },

    #[serde(rename = "devices.result")]
    DevicesResult {
        devices: Vec<crate::audio_capture::CaptureDeviceInfo>,
    },

    #[serde(rename = "error")]
    Error {
        code: Option<String>,
        message: String,
        recoverable: bool,
    },
}

pub fn protocol_error(message: impl Into<String>) -> WorkerEvent {
    WorkerEvent::Error {
        code: Some("PROTOCOL_ERROR".to_string()),
        message: message.into(),
        recoverable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkerCommand, WorkerEvent};

    const V1_COMMANDS: &str = include_str!("../test_data/protocol/v1_commands.ndjson");
    const V1_EVENTS: &str = include_str!("../test_data/protocol/v1_events.ndjson");

    #[test]
    fn parses_existing_desktop_commands() {
        let health: WorkerCommand =
            serde_json::from_str(r#"{"type":"health.check","timestamp":7}"#).unwrap();
        assert!(matches!(
            health,
            WorkerCommand::HealthCheck { timestamp: Some(7) }
        ));

        let start: WorkerCommand = serde_json::from_str(r#"{"type":"dictation.start"}"#).unwrap();
        assert!(matches!(start, WorkerCommand::DictationStart));
    }

    #[test]
    fn emits_desktop_compatible_transcript_final_shape() {
        let event = WorkerEvent::TranscriptFinal {
            text: "hello".to_string(),
            words: vec![],
            language: Some("en".to_string()),
            processing_latency_ms: 12,
        };

        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["type"], "transcript.final");
        assert_eq!(value["text"], "hello");
        assert_eq!(value["processing_latency_ms"], 12);
    }

    #[test]
    fn emits_desktop_compatible_audio_error_shape() {
        let event = WorkerEvent::AudioError {
            error: "microphone unavailable".to_string(),
            code: "AUDIO_RECORDING_FAILED".to_string(),
        };

        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["type"], "audio.error");
        assert_eq!(value["code"], "AUDIO_RECORDING_FAILED");
        assert_eq!(value["error"], "microphone unavailable");
    }

    #[test]
    fn parses_v1_command_fixture() {
        let command_types: Vec<&'static str> = V1_COMMANDS
            .lines()
            .map(|line| serde_json::from_str::<WorkerCommand>(line).unwrap())
            .map(|command| match command {
                WorkerCommand::HealthCheck { .. } => "health.check",
                WorkerCommand::ModelLoad { .. } => "model.load",
                WorkerCommand::DictationStart => "dictation.start",
                WorkerCommand::DictationStop => "dictation.stop",
                WorkerCommand::Shutdown => "shutdown",
                WorkerCommand::TranscribeFile { .. } => "transcribe.file",
                WorkerCommand::DevicesList => "devices.list",
            })
            .collect();

        assert_eq!(
            command_types,
            vec![
                "health.check",
                "model.load",
                "dictation.start",
                "dictation.stop",
                "shutdown"
            ]
        );
    }

    #[test]
    fn keeps_v1_event_fixture_type_names_stable() {
        let event_types: Vec<String> = V1_EVENTS
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .map(|event| event["type"].as_str().unwrap().to_string())
            .collect();

        assert_eq!(
            event_types,
            vec![
                "health.ok",
                "model.loaded",
                "model.error",
                "transcript.partial",
                "transcript.final",
                "transcript.error",
                "audio.error",
                "error",
            ]
        );
    }
}
