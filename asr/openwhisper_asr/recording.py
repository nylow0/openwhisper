"""Microphone recording helpers for ASR smoke tests and app integration."""

from __future__ import annotations

import importlib
import wave
from collections.abc import Sequence
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from types import TracebackType
from typing import Protocol, cast

DEFAULT_SAMPLE_RATE_HZ = 16_000
DEFAULT_CHANNELS = 1
DEFAULT_BLOCKSIZE = 1024
DEFAULT_DTYPE = "int16"
SAMPLE_WIDTH_BYTES = 2


class RecordingError(RuntimeError):
    """Raised when microphone recording cannot start or complete."""


class _InputStream(Protocol):
    def __enter__(self) -> "_InputStream": ...

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...

    def read(self, frames: int) -> tuple[object, bool]: ...


class _SoundDevice(Protocol):
    def RawInputStream(
        self,
        *,
        samplerate: int,
        channels: int,
        dtype: str,
        blocksize: int,
        device: int | str | None,
    ) -> _InputStream: ...

    def query_devices(self) -> object: ...


@dataclass(frozen=True)
class RecordingResult:
    path: Path
    duration_seconds: float
    sample_rate_hz: int
    channels: int
    frames: int


def default_recording_path() -> Path:
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    return Path(".local") / "recordings" / f"recording-{timestamp}.wav"


def list_input_devices() -> list[dict[str, object]]:
    sounddevice = _sounddevice()
    devices = sounddevice.query_devices()
    rows: list[dict[str, object]] = []

    if not isinstance(devices, Sequence) or isinstance(devices, str | bytes):
        return rows

    for index, device in enumerate(devices):
        if not isinstance(device, dict):
            continue
        max_input_channels = device.get("max_input_channels")
        if not isinstance(max_input_channels, int) or max_input_channels <= 0:
            continue
        rows.append(
            {
                "index": index,
                "name": _device_string(device.get("name")),
                "hostapi": device.get("hostapi"),
                "max_input_channels": max_input_channels,
                "default_samplerate": device.get("default_samplerate"),
            }
        )

    return rows


def record_wav(
    output_path: str | Path,
    duration_seconds: float,
    sample_rate_hz: int = DEFAULT_SAMPLE_RATE_HZ,
    channels: int = DEFAULT_CHANNELS,
    device: int | str | None = None,
    blocksize: int = DEFAULT_BLOCKSIZE,
) -> RecordingResult:
    if duration_seconds <= 0:
        raise RecordingError("Recording duration must be greater than zero")
    if sample_rate_hz <= 0:
        raise RecordingError("Sample rate must be greater than zero")
    if channels <= 0:
        raise RecordingError("Channel count must be greater than zero")

    sounddevice = _sounddevice()
    chunks: list[bytes] = []
    overflowed = False
    target_frames = max(1, int(round(duration_seconds * sample_rate_hz)))
    remaining_frames = target_frames

    try:
        with sounddevice.RawInputStream(
            samplerate=sample_rate_hz,
            channels=channels,
            dtype=DEFAULT_DTYPE,
            blocksize=blocksize,
            device=device,
        ) as stream:
            while remaining_frames > 0:
                frames_to_read = min(blocksize, remaining_frames)
                data, did_overflow = stream.read(frames_to_read)
                chunks.append(_buffer_bytes(data))
                overflowed = overflowed or did_overflow
                remaining_frames -= frames_to_read
    except Exception as exc:
        raise RecordingError(f"Failed to record audio: {exc}") from exc

    if not chunks:
        raise RecordingError("Recording produced no audio frames")

    path = Path(output_path)
    frames = write_wav(path, chunks, sample_rate_hz=sample_rate_hz, channels=channels)

    if overflowed:
        raise RecordingError("Recording completed with input overflow")

    return RecordingResult(
        path=path.resolve(),
        duration_seconds=frames / sample_rate_hz,
        sample_rate_hz=sample_rate_hz,
        channels=channels,
        frames=frames,
    )


def write_wav(
    output_path: str | Path,
    chunks: Sequence[bytes],
    sample_rate_hz: int,
    channels: int,
) -> int:
    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    frames = sum(len(chunk) for chunk in chunks) // (SAMPLE_WIDTH_BYTES * channels)

    with wave.open(str(path), "wb") as wav_file:
        wav_file.setnchannels(channels)
        wav_file.setsampwidth(SAMPLE_WIDTH_BYTES)
        wav_file.setframerate(sample_rate_hz)
        wav_file.writeframes(b"".join(chunks))

    return frames


def _sounddevice() -> _SoundDevice:
    try:
        module = importlib.import_module("sounddevice")
    except ImportError as exc:
        raise RecordingError(
            "Missing recording dependency. Run with `uv run --extra recording ...` "
            "or `uv sync --extra recording`."
        ) from exc
    return cast(_SoundDevice, module)


def _buffer_bytes(data: object) -> bytes:
    if isinstance(data, bytes):
        return data
    if isinstance(data, bytearray | memoryview):
        return bytes(data)
    try:
        return bytes(cast(Sequence[int], data))
    except TypeError as exc:
        raise RecordingError(f"Unexpected audio buffer type: {type(data).__name__}") from exc


def _device_string(value: object) -> str:
    return value if isinstance(value, str) else ""
