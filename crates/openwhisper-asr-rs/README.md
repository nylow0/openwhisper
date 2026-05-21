# OpenWhisper ASR Rust Worker

This crate is the Rust-first ASR worker bootstrap for OpenWhisper.

Current scope:

- NDJSON stdio worker compatible with the existing Python worker protocol.
- Mock dictation and file transcription paths for desktop plumbing.
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
text. Worker file transcription can use the same backend by setting:

```powershell
$env:OPENWHISPER_ASR_ENGINE = "whispercpp"
```
