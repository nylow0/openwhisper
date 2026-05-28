# AGENTS.md

## Cursor Cloud specific instructions

### Overview

OpenWhisper is a **Windows-first offline dictation desktop app** (Electron + Svelte + Rust). The Rust crates (`crates/openwhisper-native/`, `crates/openwhisper-asr-rs/`) target `x86_64-pc-windows-msvc` exclusively and **cannot be compiled on Linux**. On Linux (Cloud Agent VMs), only the TypeScript/Svelte frontend layer can be developed and tested.

### What works on Linux

| Command | What it does |
|---------|--------------|
| `bun dev` | Starts the Vite dev server (renderer UI at http://localhost:5173) |
| `bun run typecheck` | Runs `svelte-check` on the TypeScript/Svelte code |
| `npx --package=typescript tsc --noEmit` (from `apps/desktop`) | TypeScript compiler check |
| `npx vite build` (from `apps/desktop`) | Builds the renderer bundle |
| `cargo fetch --manifest-path crates/openwhisper-native/Cargo.toml` | Downloads Rust deps (won't compile) |
| `cargo fetch --manifest-path crates/openwhisper-asr-rs/Cargo.toml` | Downloads Rust deps (won't compile) |

### What does NOT work on Linux

- `cargo build/check` for `openwhisper-native` — uses Windows named pipes and Win32 APIs
- `cargo build/check` for `openwhisper-asr-rs` — targets Windows MSVC; also has a pre-existing borrow-checker issue in `speech_gate.rs` (unrelated to platform)
- `bun run check` — chains `svelte-check` + `cargo check --target x86_64-pc-windows-msvc` (Rust part fails)
- Electron app (`electron:dev`) — requires a display and Electron binary is platform-specific

### Gotchas

- The `setup` script in root `package.json` runs `cargo fetch` (no workspace-level Cargo.toml exists), so you must use manifest-path flags or run from the crate directories.
- `libasound2-dev` is required for `cargo check` on the ASR crate (ALSA audio backend via cpal). Already installed in the update script.
- Bun is the package manager — do **not** use npm/yarn/pnpm.
- There is no lockfile committed (no `bun.lockb`). `bun install` resolves versions each time.
- The Vite dev server serves both `index.html` (main window) and `overlay.html` (frameless recording HUD).

### Running lint/typecheck

```bash
bun run typecheck
```

This runs `svelte-check --tsconfig ./tsconfig.json` from `apps/desktop/`. Zero errors expected.

### Running the dev server

```bash
bun dev
```

Starts the Vite dev server on http://localhost:5173. This is the renderer UI — it shows the app interface but the Electron/IPC features (tray, hotkeys, native text injection) won't work outside Electron.
