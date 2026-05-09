use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub words: Vec<WordResult>,
    pub language: Option<String>,
    #[serde(rename = "isPartial", alias = "is_partial")]
    pub is_partial: bool,
    #[serde(rename = "processingLatencyMs", alias = "processing_latency_ms")]
    pub processing_latency_ms: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WordResult {
    pub text: String,
    #[serde(rename = "startMs", alias = "start_ms")]
    pub start_ms: u32,
    #[serde(rename = "endMs", alias = "end_ms")]
    pub end_ms: u32,
    pub confidence: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PythonSettings {
    pub language: Option<String>,
    pub chunk_duration_ms: u32,
    pub vad_enabled: bool,
}

// ─── Messages Rust → Python ───

#[allow(dead_code)]
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ToPython {
    #[serde(rename = "health.check")]
    HealthCheck { timestamp: u64 },

    #[serde(rename = "dictation.start")]
    DictationStart {
        language: Option<String>,
        chunk_duration_ms: u32,
    },

    #[serde(rename = "dictation.stop")]
    DictationStop,

    #[serde(rename = "audio.chunk")]
    AudioChunk {
        data: String,
        timestamp: u64,
        is_final: bool,
    },

    #[serde(rename = "model.load")]
    ModelLoad {
        model_path: String,
        device: String,
    },

    #[serde(rename = "settings.update")]
    SettingsUpdate { settings: PythonSettings },

    #[serde(rename = "shutdown")]
    Shutdown,
}

// ─── Messages Python → Rust ───

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FromPython {
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
