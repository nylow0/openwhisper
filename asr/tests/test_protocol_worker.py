from __future__ import annotations

import json
from io import StringIO
from pathlib import Path

import pytest

from openwhisper_asr.engine import ASREngine
from openwhisper_asr.protocol import send_health_ok
from openwhisper_asr.recording import RecordingError, RecordingResult
from openwhisper_asr.types import TranscriptionResult
from openwhisper_asr.worker.stdio import (
    RecordingDictationHandler,
    WorkerConfig,
    WorkerWriter,
    default_worker_config,
    handle_message,
    run_worker,
)

FIXTURE_DIR = Path(__file__).parents[1] / "test_data" / "protocol"


class FakeEngine(ASREngine):
    def __init__(self) -> None:
        self.loaded_model: str | None = None
        self.loaded_device: str | None = None
        self.transcribed_audio: str | None = None

    def load_model(self, model: str | None = None, device: str = "auto") -> None:
        self.loaded_model = model
        self.loaded_device = device

    def transcribe_batch(self, audio_path: str) -> TranscriptionResult:
        self.transcribed_audio = audio_path
        return TranscriptionResult(
            text="real recorded speech",
            words=[],
            language="en",
            is_partial=False,
            processing_latency_ms=42,
        )

    def transcribe_chunk(self, audio_pcm: bytes, sample_rate_hz: int) -> TranscriptionResult | None:
        return None


class FakeRecorder:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.started = False
        self.stopped = False
        self.cancelled = False

    def start(self) -> None:
        self.started = True

    def stop(self) -> RecordingResult:
        self.stopped = True
        return RecordingResult(
            path=self.path.resolve(),
            duration_seconds=1.0,
            sample_rate_hz=16_000,
            channels=1,
            frames=16_000,
        )

    def cancel(self) -> None:
        self.cancelled = True


class FailingRecorder(FakeRecorder):
    def start(self) -> None:
        raise RecordingError("microphone unavailable")


def test_send_health_ok_writes_ndjson() -> None:
    output = StringIO()

    send_health_ok(timestamp=123, output_stream=output)

    payload = json.loads(output.getvalue())
    assert payload == {"type": "health.ok", "timestamp": 123, "status": "ready"}


def test_worker_responds_to_health_check() -> None:
    input_stream = StringIO('{"type":"health.check","timestamp":7}\n{"type":"shutdown"}\n')
    output_stream = StringIO()

    exit_code = run_worker(input_stream=input_stream, output_stream=output_stream)

    assert exit_code == 0
    lines = [json.loads(line) for line in output_stream.getvalue().splitlines()]
    assert lines[0] == {"type": "health.ok", "timestamp": 7, "status": "ready"}


def test_v1_event_fixture_matches_python_protocol_helpers() -> None:
    lines = _load_fixture("v1_events.ndjson")

    assert lines[0] == {"type": "health.ok", "timestamp": 7, "status": "ready"}
    assert lines[1] == {"type": "model.loaded", "device": "cpu", "memory_mb": 0.0}
    assert lines[2] == {"type": "model.error", "error": "model load failed", "recoverable": True}
    assert lines[3] == {
        "type": "transcript.partial",
        "text": "hello",
        "is_final": False,
        "processing_latency_ms": 12,
    }
    assert lines[4]["type"] == "transcript.final"
    assert lines[4]["words"][0] == {"text": "hello", "start_ms": 0, "end_ms": 320, "confidence": 0.92}
    assert lines[5] == {"type": "transcript.error", "error": "decode failed", "chunk_timestamp": 7}
    assert lines[6] == {
        "type": "audio.error",
        "error": "microphone unavailable",
        "code": "AUDIO_RECORDING_FAILED",
    }
    assert lines[7] == {
        "type": "error",
        "code": "PROTOCOL_ERROR",
        "message": "Invalid JSON",
        "recoverable": True,
    }


def test_python_handler_accepts_v1_command_fixture() -> None:
    output = StringIO()
    writer = WorkerWriter(output)
    engine = FakeEngine()
    recorders: list[FakeRecorder] = []

    def make_recorder(path: Path) -> FakeRecorder:
        recorder = FakeRecorder(path)
        recorders.append(recorder)
        return recorder

    handler = RecordingDictationHandler(
        writer=writer,
        config=_worker_config(),
        engine_factory=lambda: engine,
        recorder_factory=make_recorder,
    )

    for command in _load_fixture("v1_commands.ndjson"):
        should_stop = handle_message(command, handler, writer)
        if should_stop:
            break

    lines = [json.loads(line) for line in output.getvalue().splitlines()]
    assert [line["type"] for line in lines] == [
        "health.ok",
        "model.loaded",
        "transcript.final",
    ]
    assert lines[-1]["text"] == "real recorded speech"


def test_dictation_stop_transcribes_the_recorded_wav_path() -> None:
    output = StringIO()
    writer = WorkerWriter(output)
    engine = FakeEngine()
    recorders: list[FakeRecorder] = []

    def make_recorder(path: Path) -> FakeRecorder:
        recorder = FakeRecorder(path)
        recorders.append(recorder)
        return recorder

    handler = RecordingDictationHandler(
        writer=writer,
        config=_worker_config(),
        engine_factory=lambda: engine,
        recorder_factory=make_recorder,
    )

    handle_message({"type": "dictation.start"}, handler, writer)
    handle_message({"type": "dictation.stop"}, handler, writer)

    lines = [json.loads(line) for line in output.getvalue().splitlines()]
    assert lines[0] == {"type": "model.loaded", "device": "cpu", "memory_mb": 0.0}
    assert lines[1]["type"] == "transcript.final"
    assert lines[1]["text"] == "real recorded speech"
    assert recorders[0].started is True
    assert recorders[0].stopped is True
    assert engine.loaded_model == "medium"
    assert engine.loaded_device == "cpu"
    assert engine.transcribed_audio == str(recorders[0].path.resolve())


def test_dictation_start_reports_audio_error_when_microphone_cannot_start() -> None:
    output = StringIO()
    writer = WorkerWriter(output)

    handler = RecordingDictationHandler(
        writer=writer,
        config=_worker_config(),
        engine_factory=FakeEngine,
        recorder_factory=FailingRecorder,
    )

    handle_message({"type": "dictation.start"}, handler, writer)

    lines = [json.loads(line) for line in output.getvalue().splitlines()]
    assert lines[0]["type"] == "model.loaded"
    assert lines[1] == {
        "type": "audio.error",
        "error": "microphone unavailable",
        "code": "AUDIO_RECORDING_FAILED",
    }


def test_default_worker_config_defaults_spoken_languages_to_english(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("OPENWHISPER_ASR_LANGUAGES", raising=False)

    config = default_worker_config()

    assert config.spoken_languages == ("en",)


def test_default_worker_config_treats_empty_spoken_languages_as_english(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("OPENWHISPER_ASR_LANGUAGES", "  ")

    config = default_worker_config()

    assert config.spoken_languages == ("en",)


def test_default_worker_config_normalizes_spoken_languages(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("OPENWHISPER_ASR_LANGUAGES", " PL, en,pl, DE ,zz,EN,yue,JA ")

    config = default_worker_config()

    assert config.spoken_languages == ("pl", "en", "de", "yue", "ja")


def test_default_worker_config_reads_auto_detect_language(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("OPENWHISPER_ASR_AUTO_DETECT_LANGUAGE", "true")

    config = default_worker_config()

    assert config.auto_detect_language is True


def test_default_worker_config_falls_back_to_english_when_all_spoken_languages_are_invalid(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("OPENWHISPER_ASR_LANGUAGES", "zz, unknown")

    config = default_worker_config()

    assert config.spoken_languages == ("en",)


def _worker_config() -> WorkerConfig:
    return WorkerConfig(
        engine_name="whispercpp",
        model="medium",
        device="cpu",
        spoken_languages=("en",),
        auto_detect_language=False,
        profile=None,
        timeout_seconds=1,
        sample_rate_hz=16_000,
        channels=1,
        recording_device=None,
    )


def _load_fixture(name: str) -> list[dict[str, object]]:
    return [json.loads(line) for line in (FIXTURE_DIR / name).read_text(encoding="utf-8").splitlines()]
