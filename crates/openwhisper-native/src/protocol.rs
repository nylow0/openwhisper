use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WordResult {
    pub text: String,
    #[serde(rename = "startMs", alias = "start_ms")]
    pub start_ms: u32,
    #[serde(rename = "endMs", alias = "end_ms")]
    pub end_ms: u32,
    pub confidence: Option<f32>,
}

// ─── Messages Rust → Python ───

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ToWorker {
    #[serde(rename = "health.check")]
    HealthCheck { timestamp: u64 },

    #[serde(rename = "dictation.start")]
    DictationStart,

    #[serde(rename = "dictation.stop")]
    DictationStop,
}

// ─── Messages Python → Rust ───

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FromWorker {
    #[serde(rename = "health.ok")]
    HealthOk { timestamp: u64, status: String },

    #[serde(rename = "model.loaded")]
    ModelLoaded { device: String, memory_mb: f64 },

    #[serde(rename = "model.error")]
    ModelError { error: String, recoverable: bool },

    #[serde(rename = "transcript.partial")]
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
    AudioError { error: String, code: String },

    #[serde(rename = "error")]
    Error {
        code: Option<String>,
        message: String,
        recoverable: bool,
    },
}
