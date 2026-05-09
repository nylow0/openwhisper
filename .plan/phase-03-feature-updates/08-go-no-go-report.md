# Update 08: Go No-Go Report

## Goal

Record the measured Phase 3 decision before Phase 4 starts.

This update is documentation only unless tiny benchmark/report formatting fixes are needed.

## Context For A Cold Agent

Phase 3 decides whether OpenWhisper should continue with `nvidia/parakeet-tdt-0.6b-v3`, test a smaller Parakeet, switch to Faster-Whisper, switch to Whisper.cpp, or trigger another contingency option.

Do not start Phase 4 product UI until this report exists.

Read before editing:

- `.plan/07-PHASE-03-ASR-ENGINE.md`
- `.plan/17-PERFORMANCE-BUDGET.md`
- `.plan/18-CONTINGENCY.md`
- Benchmark output from Update 06

## Dependencies

This update should happen after Update 06 and, if worker integration is being validated, Update 07.

## Scope

Allowed files:

- `.plan/PHASE-03-RESULTS.md`
- Optional: update `.plan/00-INDEX.md` status after the decision

Do not change code in this update unless the report exposes a tiny command-output bug that blocks evidence collection.

## Required Contents

The report must include:

- Environment versions
- Hardware tested
- Model cache size
- Runtime package size estimate
- Load time
- Batch WER
- Chunked WER by mode
- Latency distribution by mode
- Peak RAM and GPU memory
- CPU fallback result
- Decision:
  - Go with Parakeet
  - Test a smaller Parakeet
  - Switch to Faster-Whisper
  - Switch to Whisper.cpp
  - Other, with clear reasoning

## Acceptance

- The decision is explicit.
- The evidence is measured, not guessed.
- If Parakeet fails, the next action points to `.plan/18-CONTINGENCY.md`.
- Phase 4 remains blocked unless the decision is Go.

## Stop Conditions

Stop if required metrics are missing. A weak or incomplete report is worse than no report because it gives false confidence.
