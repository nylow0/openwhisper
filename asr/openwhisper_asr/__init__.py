"""Reusable ASR core for OpenWhisper products."""

from openwhisper_asr.engine import ASREngine, MockASREngine
from openwhisper_asr.engines.whisper_cpp import WhisperCppEngine
from openwhisper_asr.types import TranscriptionResult, WordResult

__version__ = "0.1.0"

__all__ = [
    "ASREngine",
    "MockASREngine",
    "WhisperCppEngine",
    "TranscriptionResult",
    "WordResult",
]
