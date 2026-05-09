# OpenWhisper Development Plan

## Complete Project Overview

**OpenWhisper** is a Windows-first offline dictation desktop application that converts speech to text in real-time using local AI models.

**Key Characteristics**:
- **Offline-first**: Works without internet after initial setup
- **Lightweight feel**: Always-available utility, not a heavy application
- **Three-tier architecture**: Electron (UI) + Rust (native) + Python (ASR)
- **Privacy-focused**: No cloud processing, opt-in telemetry only

---

## Quick Start for AI Agents

**What you need to know right now**:

1. **This is a monorepo** with three distinct layers:
   - `apps/desktop/` - Electron + Svelte + TypeScript (UI)
   - `crates/openwhisper-native/` - Rust (Windows integration)
   - `services/asr/` - Python + NeMo (speech recognition)

2. **Development order matters**: Follow phases 1-9 sequentially

3. **Phase 3 is critical**: ASR model validation is a Go/No-Go gate

4. **Do not trust old size assumptions**: Parakeet is a candidate, not a commitment. Phase 3 must measure the actual model/cache/runtime size before UI polish.

5. **Reference documents**:
   - Start with `00-INDEX.md` for navigation
   - Check `15-ASSUMPTIONS.md` for defaults
   - Read `16-ERROR-HANDLING.md` for error scenarios
   - See `17-PERFORMANCE-BUDGET.md` for targets
   - Know `18-CONTINGENCY.md` has Plan B ready
   - Read `19-PLAN-REVIEW.md` before delegating new work

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 1: Electron + Svelte + TypeScript                         │
│  Responsibility: Product experience                              │
│                                                                  │
│  Components:                                                     │
│  - Main process: App lifecycle, tray, window management         │
│  - Renderer: Svelte UI (settings, overlay, onboarding)          │
│  - Preload: Secure IPC bridge                                   │
│                                                                  │
│  Key Files:                                                      │
│  - apps/desktop/src/main/           # Main process              │
│  - apps/desktop/src/renderer/       # Svelte components         │
│  - apps/desktop/src/shared/         # Shared types              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Local IPC (named pipes/sockets)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 2: Rust Native Helper                                     │
│  Responsibility: Windows integration                             │
│                                                                  │
│  Components:                                                     │
│  - hotkeys.rs: Global hotkey registration (e.g., Ctrl+Shift+D)  │
│  - injection.rs: SendInput text injection                       │
│  - clipboard.rs: Clipboard fallback                             │
│  - window.rs: Active window detection                           │
│  - python_worker.rs: ASR process supervision                    │
│  - audio.rs: WASAPI capture (Phase 6+)                          │
│                                                                  │
│  Key Files:                                                      │
│  - crates/openwhisper-native/src/   # Rust source               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ NDJSON over stdio
                              │ (newline-delimited JSON)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 3: Python ASR Worker                                      │
│  Responsibility: Speech recognition                              │
│                                                                  │
│  Components:                                                     │
│  - engine.py: Parakeet/NeMo model loading & inference           │
│  - streaming.py: Chunked inference with overlap                 │
│  - audio/: Audio preprocessing, VAD                             │
│  - protocol.py: NDJSON message handling                         │
│                                                                  │
│  Key Files:                                                      │
│  - services/asr/src/openwhisper_asr/ # Python source            │
└─────────────────────────────────────────────────────────────────┘
```

**Core Principle**: Electron owns the product experience, Rust owns Windows integration, Python owns the model runtime.

---

## Product Experience

OpenWhisper is **not** a traditional windowed application. It behaves like a system utility:

### Primary User Flow

```
User presses global hotkey (e.g., Ctrl+Shift+D)
  → Small popup appears above all windows (doesn't steal focus)
  → User speaks
  → Live transcription appears in popup (partial results)
  → User stops speaking
  → Final text is inserted into currently focused application
  → Popup disappears or shows idle state
```

### UI Surfaces

1. **System Tray Icon**: Always visible, access to settings
2. **Floating Overlay**: Frameless, always-on-top, shows transcription
3. **Settings Window**: Main configuration interface
4. **Onboarding**: First-run model download and setup
5. **Diagnostics**: Log viewer and export (hidden in settings)

### Key Behaviors

- **Invisible until needed**: Minimal footprint when idle
- **Instant availability**: Hotkey response <100ms
- **Transparent operation**: User speaks, text appears - simple
- **Graceful degradation**: If injection fails, use clipboard

---

## Technology Stack

| Layer | Technology | Justification |
|-------|-----------|---------------|
| **Desktop Shell** | Electron | Cross-platform foundation, though v1 is Windows-only |
| **UI Framework** | Svelte + TypeScript | Fast, type-safe, minimal boilerplate |
| **Build Tool** | Vite | Fast HMR, modern bundling |
| **Styling** | Tailwind CSS | Utility-first, rapid iteration |
| **Package Manager** | Bun | Fast installs, modern toolchain |
| **Native Helper** | Rust | Windows API access, memory safety, small binaries |
| **ASR Runtime** | Python | Version must be validated with the selected ASR stack |
| **Python Environment** | uv | Fast Python package management |
| **Model Framework** | PyTorch + NeMo | NVIDIA's speech recognition toolkit |
| **Speech Model** | Parakeet TDT 0.6B v3 | Good quality/size tradeoff |
| **Audio spike** | Python sounddevice or WAV files | Validation only, disposable |
| **Audio production** | Rust WASAPI | Better Windows performance, Phase 6 |
| **IPC (Electron-Rust)** | Local sockets/pipes | Fast, reliable |
| **IPC (Rust-Python)** | NDJSON over stdio | Simple, debuggable, language-agnostic |
| **Packaging** | electron-builder | Bundles all layers into installer |
| **Installer** | NSIS | Standard Windows installer |

---

## Project Structure

```
openwhisper/                          # Repository root
│
├── apps/                             # Frontend applications
│   └── desktop/                      # Electron app
│       ├── src/
│       │   ├── main/                 # Main process (Node.js)
│       │   │   ├── index.ts          # Entry point
│       │   │   ├── ipc.ts            # IPC handlers
│       │   │   ├── tray.ts           # System tray
│       │   │   └── windows.ts        # Window management
│       │   ├── renderer/             # Renderer process (Svelte)
│       │   │   ├── App.svelte        # Root component
│       │   │   ├── components/       # Svelte components
│       │   │   ├── routes/           # Page components
│       │   │   └── stores/           # Svelte stores
│       │   └── shared/               # Shared between main/renderer
│       │       ├── types.ts          # TypeScript interfaces
│       │       └── protocol.ts       # IPC protocol client
│       ├── package.json              # Bun dependencies
│       ├── vite.config.ts            # Vite configuration
│       ├── electron-builder.yml      # Packaging config
│       └── tsconfig.json             # TypeScript config
│
├── crates/                           # Rust packages
│   └── openwhisper-native/           # Native helper
│       ├── src/
│       │   ├── main.rs               # Entry point
│       │   ├── ipc.rs                # Electron IPC
│       │   ├── python_worker.rs      # Python process supervisor
│       │   ├── hotkeys.rs            # Global hotkeys
│       │   ├── injection.rs          # Text injection
│       │   ├── clipboard.rs          # Clipboard operations
│       │   ├── window.rs             # Window detection
│       │   └── audio.rs              # WASAPI (Phase 6+)
│       └── Cargo.toml                # Rust dependencies
│
├── services/                         # Backend services
│   └── asr/                          # Python ASR worker
│       ├── src/openwhisper_asr/
│       │   ├── __init__.py
│       │   ├── __main__.py           # Entry point
│       │   ├── config.py             # Configuration
│       │   ├── protocol.py           # NDJSON protocol
│       │   ├── engine.py             # Model loading/inference
│       │   ├── streaming.py          # Chunked inference
│       │   ├── result.py             # Result types
│       │   └── audio/
│       │       ├── preprocessing.py  # Audio preprocessing
│       │       └── vad.py            # Voice activity detection
│       ├── scripts/
│       │   ├── download_model.py     # Model download
│       │   └── benchmark.py          # Performance testing
│       └── pyproject.toml            # Python dependencies
│
├── packages/                         # Shared packages
│   └── protocol/                     # Protocol schemas
│       └── schemas/
│           └── messages.json         # Shared message schemas
│
├── .plan/                            # Planning documents
│   ├── 00-INDEX.md                   # Navigation
│   ├── 01-SUMMARY.md                 # Architecture overview
│   ├── 02-PRODUCT-EXPERIENCE.md      # UX design
│   ├── 03-TECH-STACK.md              # Technology choices
│   ├── 04-PROJECT-STRUCTURE.md       # Directory layout
│   ├── 05-PHASE-01-REPO-SETUP.md     # Phase 1
│   ├── 06-PHASE-02-PROTOCOL.md       # Phase 2
│   ├── 07-PHASE-03-ASR-ENGINE.md     # Phase 3 (CRITICAL)
│   ├── 08-PHASE-04-PRODUCT-SHELL.md  # Phase 4
│   ├── 09-PHASE-05-RUST-HELPER.md    # Phase 5
│   ├── 10-PHASE-06-AUDIO.md          # Phase 6
│   ├── 11-PHASE-07-END-TO-END.md     # Phase 7
│   ├── 12-PHASE-08-PERFORMANCE.md    # Phase 8
│   ├── 13-PHASE-09-PACKAGING.md      # Phase 9
│   ├── 14-VERIFICATION.md            # Testing checklist
│   ├── 15-ASSUMPTIONS.md             # Defaults & constraints
│   ├── 16-ERROR-HANDLING.md          # Error strategy
│   ├── 17-PERFORMANCE-BUDGET.md      # Latency targets
│   ├── 18-CONTINGENCY.md             # Plan B if Parakeet fails
│   └── 19-PLAN-REVIEW.md             # Current corrected direction
│
├── README.md                         # Project overview
├── package.json                      # Root package.json (workspaces)
├── bun.lock                          # Bun lockfile
└── .gitignore                        # Git ignore rules
```

---

## Development Phases (Execute in Order)

### Phase 1: Repository Setup
**Goal**: Working monorepo with all three layers building independently

**Key Tasks**:
- Initialize Bun workspace for Electron app
- Initialize Rust crate for native helper
- Initialize Python service with uv
- Add root scripts for dev commands
- Choose the Python version required by the validated ASR stack

**Success Criteria**:
- [ ] Electron window launches
- [ ] Rust helper builds
- [ ] Python ASR worker starts and responds to health check
- [ ] Root README explains dev flow

### Phase 2: Protocol and Process Architecture
**Goal**: Stable communication between all layers

**Key Tasks**:
- Define NDJSON message types (health, audio, transcript, etc.)
- Implement Electron ↔ Rust IPC
- Implement Rust ↔ Python NDJSON over stdio
- Create shared TypeScript interfaces

**Success Criteria**:
- [ ] Electron can request status from Rust
- [ ] Rust can spawn Python worker
- [ ] Python can send mock transcript events
- [ ] UI can render live mock transcripts

### Phase 3: ASR Engine Validation ⭐ CRITICAL
**Goal**: Prove Parakeet works for streaming dictation, or choose a fallback early

**Key Tasks**:
- Load/download Parakeet through NeMo/Hugging Face mechanisms
- Implement chunked inference
- Benchmark batch vs chunked accuracy
- Measure latency and memory
- Measure actual model cache and runtime footprint

**Success Criteria (ALL must pass)**:
- [ ] Model loads on GPU and CPU
- [ ] Chunked WER within 2% of batch WER
- [ ] Latency <500ms (Fast mode) on target hardware
- [ ] Memory <2GB peak
- [ ] Model/cache size is acceptable for the product

**DECISION POINT**: If any criteria fail, execute [Contingency Plan](18-CONTINGENCY.md) - evaluate smaller Parakeet, Faster-Whisper, or Whisper.cpp.

### Phase 4: Electron/Svelte Product Shell
**Goal**: Build visible app experience (blocked until Phase 3 passes)

**Key Tasks**:
- Create settings window with all sections
- Implement system tray
- Build floating overlay (frameless, always-on-top)
- Create model loading/onboarding screen

**Success Criteria**:
- [ ] App opens to settings UI
- [ ] Tray works
- [ ] Overlay appears without stealing focus
- [ ] Settings persist locally

### Phase 5: Rust Native Helper v1
**Goal**: Move fragile Windows behavior out of Electron/Python

**Key Tasks**:
- Register global hotkey
- Implement SendInput text injection
- Implement clipboard fallback
- Launch and supervise Python worker
- Handle partial result updates (backspace + retype)

**Success Criteria**:
- [ ] Hotkey toggles dictation globally
- [ ] Text injects into Notepad
- [ ] Clipboard fallback works
- [ ] Electron receives status updates

### Phase 6: Audio Pipeline
**Goal**: Capture microphone and stream to ASR

**Key Tasks**:
- Implement Rust WASAPI capture for the product path
- Capture mic audio, convert to 16kHz mono float32
- Implement ring buffer and chunk reader
- Add VAD gate
- Keep Python audio capture only as a disposable spike if needed

**Success Criteria**:
- [ ] Speak into mic, see console transcript
- [ ] Speak into mic, see overlay transcript
- [ ] Speak into mic, inject text into Notepad

### Phase 7: End-to-End Dictation Prototype
**Goal**: Prove the complete product loop

**Key Tasks**:
- Wire everything together
- Hotkey → Rust → Python → Audio → ASR → Rust → Injection + Overlay
- Handle errors gracefully
- Show model loading state

**Success Criteria**:
- [ ] Open Notepad
- [ ] Press hotkey, speak, see text appear
- [ ] Press hotkey again to stop
- [ ] App remains in tray

### Phase 8: Performance and Reliability
**Goal**: Make it fast and stable enough for daily use

**Key Tasks**:
- Use `torch.inference_mode()` and GPU precision only where supported
- Measure and optimize each stage
- Implement worker crash recovery
- Implement audio device disconnect recovery
- Add adaptive chunk sizing

**Success Criteria**:
- [ ] Latency benchmarks for each mode
- [ ] Recovery from simulated crash works
- [ ] UI shows clear error messages
- [ ] Logs are persisted

### Phase 9: Packaging and Distribution
**Goal**: Ship Windows installer

**Key Tasks**:
- Package with electron-builder
- Bundle Rust helper binary
- Bundle embedded Python runtime
- Download model on first launch (not in installer)
- Create NSIS installer

**Success Criteria**:
- [ ] Fresh Windows machine can install
- [ ] App launches without manual Python setup
- [ ] Model downloads on first run
- [ ] Dictation works after setup

---

## Verification Checklist

Before considering the project complete, verify:

1. **ASR Smoke Test**
   - [ ] Load model
   - [ ] Transcribe WAV file
   - [ ] Compare chunked vs batch output

2. **Protocol Test**
   - [ ] Electron starts Rust
   - [ ] Rust starts Python
   - [ ] Mock events reach UI

3. **Native Helper Test**
   - [ ] Hotkey works globally
   - [ ] Text injection works
   - [ ] Clipboard fallback works

4. **Audio Test**
   - [ ] Microphone capture works
   - [ ] No buffer overruns
   - [ ] VAD gates silence

5. **End-to-End Test**
   - [ ] Full dictation loop works
   - [ ] Overlay updates
   - [ ] Text appears in target app

6. **Packaging Test**
   - [ ] Clean install works
   - [ ] Model downloads
   - [ ] Dictation works

---

## Key Constraints & Assumptions

See [Assumptions](15-ASSUMPTIONS.md) for full details.

**Quick Reference**:
- **Offline-first**: No cloud required after setup
- **Windows-only v1**: Focus before expansion
- **Model downloaded**: Not bundled (keeps installer small)
- **Contingency ready**: Plan B if Parakeet fails
- **Validate early**: Phase 3 is Go/No-Go gate
- **Size is a measured gate**: the old 2.1GB target is not assumed

---

## Performance Targets

See [Performance Budget](17-PERFORMANCE-BUDGET.md) for full details.

**Quick Reference**:
- **Latency (Fast mode)**: <500ms perceived
- **Latency (Balanced mode)**: <1000ms perceived
- **Memory**: <2GB target
- **Chunk sizes**: 0.5s (Fast), 2s (Balanced), 4s (Accurate)

---

## Error Handling Philosophy

See [Error Handling](16-ERROR-HANDLING.md) for full details.

**Quick Reference**:
- **Fatal errors**: Clear message, graceful exit
- **Recoverable errors**: Auto-retry, notify if persistent
- **User errors**: Explain, offer solution
- **Always log**: For diagnostics

---

## Getting Help

**Documents by Purpose**:
- **Navigation**: `00-INDEX.md`
- **Architecture**: `01-SUMMARY.md`, this file
- **Implementation**: Phase documents (05-13)
- **Testing**: `14-VERIFICATION.md`
- **Defaults**: `15-ASSUMPTIONS.md`
- **Errors**: `16-ERROR-HANDLING.md`
- **Performance**: `17-PERFORMANCE-BUDGET.md`
- **Fallback**: `18-CONTINGENCY.md`
- **Current review**: `19-PLAN-REVIEW.md`

---

**Last Updated**: 2026-05-09 plan review
**Status**: Reviewed 2026-05-09. Current phase is Phase 3 ASR validation.
