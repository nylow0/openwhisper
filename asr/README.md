# Local whisper.cpp tooling

This directory is reserved for **local development builds** of whisper.cpp and
CUDA runtime files used by the Rust ASR worker.

Product configuration lives in [`config/`](../config/). Production dictation uses
[`crates/openwhisper-asr-rs/`](../crates/openwhisper-asr-rs/).

## Expected layout

```text
asr/.local/whispercpp-src/build-cpu-avxvnni-local/bin/whisper-cli.exe
asr/.local/whispercpp-src/build-cpu-avxvnni-local/bin/whisper-server.exe
asr/.local/whispercpp-src/build-cpu-avxvnni-local/bin/whisper-vad-speech-segments.exe
asr/.local/whispercpp-src/build-cuda-sm120-local/bin/whisper-cli.exe
asr/.local/whispercpp-src/build-cuda-sm120-local/bin/whisper-server.exe
asr/.local/whispercpp-src/build-cuda-sm120-local/bin/whisper-vad-speech-segments.exe
asr/.local/whispercpp/cuda-bin/Release/whisper-cli.exe
asr/.local/whispercpp/cuda-bin/Release/whisper-server.exe
asr/.local/whispercpp/cuda-bin/Release/whisper-vad-speech-segments.exe
```

These paths are gitignored. Build or copy binaries here for dev and packaging;
see the root README for the Windows package flow.

Build all three binaries from the same whisper.cpp revision. Copy the CUDA
build's binaries and DLLs into `asr/.local/whispercpp/cuda-bin/Release` for
packaging. The worker uses
`whisper-server` to keep the model in memory and `whisper-vad-speech-segments`
with `models/ggml-silero-v6.2.0.bin` to detect speech before decoding. If either
binary is absent in a development checkout, transcription falls back to the
CLI path or skips VAD, respectively. The packaged app requires both binaries.
