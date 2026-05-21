# OpenWhisper

Windows-first offline dictation desktop app.

## Architecture

- **Electron + Svelte + TypeScript**: UI layer
- **Rust**: Native Windows helper (hotkeys, injection)
- **Rust ASR worker**: production desktop recording and whisper.cpp worker in `crates/openwhisper-asr-rs/`
- **ASR profiles and validation tools**: shared whisper.cpp config and test tooling in [`asr/`](asr/)
- **Models**: product-owned runtime model assets in `models/`

## Quick Start

### Prerequisites

- Windows 10/11
- Bun (`curl -fsSL https://bun.sh/install | bash`)
- Rust (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)

### Setup

```bash
# Clone and setup
git clone <repo-url>
cd openwhisper
bun run setup
```

The q8 whisper.cpp model assets live under `models/` and are tracked with Git
LFS.

### Development

```bash
# Run Electron app
bun dev

# Run Rust helper
cargo run --manifest-path crates/openwhisper-native/Cargo.toml

# Run Rust ASR worker
cargo run --manifest-path crates/openwhisper-asr-rs/Cargo.toml
```

### Windows Package

```bash
bun run --cwd apps/desktop electron:build
```

The Windows package bundles the native helper, Rust ASR worker, whisper.cpp
profiles, model assets, and CPU/GPU whisper.cpp runtime files. Packaging expects
the validated whisper.cpp build outputs under `asr/.local/whispercpp-src/` and
the CUDA runtime bundle under `asr/.local/whispercpp/` that the ASR profiles use
locally. The packaged desktop path selects Rust ASR and does not bundle Python
or `uv`.

## Project Structure

```text
apps/desktop/               # Electron app
asr/                        # whisper.cpp profiles and ASR validation tooling
crates/openwhisper-native/  # Rust helper
crates/openwhisper-asr-rs/  # Rust ASR worker
models/                     # Git LFS model assets used by the product
```
