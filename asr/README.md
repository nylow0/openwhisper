# OpenWhisper ASR Service

This directory is a thin integration wrapper for the reusable ASR core repo, expected during local development as a sibling directory named `openwhisper-asr`.

OpenWhisper still runs the worker from here so the Rust helper can keep using the existing service boundary:

```powershell
uv run python -m openwhisper_asr
```

## whisper.cpp Assets

Product-side whisper.cpp profiles live in `config/whispercpp-profiles.json`.
The currently validated q8 models are tracked under `models/`:

- `ggml-base.en-q8_0.bin`
- `ggml-medium.en-q8_0.bin`

These model files are stored with Git LFS because the medium q8 model is too
large for normal Git hosting.

The actual Python package, protocol helpers, worker implementation, engine experiments, and CLI live in `openwhisper-asr`.

During development this wrapper depends on the sibling repo through a local editable path:

```toml
[tool.uv.sources]
openwhisper-asr = { path = "../../openwhisper-asr", editable = true }
```

When the ASR core is published or pinned to Git, replace the local path with a specific version, tag, or commit. Do not depend on floating `main` for the product repo.
