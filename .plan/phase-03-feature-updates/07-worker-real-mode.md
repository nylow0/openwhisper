# Update 07: Worker Real Mode

## Goal

Wire the real ASR engine into the Python worker behind a mode/flag while keeping mock mode available for protocol testing.

## Context For A Cold Agent

The current worker emits mock transcript events from `services/asr/src/openwhisper_asr/__main__.py`. Phase 2 protocol testing still benefits from mock mode. Real ASR mode should handle `model.load` and `audio.chunk` without breaking existing Rust/Python communication.

Read before editing:

- `.plan/07-PHASE-03-ASR-ENGINE.md`
- `.plan/16-ERROR-HANDLING.md`
- `services/asr/src/openwhisper_asr/__main__.py`
- `services/asr/src/openwhisper_asr/protocol.py`
- `services/asr/src/openwhisper_asr/engine.py`

## Dependencies

This update should happen after Update 05.

## Scope

Allowed files:

- `services/asr/src/openwhisper_asr/__main__.py`
- `services/asr/src/openwhisper_asr/protocol.py`
- `services/asr/src/openwhisper_asr/engine.py` only for small integration fixes

Do not edit Electron UI or Rust helper behavior unless the protocol is impossible to test otherwise.

## Tasks

1. Add a worker mode selection such as:
   - mock mode by default, or
   - real mode when an environment variable or message field requests it
2. On `model.load` in real mode:
   - load the selected engine
   - emit real `model.loaded` metadata
   - emit `model.error` on failure
3. On `audio.chunk` in real mode:
   - decode or validate the existing audio payload format
   - feed the selected engine
   - emit `transcript.partial` and/or `transcript.final`
4. Keep mock mode intact for protocol smoke tests.
5. Keep messages generic; do not add Parakeet-specific protocol fields.

## Acceptance

- Rust can still start the worker.
- Mock mode still produces mock transcript events.
- Real mode can load the model and process a supported audio chunk payload.
- Real mode emits `model.loaded`, `transcript.partial`, and `transcript.final` where appropriate.

## Stop Conditions

Stop if audio payload format is not defined enough to safely implement. In that case, document the missing protocol contract instead of guessing.
