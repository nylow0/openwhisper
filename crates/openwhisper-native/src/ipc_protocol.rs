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
        #[serde(rename = "speechSegments")]
        speech_segments: Vec<crate::protocol::SpeechSegment>,
        #[serde(rename = "audioPath", skip_serializing_if = "Option::is_none")]
        audio_path: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::IpcEvent;
    use crate::protocol::FromWorker;

    #[test]
    fn transcript_metadata_survives_worker_to_desktop_protocol() {
        let from_worker: FromWorker = serde_json::from_str(r#"{
            "type":"transcript.final","text":" Привіт","language":"uk",
            "processing_latency_ms":120,
            "words":[{"text":" Привіт","start_ms":100,"end_ms":650,"confidence":0.94}],
            "speech_segments":[{"start_ms":80,"end_ms":700}],
            "audio_path":"C:\\recordings\\clip.wav"
        }"#).unwrap();
        let FromWorker::TranscriptFinal { text, words, language, processing_latency_ms, speech_segments, audio_path } = from_worker else {
            panic!("expected transcript.final");
        };
        let event = IpcEvent::TranscriptFinal { text, words, language, processing_latency_ms, speech_segments, audio_path };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["words"][0]["startMs"], 100);
        assert!((value["words"][0]["confidence"].as_f64().unwrap() - 0.94).abs() < 0.001);
        assert_eq!(value["speechSegments"][0]["endMs"], 700);
        assert_eq!(value["audioPath"], "C:\\recordings\\clip.wav");
    }
}
