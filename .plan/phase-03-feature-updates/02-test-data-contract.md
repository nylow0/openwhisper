# Update 02: Test Data Contract

## Goal

Define the local WAV and reference-text contract used by Phase 3 spikes and benchmarks.

This update should make benchmark inputs reproducible without committing large audio files.

## Context For A Cold Agent

Phase 3 must compare batch and chunked ASR quality. The benchmark later needs known audio and matching reference text so WER can be measured. The repo currently has a Python ASR worker under `services/asr/`.

Read before editing:

- `.plan/07-PHASE-03-ASR-ENGINE.md`
- `.plan/17-PERFORMANCE-BUDGET.md`

## Scope

Allowed files:

- `services/asr/test_data/README.md`
- Optional small placeholder files under `services/asr/test_data/`

Do not commit large WAV datasets. Do not add generated cache/model files.

## Tasks

1. Create `services/asr/test_data/README.md`.
2. Define the expected local layout for test WAVs and references.
3. Specify audio requirements:
   - WAV
   - 16 kHz preferred
   - mono preferred
   - clean speech sample
   - at least one 10-30 second sample
   - at least one 60+ second sample if available
4. Define a simple reference format that the benchmark can read later.
5. Include clear instructions that local test audio is not committed unless it is tiny and license-safe.

## Suggested Layout

```text
services/asr/test_data/
  README.md
  samples/
    sample-clean-short.wav        # local only unless license-safe
    sample-clean-short.txt
    sample-clean-long.wav         # local only unless license-safe
    sample-clean-long.txt
```

## Acceptance

- [x] A future benchmark command can discover what files to use.
- [x] The README explains how to add local samples.
- [x] The repo stays clean: no large or questionable audio files are committed.

## Result

- `services/asr/test_data/README.md` created with full contract.
- `services/asr/test_data/.gitignore` created to block audio and run artifacts.
- `services/asr/test_data/samples/` directory created with `.gitkeep` for structure.
- Document references benchmark discovery pseudocode, filename conventions, and reference-text rules.

## Stop Conditions

Stop if the work requires downloading a dataset. That belongs in a separate explicit task because dataset licensing and size matter.

**None triggered.**
