# Update 04: Parakeet Chunked Spike

## Goal

Prove whether Parakeet can produce acceptable chunk-style transcription for dictation before integrating it into the worker.

## Context For A Cold Agent

OpenWhisper is a dictation app, so batch transcription is not enough. Phase 3 must test chunk sizes that map to product modes:

- Fast: `0.5s`
- Balanced: `2s`
- Accurate: `4s`

Chunked quality must be close enough to batch quality, preferably within 5 WER percentage points on clean speech.

Read before editing:

- `.plan/07-PHASE-03-ASR-ENGINE.md`
- `.plan/17-PERFORMANCE-BUDGET.md`
- Existing script from Update 03

## Dependencies

This update should happen after Update 03.

## Scope

Allowed files:

- `services/asr/scripts/parakeet_chunked_spike.py`
- Optional shared helper under `services/asr/scripts/` if it removes real duplication with the batch spike

Do not edit the Python worker entry point in this update.

## Tasks

1. Add a CLI script that accepts:
   - `--audio <path>`
   - `--chunk-seconds 0.5|2|4`
   - `--device auto|cpu|cuda`
   - optional `--overlap-seconds`
2. Use NVIDIA/NeMo chunked inference guidance where possible.
3. If true streaming APIs are not available, use an honest WAV chunking approximation and label it clearly in output.
4. Print:
   - chunk size
   - overlap
   - number of chunks
   - per-chunk latency
   - average, p50, p95, p99 latency
   - concatenated transcript
   - warnings about approximation limits

## Acceptance

- One command runs chunked transcription for a local WAV.
- All three chunk sizes can be tested.
- The output makes it obvious whether this is true streaming/chunked inference or a temporary approximation.

## Stop Conditions

Stop if chunking clearly destroys transcript quality or requires a production audio system. Phase 3 uses WAVs first.
