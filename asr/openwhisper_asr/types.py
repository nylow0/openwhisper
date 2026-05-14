"""Shared ASR result types."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class WordResult:
    text: str
    start_ms: int
    end_ms: int
    confidence: float | None = None


@dataclass(frozen=True)
class TranscriptionResult:
    text: str
    words: list[WordResult]
    language: str | None
    is_partial: bool
    processing_latency_ms: int
