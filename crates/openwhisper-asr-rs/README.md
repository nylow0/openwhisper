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
