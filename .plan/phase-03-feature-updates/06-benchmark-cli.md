# Update 06: Benchmark CLI

## Goal

Create a reproducible benchmark command that compares batch and chunked transcription quality, latency, memory, and cache size.

## Context For A Cold Agent

Phase 3 must produce measured evidence for a Go/No-Go decision. The important comparison is batch transcription vs chunked modes for dictation.

Read before editing:

- `.plan/07-PHASE-03-ASR-ENGINE.md`
- `.plan/17-PERFORMANCE-BUDGET.md`
- `services/asr/test_data/README.md`
- Engine code from Update 05, if present

## Dependencies

This update should happen after Updates 02, 04, and 05.

## Scope

Allowed files:

- `services/asr/scripts/benchmark.py`
- Optional small helper modules under `services/asr/scripts/`
- Optional updates to `services/asr/test_data/README.md`

Do not add product UI. Do not require Rust or Electron.

## Tasks

1. Add a CLI that accepts:
   - `--audio <path>`
   - `--reference <path>`
   - `--modes batch,chunk-0.5s,chunk-2s,chunk-4s`
   - optional `--json <path>`
   - `--device auto|cpu|cuda`
2. Calculate WER against reference text.
3. Measure latency per mode:
   - average
   - p50
   - p95
   - p99
4. Measure peak RAM.
5. Measure GPU memory if available.
6. Measure model/cache size if available.
7. Emit human-readable output and optional JSON.

## Acceptance

- The benchmark covers `batch`, `chunk-0.5s`, `chunk-2s`, and `chunk-4s`.
- Results are reproducible with local test data.
- Output includes enough data to fill the Phase 3 result report.

## Stop Conditions

Stop if measurement becomes fake or guessed. Missing metrics should be printed as unavailable, not invented.
