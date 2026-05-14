# ASR Environment Compatibility

## Status

Historical Phase 3 environment note.

The reusable Python ASR package now lives in the sibling repo:

```text
C:\Business\openwhisper-asr
```

This `services/asr` directory is only the OpenWhisper integration wrapper. Run NeMo/Parakeet environment checks from the ASR core repo, not from this wrapper.

## Exact Versions

| Component | Version |
|-----------|---------|
| Python | 3.11.15 (MSC v.1944 64 bit AMD64) |
| torch | 2.11.0+cpu |
| torchaudio | 2.11.0+cpu |
| nemo-toolkit | 2.7.3 |
| CUDA available | False (CPU-only PyTorch installed) |

## Verification Commands

```powershell
cd C:\Business\openwhisper-asr
uv sync --extra nemo
uv run python -c "import sys, torch, torchaudio, nemo.collections.asr as asr; print(sys.version); print(torch.__version__); print(torchaudio.__version__); print(torch.cuda.is_available())"
```

## Notes

- NeMo ASR imports successfully on Windows with the above stack.
- PyTorch resolved to CPU-only wheels (`+cpu`). GPU support would require a CUDA-capable PyTorch index or local CUDA toolkit.
- Several non-fatal warnings appear on import:
  - Megatron/Apex fallback messages (expected on Windows).
  - `torch.distributed.elastic` redirect warning on Windows/Mac (expected).
  - OneLogger telemetry disabled (expected).
  - `pydub` warns about missing `ffmpeg`/`avconv`; this does not block NeMo ASR import but may matter later for certain audio file conversions.
- `uv` was not initially on PATH; installed via `https://astral.sh/uv/install.ps1` to `C:\Users\odare\.local\bin`.
- `uv python find` resolved to `cpython-3.11.15-windows-x86_64-none` managed by `uv`.

## Blockers

None for the environment spike. The next step is a model-load test (Parakeet batch spike) to confirm the runtime works end-to-end.
