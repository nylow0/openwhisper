# OpenWhisper ASR Core

Reusable Python ASR core for OpenWhisper and future voice tools.

This directory owns the Python package `openwhisper_asr`. The desktop app talks
to it through one of these interfaces:

- Python library API for direct experiments and tools
- CLI commands for local transcription and benchmarks
- NDJSON worker mode for desktop app integration

## Development

```powershell
uv sync --extra dev
uv run python -m openwhisper_asr --help
uv run ow-asr serve
```

## Interfaces

```powershell
# Worker mode used by OpenWhisper
uv run ow-asr serve

# Protocol smoke test
'{"type":"health.check","timestamp":1}' | uv run ow-asr serve

# Mock transcription for protocol/UI work
uv run ow-asr mock
```

Heavy model runtimes are optional. Install them only when testing that engine:

```powershell
uv sync --extra nemo
```

## Model Assets

This directory owns ASR code and runtime profiles, but not heavy model binaries.
OpenWhisper product assets live in the repo-level `models/` directory.

For local whisper.cpp experiments, point the ASR runtime at those assets:

```powershell
$env:OPENWHISPER_MODEL_DIR = "..\models"
```

Validated whisper.cpp profiles are stored in `config/whispercpp-profiles.json`.
