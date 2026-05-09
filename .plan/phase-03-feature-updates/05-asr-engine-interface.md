# Update 05: ASR Engine Interface

## Goal

Create a small ASR engine boundary so Parakeet can be replaced by Faster-Whisper or Whisper.cpp later without changing the Rust/Python protocol.

## Context For A Cold Agent

Phase 3 must avoid model lock-in. Parakeet may fail. The worker currently sends mock transcript events from `services/asr/src/openwhisper_asr/__main__.py`, and protocol helpers live in `services/asr/src/openwhisper_asr/protocol.py`.

Read before editing:

- `.plan/07-PHASE-03-ASR-ENGINE.md`
- `.plan/18-CONTINGENCY.md`
- `services/asr/src/openwhisper_asr/__main__.py`
- `services/asr/src/openwhisper_asr/protocol.py`

## Dependencies

This update should happen after Update 03 and ideally after Update 04.

## Scope

Allowed files:

- `services/asr/src/openwhisper_asr/engine.py`
- Optional tests if a test structure already exists

Avoid changing worker behavior in this update unless a tiny import path adjustment is unavoidable.

## Required API

```python
class ASREngine:
    def load(self) -> None: ...
    def transcribe_batch(self, audio_path: str) -> object: ...
    def transcribe_chunk(self, audio: object) -> object: ...
    def get_model_info(self) -> dict[str, object]: ...
```

## Tasks

1. Define typed result structures for batch and chunk transcription.
2. Define the `ASREngine` interface without using `any`.
3. Implement `ParakeetEngine` behind that interface.
4. Keep Parakeet-specific imports, model names, and inference details inside `ParakeetEngine`.
5. Keep the public engine API generic.

## Acceptance

- Faster-Whisper or Whisper.cpp could be added later without changing protocol message types.
- Parakeet details do not leak into `protocol.py`.
- Type hints are clear and avoid `Any` unless there is no reasonable alternative.

## Stop Conditions

Stop if the abstraction starts becoming a large framework. This should be a small boundary, not a new architecture project.
