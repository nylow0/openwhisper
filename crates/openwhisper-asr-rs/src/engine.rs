use std::path::Path;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Transcription {
    pub text: String,
    pub language: Option<String>,
    pub is_partial: bool,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("audio path does not exist: {0}")]
    MissingAudio(String),
    #[error("engine backend error: {0}")]
    Backend(String),
}

pub trait AsrEngine {
    fn name(&self) -> &'static str;
    fn transcribe_file(&self, audio_path: &Path) -> Result<Transcription, EngineError>;
}

#[derive(Default)]
pub struct MockEngine;

impl AsrEngine for MockEngine {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn transcribe_file(&self, audio_path: &Path) -> Result<Transcription, EngineError> {
        if !audio_path.is_file() {
            return Err(EngineError::MissingAudio(audio_path.display().to_string()));
        }

        Ok(Transcription {
            text: format!("mock transcription for {}", audio_path.display()),
            language: Some("en".to_string()),
            is_partial: false,
        })
    }
}
