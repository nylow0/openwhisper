"""ASR engine implementations and spikes."""

from openwhisper_asr.engines.whisper_cpp import WhisperCppEngine, WhisperCppError

__all__ = ["WhisperCppEngine", "WhisperCppError"]
