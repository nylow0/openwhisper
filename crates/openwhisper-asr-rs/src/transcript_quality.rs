//! Language-agnostic filters for whisper.cpp output on silence/noise.

use crate::speech_analysis::SpeechAnalysis;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DecodeQuality {
    pub audio_duration_ms: u64,
    pub content_token_count: usize,
    pub avg_content_token_probability: Option<f32>,
}

impl DecodeQuality {
    pub fn from_whisper_payload(payload: &serde_json::Value, audio_duration_ms: u64) -> Self {
        let mut probabilities = Vec::new();

        if let Some(segments) = payload
            .get("transcription")
            .and_then(|value| value.as_array())
        {
            for segment in segments {
                let Some(tokens) = segment.get("tokens").and_then(|value| value.as_array()) else {
                    continue;
                };
                for token in tokens {
                    let text = token
                        .get("text")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    if is_special_whisper_token(text) {
                        continue;
                    }
                    let Some(probability) = token.get("p").and_then(|value| value.as_f64()) else {
                        continue;
                    };
                    probabilities.push(probability as f32);
                }
            }
        }

        let content_token_count = probabilities.len();
        let avg_content_token_probability = if probabilities.is_empty() {
            None
        } else {
            Some(probabilities.iter().sum::<f32>() / probabilities.len() as f32)
        };

        Self {
            audio_duration_ms,
            content_token_count,
            avg_content_token_probability,
        }
    }
}

/// Returns empty string when decode output should be treated as silence hallucination.
pub fn scrub_transcript(text: &str, quality: DecodeQuality, speech: SpeechAnalysis) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() || should_discard_transcript(trimmed, quality, speech) {
        return String::new();
    }
    trimmed.to_string()
}

pub fn should_discard_transcript(
    text: &str,
    quality: DecodeQuality,
    speech: SpeechAnalysis,
) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }

    if is_repetitive_token_sequence(trimmed) {
        return true;
    }

    if is_sparse_text_on_long_audio(trimmed, quality.audio_duration_ms) {
        return true;
    }

    if let Some(avg_probability) = quality.avg_content_token_probability {
        if quality.content_token_count <= 2 && avg_probability < 0.45 {
            return true;
        }
        if trimmed.chars().count() <= 40 && avg_probability < 0.35 {
            return true;
        }
        if quality.content_token_count <= 6 && avg_probability < 0.30 {
            return true;
        }
        return false;
    }

    // Fallback when token probabilities are unavailable (no -ojf).
    speech.peak_to_noise_ratio() < 4.0 && trimmed.chars().count() <= 24
}

fn is_sparse_text_on_long_audio(text: &str, audio_duration_ms: u64) -> bool {
    if audio_duration_ms < 3_000 {
        return false;
    }

    let char_count = text.chars().count();
    if char_count >= 40 {
        return false;
    }

    let chars_per_second = char_count as f32 / (audio_duration_ms as f32 / 1_000.0);
    chars_per_second < 1.0
}

fn is_repetitive_token_sequence(text: &str) -> bool {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    tokens.len() >= 3 && tokens.iter().all(|token| *token == tokens[0])
}

fn is_special_whisper_token(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.is_empty() || trimmed.starts_with("<|")
}

#[cfg(test)]
mod tests {
    use super::{scrub_transcript, DecodeQuality};
    use super::DecodeQuality::from_whisper_payload;
    use crate::speech_analysis::{analyze_speech, SpeechAnalysis};
    use crate::vad::VadConfig;

    fn flat_noise_speech() -> SpeechAnalysis {
        SpeechAnalysis {
            sample_rate_hz: 16_000,
            noise_floor_rms: 0.02,
            peak_rms: 0.023,
            effective_threshold: 0.09,
            speech_sample_count: 4_000,
            total_samples: 32_000,
        }
    }

    #[test]
    fn low_confidence_you_on_long_audio_is_discarded_without_english_word_list() {
        let payload = serde_json::json!({
            "transcription": [{
                "text": " you",
                "tokens": [
                    {"text": " you", "p": 0.17},
                    {"text": "<|endoftext|>", "p": 0.87}
                ]
            }]
        });
        let quality = DecodeQuality::from_whisper_payload(&payload, 30_000);
        assert!(quality.avg_content_token_probability.unwrap() < 0.35);
        assert_eq!(
            scrub_transcript("you", quality, flat_noise_speech()),
            ""
        );
    }

    #[test]
    fn high_confidence_sentence_is_kept() {
        let payload = serde_json::json!({
            "transcription": [{
                "text": " Schedule the meeting tomorrow morning",
                "tokens": [
                    {"text": " Schedule", "p": 0.91},
                    {"text": " the", "p": 0.88},
                    {"text": " meeting", "p": 0.86},
                    {"text": " tomorrow", "p": 0.84},
                    {"text": " morning", "p": 0.82}
                ]
            }]
        });
        let quality = DecodeQuality::from_whisper_payload(&payload, 4_000);
        let speech = analyze_speech(&vec![0.08; 8_000], VadConfig::default(), 16_000);
        let text = "Schedule the meeting tomorrow morning";
        assert_eq!(scrub_transcript(text, quality, speech), text);
    }
}
