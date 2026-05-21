use std::io::{self, BufRead, Write};

use serde_json::json;

use crate::audio_capture::list_input_devices;
use crate::buffering::RingPcmBuffer;
use crate::engine::{AsrEngine, MockEngine};
use crate::protocol::{Request, Response};
use crate::vad::{is_speech, VadConfig};

pub fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let output = match serde_json::from_str::<Request>(&line) {
            Ok(req) => handle_request(req),
            Err(err) => Response {
                kind: "error",
                ok: false,
                timestamp: 0,
                message: Some(format!("invalid_json: {err}")),
                payload: None,
            },
        };

        serde_json::to_writer(&mut stdout, &output)?;
        writeln!(&mut stdout)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_request(req: Request) -> Response<'static> {
    match req.kind.as_str() {
        "health.check" => Response {
            kind: "health.ok",
            ok: true,
            timestamp: req.timestamp.unwrap_or_default(),
            message: Some("openwhisper-asr-rs ready".to_string()),
            payload: Some(json!({"engine": "mock", "version": env!("CARGO_PKG_VERSION")})),
        },
        "transcribe.file" => {
            let Some(audio_path) = payload_audio_path(&req.payload) else {
                return Response {
                    kind: "error",
                    ok: false,
                    timestamp: req.timestamp.unwrap_or_default(),
                    message: Some("missing_payload_audio_path".to_string()),
                    payload: Some(json!({"expected": {"audio_path": "<path>"}})),
                };
            };

            let engine = MockEngine;
            match engine.transcribe_file(std::path::Path::new(audio_path)) {
                Ok(t) => Response {
                    kind: "transcription.final",
                    ok: true,
                    timestamp: req.timestamp.unwrap_or_default(),
                    message: None,
                    payload: Some(
                        json!({"text": t.text, "language": t.language, "partial": t.is_partial, "engine": engine.name()}),
                    ),
                },
                Err(err) => Response {
                    kind: "error",
                    ok: false,
                    timestamp: req.timestamp.unwrap_or_default(),
                    message: Some(err.to_string()),
                    payload: None,
                },
            }
        }
        "devices.list" => Response {
            kind: "devices.result",
            ok: true,
            timestamp: req.timestamp.unwrap_or_default(),
            message: None,
            payload: Some(json!({"devices": list_input_devices()})),
        },
        "transcribe.mock" => Response {
            kind: "transcription.final",
            ok: true,
            timestamp: req.timestamp.unwrap_or_default(),
            message: None,
            payload: Some(json!({"text": "hello from rust asr mock"})),
        },

        "stream.mock" => {
            let samples: Vec<f32> = vec![0.0, 0.02, -0.01, 0.03, -0.02, 0.0, 0.01];
            let mut ring = RingPcmBuffer::new(16_000);
            ring.push_chunk(&samples);
            let window = ring.tail_window(8_000);
            let speech = is_speech(&window, VadConfig::default());

            Response {
                kind: "transcription.partial",
                ok: true,
                timestamp: req.timestamp.unwrap_or_default(),
                message: None,
                payload: Some(json!({
                    "text": if speech { "mock partial speech" } else { "" },
                    "speech_detected": speech,
                    "window_samples": window.len()
                })),
            }
        }
        "worker.shutdown" => Response {
            kind: "worker.bye",
            ok: true,
            timestamp: req.timestamp.unwrap_or_default(),
            message: Some("shutdown requested".to_string()),
            payload: None,
        },
        other => Response {
            kind: "error",
            ok: false,
            timestamp: req.timestamp.unwrap_or_default(),
            message: Some(format!("unsupported_request_type: {other}")),
            payload: Some(
                json!({"supported": ["health.check", "transcribe.mock", "worker.shutdown", "stream.mock", "transcribe.file", "devices.list"]}),
            ),
        },
    }
}

fn payload_audio_path(payload: &serde_json::Value) -> Option<&str> {
    payload
        .as_object()
        .and_then(|obj| obj.get("audio_path"))
        .and_then(|v| v.as_str())
}

#[cfg(test)]
mod tests {
    use super::handle_request;
    use crate::protocol::Request;
    use serde_json::json;

    #[test]
    fn health_check_returns_ready() {
        let response = handle_request(Request {
            kind: "health.check".to_string(),
            timestamp: Some(1),
            payload: json!({}),
        });

        assert!(response.ok);
        assert_eq!(response.kind, "health.ok");
    }
}
