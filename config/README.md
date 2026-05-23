# whisper.cpp Profiles

This directory stores ASR runtime profiles for whisper.cpp.

The model binaries are not stored in this directory. OpenWhisper keeps
product-owned model assets in the repo-level `models/` directory. Runtime code
should resolve the model directory from `OPENWHISPER_MODEL_DIR`, falling back to
the local path described in `whispercpp-profiles.json` for development.

Local whisper.cpp build outputs live under `asr/.local/`. See `asr/README.md`.

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

Rust streaming dictation reads each profile's `vad.rms_threshold` and
`vad.silence_ms` defaults. `OPENWHISPER_ASR_VAD_RMS_THRESHOLD` and
`OPENWHISPER_ASR_VAD_SILENCE_MS` override them for local tuning.

Before calling whisper.cpp, the worker checks that the recording has enough
voiced audio with clear dynamics above the mic noise floor (not just a steady
hum above a fixed RMS). `OPENWHISPER_ASR_MIN_SPEECH_MS` overrides the default
250ms minimum. Silent or noise-only sessions return an empty transcript instead
of running the decoder. Dictation audio is trimmed to the first→last voiced
region (plus ~150/400ms padding) so releasing the hotkey after a short phrase
does not re-decode minutes of leading/trailing silence.

If whisper still emits junk, a language-agnostic post-filter uses token
probabilities from `-ojf`, audio dynamics, repetition, and text density — not an
English word list. Profiles pass `-ojf`, `-sns`, and `-nth 0.92`.

`streaming.step_ms`, `streaming.length_ms`, and `streaming.keep_ms` configure
Rust live decode windows. `OPENWHISPER_ASR_STREAM_STEP_MS`,
`OPENWHISPER_ASR_STREAM_LENGTH_MS`, and `OPENWHISPER_ASR_STREAM_KEEP_MS`
override them while measuring latency and decoder pressure.

## Path manifest

`openwhisper-paths.json` is the shared source of truth for debug `target/`
layout and executable names. The `openwhisper-paths` Rust crate and the desktop
main process both read it so native/ASR binary discovery stays aligned.
