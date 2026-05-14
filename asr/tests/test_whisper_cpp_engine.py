from __future__ import annotations

from openwhisper_asr.engines.whisper_cpp import parse_whisper_cpp_payload


def test_parse_whisper_cpp_payload_returns_final_text() -> None:
    payload: dict[str, object] = {
        "result": {"language": "en"},
        "transcription": [
            {"text": " Concorde returned"},
            {"text": " to its place."},
        ],
    }

    result = parse_whisper_cpp_payload(payload, processing_latency_ms=1234)

    assert result.text == "Concorde returned to its place."
    assert result.language == "en"
    assert result.is_partial is False
    assert result.processing_latency_ms == 1234
