# Phase 9: Packaging and Distribution

## Context

This is the final packaging phase. It should not start until the ASR engine, audio path, native helper, and performance work have passed their gates.

The old packaging plan assumed a 1.2 GB FP16 model and a direct `model.fp16.bin` download. That is not reliable. Packaging must use the actual ASR artifact selected in Phase 3 and the actual runtime footprint measured in Phase 8.

---

## Goal

Create a Windows installer that:

1. Installs per-user without admin.
2. Bundles Electron, the Rust helper, and the selected ASR runtime.
3. Downloads or provisions the selected ASR model after install.
4. Works on a clean Windows machine without requiring developer tools.
5. Can be uninstalled cleanly.
6. Preserves user settings and model cache across updates.

---

## Packaging Gates

| Metric | Preferred | Hard Gate |
|---|---:|---:|
| Installer without model | < 1.0 GB | < 1.5 GB |
| Model cache | < 2.75 GB | < 3.25 GB |
| Installed after model | < 3.5 GB | < 4.5 GB |
| First-run free disk required | < 6 GB | < 8 GB |

If the chosen ASR path cannot meet the hard gates, do not hide it. Either choose a smaller model/runtime or make an explicit business decision that the product will ship as a large local-AI app.

---

## What to Bundle

| Component | Bundled? | Notes |
|---|---|---|
| Electron app | Yes | Built renderer/main output only |
| Rust helper | Yes | Release binary |
| Python runtime / ASR executable | Yes | Exact strategy depends on Phase 8 packaging test |
| PyTorch/NeMo or alternative runtime | Yes if Python ASR remains | Keep only required modules |
| ASR model | No by default | Download/provision on first run |
| Dev tools | No | No compilers, test frameworks, source maps, or docs |

---

## Model Download Strategy

Do not use a hard-coded `model.fp16.bin` URL.

Allowed strategies:

- `huggingface_hub.snapshot_download` with pinned revision and allow/ignore patterns.
- NeMo `from_pretrained` into a controlled cache directory, followed by cache cleanup and size measurement.
- A project-hosted manifest that points to the exact selected artifact after Phase 3.

Required behavior:

- Show progress.
- Support resume where possible.
- Verify checksum or pin revision.
- Measure final cache size.
- Avoid keeping both `.nemo` and safetensors copies unless both are truly needed.

---

## Build Strategy

### Rust Helper

```bash
cargo build --release --manifest-path crates/openwhisper-native/Cargo.toml
```

Do not use UPX by default. It can cause antivirus and signing problems for a tiny size win.

### Python ASR Runtime

Options to evaluate after Phase 3:

| Option | Use When | Risk |
|---|---|---|
| PyInstaller executable | Fastest packaging path | Large binary, hidden import issues |
| Embedded Python + venv/site-packages | More transparent | More installer complexity |
| Alternative runtime bundle | If Faster-Whisper/Whisper.cpp wins | Depends on chosen backend |

The chosen strategy must be tested on a clean Windows VM.

### Electron App

Use electron-builder/NSIS unless a later phase proves it is unsuitable.

Keep the package lean:

- No source maps in production artifacts.
- No tests/docs from dependencies.
- No dev dependencies.
- No model files inside the installer unless explicitly approved.

---

## First Run Experience

1. Launch app.
2. Detect whether model/runtime is ready.
3. If missing, show onboarding with measured model size and required free disk.
4. Download/provision model.
5. Verify integrity.
6. Run a short health check.
7. Mark the app ready.

The UI must display measured sizes, not stale planning numbers.

---

## Install Locations

```text
%LOCALAPPDATA%\Programs\OpenWhisper\
  OpenWhisper.exe
  resources\
    app.asar
    bin\
      openwhisper-native.exe
      openwhisper-asr.exe or embedded-python\

%LOCALAPPDATA%\OpenWhisper\
  models\
  cache\

%APPDATA%\OpenWhisper\
  config.json
  state.json
  logs\
  diagnostics\
```

---

## Code Signing

Unsigned builds are acceptable for local development and private beta only.

Public distribution requires signing. Without signing, Windows SmartScreen will hurt conversion and trust.

---

## Success Criteria

- [ ] Fresh Windows VM can install the app.
- [ ] App launches without Python, Rust, Node, Bun, or uv installed globally.
- [ ] First-run model provisioning works.
- [ ] Model download can resume or recover gracefully.
- [ ] Dictation works after setup.
- [ ] Uninstall removes app binaries.
- [ ] User settings and model cache behavior is intentional and documented.
- [ ] Installer size is under the hard gate.
- [ ] Installed size is under the hard gate.
- [ ] Signed build path is documented before public release.

---

## Related Documents

- [03-TECH-STACK.md](03-TECH-STACK.md)
- [07-PHASE-03-ASR-ENGINE.md](07-PHASE-03-ASR-ENGINE.md)
- [12-PHASE-08-PERFORMANCE.md](12-PHASE-08-PERFORMANCE.md)
- [14-VERIFICATION.md](14-VERIFICATION.md)
- [15-ASSUMPTIONS.md](15-ASSUMPTIONS.md)
