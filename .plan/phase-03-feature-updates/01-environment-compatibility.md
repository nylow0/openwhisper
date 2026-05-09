# Update 01: Environment Compatibility

## Goal

Find a working Windows-compatible Python ASR environment for the Parakeet spike.

This update only proves dependency compatibility. It does not implement transcription, UI, packaging, or worker integration.

## Context For A Cold Agent

OpenWhisper is a Windows-first offline dictation app. The repo has three layers:

- `apps/desktop/`: Electron + Svelte + TypeScript
- `crates/openwhisper-native/`: Rust helper
- `services/asr/`: Python ASR worker

Phase 3 is the current gate. Parakeet is only a candidate. The old direct-download and size assumptions are not trusted.

Read before editing:

- `.plan/07-PHASE-03-ASR-ENGINE.md`
- `.plan/17-PERFORMANCE-BUDGET.md`
- `.plan/18-CONTINGENCY.md`
- `.plan/19-PLAN-REVIEW.md`

## Scope

Allowed files:

- `services/asr/pyproject.toml`
- `services/asr/uv.lock`
- Optional: `services/asr/ENVIRONMENT.md`

Do not edit UI, Rust, protocol schemas, or worker runtime code in this update.

## Tasks

1. Identify current compatibility evidence for Python, PyTorch, torchaudio, NeMo, and CUDA on Windows.
2. Update `services/asr/pyproject.toml` with the smallest dependencies needed to import NeMo ASR.
3. Run `uv sync` from `services/asr`.
4. Verify imports:

   ```powershell
   cd services/asr
   uv run python -c "import sys, torch, torchaudio, nemo.collections.asr as asr; print(sys.version); print(torch.__version__); print(torchaudio.__version__); print(torch.cuda.is_available())"
   ```

5. Record the exact versions and whether CUDA is available.

## Acceptance

- `uv sync` succeeds.
- `uv run python -c "import nemo.collections.asr"` succeeds.
- Exact Python, torch, torchaudio, CUDA availability, and NeMo versions are recorded.
- No product UI or packaging work is added.

## Stop Conditions

Stop and document the blocker if NeMo has no credible Windows-compatible install path. Do not spend time building around a broken dependency stack.
