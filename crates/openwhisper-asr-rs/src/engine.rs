use std::path::Path;

use anyhow::Result;

use crate::protocol::WordResult;

#[derive(Debug, Clone, PartialEq)]
pub struct Transcription {
    pub text: String,
    pub words: Vec<WordResult>,
    pub language: Option<String>,
    pub processing_latency_ms: u32,
}

pub trait AsrEngine {
    fn load_model(&mut self, model: &str, device: &str) -> Result<ModelLoadInfo>;
    fn transcribe_file(&mut self, audio_path: &Path) -> Result<Transcription>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelLoadInfo {
    pub device: String,
    pub memory_mb: f64,
}

#[derive(Debug, Default)]
pub struct MockEngine {
    loaded: bool,
}

impl AsrEngine for MockEngine {
    fn load_model(&mut self, _model: &str, device: &str) -> Result<ModelLoadInfo> {
        self.loaded = true;
        Ok(ModelLoadInfo {
            device: if device.is_empty() {
                "cpu".to_string()
            } else {
                device.to_string()
            },
            memory_mb: 512.0,
        })
    }

    fn transcribe_file(&mut self, audio_path: &Path) -> Result<Transcription> {
        if !self.loaded {
            self.load_model("mock", "cpu")?;
        }

        let name = audio_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("audio");
        let text = format!("Mock transcript for {name}");

        Ok(Transcription {
            words: mock_words(&text),
            text,
            language: Some("en".to_string()),
            processing_latency_ms: 1,
        })
    }
}

pub fn mock_words(text: &str) -> Vec<WordResult> {
    let mut start_ms = 0;
    text.split_whitespace()
        .map(|word| {
            let duration_ms = (word.len() as u32 * 60).max(120);
            let result = WordResult {
                text: word.to_string(),
                start_ms,
                end_ms: start_ms + duration_ms,
                confidence: Some(0.92),
            };
            start_ms += duration_ms + 40;
            result
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{AsrEngine, MockEngine};

    #[test]
    fn mock_file_transcription_includes_source_name() {
        let mut engine = MockEngine::default();
        let result = engine.transcribe_file(Path::new("sample.wav")).unwrap();

        assert_eq!(result.text, "Mock transcript for sample.wav");
        assert_eq!(result.language.as_deref(), Some("en"));
        assert!(!result.words.is_empty());
    }
}
