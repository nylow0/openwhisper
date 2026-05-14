# OpenWhisper

Windows-first offline dictation desktop app.

## Architecture

- **Electron + Svelte + TypeScript**: UI layer
- **Rust**: Native Windows helper (hotkeys, injection)
- **Python ASR core**: reusable ASR worker package in [`asr/`](asr/)
- **Models**: product-owned runtime model assets in `models/`

## Quick Start

### Prerequisites

- Windows 10/11
- Bun (`curl -fsSL https://bun.sh/install | bash`)
- Rust (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Python 3.10 or 3.11
- uv (`pip install uv`)

### Setup

```bash
# Clone and setup
git clone <repo-url>
cd openwhisper
bun run setup
```

The Python ASR core lives in [`asr/`](asr/). The q8 whisper.cpp model assets
live under `models/` and are tracked with Git LFS.

### Development

```bash
# Run Electron app
bun dev

# Run Rust helper
cargo run --manifest-path crates/openwhisper-native/Cargo.toml

# Run Python ASR worker
cd asr && uv run python -m openwhisper_asr
```

## Project Structure

```text
apps/desktop/               # Electron app
asr/                        # Python ASR runtime and whisper.cpp profiles
crates/openwhisper-native/  # Rust helper
models/                     # Git LFS model assets used by the product
packages/protocol/          # Shared schemas
```
