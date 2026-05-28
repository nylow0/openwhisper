## ASR Benchmarking

- When asked to bench or test against audio, use files from `asr/.local/hard_eval`.
- Prefer the hard-eval datasets over `bench-audio.wav` or old ad hoc recordings in `asr/.local/recordings`.
- Use `asr/.local/hard_eval/fleurs-parakeet` for multilingual English/Ukrainian checks.
- Use `asr/.local/hard_eval/LibriSpeech` for English speech recognition checks.
- If hard-eval audio is in `.flac`, convert it to `.wav` with `ffmpeg` before benching because the Rust bench duration path expects WAV input. After confirming each `.wav` exists, delete the converted `.flac` files.
