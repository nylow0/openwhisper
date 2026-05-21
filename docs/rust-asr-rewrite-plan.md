# Rust ASR Rewrite Plan for OpenWhisper

## Goal

Replace the Python ASR runtime with a production-grade Rust ASR subsystem while preserving the OpenWhisper product UX and protocol behavior, then progressively improve latency, stability, packaging, and streaming quality.

## North-star outcomes

1. **Protocol compatibility** with current desktop integration (no UI regression during migration).
2. **Performance gains** for streaming and dictation latency on Windows-first deployments.
3. **Operational simplicity** by removing Python runtime from production path.
4. **Feature parity first**, then iterative enhancements (VAD, partials, confidence, diarization-ready hooks).

---

## Reference repo synthesis (what to borrow)

### 1) RustAudio/cpal
**Borrow:**
- Device discovery and selection patterns.
- Callback-driven, low-latency input stream model.
- Cross-platform sample-format conversion strategy.

**How we apply in OpenWhisper:**
- Build a dedicated `audio_capture` module using CPAL as foundational input layer.
- Standardize internal PCM format to `f32 mono 16kHz` before handing to ASR pipeline.
- Add hot-swap behavior for input device changes.

### 2) CPAL record WAV example
**Borrow:**
- Minimal capture-to-buffer pipeline.
- Practical handling for stream lifecycle and buffering.

**How we apply in OpenWhisper:**
- Create reference test harness to record deterministic clips for regression tests.
- Use it as the baseline for ASR capture diagnostics mode.

### 3) ruuda/hound
**Borrow:**
- Simple, stable WAV writer/reader utilities.

**How we apply in OpenWhisper:**
- Add optional “save debug audio” path for bug reports.
- Build golden test vectors and replayable integration tests.

### 4) tazz4843/whisper-rs
**Borrow:**
- Core Whisper binding patterns in Rust.
- Model loading lifecycle, context/session management.

**How we apply in OpenWhisper:**
- Build `whisper_engine` abstraction around whisper-rs first (fastest path to parity).
- Mirror existing model/profile options and runtime flags.

### 5) whisper-cpp-plus-rs
**Borrow:**
- Safer wrapper patterns and potentially stream/VAD-friendly interfaces.

**How we apply in OpenWhisper:**
- Evaluate for stricter safety ergonomics and maintainability.
- Use as alternative backend strategy if whisper-rs gaps appear.

### 6) whisper-stream-rs
**Borrow:**
- Streaming orchestration ideas: chunking, partial emission, windowing cadence.

**How we apply in OpenWhisper:**
- Implement real-time “partial transcript” channel and final-on-silence mode.
- Tune `step_ms/length_ms/keep_ms` presets for dictation UX.

### 7) Whisper Real-Time Transcription style projects
**Borrow:**
- End-to-end realtime pipeline shape and user-facing behavior conventions.

**How we apply in OpenWhisper:**
- Improve UX logic (debounce/finalization) and event semantics.
- Adopt practical VAD tuning workflow and field diagnostics.

---

## Target architecture (Rust-first ASR)

## Workspace additions

- New crate: `crates/openwhisper-asr-rs`
- Optional transitional binary: `ow-asr-rs` (CLI + worker mode)

## Core modules

1. `audio_capture`
   - CPAL input stream
   - device list/select
   - resample + channel mixdown
   - frame timestamps

2. `vad`
   - start with energy/spectral heuristic
   - pluggable interface for future stronger VAD

3. `buffering`
   - ring buffer
   - sliding window extractor
   - keep/step/length policy

4. `engine`
   - `AsrEngine` trait
   - `WhisperRsEngine` implementation
   - language configuration + decoding options

5. `streaming`
   - orchestrator loop
   - partial and final event emission
   - silence finalize and context retention strategy

6. `protocol`
   - NDJSON worker protocol compatible with current app expectations
   - health, start, stop, transcribe, diagnostics commands

7. `profiles`
   - migrate `whispercpp-profiles.json` semantics to Rust config loading
   - compatibility shim for model aliases and devices

8. `observability`
   - structured logs
   - timings: capture latency, decode latency, e2e latency

---

## Migration plan (detailed)

### Phase 0 — Discovery and locking contracts (2-3 days)
- Freeze current Python worker command/event contract.
- Capture real desktop traffic traces for “golden protocol sessions”.
- Define parity checklist:
  - command semantics
  - error behavior
  - startup health semantics

**Exit criteria:** signed-off protocol contract doc and replay fixtures.

### Phase 1 — Rust worker skeleton with protocol parity (3-4 days)
- Scaffold `openwhisper-asr-rs` crate.
- Implement NDJSON stdio worker loop and health endpoints.
- Add no-op/mock transcribe path for plumbing.

**Exit criteria:** desktop app can talk to Rust worker in mock mode.

### Phase 2 — Batch transcription parity (4-6 days)
- Integrate whisper-rs backend.
- Support model path/profile selection and language flags.
- Implement file transcription command parity.

**Exit criteria:** offline file transcribe outputs match baseline quality envelope.

### Phase 3 — Live audio capture foundation (4-6 days)
- CPAL device listing and default capture path.
- Robust sample-format conversion to internal PCM.
- Record diagnostics mode (WAV output via hound).

**Exit criteria:** stable long-running capture sessions on Windows test matrix.

### Phase 4 — Streaming dictation engine (5-8 days)
- Ring buffer + window chunk policy (`step/length/keep`).
- Partial transcript emission schedule.
- Finalization logic and punctuation post-processing hooks.

**Exit criteria:** stable live dictation with visible partials and finals.

### Phase 5 — VAD-driven mode (4-6 days)
- Implement baseline VAD trigger.
- Silence-based finalize flow.
- Configurable VAD threshold per profile.

**Exit criteria:** final-on-silence mode behaves consistently in noisy rooms.

### Phase 6 — Performance and reliability pass (5-7 days)
- Threading and queue backpressure tuning.
- Memory profiling and ring buffer bounds hardening.
- Crash recovery and timeout safety checks.

**Exit criteria:** latency and stability targets met on representative machines.

### Phase 7 — App integration and switchover (3-5 days)
- Add runtime flag in desktop app to pick Python vs Rust worker.
- Canary rollout path; fallback switch preserved.
- Telemetry comparison (latency, failure rate).

**Exit criteria:** Rust worker can be default with fallback retained.

### Phase 8 — Decommission Python runtime (optional final step)
- Remove Python ASR from production path after sustained stability window.
- Keep compatibility shim/adapter scripts for dev only if needed.

**Exit criteria:** production packaging no longer requires Python.

---

## Engineering standards for the rewrite

1. **Parity-first rule:** do not add fancy features until baseline behavior is equivalent.
2. **Replay-driven validation:** every protocol/behavior change validated on golden fixtures.
3. **Single internal audio format:** normalize early, simplify downstream logic.
4. **Hard real-time boundaries:** bounded queues, no unbounded growth.
5. **Feature flags for risk isolation:** streaming/VAD toggles independently deployable.

---

## Testing strategy

## Unit tests
- PCM conversion correctness
- ring buffer/window extraction
- VAD threshold behavior
- profile parsing and model alias resolution

## Integration tests
- NDJSON protocol session replays
- file transcription deterministic sanity checks
- streaming state machine scenarios (speech/silence alternation)

## System tests
- 30+ minute continuous dictation soak
- device unplug/replug recovery
- startup/shutdown race tests

## Performance benchmarks
- first partial latency
- final latency after silence
- realtime factor (RTF)
- memory footprint and CPU usage

---

## Risks and mitigations

1. **Binding instability / upstream changes**
   - Pin versions, wrap backend behind trait, keep adapter boundary thin.

2. **Windows audio edge cases**
   - Prioritize WASAPI paths and broad device matrix early.

3. **Quality drift vs Python path**
   - Golden set comparisons and configurable decode params by profile.

4. **Migration churn in desktop integration**
   - Preserve protocol compatibility and provide runtime fallback switch.

---

## Initial execution backlog (first sprint)

1. Create `openwhisper-asr-rs` crate and CI wiring.
2. Implement protocol mock worker and health checks.
3. Add whisper-rs batch transcription command.
4. Implement CPAL capture + WAV diagnostics.
5. Build ring buffer + simple streaming partials.
6. Add baseline VAD finalize-on-silence mode.

---

## Decision gates

- **Gate A (after Phase 2):** Is batch parity acceptable? If no, halt and resolve backend/config gaps.
- **Gate B (after Phase 4):** Is live latency materially better than Python path? If no, optimize before broader rollout.
- **Gate C (after Phase 7):** Is crash/failure rate equal or lower? If yes, promote Rust worker as default.

