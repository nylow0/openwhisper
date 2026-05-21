# openwhisper-asr-rs

Bootstrap Rust ASR worker for OpenWhisper.

## Current status

- NDJSON stdio worker loop
- Health check endpoint (`health.check` -> `health.ok`)
- Mock transcription endpoint (`transcribe.mock` -> `transcription.final`)
- Mock streaming endpoint (`stream.mock` -> `transcription.partial`) with baseline VAD + ring buffer primitives
- File transcription contract endpoint (`transcribe.file` with `payload.audio_path`)
- Device listing endpoint (`devices.list`) backed by CPAL host input enumeration

## Run

```bash
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml
```

Then send lines of NDJSON, for example:

```json
{"type":"health.check","timestamp":1}
```
