# whisper.cpp Profiles

This directory stores OpenWhisper product-side whisper.cpp runtime profiles.

The model paths in `whispercpp-profiles.json` are relative to that JSON file. The
binary paths are local development hints only; the actual optimized whisper.cpp
executables are build artifacts and are not committed here.

## Recommended Profiles

| Profile | Use case |
|---|---|
| `gpu_cuda_sm120_medium_en_q8` | Best current medium q8 profile when the RTX 5060 CUDA build is available. |
| `gpu_cuda_sm120_base_en_q8` | Fastest validated base q8 GPU profile. |
| `cpu_avx_vnni_medium_en_q8` | Best current medium q8 CPU fallback profile. |
| `cpu_avx_vnni_base_en_q8` | Best current base q8 CPU fallback profile. |

Benchmarks were run on 25 LibriSpeech `test-other` clips totaling `136.52s` of
audio. `first_result_seconds` is the time until the first per-sample transcript
JSON appeared.
