# Local whisper.cpp tooling

This directory is reserved for **local development builds** of whisper.cpp and
CUDA runtime files used by the Rust ASR worker.

Product configuration lives in [`config/`](../config/). Production dictation uses
[`crates/openwhisper-asr-rs/`](../crates/openwhisper-asr-rs/).

## Expected layout

```text
asr/.local/whispercpp-src/build-cpu-avxvnni-local/bin/whisper-cli.exe
asr/.local/whispercpp-src/build-cuda-sm120-local/bin/whisper-cli.exe
asr/.local/whispercpp/cuda-bin/Release/whisper-cli.exe
```

These paths are gitignored. Build or copy binaries here for dev and packaging;
see the root README for the Windows package flow.
