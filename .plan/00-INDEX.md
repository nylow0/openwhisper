# OpenWhisper Plan Index

## Mission

Build OpenWhisper: a Windows-first offline dictation desktop application using a hybrid Electron/Svelte, Rust, and Python architecture.

**Core Principle**: Electron owns the product experience, Rust owns Windows integration, Python owns the model runtime.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  Electron + Svelte + TypeScript                                  │
│  - UI, tray, overlay, settings, onboarding, app lifecycle        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Local IPC (named pipes/sockets)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Rust Native Helper                                              │
│  - Hotkeys, text injection, clipboard fallback                  │
│  - Active window checks, process supervision                    │
│  - Audio capture (Phase 6+ via WASAPI)                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ NDJSON over stdio
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Python ASR Worker                                               │
│  - Model loading (Parakeet/NeMo)                                 │
│  - Chunked inference, VAD, timestamps                           │
└─────────────────────────────────────────────────────────────────┘
```

**Key Design Decisions**:
- Offline-first: No internet required after setup
- Model downloaded on first run (not bundled)
- Validate ASR early (Phase 3) before heavy UI investment
- Contingency plan ready if Parakeet fails
- Parakeet is a candidate, not a commitment, until Phase 3 passes measured gates
- Production audio belongs in Rust WASAPI; Python `sounddevice` is only for disposable spikes

---

## Document Navigation

### Foundation Documents (Read First)

| # | Document | Purpose | Status |
|---|----------|---------|--------|
| 01 | [Summary & Architecture](01-SUMMARY.md) | Core principles & system design | Reviewed |
| 02 | [Product Experience](02-PRODUCT-EXPERIENCE.md) | User flow & interaction model | Reviewed |
| 03 | [Tech Stack](03-TECH-STACK.md) | Languages, frameworks, tools | Corrected |
| 04 | [Project Structure](04-PROJECT-STRUCTURE.md) | Monorepo directory layout | Corrected |

### Implementation Phases (Execute in Order)

| # | Document | Goal | Dependencies |
|---|----------|------|--------------|
| 05 | [Phase 1: Repo Setup](05-PHASE-01-REPO-SETUP.md) | Initialize monorepo with all 3 layers | Complete |
| 06 | [Phase 2: Protocol](06-PHASE-02-PROTOCOL.md) | IPC contracts between layers | Complete enough for Phase 3 |
| 07 | [Phase 3: ASR Engine](07-PHASE-03-ASR-ENGINE.md) | Validate Parakeet or choose fallback | Current phase |
| 08 | [Phase 4: Product Shell](08-PHASE-04-PRODUCT-SHELL.md) | Build Electron/Svelte UI | Phase 3 |
| 09 | [Phase 5: Rust Helper](09-PHASE-05-RUST-HELPER.md) | Native Windows integration | Phase 4 |
| 10 | [Phase 6: Audio Pipeline](10-PHASE-06-AUDIO.md) | Audio capture & streaming | Phase 5 |
| 11 | [Phase 7: End-to-End](11-PHASE-07-END-TO-END.md) | Full dictation loop | Phase 6 |
| 12 | [Phase 8: Performance](12-PHASE-08-PERFORMANCE.md) | Optimization & stability | Phase 7 |
| 13 | [Phase 9: Packaging](13-PHASE-09-PACKAGING.md) | Distribution & installer | Phase 8 |

### Support Documents (Reference as Needed)

| # | Document | Purpose | When to Read |
|---|----------|---------|--------------|
| 14 | [Verification Plan](14-VERIFICATION.md) | Testing & QA checklist | Before each phase |
| 15 | [Assumptions & Defaults](15-ASSUMPTIONS.md) | Constraints & default behaviors | Before implementation |
| 16 | [Error Handling](16-ERROR-HANDLING.md) | Error recovery & diagnostics | Phase 2, 5, 7, 8 |
| 17 | [Performance Budget](17-PERFORMANCE-BUDGET.md) | Latency targets & metrics | Phase 3, 6, 8 |
| 18 | [Contingency Plan](18-CONTINGENCY.md) | Plan B if Parakeet fails | Phase 3 (critical) |
| 19 | [Plan Review](19-PLAN-REVIEW.md) | Direction review and corrections | Before delegating new work |

---

## Reading Order

### For New Contributors

1. Read **01-Summary** and **02-Product-Experience** for context
2. Read **19-Plan-Review** for the current corrected direction
3. Review **03-Tech-Stack** and **04-Project-Structure** for technical setup
4. Follow phases sequentially from **Phase 1** to **Phase 9**
5. Reference **14-Verification** before marking any phase complete
6. Read **16-Error-Handling** before Phase 5 (Rust Helper)
7. Read **17-Performance-Budget** before Phase 3 (ASR validation)
8. Read **18-Contingency** before Phase 3 (critical decision point)

### For Implementation

Each Phase document contains:
- **Goal**: What this phase achieves
- **Tasks**: Specific work items with checkboxes
- **Defaults**: Configuration and behavior defaults
- **Success Criteria**: Objective completion criteria
- **Related**: Links to prerequisite and next phases

**Important**: Phases are sequential. Do not start Phase N+1 until Phase N success criteria are met.

**Current exception**: The repo already contains some Phase 5-shaped Rust supervision and IPC work. Treat that as foundation already built, not permission to skip the ASR gate.

---

## Critical Path

```
Phase 1 (Setup) 
    → Phase 2 (Protocol)
        → Phase 3 (ASR Validation) ← CRITICAL DECISION POINT
            → [If Parakeet fails: execute Contingency Plan]
            → Phase 4 (UI)
                → Phase 5 (Rust)
                    → Phase 6 (Audio)
                        → Phase 7 (Integration)
                            → Phase 8 (Optimization)
                                → Phase 9 (Packaging)
```

**Phase 3 is the major technical risk**. If Parakeet streaming is unacceptable, stop and execute Contingency Plan (Document 18) before continuing.

---

## Status Tracking

Update status as you complete phases:

```markdown
| Document | Status |
|----------|--------|
| Phase 1 | Complete |
| Phase 2 | Complete enough for Phase 3 |
| Phase 3 | Current |
| Phase 4+ | Blocked until Phase 3 Go decision |
```

Current Status: **Phase 3 ASR validation is the next real work.**

---

## Quick Reference

### Default Settings

| Setting | Default Value |
|---------|--------------|
| Audio chunk size | 2s (Balanced) |
| Injection method | Auto (clipboard >10 chars) |
| Sample rate | 16kHz |
| GPU mode | Auto-detect |
| Model storage | `%LOCALAPPDATA%\OpenWhisper\models` |
| Config storage | `%APPDATA%\OpenWhisper\config.json` |

### Key Constraints

- Windows 10+ only (v1)
- Python version dictated by the selected ASR stack; validate and lock in Phase 3
- Offline operation (no cloud required)
- Target latency: <500ms (Fast mode)
- Memory target: <2GB
- Installed-size target is now a measured gate, not a promised 2.1GB number

### Important Files

```
apps/desktop/              # Electron app
crates/openwhisper-native/ # Rust helper
services/asr/              # Python ASR worker
packages/protocol/         # Shared schemas
```

---

## Support

- Review [Assumptions](15-ASSUMPTIONS.md) when making decisions
- Check [Error Handling](16-ERROR-HANDLING.md) when implementing recovery
- Reference [Performance Budget](17-PERFORMANCE-BUDGET.md) when optimizing
- Consult [Contingency Plan](18-CONTINGENCY.md) if ASR validation fails

---

**Last Updated**: 2026-05-09 plan review
**Next Review**: After Phase 3 Go/No-Go decision
