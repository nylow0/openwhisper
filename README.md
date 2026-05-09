# OpenWhisper

Windows-first offline dictation desktop app.

## Architecture

- **Electron + Svelte + TypeScript**: UI layer
- **Rust**: Native Windows helper (hotkeys, injection)
- **Python + NeMo**: ASR worker (speech recognition)

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

### Development

```bash
# Run Electron app
bun dev

# Run Rust helper
cargo run --manifest-path crates/openwhisper-native/Cargo.toml

# Run Python ASR worker
cd services/asr && uv run python -m openwhisper_asr
```

## Project Structure

```
apps/desktop/          # Electron app
crates/openwhisper-native/  # Rust helper
services/asr/          # Python ASR worker
packages/protocol/     # Shared schemas
.plan/                 # Planning documents
```

## Documentation

See `.plan/` directory for detailed planning documents.

Start with: `.plan/00-INDEX.md`