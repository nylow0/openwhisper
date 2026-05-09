# Phase 3 Feature Updates

These briefs split Phase 3 into small implementation updates so an agent can work on one update without being overwhelmed.

The source Phase 3 overview remains [`../07-PHASE-03-ASR-ENGINE.md`](../07-PHASE-03-ASR-ENGINE.md). Do not delete or replace it. These files are execution tickets that point back to the main phase.

## Current Rule

Phase 3 is a hard ASR Go/No-Go gate. Do not build product UI, onboarding, packaging, or production microphone capture here. Production microphone capture belongs to Rust WASAPI in Phase 6. Phase 3 should use local WAV files and controlled worker messages.

## How To Use These Updates

Give an agent exactly one numbered update at a time.

Each update is written to be self-contained because the next agent may only have the whole codebase and this file. The agent should still read:

- [`../00-INDEX.md`](../00-INDEX.md)
- [`../07-PHASE-03-ASR-ENGINE.md`](../07-PHASE-03-ASR-ENGINE.md)
- [`../17-PERFORMANCE-BUDGET.md`](../17-PERFORMANCE-BUDGET.md)
- [`../18-CONTINGENCY.md`](../18-CONTINGENCY.md)
- [`../19-PLAN-REVIEW.md`](../19-PLAN-REVIEW.md)

## Order

| Update | Status | Purpose |
|---|---|---|
| [01 Environment Compatibility](01-environment-compatibility.md) | Not started | Find working Python/PyTorch/NeMo versions on Windows |
| [02 Test Data Contract](02-test-data-contract.md) | Not started | Define reproducible local WAV/reference inputs |
| [03 Parakeet Batch Spike](03-parakeet-batch-spike.md) | Not started | Prove model load and WAV batch transcription |
| [04 Parakeet Chunked Spike](04-parakeet-chunked-spike.md) | Not started | Prove chunk-style inference before worker integration |
| [05 ASR Engine Interface](05-asr-engine-interface.md) | Not started | Add a generic engine boundary with Parakeet behind it |
| [06 Benchmark CLI](06-benchmark-cli.md) | Not started | Measure WER, latency, memory, and cache size |
| [07 Worker Real Mode](07-worker-real-mode.md) | Not started | Wire real ASR into the existing Python worker behind a mode |
| [08 Go No-Go Report](08-go-no-go-report.md) | Not started | Record measured decision before Phase 4 |

## Stop Conditions

Stop Phase 3 and write down the blocker if any of these happens:

- Parakeet cannot load reliably on Windows.
- Chunked mode is clearly unusable for dictation.
- Cache/runtime size is far over the Phase 3 gates.
- The implementation needs product UI, installer work, or production audio capture to continue.

If Parakeet fails, follow [`../18-CONTINGENCY.md`](../18-CONTINGENCY.md) instead of pushing forward blindly.
