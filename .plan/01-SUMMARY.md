# Summary & Architecture

## What We Are Building

**OpenWhisper** is a **Windows-first offline dictation desktop application**. It allows users to press a global hotkey, speak naturally, and have their speech transcribed into text in any application.

**Key Characteristics**:
- **Offline-first**: No internet connection required after initial setup
- **Lightweight utility**: Always-available, not a heavy "application"
- **Privacy-focused**: Speech never leaves your computer
- **Fast**: Real-time transcription with <500ms latency (target)

**Similar to**: WhisperFlow, Windows Speech Recognition (but better), macOS Dictation (but for Windows)

---

## Core Architecture

OpenWhisper uses a **three-tier hybrid architecture** that combines the best tools for each layer:

```
┌─────────────────────────────────────────────────────────────────────┐
│ LAYER 1: Electron + Svelte + TypeScript                              │
│ Responsibility: Product Experience                                   │
│                                                                      │
│ - UI, windows, system tray, floating overlay                        │
│ - Settings management and persistence                               │
│ - App lifecycle and updates                                         │
│ - Displays transcription in real-time                               │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ Local IPC (named pipes/sockets)
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│ LAYER 2: Rust Native Helper                                          │
│ Responsibility: Windows Integration                                  │
│                                                                      │
│ - Global hotkey registration (works everywhere)                     │
│ - Text injection into focused applications                          │
│ - Clipboard fallback for blocked applications                       │
│ - Active window detection                                           │
│ - Python ASR worker supervision                                     │
│ - (Phase 6+) Audio capture via WASAPI                               │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ NDJSON over stdio
                              │ (newline-delimited JSON)
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│ LAYER 3: Python ASR Worker                                           │
│ Responsibility: Speech Recognition                                   │
│                                                                      │
│ - Model loading (Parakeet/NeMo)                                     │
│ - ASR inference and model benchmarking                              │
│ - Chunked inference with streaming                                  │
│ - Voice Activity Detection (VAD)                                    │
│ - Returns partial and final transcripts                             │
└─────────────────────────────────────────────────────────────────────┘
```

**The Main Principle**: Electron owns the product experience, Rust owns Windows integration, Python owns the model runtime.

---

## Why This Architecture?

### Why Electron?

**Pros**:
- Modern UI development with HTML/CSS/JS
- Rich ecosystem (Svelte, Tailwind, Vite)
- Cross-platform foundation (though v1 is Windows-only)
- Easy to build beautiful, responsive interfaces

**Cons**:
- Larger bundle size (~150MB)
- Cannot access Windows APIs directly (needs Rust helper)

**Mitigation**: Use Rust for native functionality; keep Electron lean.

### Why Rust?

**Pros**:
- Direct Windows API access (SendInput, hotkeys, WASAPI)
- Memory safety (no crashes from buffer overflows)
- Small binary size (~5MB)
- Fast performance
- Easier to distribute than C++

**Cons**:
- Steeper learning curve than C#
- Smaller ecosystem than C++

**Alternative considered**: C# with P/Invoke. Chose Rust for memory safety and smaller binaries.

### Why Python?

**Pros**:
- NVIDIA NeMo (our chosen ASR framework) is Python-native
- Excellent ML ecosystem (PyTorch, NumPy)
- Fast prototyping
- Easy model management

**Cons**:
- Slower than native code for audio processing
- Larger bundle size (~800MB with dependencies)
- GIL limitations for true parallelism

**Mitigation**: Use Python for model inference and benchmarking. Production microphone capture belongs in Rust WASAPI.

---

## Guiding Principles

| Principle | Description | Implication |
|-----------|-------------|-------------|
| **Separation of Concerns** | Each layer has one job | UI doesn't touch Windows API, Rust doesn't do ML |
| **Offline-first** | No cloud required | Model runs locally, no internet needed |
| **Lightweight feel** | Should feel like a utility, not an app | Minimal UI, global hotkey, stays in tray |
| **Validate early** | Test the hardest part first | Phase 3 validates ASR before UI investment |
| **Graceful degradation** | When things fail, recover smoothly | Clipboard fallback, CPU fallback, worker restart |

---

## Key Technical Decisions

### 1. Communication Protocols

**Electron ↔ Rust**: Local IPC (named pipes on Windows)
- Fast, reliable, language-agnostic
- TypeScript interfaces on Electron side, Rust structs on native side

**Rust ↔ Python**: NDJSON over stdio
- Simple: One JSON object per line
- Debuggable: Can inspect with `cat` or `tail`
- No dependencies: Works with any language
- Reliable: Line boundaries are unambiguous

### 2. Model Delivery

**Download on first run, not bundled**
- Installer size: ~1GB vs ~2.2GB
- Faster initial download
- Can update model independently
- Users can skip and download later

### 3. Audio Capture Strategy

**Phase 3**: WAV-file benchmarks first. Python `sounddevice` is allowed only as a disposable microphone spike.

**Phase 6+**: Rust WASAPI for the product.
- Better performance (lower latency)
- More control over audio pipeline
- Python becomes inference-only
- Avoids building a Python audio path that we later throw away

### 4. Chunked Inference

Parakeet is not inherently streaming. We simulate streaming:
- Overlapping chunks (20% overlap)
- Discard boundary regions
- Balance latency vs accuracy with chunk size

Chunk sizes:
- **Fast** (0.5s): Real-time feel, lower accuracy
- **Balanced** (2s): Good tradeoff (default)
- **Accurate** (4s): Best accuracy, higher latency

---

## Data Flow

### Normal Dictation Flow

```
1. User presses hotkey (Ctrl+Shift+D)
   ↓
2. Rust detects hotkey via Windows API
   ↓
3. Rust sends "dictation.start" to Python
   ↓
4. Rust starts microphone capture when the audio phase is implemented
   ↓
5. Audio chunks flow: Mic -> Rust WASAPI -> Python worker
   ↓
6. Python runs ASR inference on each chunk
   ↓
7. Python sends "transcript.partial" to Rust
   ↓
8. Rust injects text into focused window (SendInput)
   ↓
9. Rust sends transcript to Electron for overlay
   ↓
10. User sees text appear in both target app and overlay
   ↓
11. User presses hotkey again to stop
   ↓
12. Final text committed, overlay closes
```

### Error Recovery Flow

```
Python worker crashes
   ↓
Rust detects process exit
   ↓
Log crash details
   ↓
Attempt restart (max 3 times)
   ↓
Back off: 1s, 5s, 30s between attempts
   ↓
Notify Electron: "Dictation service recovered" or "Service unavailable"
   ↓
Continue or pause dictation
```

---

## Project Structure

```
openwhisper/
│
├── apps/
│   └── desktop/              # Electron + Svelte app
│       ├── src/
│       │   ├── main/         # Main process (Node.js)
│       │   ├── renderer/     # Svelte UI
│       │   └── shared/       # Shared types
│       ├── package.json
│       └── vite.config.ts
│
├── crates/
│   └── openwhisper-native/   # Rust native helper
│       ├── src/
│       │   ├── main.rs       # Entry point
│       │   ├── ipc.rs        # Electron IPC
│       │   ├── hotkeys.rs    # Global hotkeys
│       │   ├── injection.rs  # Text injection
│       │   └── ...
│       └── Cargo.toml
│
├── services/
│   └── asr/                  # Python ASR worker
│       ├── src/openwhisper_asr/
│       │   ├── __main__.py   # Entry point
│       │   ├── engine.py     # Model inference
│       │   ├── audio/        # Audio capture
│       │   └── ...
│       └── pyproject.toml
│
├── packages/
│   └── protocol/             # Shared schemas
│       └── schemas/
│
└── .plan/                    # Planning documents
    ├── 00-INDEX.md           # Start here
    ├── 01-SUMMARY.md         # This file
    ├── 02-PRODUCT-EXPERIENCE.md
    ├── 03-TECH-STACK.md
    ├── 04-PROJECT-STRUCTURE.md
    ├── 05-PHASE-01-REPO-SETUP.md
    ├── 06-PHASE-02-PROTOCOL.md
    ├── 07-PHASE-03-ASR-ENGINE.md      # CRITICAL
    ├── 08-PHASE-04-PRODUCT-SHELL.md
    ├── 09-PHASE-05-RUST-HELPER.md
    ├── 10-PHASE-06-AUDIO.md
    ├── 11-PHASE-07-END-TO-END.md
    ├── 12-PHASE-08-PERFORMANCE.md
    ├── 13-PHASE-09-PACKAGING.md
    ├── 14-VERIFICATION.md
    ├── 15-ASSUMPTIONS.md
    ├── 16-ERROR-HANDLING.md
    ├── 17-PERFORMANCE-BUDGET.md
    └── 18-CONTINGENCY.md       # Plan B
```

---

## Critical Path

The development phases must be executed in order:

```
Phase 1: Repository Setup
    ↓
Phase 2: Protocol Definition
    ↓
Phase 3: ASR Engine Validation ⭐ CRITICAL
    ↓
    [If Parakeet fails → Execute Contingency Plan]
    ↓
Phase 4: Product Shell (UI)
    ↓
Phase 5: Rust Native Helper
    ↓
Phase 6: Audio Pipeline
    ↓
Phase 7: End-to-End Integration
    ↓
Phase 8: Performance & Reliability
    ↓
Phase 9: Packaging & Distribution
```

**Phase 3 is the major technical risk**. If Parakeet's streaming quality, Windows support, latency, memory, or model size is unacceptable, we must stop and evaluate alternatives.

---

## Success Definition

The project is successful when:

1. **Functional**: User can press hotkey, speak, and see text appear in any application
2. **Fast**: Latency <500ms in Fast mode on target hardware
3. **Reliable**: Works for 1+ hour continuous dictation without crashes
4. **Usable**: No manual Python/Rust setup required (packaged installer)
5. **Private**: No cloud processing, no data leaves the computer

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Parakeet streaming quality poor | Medium | High | Contingency Plan ready (Doc 18) |
| Windows injection blocked | Low | Medium | Clipboard fallback |
| High memory usage | Medium | Medium | Model unloading, optimization |
| Audio device compatibility | Low | Medium | Multiple backend support |
| Build complexity | Low | Low | Clear phase separation |

---

## Getting Started

1. Read [00-INDEX.md](00-INDEX.md) for navigation
2. Review [02-PRODUCT-EXPERIENCE.md](02-PRODUCT-EXPERIENCE.md) for UX design
3. Check [03-TECH-STACK.md](03-TECH-STACK.md) for technology choices
4. Begin with [05-PHASE-01-REPO-SETUP.md](05-PHASE-01-REPO-SETUP.md)

---

## Key Documents

| Document | Purpose |
|----------|---------|
| **00-INDEX.md** | Navigation and overview |
| **01-SUMMARY.md** | This file - architecture overview |
| **02-PRODUCT-EXPERIENCE.md** | User flow and interaction design |
| **03-TECH-STACK.md** | Technology choices and rationale |
| **15-ASSUMPTIONS.md** | Constraints and default behaviors |
| **16-ERROR-HANDLING.md** | Error recovery strategy |
| **17-PERFORMANCE-BUDGET.md** | Latency and memory targets |
| **18-CONTINGENCY.md** | Plan B if Parakeet fails |
