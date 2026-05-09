# Verification Plan

## Context

This document defines the testing and quality assurance checklist for OpenWhisper - a Windows-first offline dictation desktop app. Verification happens throughout development (per-phase) and at completion (final QA).

**Testing Philosophy**:
- **Automate what you can**: Unit tests, protocol validation
- **Manual for UX**: UI feel, latency perception
- **Test on real hardware**: Not just development machines
- **Edge cases matter**: Errors, disconnections, low resources

---

## Test Environment Requirements

### Minimum Test Hardware

You must test on at least:

1. **Development Machine** (high-end)
   - Modern CPU + GPU
   - 16GB+ RAM
   - SSD
   - Windows 10/11

2. **Target Hardware** (mid-range)
   - Intel i5-8400 / Ryzen 5 2600
   - Target GPU class determined by Phase 3 ASR benchmarks
   - 16GB RAM
   - SSD
   - Windows 10

3. **Minimum Hardware** (low-end)
   - Intel i3-6100 / Ryzen 3 1200
   - No GPU (CPU-only)
   - 8GB RAM
   - HDD or SSD
   - Windows 10

### Test Audio Sources

- **Clean speech**: Librispeech samples or recorded WAVs
- **Noisy speech**: Record in typical office environment
- **Accented speech**: Various English accents
- **Long dictation**: 30+ minutes continuous

---

## Phase Verification

### Phase 1: Repo Setup Verification

**Automated Tests**:
```bash
# Run from repository root
bun install          # Should complete without errors
cd apps/desktop && bun dev    # Should launch Electron window
cd crates/openwhisper-native && cargo build  # Should build successfully
cd services/asr && uv run python -m openwhisper_asr --health-check  # Should respond
```

**Manual Checks**:
- [ ] Electron window appears and is interactive
- [ ] Rust binary exists at `target/debug/openwhisper-native.exe`
- [ ] Python worker prints health check response

**Success Criteria**: All automated tests pass, all manual checks verified

---

### Phase 2: Protocol Verification

**Test 1: Electron ↔ Rust Communication**
```typescript
// In Electron DevTools console
await window.api.getStatus()
// Expected: { status: "ready", version: "0.1.0" }
```

**Test 2: Rust ↔ Python Communication**
```bash
# Terminal 1: Start Rust helper
cd crates/openwhisper-native && cargo run

# Terminal 2: Send NDJSON to Python via Rust
# The current implementation uses Windows named pipes, not localhost TCP.
# Start the Electron app and call:
# await window.api.getStatus()
# Expected: status object from Rust, backed by Python health checks.
```

**Test 3: End-to-End Mock Transcript**
- Start all three layers
- Send mock `transcript.partial` from Python
- Verify it appears in Electron overlay
- Send mock `transcript.final`
- Verify final result is committed

**Success Criteria**:
- [ ] All communication paths work
- [ ] Mock transcripts reach UI
- [ ] Protocol handles malformed messages gracefully

---

### Phase 3: ASR Engine Verification ⭐ CRITICAL

This phase has a Go/No-Go decision. See [Contingency Plan](18-CONTINGENCY.md) if any test fails.

**Test 1: Model Loading**
```bash
cd services/asr
uv run python -c "
from openwhisper_asr.engine import ParakeetEngine
engine = ParakeetEngine()
engine.load()
print('Model loaded successfully')
print(f'Device: {engine.device}')
print(f'Memory: {get_memory_usage()}MB')
"
```
- [ ] Loads on GPU (if available)
- [ ] Loads on CPU
- [ ] Reports memory usage

**Test 2: Batch Transcription Accuracy**
```bash
python scripts/benchmark.py \
    --input test_data/clean_speech.wav \
    --reference test_data/clean_speech.txt \
    --mode batch
```
- [ ] WER < 12% on clean speech
- [ ] Completes without errors

**Test 3: Chunked Transcription Accuracy**
```bash
python scripts/benchmark.py \
    --input test_data/clean_speech.wav \
    --reference test_data/clean_speech.txt \
    --mode chunk-0.5s

python scripts/benchmark.py \
    --input test_data/clean_speech.wav \
    --reference test_data/clean_speech.txt \
    --mode chunk-2s

python scripts/benchmark.py \
    --input test_data/clean_speech.wav \
    --reference test_data/clean_speech.txt \
    --mode chunk-4s
```
- [ ] Chunked WER within 2% of batch WER for all chunk sizes
- [ ] 0.5s chunk WER < 15%
- [ ] 2s chunk WER < 13%

**Test 4: Latency Benchmark**
```bash
python scripts/benchmark.py \
    --input test_data/test.wav \
    --mode chunk-0.5s \
    --iterations 100 \
    --latency-profile
```
- [ ] Average latency < 500ms on target hardware
- [ ] P95 latency < 800ms
- [ ] P99 latency < 1200ms

**Test 5: Memory Usage**
```bash
python scripts/benchmark.py \
    --input test_data/long_speech.wav \
    --mode chunk-2s \
    --profile-memory
```
- [ ] Peak memory < 2GB
- [ ] No memory leaks (steady state after warmup)

**Test 6: Real-time Streaming**
```bash
python scripts/streaming_test.py --duration 60
# Speak into microphone for 60 seconds
```
- [ ] No buffer overruns
- [ ] Transcription keeps up with speech
- [ ] Quality acceptable (subjective)

**Go/No-Go Decision**:
- [ ] ALL accuracy tests pass
- [ ] ALL latency tests pass
- [ ] ALL memory tests pass
- [ ] Real-time test feels usable

**If ANY test fails**: Execute Contingency Plan (Document 18)

---

### Phase 4: Product Shell Verification

**Test 1: Settings Window**
- [ ] Opens from tray menu
- [ ] All settings sections visible
- [ ] Changes persist after restart
- [ ] Validation works (e.g., invalid hotkey shows error)

**Test 2: Tray Menu**
- [ ] Icon appears in system tray
- [ ] Right-click shows menu
- [ ] Start/Stop dictation works
- [ ] Settings opens window
- [ ] Quit exits app

**Test 3: Overlay**
- [ ] Appears when dictation starts
- [ ] Frameless, always-on-top
- [ ] Does NOT steal focus from other apps
- [ ] Shows mock transcript updates
- [ ] Shows recording state
- [ ] Can be disabled in settings

**Test 4: Onboarding**
- [ ] Shows on first run
- [ ] Model download progress visible
- [ ] Can be skipped and resumed later
- [ ] Completes and shows main UI

**Success Criteria**: All UI components work, settings persist, no crashes

---

### Phase 5: Rust Helper Verification

**Test 1: Global Hotkey**
- Open Notepad
- Set hotkey to Ctrl+Shift+D in settings
- Press hotkey while Notepad is focused
- [ ] Dictation starts (overlay appears)
- Press hotkey again
- [ ] Dictation stops

**Test 2: Text Injection**
```rust
// Test injection into various apps
let test_cases = vec![
    ("notepad.exe", "Hello World"),
    ("chrome.exe", "Testing 123"),  // Address bar
    ("code.exe", "function test()"), // VS Code
];
```
- [ ] Injects into Notepad
- [ ] Injects into browser address bar
- [ ] Injects into code editors
- [ ] Respects target app input restrictions

**Test 3: Clipboard Fallback**
- [ ] Falls back to clipboard for long text (>10 chars by default)
- [ ] Falls back when injection blocked
- [ ] Restores previous clipboard content
- [ ] User sees fallback notification

**Test 4: Elevation Detection**
- Run Notepad as Administrator
- Run OpenWhisper as normal user
- Try to inject text
- [ ] Reports elevation limitation
- [ ] Falls back to clipboard

**Test 5: Worker Supervision**
- Kill Python worker process manually
- [ ] Rust detects crash
- [ ] Attempts restart (up to 3 times)
- [ ] Reports status to Electron

**Success Criteria**: All tests pass on target hardware

---

### Phase 6: Audio Pipeline Verification

**Test 1: Audio Capture**
- [ ] Captures from default microphone
- [ ] Audio is 16kHz mono float32
- [ ] No distortion or artifacts
- [ ] Volume levels reasonable

**Test 2: Buffer Management**
```bash
# Run for 30 minutes
python scripts/audio_stress_test.py --duration 1800
```
- [ ] No buffer overruns
- [ ] No memory growth >10%
- [ ] Consistent latency

**Test 3: VAD (Voice Activity Detection)**
- Start dictation
- [ ] Speak: transcription starts
- [ ] Stop speaking: transcription pauses after delay
- [ ] Resume speaking: transcription resumes

**Test 4: Device Changes**
- Start dictation
- Unplug microphone
- [ ] Detects disconnection gracefully
- [ ] Shows error in UI
- Plug microphone back in
- [ ] Resumes automatically or offers retry

**Test 5: Multiple Devices**
- [ ] Can select audio device in settings
- [ ] Change takes effect immediately
- [ ] Remembers selection across restarts

**Success Criteria**: Audio works reliably, handles device changes gracefully

---

### Phase 7: End-to-End Verification

**The Ultimate Test**: Full dictation loop

**Scenario 1: Happy Path**
1. Open Notepad
2. Press hotkey
3. Speak: "Hello world, this is a test of OpenWhisper"
4. Press hotkey to stop

Expected:
- [ ] Overlay appeared on hotkey
- [ ] Partial results appeared while speaking
- [ ] Final text appeared in Notepad: "Hello world, this is a test of OpenWhisper"
- [ ] Overlay closed or returned to idle

**Scenario 2: Long Dictation**
1. Dictate continuously for 5 minutes

Expected:
- [ ] No crashes
- [ ] Memory stable
- [ ] Quality consistent
- [ ] All text appears

**Scenario 3: Rapid Toggle**
1. Press hotkey 10 times rapidly

Expected:
- [ ] No crashes
- [ ] State remains consistent
- [ ] No resource leaks

**Scenario 4: Error Recovery**
1. Start dictation
2. Unplug microphone
3. Plug microphone back in
4. Continue dictation

Expected:
- [ ] Error shown when unplugged
- [ ] Recovery when plugged back in
- [ ] Can continue dictation

**Success Criteria**: All scenarios pass on minimum, target, and development hardware

---

### Phase 8: Performance Verification

**Test 1: Latency Benchmarks**
```bash
# Run full pipeline benchmarks
scripts/benchmark_e2e.py --mode fast
scripts/benchmark_e2e.py --mode balanced
scripts/benchmark_e2e.py --mode accurate
```

Results must match [Performance Budget](17-PERFORMANCE-BUDGET.md):
- [ ] Fast mode: <500ms average perceived latency
- [ ] Balanced mode: <1000ms average
- [ ] Accurate mode: <1500ms average

**Test 2: Memory Profiling**
```bash
scripts/profile_memory.py --duration 3600
```
- [ ] Peak memory <2GB
- [ ] No memory leaks over 1 hour

**Test 3: Stress Test**
```bash
scripts/stress_test.py --duration 1800 --toggles 50
```
- [ ] 30 minutes continuous operation
- [ ] 50 start/stop cycles
- [ ] No crashes or resource exhaustion

**Test 4: Recovery Test**
1. Kill Python worker 3 times during dictation
2. [ ] Recovers each time
3. [ ] Reports recovery to user
4. [ ] Quality maintained

**Success Criteria**: Performance targets met, stable under stress

---

### Phase 9: Packaging Verification

**Test 1: Clean Install**
1. Fresh Windows VM or machine
2. Download installer
3. Run installer
4. Launch app

Expected:
- [ ] Installs without errors
- [ ] No admin required (per-user install)
- [ ] Shortcuts created
- [ ] App launches

**Test 2: First Run Experience**
1. Launch app on fresh install
2. [ ] Onboarding appears
3. [ ] Model downloads automatically
4. [ ] Progress shown
5. [ ] Can skip and resume later
6. [ ] Dictation works after download

**Test 3: No Python Required**
1. Uninstall any system Python
2. Launch OpenWhisper
3. [ ] App works (bundled Python used)

**Test 4: Uninstall**
1. Uninstall app via Windows Settings
2. [ ] App removed
3. [ ] User data preserved (settings, models)
4. [ ] Can reinstall and resume

**Test 5: Update Scenario**
1. Install v1.0
2. Use app (create settings, download model)
3. Install v1.1 (update)
4. [ ] Settings preserved
5. [ ] Model not re-downloaded
6. [ ] App works

**Success Criteria**: All installation scenarios work cleanly

---

## Final QA Checklist

Before release, verify ALL of the following:

### Functionality
- [ ] Dictation works in 10+ common apps (Notepad, Word, Chrome, VS Code, etc.)
- [ ] All three modes work (Fast, Balanced, Accurate)
- [ ] Settings persist correctly
- [ ] Hotkey can be changed and works
- [ ] Language selection works (if multi-language supported)

### Error Handling
- [ ] No microphone: Clear error message
- [ ] No disk space: Clear error, offer cleanup
- [ ] GPU OOM: Falls back to CPU gracefully
- [ ] Worker crash: Auto-recovers
- [ ] Audio disconnect: Handles gracefully

### Performance
- [ ] Latency targets met on all test hardware
- [ ] Memory targets met
- [ ] No UI freezing during dictation
- [ ] App remains responsive

### Edge Cases
- [ ] Very long text (>1000 words)
- [ ] Very short text (single word)
- [ ] Numbers and special characters
- [ ] Non-English words (if supported)
- [ ] Rapid start/stop (20+ times)
- [ ] 30+ minute continuous dictation

### Accessibility
- [ ] High contrast mode works
- [ ] Keyboard navigation works
- [ ] Screen reader compatible (labels, roles)

### Security
- [ ] No hardcoded secrets
- [ ] Logs don't contain sensitive data
- [ ] Model download uses HTTPS
- [ ] Installer properly signed (before public release)

---

## Regression Testing

After any significant change, run:

1. **Smoke Test** (5 minutes):
   - Can start app
   - Can dictate in Notepad
   - Can change settings

2. **Core Test** (30 minutes):
   - All Phase 7 scenarios
   - All Phase 8 benchmarks

3. **Full Test** (2 hours):
   - All verification items
   - All edge cases
   - All hardware configurations

---

## Bug Reporting Template

When filing bugs, include:

```markdown
**Environment**:
- OpenWhisper version: 
- Windows version:
- Hardware (CPU/GPU/RAM):
- Audio device:

**Steps to Reproduce**:
1. 
2. 
3. 

**Expected Behavior**:

**Actual Behavior**:

**Logs**:
```
%APPDATA%\OpenWhisper\logs\openwhisper.log
```

**Screenshots/Videos**:

**Additional Context**:
```

---

## Success Criteria Summary

| Phase | Key Metric |
|-------|-----------|
| 1 | All layers build and run |
| 2 | All communication paths work |
| 3 | ASR meets accuracy/latency/memory targets |
| 4 | All UI components functional |
| 5 | Hotkey and injection work |
| 6 | Audio capture and VAD work |
| 7 | Full dictation loop works |
| 8 | Performance targets met |
| 9 | Installer works on clean Windows |

---

## Related Documents

- **Error Handling**: [Document 16](16-ERROR-HANDLING.md) - Error scenarios to test
- **Performance Budget**: [Document 17](17-PERFORMANCE-BUDGET.md) - Performance targets
- **Contingency Plan**: [Document 18](18-CONTINGENCY.md) - Alternative to test if Phase 3 fails
- **All Phases**: Documents 05-13 for phase-specific details
