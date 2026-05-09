# Plan Review

## Verdict

The plan is a good fit for OpenWhisper's direction with four corrections:

1. Phase 3 is the current phase and must remain a hard ASR Go/No-Go gate.
2. Parakeet TDT 0.6B v3 is a candidate, not a guaranteed final model.
3. The old 1.2 GB model and 2.1 GB installed-size assumptions are not proven and should not guide work.
4. Production microphone capture should be Rust WASAPI, not a Python `sounddevice` pipeline that is later replaced.

## Evidence Checked

- Current repo shape: Electron app, Rust helper, Python worker, protocol types, and mock transcript flow already exist.
- `cargo check --manifest-path crates/openwhisper-native/Cargo.toml` passed in this environment.
- JS/Python checks were blocked by missing local tools (`svelte-check`, `uv`, and `python` not available on PATH here).
- Hugging Face model card for `nvidia/parakeet-tdt-0.6b-v3` shows NeMo usage, streaming guidance, 25 languages, and current model artifact sizes that invalidate the old fake direct-download plan.

## Major Fixes Applied

| Area | Old Problem | Fixed Direction |
|---|---|---|
| Status | Index said all phases were not started | Phase 1 complete, Phase 2 complete enough, Phase 3 current |
| ASR | Treated Parakeet as guaranteed | Parakeet must pass measured Phase 3 gates |
| Model download | Assumed `model.fp16.bin` | Use NeMo/Hugging Face snapshot/revision mechanisms |
| Size budget | Promised 2.1 GB installed | Replaced with measured preferred/hard gates |
| Audio | Planned Python audio then Rust later | Rust WASAPI is production path; Python audio is spike-only |
| Packaging | Planned around stale size numbers | Packaging waits for measured ASR footprint |
| Structure | Suggested creating many empty folders | Create files only when a phase needs them |
| Verification | Referenced localhost TCP test | Corrected for current named-pipe IPC |

## Current Execution Path

1. Finish Phase 3 ASR validation.
2. Record a Go/No-Go result.
3. If Parakeet passes, continue to Phase 4.
4. If Parakeet fails, execute contingency before UI polish.
5. Build production audio in Rust during Phase 6.
6. Do packaging only after ASR/runtime size is measured.

## Do Not Delegate These Yet

- Product-shell polish beyond minimal status surfaces.
- Full onboarding/model-download UX.
- Packaging scripts.
- Python `sounddevice` production pipeline.

Those tasks would create rework if the ASR model choice changes.

## References

- `nvidia/parakeet-tdt-0.6b-v3` model card: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3
- Model files view: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3/tree/main
