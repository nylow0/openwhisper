"""ASR engine boundaries used by tools and product integrations."""

from __future__ import annotations

from abc import ABC, abstractmethod

from openwhisper_asr.types import TranscriptionResult, WordResult


class ASREngine(ABC):
    """Small boundary so product code does not depend on one ASR runtime."""

    @abstractmethod
    def load_model(self, model: str | None = None, device: str = "auto") -> None:
        """Load or prepare the selected model."""

    @abstractmethod
    def transcribe_batch(self, audio_path: str) -> TranscriptionResult:
        """Transcribe a complete audio file."""

    @abstractmethod
    def transcribe_chunk(self, audio_pcm: bytes, sample_rate_hz: int) -> TranscriptionResult | None:
        """Transcribe one audio chunk, returning None when no transcript is ready."""


class MockASREngine(ASREngine):
    """Deterministic engine for protocol and UI development."""

    def __init__(self, text: str = "Hello world this is a test of the OpenWhisper dictation system") -> None:
        self._text = text
        self._loaded = False

    def load_model(self, model: str | None = None, device: str = "auto") -> None:
        self._loaded = True

    def transcribe_batch(self, audio_path: str) -> TranscriptionResult:
        words = self._words_for_text(self._text)
        return TranscriptionResult(
            text=self._text,
            words=words,
            language="en",
            is_partial=False,
            processing_latency_ms=120,
        )

    def transcribe_chunk(self, audio_pcm: bytes, sample_rate_hz: int) -> TranscriptionResult | None:
        if not audio_pcm:
            return None
        words = self._words_for_text(self._text)
        return TranscriptionResult(
            text=self._text,
            words=words,
            language="en",
            is_partial=False,
            processing_latency_ms=120,
        )

    @staticmethod
    def _words_for_text(text: str) -> list[WordResult]:
        results: list[WordResult] = []
        start_ms = 0
        for word in text.split():
            duration_ms = max(100, len(word) * 80)
            results.append(
                WordResult(
                    text=word,
                    start_ms=start_ms,
                    end_ms=start_ms + duration_ms,
                    confidence=0.92,
                )
            )
            start_ms += duration_ms + 50
        return results
