# whisper.cpp Profiles

This directory stores ASR runtime profiles for whisper.cpp.

The model binaries are not stored in this directory. OpenWhisper keeps
product-owned model assets in the repo-level `models/` directory. Runtime code
should resolve the model directory from `OPENWHISPER_MODEL_DIR`, falling back to
the local path described in `whispercpp-profiles.json` for development.

## Recommended Profiles

| Profile | Use case |
|---|---|
| `gpu_cuda_sm120_large_v3_turbo_q8` | Best current multilingual q8 profile; recommended upgrade candidate. |
| `gpu_cuda_sm120_medium_en_q8` | Best current medium q8 profile when the RTX 5060 CUDA build is available. |
| `cpu_avx_vnni_large_v3_turbo_q8` | CPU fallback for the multilingual turbo q8 model. |
| `cpu_avx_vnni_medium_en_q8` | Best current medium q8 CPU fallback profile. |

Benchmarks were run on 25 LibriSpeech `test-other` clips totaling `136.52s` of
audio. `first_result_seconds` is the time until the first per-sample transcript
JSON appeared.
