"""NDJSON stdio worker used by product integrations."""

from __future__ import annotations

import logging
import sys
import threading
import time
from typing import TextIO

from openwhisper_asr.protocol import (
    JsonObject,
    JsonValue,
    read_messages,
    send_error,
    send_health_ok,
    send_message,
    send_model_loaded,
    send_transcript_final,
    send_transcript_partial,
)
from openwhisper_asr.types import WordResult

logger = logging.getLogger(__name__)

MOCK_SENTENCES = [
    "Hello world this is a test of the OpenWhisper dictation system",
    "The quick brown fox jumps over the lazy dog",
    "Speech recognition is working correctly in offline mode",
    "This is a mock transcript for testing the protocol layer",
    "OpenWhisper runs entirely on your local machine for privacy",
]


class WorkerWriter:
    """Thread-safe protocol writer."""

    def __init__(self, output_stream: TextIO) -> None:
        self._output_stream = output_stream
        self._lock = threading.Lock()

    def send(self, msg: JsonObject) -> None:
        with self._lock:
            send_message(msg, output_stream=self._output_stream)

    def error(self, message: str, recoverable: bool = True, code: str = "PROTOCOL_ERROR") -> None:
        with self._lock:
            send_error(message=message, recoverable=recoverable, code=code, output_stream=self._output_stream)

    def health_ok(self, timestamp: int, status: str = "ready") -> None:
        with self._lock:
            send_health_ok(timestamp=timestamp, status=status, output_stream=self._output_stream)

    def model_loaded(self, device: str, memory_mb: float) -> None:
        with self._lock:
            send_model_loaded(device=device, memory_mb=memory_mb, output_stream=self._output_stream)

    def transcript_partial(self, text: str, processing_latency_ms: int) -> None:
        with self._lock:
            send_transcript_partial(
                text=text,
                processing_latency_ms=processing_latency_ms,
                output_stream=self._output_stream,
            )

    def transcript_final(
        self,
        text: str,
        words: list[WordResult],
        language: str | None,
        processing_latency_ms: int,
    ) -> None:
        with self._lock:
            send_transcript_final(
                text=text,
                words=words,
                language=language,
                processing_latency_ms=processing_latency_ms,
                output_stream=self._output_stream,
            )


class MockTranscriptGenerator:
    """Generates mock transcript events for protocol testing."""

    def __init__(self, writer: WorkerWriter) -> None:
        self._writer = writer
        self._running = False
        self._thread: threading.Thread | None = None
        self._sentence_idx = 0

    def start(self) -> None:
        if self._running:
            return
        self._running = True
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()
        logger.info("Mock transcript generator started")

    def stop(self) -> None:
        self._running = False
        if self._thread is not None:
            self._thread.join(timeout=1.0)
        logger.info("Mock transcript generator stopped")

    def _run(self) -> None:
        while self._running:
            sentence = MOCK_SENTENCES[self._sentence_idx % len(MOCK_SENTENCES)]
            words = sentence.split()

            for index in range(1, len(words) + 1):
                if not self._running:
                    return
                partial_text = " ".join(words[:index])
                self._writer.transcript_partial(partial_text, processing_latency_ms=50)
                time.sleep(0.3)

            if self._running:
                word_results: list[WordResult] = []
                start_ms = 0
                for word in words:
                    duration_ms = max(100, len(word) * 80)
                    word_results.append(
                        WordResult(
                            text=word,
                            start_ms=start_ms,
                            end_ms=start_ms + duration_ms,
                            confidence=0.92,
                        )
                    )
                    start_ms += duration_ms + 50

                self._writer.transcript_final(
                    text=sentence,
                    words=word_results,
                    language="en",
                    processing_latency_ms=120,
                )
                time.sleep(1.0)

            self._sentence_idx += 1


def _string_value(value: JsonValue | None, fallback: str) -> str:
    return value if isinstance(value, str) else fallback


def _int_value(value: JsonValue | None, fallback: int) -> int:
    return value if isinstance(value, int) else fallback


def handle_message(msg: JsonObject, generator: MockTranscriptGenerator, writer: WorkerWriter) -> bool:
    """Handle one incoming message. Returns True when the worker should stop."""
    msg_type = _string_value(msg.get("type"), "")

    if msg_type == "health.check":
        writer.health_ok(
            timestamp=_int_value(msg.get("timestamp"), int(time.time())),
            status="ready",
        )
        return False

    if msg_type == "dictation.start":
        logger.info("Dictation start requested")
        generator.start()
        return False

    if msg_type == "dictation.stop":
        logger.info("Dictation stop requested")
        generator.stop()
        return False

    if msg_type == "shutdown":
        logger.info("Shutdown requested")
        generator.stop()
        return True

    if msg_type == "model.load":
        logger.info("Model load requested: %s", msg)
        writer.model_loaded(device=_string_value(msg.get("device"), "cpu"), memory_mb=512.0)
        return False

    if msg_type == "settings.update":
        logger.info("Settings update: %s", msg)
        return False

    if msg_type == "audio.chunk":
        return False

    logger.warning("Unknown message type: %s", msg_type)
    writer.error(message=f"Unknown message type: {msg_type}", recoverable=True, code="PROTOCOL_ERROR")
    return False


def run_worker(input_stream: TextIO | None = None, output_stream: TextIO | None = None) -> int:
    """Run the stdio worker until shutdown or EOF."""
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        stream=sys.stderr,
    )
    logger.info("ASR worker starting")

    source = input_stream or sys.stdin
    target = output_stream or sys.stdout
    writer = WorkerWriter(target)
    generator = MockTranscriptGenerator(writer)

    try:
        for msg in read_messages(input_stream=source, error_stream=target):
            should_stop = handle_message(msg, generator, writer)
            if should_stop:
                break
    except KeyboardInterrupt:
        logger.info("Interrupted by user")
    finally:
        generator.stop()

    logger.info("ASR worker shutting down")
    return 0
