"""OpenWhisper ASR protocol helpers: NDJSON over stdio."""

from __future__ import annotations

import json
import sys
from collections.abc import Iterator
from typing import TextIO, TypeAlias, cast

from openwhisper_asr.types import WordResult

JsonValue: TypeAlias = str | int | float | bool | None | list["JsonValue"] | dict[str, "JsonValue"]
JsonObject: TypeAlias = dict[str, JsonValue]


def read_messages(input_stream: TextIO | None = None, error_stream: TextIO | None = None) -> Iterator[JsonObject]:
    """Read JSON objects from a newline-delimited stream."""
    source = input_stream or sys.stdin
    errors = error_stream or sys.stdout

    for raw_line in source:
        line = raw_line.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError as exc:
            send_error(f"Invalid JSON: {exc}", recoverable=True, output_stream=errors)
            continue

        if not isinstance(payload, dict):
            send_error("Protocol message must be a JSON object", recoverable=True, output_stream=errors)
            continue

        yield cast(JsonObject, payload)


def send_message(msg: JsonObject, output_stream: TextIO | None = None) -> None:
    """Send one JSON object as an NDJSON line."""
    target = output_stream or sys.stdout
    print(json.dumps(msg), file=target, flush=True)


def send_error(
    message: str,
    recoverable: bool = True,
    code: str = "PROTOCOL_ERROR",
    details: JsonObject | None = None,
    output_stream: TextIO | None = None,
) -> None:
    """Send an error message."""
    msg: JsonObject = {
        "type": "error",
        "code": code,
        "message": message,
        "recoverable": recoverable,
    }
    if details is not None:
        msg["details"] = details
    send_message(msg, output_stream=output_stream)


def send_health_ok(timestamp: int, status: str = "ready", output_stream: TextIO | None = None) -> None:
    """Send a health.ok response."""
    send_message(
        {
            "type": "health.ok",
            "timestamp": timestamp,
            "status": status,
        },
        output_stream=output_stream,
    )


def send_transcript_partial(
    text: str,
    processing_latency_ms: int,
    output_stream: TextIO | None = None,
) -> None:
    """Send a partial transcript event."""
    send_message(
        {
            "type": "transcript.partial",
            "text": text,
            "is_final": False,
            "processing_latency_ms": processing_latency_ms,
        },
        output_stream=output_stream,
    )


def send_transcript_final(
    text: str,
    words: list[WordResult],
    language: str | None,
    processing_latency_ms: int,
    output_stream: TextIO | None = None,
) -> None:
    """Send a final transcript event."""
    send_message(
        {
            "type": "transcript.final",
            "text": text,
            "words": [
                {
                    "text": word.text,
                    "start_ms": word.start_ms,
                    "end_ms": word.end_ms,
                    "confidence": word.confidence,
                }
                for word in words
            ],
            "language": language,
            "processing_latency_ms": processing_latency_ms,
        },
        output_stream=output_stream,
    )


def send_model_loaded(device: str, memory_mb: float, output_stream: TextIO | None = None) -> None:
    """Send a model.loaded event."""
    send_message(
        {
            "type": "model.loaded",
            "device": device,
            "memory_mb": memory_mb,
        },
        output_stream=output_stream,
    )


def send_model_error(error: str, recoverable: bool, output_stream: TextIO | None = None) -> None:
    """Send a model.error event."""
    send_message(
        {
            "type": "model.error",
            "error": error,
            "recoverable": recoverable,
        },
        output_stream=output_stream,
    )


def send_audio_error(error: str, code: str, output_stream: TextIO | None = None) -> None:
    """Send an audio.error event."""
    send_message(
        {
            "type": "audio.error",
            "error": error,
            "code": code,
        },
        output_stream=output_stream,
    )


def send_transcript_error(error: str, chunk_timestamp: int, output_stream: TextIO | None = None) -> None:
    """Send a transcript.error event."""
    send_message(
        {
            "type": "transcript.error",
            "error": error,
            "chunk_timestamp": chunk_timestamp,
        },
        output_stream=output_stream,
    )
