# Assumptions and Defaults

## Context

This file is the rulebook for decisions that are not phase-specific. The original version treated several ASR and size assumptions as facts. They are now explicitly marked as hypotheses until Phase 3 proves them.

---

## Core Assumptions

| # | Assumption | Status | Rationale |
|---|---|---|---|
| 1 | Electron + Svelte + TypeScript is the UI stack | Locked | Already scaffolded and a good fit |
| 2 | Bun is the JS package manager | Locked | Matches project preference and repo |
| 3 | Rust owns Windows integration | Locked | Correct for hotkeys, injection, clipboard, WASAPI |
| 4 | Python owns ASR model runtime | Provisional | Correct while using NeMo/Parakeet |
| 5 | NDJSON over stdio between Rust and Python | Locked | Simple and already implemented |
| 6 | Windows 10/11 is the v1 target | Locked | Focus matters |
| 7 | Model downloads after install | Likely | Keeps installer smaller and allows model updates |
| 8 | Parakeet TDT 0.6B v3 is the primary model | Candidate | Must pass Phase 3 |
| 9 | Offline-first is required | Locked | Core product value |
| 10 | Total installed size around 2.1 GB | Rejected as unproven | Current Parakeet artifacts appear larger |
| 11 | Production audio capture should use Rust WASAPI | Locked | Avoids Python audio rework and latency issues |

---

## Default Behaviors

| Category | Setting | Default | Notes |
|---|---|---|---|
| Audio | Sample rate | 16 kHz | Resample from device rate if needed |
| Audio | Chunk size | 2 seconds | Balanced default |
| Audio | VAD | Enabled | Exact VAD implementation chosen in Phase 6 |
| Injection | Method | Auto | Clipboard for longer commits, keystrokes for small edits |
| Compute | GPU mode | Auto-detect | Must fall back to CPU |
| Compute | Precision | FP16/BF16 only where supported | CPU should stay FP32 unless proven otherwise |
| UI | Overlay position | Bottom-center | Configurable later |
| UI | Tray on close | Minimize to tray | Utility behavior |
| System | Auto-start | Off | User choice |
| System | Log level | Info | DEBUG for diagnostics |

---

## Size Budget

These are gates, not promises.

| Metric | Preferred | Hard Gate | Notes |
|---|---:|---:|---|
| Installer without model | < 1.0 GB | < 1.5 GB | Depends on Python/PyTorch packaging |
| Model cache | < 2.75 GB | < 3.25 GB | Measure after duplicate cleanup |
| Installed after model | < 3.5 GB | < 4.5 GB | Above this requires an explicit product decision |
| Runtime RAM | < 2.0 GB | < 3.0 GB | Peak during dictation |

The old 2.1 GB target can remain aspirational only if Phase 3 finds a smaller artifact, conversion path, or different model. Do not plan packaging around it.

---

## File System Locations

Use Windows per-user directories:

```text
%LOCALAPPDATA%\OpenWhisper\
  models\      Large model files and Hugging Face/NeMo cache
  cache\       Temporary local cache

%APPDATA%\OpenWhisper\
  config.json
  state.json
  logs\
  diagnostics\
```

Rules:

- Large, replaceable files go under `%LOCALAPPDATA%`.
- User settings and diagnostics go under `%APPDATA%`.
- Never put secrets in logs or diagnostics.
- Never rely on a path without handling spaces in usernames.

---

## Configuration Schema

Use camelCase for Electron-facing settings and map explicitly in Rust/Python where needed.

```json
{
  "version": "1.0.0",
  "model": {
    "name": "nvidia/parakeet-tdt-0.6b-v3",
    "provider": "nemo",
    "cacheRevision": null
  },
  "audio": {
    "deviceId": "default",
    "sampleRate": 16000,
    "chunkDurationMs": 2000,
    "vadEnabled": true
  },
  "dictation": {
    "hotkey": "Ctrl+Shift+D",
    "injectionMethod": "auto",
    "language": "auto",
    "punctuation": true
  },
  "ui": {
    "overlayEnabled": true,
    "overlayPosition": "bottom-center",
    "minimizeToTray": true,
    "theme": "system"
  },
  "system": {
    "autoStart": false,
    "logLevel": "info",
    "gpuEnabled": true
  }
}
```

---

## Settings Migration Strategy

Rules:

1. Never delete user settings without explicit consent.
2. Preserve unknown fields.
3. Apply defaults for new fields only.
4. Log migrations.
5. Avoid TypeScript `any`; use `unknown` and validators.

Example:

```typescript
function migrateConfig(input: unknown): Config {
  const config = parseConfigOrDefault(input);
  if (config.version === CURRENT_CONFIG_VERSION) return config;

  return {
    ...config,
    version: CURRENT_CONFIG_VERSION,
    migratedFrom: config.version,
  };
}
```

---

## Installer Defaults

| Aspect | Default | Rationale |
|---|---|---|
| Type | NSIS via electron-builder | Standard Electron Windows path |
| Scope | Per-user | No admin requirement |
| Shortcuts | Start Menu, optional Desktop | Expected on Windows |
| Auto-start | Off | User choice |
| Model bundled | No | Keeps installer smaller |
| Code signing | Required before public release | Trust and SmartScreen |

---

## Hardware Requirements

Minimum requirements are provisional until Phase 3:

| Requirement | Minimum | Recommended |
|---|---|---|
| OS | Windows 10 1809+ | Windows 10/11 64-bit |
| CPU | Modern 4-core CPU | Ryzen 5 / Intel i5 or better |
| RAM | 8 GB | 16 GB |
| Storage | 5 GB free until measured | SSD with 8 GB free |
| GPU | None required if CPU fallback passes | NVIDIA GPU for low latency |
| Microphone | Windows-compatible input | USB/headset mic |

Do not promise CPU-only performance until measured.

---

## Security Defaults

- Telemetry disabled by default.
- Crash reports opt-in.
- Model download over HTTPS only.
- Pin model revision or verify checksums.
- Electron context isolation stays enabled.
- No admin requirement for normal operation.

---

## Changing These Assumptions

To change a core assumption:

1. Update this file.
2. Update all affected phase docs.
3. Record the reason in the relevant phase result document.

Current change log:

- 2026-05-09: Rejected unproven 2.1 GB size assumption, made Parakeet a candidate rather than a guarantee, and moved production audio ownership to Rust WASAPI.

---

## Related Documents

- [03-TECH-STACK.md](03-TECH-STACK.md)
- [07-PHASE-03-ASR-ENGINE.md](07-PHASE-03-ASR-ENGINE.md)
- [10-PHASE-06-AUDIO.md](10-PHASE-06-AUDIO.md)
- [13-PHASE-09-PACKAGING.md](13-PHASE-09-PACKAGING.md)
- [18-CONTINGENCY.md](18-CONTINGENCY.md)
