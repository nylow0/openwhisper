from __future__ import annotations

import wave
from pathlib import Path

from openwhisper_asr.recording import write_wav


def test_write_wav_creates_16khz_mono_pcm_file(tmp_path: Path) -> None:
    audio_path = tmp_path / "sample.wav"

    frames = write_wav(
        output_path=audio_path,
        chunks=[b"\x00\x00" * 160, b"\x01\x00" * 160],
        sample_rate_hz=16_000,
        channels=1,
    )

    assert frames == 320
    with wave.open(str(audio_path), "rb") as wav_file:
        assert wav_file.getnchannels() == 1
        assert wav_file.getsampwidth() == 2
        assert wav_file.getframerate() == 16_000
        assert wav_file.getnframes() == 320
