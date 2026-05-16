from __future__ import annotations

import json
from io import StringIO
from pathlib import Path

from openwhisper_asr.engine import ASREngine
from openwhisper_asr.protocol import send_health_ok
from openwhisper_asr.recording import RecordingError, RecordingResult
from openwhisper_asr.types import TranscriptionResult
from openwhisper_asr.worker.stdio import (
    RecordingDictationHandler,
    WorkerConfig,
    WorkerWriter,
    handle_message,
    run_worker,
)


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
    assert engine.loaded_model == "base"
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


def _worker_config() -> WorkerConfig:
    return WorkerConfig(
        engine_name="whispercpp",
        model="base",
        device="cpu",
        profile=None,
        timeout_seconds=1,
        sample_rate_hz=16_000,
        channels=1,
        recording_device=None,
    )
