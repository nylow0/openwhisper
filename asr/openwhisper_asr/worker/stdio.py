"""NDJSON stdio worker used by product integrations."""

from __future__ import annotations

import logging
import os
import sys
import threading
import time
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol, TextIO

from openwhisper_asr.engine import ASREngine
from openwhisper_asr.engines.whisper_cpp import WhisperCppEngine
from openwhisper_asr.protocol import (
    JsonObject,
    JsonValue,
    read_messages,
    send_audio_error,
    send_error,
    send_health_ok,
    send_message,
    send_model_error,
    send_model_loaded,
    send_transcript_error,
    send_transcript_final,
    send_transcript_partial,
)
from openwhisper_asr.recording import (
    DEFAULT_CHANNELS,
    DEFAULT_SAMPLE_RATE_HZ,
    RecordingError,
    RecordingResult,
    RecordingSession,
    default_recording_path,
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


@dataclass(frozen=True)
class WorkerConfig:
    engine_name: str
    model: str
    device: str
    profile: str | None
    timeout_seconds: int
    sample_rate_hz: int
    channels: int
    recording_device: int | str | None


class Recorder(Protocol):
    path: Path

    def start(self) -> None: ...

    def stop(self) -> RecordingResult: ...

    def cancel(self) -> None: ...


class DictationHandler(Protocol):
    def start(self, msg: JsonObject) -> None: ...

    def stop(self) -> None: ...

    def shutdown(self) -> None: ...

    def load_model(self, msg: JsonObject) -> None: ...


EngineFactory = Callable[[], ASREngine]
RecorderFactory = Callable[[Path], Recorder]


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

    def audio_error(self, error: str, code: str = "AUDIO_ERROR") -> None:
        with self._lock:
            send_audio_error(error=error, code=code, output_stream=self._output_stream)

    def model_error(self, error: str, recoverable: bool = True) -> None:
        with self._lock:
            send_model_error(error=error, recoverable=recoverable, output_stream=self._output_stream)

    def transcript_error(self, error: str, chunk_timestamp: int) -> None:
        with self._lock:
            send_transcript_error(error=error, chunk_timestamp=chunk_timestamp, output_stream=self._output_stream)

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

    def start(self, msg: JsonObject | None = None) -> None:
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

    def shutdown(self) -> None:
        self.stop()

    def load_model(self, msg: JsonObject) -> None:
        writer_device = _string_value(msg.get("device"), "cpu")
        self._writer.model_loaded(device=writer_device, memory_mb=512.0)

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


class RecordingDictationHandler:
    """Owns the real record-then-transcribe dictation flow."""

    def __init__(
        self,
        writer: WorkerWriter,
        config: WorkerConfig | None = None,
        engine_factory: EngineFactory | None = None,
        recorder_factory: RecorderFactory | None = None,
    ) -> None:
        self._writer = writer
        self._config = config or default_worker_config()
        self._engine_factory = engine_factory or self._create_engine
        self._recorder_factory = recorder_factory or self._create_recorder
        self._engine: ASREngine | None = None
        self._session: Recorder | None = None

    def start(self, msg: JsonObject) -> None:
        if self._session is not None:
            self._writer.error(
                message="Dictation is already recording",
                recoverable=True,
                code="DICTATION_ALREADY_RUNNING",
            )
            return

        try:
            self._ensure_engine_loaded()
        except Exception as exc:
            self._writer.model_error(error=str(exc), recoverable=True)
            return

        try:
            session = self._recorder_factory(default_recording_path())
            session.start()
        except Exception as exc:
            self._writer.audio_error(error=str(exc), code="AUDIO_RECORDING_FAILED")
            return

        self._session = session
        logger.info("Recording started: %s", session.path)

    def stop(self) -> None:
        session = self._session
        if session is None:
            self._writer.error(
                message="Dictation is not recording",
                recoverable=True,
                code="DICTATION_NOT_RUNNING",
            )
            return

        self._session = None
        try:
            recording = session.stop()
        except RecordingError as exc:
            self._writer.audio_error(error=str(exc), code="AUDIO_RECORDING_FAILED")
            return

        logger.info("Recording stopped: %s", recording.path)
        try:
            result = self._ensure_engine_loaded().transcribe_batch(str(recording.path))
        except Exception as exc:
            self._writer.transcript_error(error=str(exc), chunk_timestamp=int(time.time()))
            return

        self._writer.transcript_final(
            text=result.text,
            words=result.words,
            language=result.language,
            processing_latency_ms=result.processing_latency_ms,
        )

    def shutdown(self) -> None:
        if self._session is not None:
            self._session.cancel()
            self._session = None

    def load_model(self, msg: JsonObject) -> None:
        try:
            self._ensure_engine_loaded()
        except Exception as exc:
            self._writer.model_error(error=str(exc), recoverable=True)

    def _ensure_engine_loaded(self) -> ASREngine:
        if self._engine is not None:
            return self._engine

        engine = self._engine_factory()
        engine.load_model(model=self._config.model, device=self._config.device)
        self._engine = engine
        self._send_model_loaded(engine)
        return engine

    def _send_model_loaded(self, engine: ASREngine) -> None:
        device = self._config.device
        memory_mb = 0.0
        if isinstance(engine, WhisperCppEngine):
            device = engine.loaded_device or device
            memory_mb = engine.loaded_memory_mb
        self._writer.model_loaded(device=device, memory_mb=memory_mb)

    def _create_engine(self) -> ASREngine:
        if self._config.engine_name != "whispercpp":
            raise RuntimeError(f"Unsupported ASR worker engine: {self._config.engine_name}")
        return WhisperCppEngine(
            profile=self._config.profile,
            timeout_seconds=self._config.timeout_seconds,
        )

    def _create_recorder(self, path: Path) -> Recorder:
        return RecordingSession(
            output_path=path,
            sample_rate_hz=self._config.sample_rate_hz,
            channels=self._config.channels,
            device=self._config.recording_device,
        )


def _string_value(value: JsonValue | None, fallback: str) -> str:
    return value if isinstance(value, str) else fallback


def _int_value(value: JsonValue | None, fallback: int) -> int:
    return value if isinstance(value, int) else fallback


def handle_message(msg: JsonObject, dictation: DictationHandler, writer: WorkerWriter) -> bool:
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
        dictation.start(msg)
        return False

    if msg_type == "dictation.stop":
        logger.info("Dictation stop requested")
        dictation.stop()
        return False

    if msg_type == "shutdown":
        logger.info("Shutdown requested")
        dictation.shutdown()
        return True

    if msg_type == "model.load":
        logger.info("Model load requested: %s", msg)
        dictation.load_model(msg)
        return False

    if msg_type == "settings.update":
        logger.info("Settings update: %s", msg)
        return False

    if msg_type == "audio.chunk":
        return False

    logger.warning("Unknown message type: %s", msg_type)
    writer.error(message=f"Unknown message type: {msg_type}", recoverable=True, code="PROTOCOL_ERROR")
    return False


def default_worker_config() -> WorkerConfig:
    return WorkerConfig(
        engine_name=os.environ.get("OPENWHISPER_ASR_ENGINE", "whispercpp").lower(),
        model=os.environ.get("OPENWHISPER_ASR_MODEL", "medium_en_q8"),
        device=os.environ.get("OPENWHISPER_ASR_DEVICE", "auto"),
        profile=_optional_env("OPENWHISPER_ASR_PROFILE"),
        timeout_seconds=_int_env("OPENWHISPER_ASR_TIMEOUT_SECONDS", 300),
        sample_rate_hz=_int_env("OPENWHISPER_RECORDING_SAMPLE_RATE_HZ", DEFAULT_SAMPLE_RATE_HZ),
        channels=_int_env("OPENWHISPER_RECORDING_CHANNELS", DEFAULT_CHANNELS),
        recording_device=_device_arg(_optional_env("OPENWHISPER_RECORDING_DEVICE")),
    )


def _optional_env(name: str) -> str | None:
    value = os.environ.get(name)
    if value is None or not value.strip():
        return None
    return value


def _int_env(name: str, fallback: int) -> int:
    raw = os.environ.get(name)
    if raw is None:
        return fallback
    try:
        return int(raw)
    except ValueError:
        logger.warning("Ignoring invalid integer env %s=%s", name, raw)
        return fallback


def _device_arg(value: str | None) -> int | str | None:
    if value is None:
        return None
    try:
        return int(value)
    except ValueError:
        return value


def _create_default_dictation_handler(writer: WorkerWriter) -> DictationHandler:
    config = default_worker_config()
    if config.engine_name == "mock":
        return MockTranscriptGenerator(writer)
    return RecordingDictationHandler(writer, config=config)


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
    dictation = _create_default_dictation_handler(writer)

    try:
        for msg in read_messages(input_stream=source, error_stream=target):
            should_stop = handle_message(msg, dictation, writer)
            if should_stop:
                break
    except KeyboardInterrupt:
        logger.info("Interrupted by user")
    finally:
        dictation.shutdown()

    logger.info("ASR worker shutting down")
    return 0
