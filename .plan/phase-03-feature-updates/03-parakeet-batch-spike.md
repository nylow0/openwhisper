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

- [x] One command runs the batch spike against a local WAV.
- [x] The output is human-readable.
- [x] The script does not require Electron or Rust.
- [x] The script does not create product-facing abstractions yet.

## Result

- `services/asr/scripts/parakeet_batch_spike.py` created.
- `nvidia/parakeet-tdt-0.6b-v3` loaded successfully on Windows CPU through NeMo.
- Batch transcription ran successfully against a temporary synthetic 5s 16 kHz mono WAV.
- Synthetic non-speech audio produced an empty transcript, as expected.
- Warm-cache load time was about 11 seconds.
- Transcription time was about 0.42 seconds for 5 seconds of audio.
- Hugging Face cache size was about 2.34 GB for the model.
- Peak CPU RAM was about 5.5 GB, which exceeds the Phase 3 hard gate of 3 GB and is a serious risk to record in the final Go/No-Go report.
- CUDA/GPU performance was not measured because the environment uses CPU-only PyTorch.
- No real speech WAV was available, so WER and real transcript quality remain unmeasured.

## Stop Conditions

Stop and document the failure if the model cannot load or the cache is unexpectedly huge. Do not hide a bad result behind retries or unrelated refactors.

**None triggered for batch loading/transcription.** The peak RAM result is a major warning for later Phase 3 evaluation.
