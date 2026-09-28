# OpenWhisper ASR Rust Worker

This crate is the production ASR worker for OpenWhisper.

Current scope:

- NDJSON stdio worker compatible with the desktop bridge protocol.
- Live microphone capture and final file transcription through whisper.cpp.
- CPAL device discovery and PCM conversion helpers.
- Silero VAD speech segments before final decoding. Silent recordings skip Whisper.
- A persistent `whisper-server` process when bundled, with `whisper-cli` fallback.
- Decoder token timing and probabilities in `transcript.final`.

Run locally:

```powershell
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml --bin ow-asr-rs
```

Example health check:

```powershell
'{"type":"health.check","timestamp":7}' | cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml --bin ow-asr-rs
```

Record a microphone WAV through the Rust capture path:

```powershell
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml --bin ow-asr-rs -- record .\recording.wav --seconds 30
```

The record command uses the default input device, mixes input to mono, resamples
to 16 kHz, and writes a 16-bit PCM WAV.

Transcribe an audio file through the Rust-controlled whisper.cpp backend:

```powershell
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml --bin ow-asr-rs -- transcribe .\sample.wav --device cpu --model medium_en_q8
```

The transcribe command resolves `config/whispercpp-profiles.json`, validates the
model file, and prints the final text. The stdio worker keeps `whisper-server`
loaded across requests if it is beside `whisper-cli`. The first request starts
the server; subsequent requests reuse it. If the server is absent or fails, the
worker uses `whisper-cli` for that session. Explicit auto-detection across all
Whisper languages also uses the CLI so language codes remain compatible.

Measure the Rust-controlled backend against a WAV file:

```powershell
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml --bin ow-asr-rs -- bench .\sample.wav --device cpu --model medium_en_q8
```

Compare language settings (mirrors desktop `OPENWHISPER_ASR_*` env vars):

```powershell
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml --bin ow-asr-rs -- bench .\sample.wav --model large_v3_turbo_q8 --device gpu --languages en --auto-detect-language
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml --bin ow-asr-rs -- bench .\sample.wav --model large_v3_turbo_q8 --device gpu --languages en,de,fr
```

The worker uses `-l auto` when auto-detect is enabled or when multiple spoken
languages are configured, and `-l <lang>` when exactly one language is selected.
The server path currently handles English and Ukrainian; other language sets
continue through the CLI. Silero VAD reports speech spans on the original audio
timeline. Whisper decodes the full recording once, preserving context and avoiding
the old overlapping sliding-window transcription. The worker stores successful
dictation WAVs when `OPENWHISPER_RECORDINGS_DIR` is set, and includes the path,
speech spans, and decoder token data in `transcript.final`.

The per-file bench command prints JSON with WAV duration and total decode time.
For representative English/Ukrainian accuracy, warm latency, language detection,
and metadata coverage, use `scripts/asr_hard_eval_benchmark.py` with the FLEURS
test set under `asr/.local/hard_eval`.
