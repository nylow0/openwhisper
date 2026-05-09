"""OpenWhisper ASR Protocol - NDJSON over stdio."""

import sys
import json
import logging
from typing import Iterator, Dict, Any, Optional, List
from dataclasses import dataclass

logger = logging.getLogger(__name__)


@dataclass
class WordResult:
    text: str
    start_ms: int
    end_ms: int
    confidence: Optional[float] = None


@dataclass
class TranscriptionResult:
    text: str
    words: List[WordResult]
    language: Optional[str]
    is_partial: bool
    processing_latency_ms: int


def read_messages() -> Iterator[Dict[str, Any]]:
    """Read NDJSON messages from stdin."""
    for line in sys.stdin:
        line = line.strip()
        if line:
            try:
                yield json.loads(line)
            except json.JSONDecodeError as e:
                send_error(f"Invalid JSON: {e}", recoverable=True)


def send_message(msg: Dict[str, Any]) -> None:
    """Send NDJSON message to stdout."""
    print(json.dumps(msg), flush=True)


def send_error(message: str, recoverable: bool = True, code: str = "PROTOCOL_ERROR", details: Optional[Dict[str, Any]] = None) -> None:
    """Send error message."""
    msg: Dict[str, Any] = {
        "type": "error",
        "code": code,
        "message": message,
        "recoverable": recoverable,
    }
    if details:
        msg["details"] = details
    send_message(msg)


def send_health_ok(timestamp: int, status: str = "ready") -> None:
    """Send health.ok response."""
    send_message({
        "type": "health.ok",
        "timestamp": timestamp,
        "status": status,
    })


def send_transcript_partial(text: str, processing_latency_ms: int) -> None:
    """Send partial transcript."""
    send_message({
        "type": "transcript.partial",
        "text": text,
        "is_final": False,
        "processing_latency_ms": processing_latency_ms,
    })


def send_transcript_final(text: str, words: List[WordResult], language: Optional[str], processing_latency_ms: int) -> None:
    """Send final transcript."""
    send_message({
        "type": "transcript.final",
        "text": text,
        "words": [
            {
                "text": w.text,
                "start_ms": w.start_ms,
                "end_ms": w.end_ms,
                "confidence": w.confidence,
            }
            for w in words
        ],
        "language": language,
        "processing_latency_ms": processing_latency_ms,
    })


def send_model_loaded(device: str, memory_mb: float) -> None:
    """Send model.loaded event."""
    send_message({
        "type": "model.loaded",
        "device": device,
        "memory_mb": memory_mb,
    })


def send_model_error(error: str, recoverable: bool) -> None:
    """Send model.error event."""
    send_message({
        "type": "model.error",
        "error": error,
        "recoverable": recoverable,
    })
