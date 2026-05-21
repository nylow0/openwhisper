# OpenWhisper ASR Rust Worker

This crate is the production ASR worker for OpenWhisper.

Current scope:

- NDJSON stdio worker compatible with the desktop bridge protocol.
- Live microphone dictation and file transcription through whisper.cpp.
- CPAL device discovery and PCM conversion helpers.
- WAV diagnostics writer via `hound`.
- Ring buffer and baseline RMS VAD primitives for streaming work.

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

The transcribe command resolves `asr/config/whispercpp-profiles.json`, validates
the configured model file, runs the selected `whisper-cli`, and prints the final
text. The stdio worker uses the same backend.

Measure the Rust-controlled backend against a WAV file:

```powershell
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml --bin ow-asr-rs -- bench .\sample.wav --device cpu --model medium_en_q8
```

The benchmark prints JSON with WAV duration, backend setup time, decode wall
time, realtime factor, selected profile memory, and process CPU/working-set
samples when the platform exposes them. Live streaming logs also report the
configured window policy, first partial latency, silence final latency, decode
wall time, and stop-time CPU and memory samples.

whisper.cpp profiles may set `streaming.step_ms`, `streaming.length_ms`, and
`streaming.keep_ms`. For local measurements those values can be overridden with
`OPENWHISPER_ASR_STREAM_STEP_MS`, `OPENWHISPER_ASR_STREAM_LENGTH_MS`, and
`OPENWHISPER_ASR_STREAM_KEEP_MS`.
