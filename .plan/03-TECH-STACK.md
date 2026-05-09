# Tech Stack

## Context

This document defines the technology choices for OpenWhisper: a Windows-first offline dictation desktop app with three layers:

- Electron + Svelte + TypeScript for the product experience
- Rust for Windows integration
- Python for ASR model runtime and benchmarking

The overall direction is a good fit for the product. The main correction from the original plan is this: the ASR stack and packaging size are not proven yet. Treat Parakeet as a primary candidate, not as a guaranteed final dependency.

---

## Direction Lock

Keep these choices unless a phase gate proves they are wrong:

| Area | Decision | Why |
|---|---|---|
| UI shell | Electron + Svelte + TypeScript | Fast product iteration, already scaffolded |
| Styling | Tailwind CSS | Fits the existing preference and app shape |
| JS package manager | Bun | Matches the repo and Dany's preference |
| Native helper | Rust | Correct home for hotkeys, injection, clipboard, Windows audio |
| Rust bindings | `windows` / windows-rs | Direct Windows API access |
| Python manager | uv | Correct package manager for the ASR service |
| IPC, Electron to Rust | Windows named pipe | Already implemented and appropriate |
| IPC, Rust to Python | NDJSON over stdio | Simple, debuggable, good enough |
| Primary ASR candidate | `nvidia/parakeet-tdt-0.6b-v3` | Strong candidate, but must pass Phase 3 |
| Production audio | Rust WASAPI | Avoid building and later replacing a Python audio pipeline |

## Current Repo Baseline

The repo is already past the pure setup stage:

- Electron main/preload/renderer exist.
- Rust named-pipe IPC exists.
- Rust can spawn the Python ASR worker.
- Python currently produces mock transcript events.
- `cargo check` passes for the Rust helper in this environment.

Treat Phase 1 as complete and Phase 2 as complete enough for Phase 3. The remaining Phase 2 verification is environment/tooling cleanup, not a reason to delay ASR validation.

---

## Version Policy

Do not hard-pin old versions in planning docs unless the implementation has proved that pin is required.

| Dependency | Policy |
|---|---|
| Electron/Svelte/Vite/TypeScript | Keep current repo versions during Phase 3. Revisit upgrades before Phase 4 or packaging. |
| Python | Use the version range that NeMo and the model actually support. Validate on Windows in Phase 3. |
| PyTorch | Pick CPU/CUDA builds from the NeMo compatibility matrix during the ASR spike. Do not assume `torch==2.1.0+cu118` is still the right build. |
| NeMo | Use the version recommended by the Parakeet model card or current NeMo docs, then lock it after the spike. |
| CUDA | Benchmark CUDA and CPU separately. CUDA can be a performance path, but shipping CUDA dependencies is a packaging decision, not a default assumption. |

## ASR Reality Check

The original plan assumed a 1.2 GB FP16 Parakeet file and a direct `model.fp16.bin` download. That is not safe.

As of this review, the Hugging Face model page for `nvidia/parakeet-tdt-0.6b-v3` shows:

- NeMo usage via `ASRModel.from_pretrained("nvidia/parakeet-tdt-0.6b-v3")`
- 25 supported languages
- streaming guidance through NVIDIA's chunked inference script
- repository files including `model.safetensors` and `.nemo` artifacts around 2.51 GB each, with repo storage around 5.02 GB

Implication: the 2.1 GB installed-size claim is not currently proven. Phase 3 must measure the actual cache size after loading and clean up duplicate artifacts. If Parakeet requires a multi-GB runtime/model footprint, that becomes a product decision or contingency trigger.

---

## Layer Breakdown

| Layer | Technology | Status | Notes |
|---|---|---|---|
| Desktop shell | Electron | Keep | Already scaffolded |
| UI framework | Svelte | Keep | Existing app uses Svelte |
| UI language | TypeScript | Keep | No `any` unless truly unavoidable |
| Build tool | Vite | Keep | Already scaffolded |
| Styling | Tailwind CSS | Keep | Keep UI utility-like, not marketing-heavy |
| Native helper | Rust | Keep | Owns Windows APIs |
| ASR runtime | Python | Keep for Phase 3 | Re-evaluate only if Parakeet/NeMo fails |
| Primary model | Parakeet TDT 0.6B v3 | Candidate | Must pass Phase 3 gates |
| Fallback models | Faster-Whisper, Whisper.cpp | Contingency | Prepare only if Parakeet fails gates |
| Audio capture | Rust WASAPI for product | Corrected | Python `sounddevice` is allowed only for disposable spikes |
| Packaging | electron-builder + NSIS | Likely | Final packaging depends on ASR runtime size |

---

## Audio Decision

The old plan suggested building Phase 6 around Python `sounddevice` and later moving to Rust WASAPI. That creates avoidable rework.

Corrected direction:

- Phase 3 may use WAV files and optional Python microphone spikes for ASR validation.
- Production audio capture belongs in Rust WASAPI.
- Python should become inference-focused: receive 16 kHz mono float32 chunks, run ASR, send transcript events.

This better matches the architecture and the product goal: a fast Windows utility, not a Python audio app hidden inside Electron.

---

## Size Targets

Size targets are now gates, not promises.

| Metric | Preferred | Hard Gate | Notes |
|---|---:|---:|---|
| Installer without model | < 1.0 GB | < 1.5 GB | Depends heavily on Python/PyTorch packaging |
| Model cache | < 2.75 GB | < 3.25 GB | Must avoid duplicate `.nemo` and safetensors copies if possible |
| Installed after model | < 3.5 GB | < 4.5 GB | If above this, make an explicit business/product decision |
| Runtime memory | < 2.0 GB | < 3.0 GB | Peak during normal dictation |

If these gates are too large for the market you want, do not force Parakeet. Move to the contingency plan early.

---

## Security and Packaging Notes

- Use per-user install by default.
- Do not require admin for normal use.
- Model downloads must use HTTPS and checksum or Hugging Face snapshot revision pinning.
- Do not use UPX compression by default. It can create antivirus/signing problems that are not worth saving a few MB.
- Code signing is required before public distribution. Unsigned builds are acceptable only for local/private beta.

---

## Success Criteria

- All layers build and communicate locally.
- Phase 3 proves an ASR stack on Windows with measured latency, memory, disk size, and accuracy.
- The chosen ASR stack has a packaging path that does not destroy the product economics.
- Rust owns hotkeys, injection, clipboard, supervision, and production audio.
- Python remains replaceable behind a generic ASR interface.

---

## Related Documents

- [01-SUMMARY.md](01-SUMMARY.md)
- [07-PHASE-03-ASR-ENGINE.md](07-PHASE-03-ASR-ENGINE.md)
- [10-PHASE-06-AUDIO.md](10-PHASE-06-AUDIO.md)
- [13-PHASE-09-PACKAGING.md](13-PHASE-09-PACKAGING.md)
- [18-CONTINGENCY.md](18-CONTINGENCY.md)
