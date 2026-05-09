# Update 03: Parakeet Batch Spike

## Goal

Prove that `nvidia/parakeet-tdt-0.6b-v3` can load and transcribe a local WAV file in batch mode.

This is a spike script, not production worker integration.

## Context For A Cold Agent

OpenWhisper needs a local ASR model. Parakeet is a candidate, not a commitment. Phase 3 must measure reality before product UI polish. Do not use a fake direct model URL. Use NeMo or Hugging Face mechanisms that are actually supported.

Read before editing:

- `.plan/07-PHASE-03-ASR-ENGINE.md`
- `.plan/17-PERFORMANCE-BUDGET.md`
- `.plan/19-PLAN-REVIEW.md`
- `services/asr/test_data/README.md` if it exists

## Dependencies

This update should happen after Update 01.

## Scope

Allowed files:

- `services/asr/scripts/parakeet_batch_spike.py`
- Optional: `services/asr/scripts/README.md`

Do not edit the worker entry point in this update.

## Tasks

1. Add a CLI script that accepts:
   - `--audio <path>`
   - `--device auto|cpu|cuda`
   - optional `--model nvidia/parakeet-tdt-0.6b-v3`
2. Load the model through NeMo/Hugging Face supported APIs.
3. Transcribe one WAV file in batch mode.
4. Print:
   - model name
   - device used
   - CUDA availability
   - load time
   - transcription time
   - real-time factor if audio duration is available
   - peak RAM if reasonably available
   - model/cache paths and approximate cache size if reasonably available
   - transcript text

## Acceptance

- One command runs the batch spike against a local WAV.
- The output is human-readable.
- The script does not require Electron or Rust.
- The script does not create product-facing abstractions yet.

## Stop Conditions

Stop and document the failure if the model cannot load or the cache is unexpectedly huge. Do not hide a bad result behind retries or unrelated refactors.
