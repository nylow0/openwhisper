# Project Structure

## Context

This document defines the directory layout and organization for OpenWhisper - a Windows-first offline dictation desktop app. This structure supports a three-tier architecture (Electron/Svelte UI, Rust native helper, Python ASR worker) in a monorepo format.

**Philosophy**: Clear separation by layer, minimal cross-dependencies, intuitive navigation.

**Current rule**: This is a target structure, not a command to create empty folders. Create files and folders only when a phase needs them. Keep the repo clean; no dead scaffolding.

---

## Monorepo Layout

```
openwhisper/                          # Repository root
│
├── 📁 apps/                          # Frontend applications
│   └── 📁 desktop/                   # Electron + Svelte + TypeScript
│       ├── 📁 src/
│       │   ├── 📁 main/              # Electron main process (Node.js)
│       │   │   ├── 📄 index.ts       # Main entry point
│       │   │   ├── 📄 ipc.ts         # IPC handlers (Electron ↔ Rust)
│       │   │   ├── 📄 tray.ts        # System tray management
│       │   │   ├── 📄 windows.ts     # Window management
│       │   │   └── 📄 preload.ts     # Preload script (bridge)
│       │   │
│       │   ├── 📁 renderer/          # Svelte UI (Chromium)
│       │   │   ├── 📄 App.svelte     # Root component
│       │   │   ├── 📄 main.ts        # Renderer entry
│       │   │   ├── 📄 app.css        # Global styles (Tailwind)
│       │   │   ├── 📁 components/    # Reusable Svelte components
│       │   │   │   ├── 📄 Overlay.svelte
│       │   │   │   ├── 📄 Settings.svelte
│       │   │   │   └── 📄 AudioLevel.svelte
│       │   │   │
│       │   │   ├── 📁 routes/        # Page components
│       │   │   │   ├── 📄 Home.svelte
│       │   │   │   └── 📄 Onboarding.svelte
│       │   │   │
│       │   │   └── 📁 stores/        # Svelte stores (state)
│       │   │       ├── 📄 settings.ts
│       │   │       └── 📄 dictation.ts
│       │   │
│       │   └── 📁 shared/            # Shared between main/renderer
│       │       ├── 📄 types.ts       # TypeScript interfaces
│       │       └── 📄 protocol.ts    # IPC protocol client
│       │
│       ├── 📄 package.json           # Bun dependencies
│       ├── 📄 tsconfig.json          # TypeScript config
│       ├── 📄 vite.config.ts         # Vite build config
│       ├── 📄 tailwind.config.js     # Tailwind CSS config
│       ├── 📄 svelte.config.js       # Svelte config
│       ├── 📄 electron-builder.yml   # Packaging config
│       ├── 📄 index.html             # HTML template
│       └── 📄 README.md              # App-specific docs
│
├── 📁 crates/                        # Rust packages
│   └── 📁 openwhisper-native/        # Native helper crate
│       ├── 📁 src/
│       │   ├── 📄 main.rs            # Rust entry point
│       │   ├── 📄 lib.rs             # Library root (for tests)
│       │   ├── 📄 ipc.rs             # Electron IPC (named pipes)
│       │   ├── 📄 python_worker.rs   # Python process supervisor
│       │   ├── 📄 protocol.rs        # NDJSON protocol (Rust-Python)
│       │   ├── 📄 hotkeys.rs         # Global hotkey registration
│       │   ├── 📄 injection.rs       # SendInput text injection
│       │   ├── 📄 clipboard.rs       # Clipboard operations
│       │   ├── 📄 window.rs          # Active window detection
│       │   ├── 📄 audio.rs           # WASAPI capture (Phase 6+)
│       │   ├── 📄 dictation.rs       # Dictation orchestration
│       │   └── 📄 error.rs           # Error types
│       │
│       ├── 📁 tests/                 # Rust tests
│       │   └── 📄 integration_tests.rs
│       │
│       ├── 📄 Cargo.toml             # Rust dependencies
│       └── 📄 README.md              # Crate-specific docs
│
├── 📁 services/                      # Backend services
│   └── 📁 asr/                       # Python ASR worker
│       ├── 📁 src/openwhisper_asr/   # Python package
│       │   ├── 📄 __init__.py        # Package init
│       │   ├── 📄 __main__.py        # Entry point (python -m)
│       │   ├── 📄 config.py          # Configuration
│       │   ├── 📄 protocol.py        # NDJSON protocol
│       │   ├── 📄 engine.py          # ASR model (Parakeet)
│       │   ├── 📄 streaming.py       # Chunked inference
│       │   ├── 📄 result.py          # Transcription result types
│       │   ├── 📁 audio/             # Audio processing
│       │   │   ├── 📄 __init__.py
│       │   │   ├── 📄 preprocessing.py
│       │   │   ├── 📄 vad.py         # Voice Activity Detection
│       │   │   └── 📄 resample.py
│       │   │
│       │   └── 📁 models/            # Model utilities
│       │       ├── 📄 __init__.py
│       │       ├── 📄 download.py    # Model download
│       │       └── 📄 cache.py       # Model cache management
│       │
│       ├── 📁 scripts/               # Standalone scripts
│       │   ├── 📄 download_model.py  # Download model manually
│       │   ├── 📄 benchmark.py       # Performance benchmarks
│       │   └── 📄 test_asr.py        # ASR validation tests
│       │
│       ├── 📁 tests/                 # Python tests
│       │   ├── 📄 test_protocol.py
│       │   └── 📄 test_engine.py
│       │
│       ├── 📄 pyproject.toml         # Python dependencies (uv)
│       ├── 📄 uv.lock                # Locked dependencies
│       └── 📄 README.md              # Service-specific docs
│
├── 📁 packages/                      # Shared packages
│   └── 📁 protocol/                  # Protocol definitions
│       ├── 📁 schemas/               # JSON schemas
│       │   ├── 📄 messages.json      # Message type schemas
│       │   └── 📄 config.json        # Config schema
│       │
│       ├── 📁 typescript/            # Generated TypeScript
│       │   └── 📄 types.ts
│       │
│       ├── 📁 python/                # Generated Python
│       │   └── 📄 types.py
│       │
│       └── 📄 README.md              # Protocol documentation
│
├── 📁 .plan/                         # Planning documents
│   ├── 📄 00-INDEX.md                # Start here
│   ├── 📄 01-SUMMARY.md              # Architecture overview
│   ├── 📄 02-PRODUCT-EXPERIENCE.md   # UX design
│   ├── 📄 03-TECH-STACK.md           # Technology choices
│   ├── 📄 04-PROJECT-STRUCTURE.md    # This file
│   ├── 📄 05-PHASE-01-REPO-SETUP.md
│   ├── 📄 06-PHASE-02-PROTOCOL.md
│   ├── 📄 07-PHASE-03-ASR-ENGINE.md
│   ├── 📄 08-PHASE-04-PRODUCT-SHELL.md
│   ├── 📄 09-PHASE-05-RUST-HELPER.md
│   ├── 📄 10-PHASE-06-AUDIO.md
│   ├── 📄 11-PHASE-07-END-TO-END.md
│   ├── 📄 12-PHASE-08-PERFORMANCE.md
│   ├── 📄 13-PHASE-09-PACKAGING.md
│   ├── 📄 14-VERIFICATION.md         # Testing checklist
│   ├── 📄 15-ASSUMPTIONS.md          # Defaults
│   ├── 📄 16-ERROR-HANDLING.md       # Error strategy
│   ├── 📄 17-PERFORMANCE-BUDGET.md   # Performance targets
│   ├── 📄 18-CONTINGENCY.md          # Plan B
│   └── 📄 19-PLAN-REVIEW.md          # Current corrected direction
│
├── 📁 scripts/                       # Development scripts
│   ├── 📄 setup.ps1                  # Windows setup script
│   ├── 📄 dev.ps1                    # Start dev environment
│   ├── 📄 build.ps1                  # Build all layers
│   ├── 📄 test.ps1                   # Run all tests
│   └── 📄 package.ps1                # Create installer
│
├── 📁 docs/                          # Additional documentation
│   ├── 📄 API.md                     # API documentation
│   ├── 📄 DEPLOYMENT.md              # Deployment guide
│   └── 📁 assets/                    # Images, diagrams
│
├── 📄 README.md                      # Project overview
├── 📄 LICENSE                        # License file
├── 📄 .gitignore                     # Git ignore rules
├── 📄 package.json                   # Root package.json (workspaces)
└── 📄 bun.lock                       # Bun lockfile
```

---

## Key Design Choices

### Top-Level Organization

| Directory | Purpose | Contents |
|-----------|---------|----------|
| `apps/` | User-facing applications | Electron desktop app |
| `crates/` | Rust packages | Native helper |
| `services/` | Backend services | Python ASR worker |
| `packages/` | Shared code | Protocol definitions |
| `.plan/` | Planning | All design documents |
| `scripts/` | Automation | Build, dev, test scripts |
| `docs/` | Documentation | API, deployment guides |

**Rationale**: Clear separation of concerns. Each top-level directory has a distinct purpose.

### Language-Specific Conventions

**TypeScript/JavaScript (apps/desktop)**:
- `src/main/` - Node.js main process code
- `src/renderer/` - Browser/Chromium code
- `src/shared/` - Code used by both

**Rust (crates/openwhisper-native)**:
- `src/` - Library/binary source
- `tests/` - Integration tests
- Standard Rust project structure

**Python (services/asr)**:
- `src/openwhisper_asr/` - Package source
- `scripts/` - Standalone utilities
- `tests/` - Unit tests
- Standard Python package structure

### Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Directories | kebab-case | `openwhisper-native/` |
| Files | kebab-case | `python-worker.rs` |
| Rust modules | snake_case | `python_worker.rs` |
| TypeScript files | camelCase | `ipcHandler.ts` |
| Python modules | snake_case | `audio_preprocessing.py` |
| Svelte components | PascalCase | `Overlay.svelte` |

---

## Build Artifacts

### Development

```
apps/desktop/
├── dist/                    # Vite build output
│   ├── main/               # Compiled main process
│   └── renderer/           # Compiled renderer
├── node_modules/           # Dependencies
└── .vite/                  # Vite cache

crates/openwhisper-native/
├── target/                 # Cargo build output
│   ├── debug/             # Debug builds
│   └── release/           # Release builds
└── Cargo.lock             # Dependency lock

services/asr/
├── .venv/                 # Python virtual environment
├── __pycache__/          # Python cache
└── *.egg-info/           # Package metadata
```

### Distribution

```
apps/desktop/dist-electron/
├── OpenWhisper-Setup-0.1.0.exe    # Main installer
├── OpenWhisper-0.1.0.exe          # Portable (optional)
├── latest.yml                      # Auto-update manifest
└── builder-effective-config.yaml   # Build config
```

---

## Dependencies Between Layers

### Allowed Dependencies

```
apps/desktop (Electron)
    ↓ can import
packages/protocol (schemas)

crates/openwhisper-native (Rust)
    ↓ can import
packages/protocol (schemas)

services/asr (Python)
    ↓ can import
packages/protocol (schemas)
```

### Runtime Communication (NOT import)

```
apps/desktop ↔ crates/openwhisper-native
    (via IPC - named pipes)

crates/openwhisper-native ↔ services/asr
    (via NDJSON over stdio)
```

### Forbidden Dependencies

```
apps/desktop → services/asr
(Do not import Python from Electron)

crates/openwhisper-native → apps/desktop
(Do not import Electron from Rust)

services/asr → crates/openwhisper-native
(Do not import Rust from Python)
```

---

## Configuration Files

### Root Level

| File | Purpose |
|------|---------|
| `package.json` | Bun workspaces, root scripts |
| `bun.lock` | Locked JS dependencies |
| `.gitignore` | Git exclusions |
| `README.md` | Project overview |
| `LICENSE` | License |

### App Level (apps/desktop)

| File | Purpose |
|------|---------|
| `package.json` | App dependencies |
| `tsconfig.json` | TypeScript config |
| `vite.config.ts` | Build tool config |
| `tailwind.config.js` | CSS framework config |
| `svelte.config.js` | UI framework config |
| `electron-builder.yml` | Packaging config |

### Rust Level (crates/openwhisper-native)

| File | Purpose |
|------|---------|
| `Cargo.toml` | Rust dependencies |
| `Cargo.lock` | Locked dependencies |

### Python Level (services/asr)

| File | Purpose |
|------|---------|
| `pyproject.toml` | Python dependencies, metadata |
| `uv.lock` | Locked dependencies |

---

## Adding New Code

### Adding a New UI Component

```bash
# Create in appropriate directory
touch apps/desktop/src/renderer/components/NewComponent.svelte

# Import in parent
# import NewComponent from './components/NewComponent.svelte'
```

### Adding a New Rust Module

```bash
# Create file
touch crates/openwhisper-native/src/new_module.rs

# Add to lib.rs or main.rs
# pub mod new_module;
```

### Adding a New Python Module

```bash
# Create file
touch services/asr/src/openwhisper_asr/new_module.py

# Import where needed
# from .new_module import some_function
```

---

## Success Criteria

- [ ] Structure is intuitive for new developers
- [ ] Clear separation between layers
- [ ] No circular dependencies
- [ ] Build artifacts isolated from source
- [ ] Configuration files in appropriate locations
- [ ] Documentation co-located with code or in `.plan/`

---

## Related Documents

- **Architecture**: [01-SUMMARY.md](01-SUMMARY.md) - How structure maps to architecture
- **Tech Stack**: [03-TECH-STACK.md](03-TECH-STACK.md) - Technologies in each directory
- **Phase 1**: [05-PHASE-01-REPO-SETUP.md](05-PHASE-01-REPO-SETUP.md) - Creating this structure
