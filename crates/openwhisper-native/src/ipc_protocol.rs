use serde::{Deserialize, Serialize};

// ─── Electron ↔ Rust IPC Messages ───

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum IpcCommand {
    #[serde(rename = "dictation.start")]
    DictationStart,

    #[serde(rename = "dictation.stop")]
    DictationStop,

    #[serde(rename = "status.get")]
    GetStatus,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum IpcEvent {
    #[serde(rename = "dictation.started")]
    DictationStarted { timestamp: u64 },

    #[serde(rename = "dictation.stopped")]
    DictationStopped { timestamp: u64 },

    #[serde(rename = "transcript.partial")]
    TranscriptPartial {
        text: String,
        #[serde(rename = "isFinal")]
        is_final: bool,
        #[serde(rename = "processingLatencyMs")]
        processing_latency_ms: u32,
    },

    #[serde(rename = "transcript.final")]
    TranscriptFinal {
        text: String,
        words: Vec<crate::protocol::WordResult>,
        language: Option<String>,
        #[serde(rename = "processingLatencyMs")]
        processing_latency_ms: u32,
    },

    #[serde(rename = "status")]
    Status {
        #[serde(rename = "isDictating")]
        is_dictating: bool,
        #[serde(rename = "isModelLoaded")]
        is_model_loaded: bool,
        #[serde(rename = "workerHealthy")]
        worker_healthy: bool,
    },

    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}
