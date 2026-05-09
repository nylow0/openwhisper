# Phase 8: Performance and Reliability

## Context

This is **Phase 8** of 9 in building OpenWhisper, a Windows-first offline dictation desktop app. At this point, we have a working end-to-end prototype (completed in Phase 7). This phase makes it fast and stable enough for daily use.

**Purpose**: Optimize for speed, reduce resource usage, and ensure the app can run for hours without issues. This is where we move from "it works" to "it works well."

---

## Prerequisites

- [ ] Phase 1-7 complete
- [ ] End-to-end dictation loop works
- [ ] Read [Performance Budget](17-PERFORMANCE-BUDGET.md) for targets
- [ ] Read [Error Handling](16-ERROR-HANDLING.md) for recovery strategies
- [ ] Profiling tools available (can use simple timing logs)

---

## Goal

Make OpenWhisper feel fast, responsive, and reliable for daily use.

**Quantitative targets** (from Performance Budget):
- Perceived latency <500ms in Fast mode
- Perceived latency <1000ms in Balanced mode  
- Memory usage preferably <2GB peak, with hard gates inherited from Phase 3/15 assumptions
- No crashes during 1-hour continuous use
- Graceful recovery from all recoverable errors

---

## Optimization Strategies

### 1. PyTorch Optimizations

**Use inference mode** (disables gradient computation):
```python
# services/asr/src/openwhisper_asr/engine.py
import torch

def transcribe_chunk(self, audio):
    with torch.inference_mode():
        # 2-3x faster than no_grad for inference
        result = self.model(audio)
    return result
```

**Use CUDA float16** (where supported):
```python
if self.device == "cuda":
    self.model = self.model.to(dtype=torch.float16)
    audio = audio.to(dtype=torch.float16)
```

**Keep CPU float32** (for compatibility):
- Always maintain working CPU path
- Some CPUs don't support float16 efficiently
- Test on minimum hardware

### 2. Model Lifecycle

**Reuse model instance**:
- Load once at startup, keep in memory
- Don't reload between chunks
- Pre-allocate tensors where possible

**Warm up on load**:
```python
def warmup(self):
    """Process silence to warm up caches."""
    silence = torch.zeros(self.sample_rate, dtype=torch.float32)
    with torch.inference_mode():
        _ = self.model.transcribe([silence])
```

**Optional: Auto-unload after idle**:
- Unload model after 30 minutes of no dictation
- Only if memory pressure detected
- Must reload quickly when dictation resumes

### 3. Timing Instrumentation

Measure every stage to identify bottlenecks:

```python
class TimingContext:
    def __init__(self, name: str):
        self.name = name
        self.start = time.perf_counter()
    
    def __enter__(self):
        return self
    
    def __exit__(self, *args):
        elapsed = (time.perf_counter() - self.start) * 1000
        logger.debug(f"[TIMING] {self.name}: {elapsed:.1f}ms")

# Usage:
with TimingContext("preprocessing"):
    audio = preprocess(raw_audio)

with TimingContext("inference"):
    result = model.transcribe(audio)
```

**Metrics to track**:
- Audio buffer wait time
- Preprocessing time
- Model inference time
- Post-processing (decode) time
- IPC latency (Rust ↔ Python)
- Text injection time
- Total perceived latency

### 4. Adaptive Chunk Sizing

Automatically adjust chunk size based on measured performance:

```python
class AdaptiveChunkManager:
    def __init__(self):
        self.target_latency_ms = 500
        self.current_chunk_ms = 2000  # Start with Balanced
        self.latency_history = deque(maxlen=10)
    
    def report_latency(self, latency_ms: float):
        self.latency_history.append(latency_ms)
        avg_latency = sum(self.latency_history) / len(self.latency_history)
        
        # If consistently slow, increase chunk size
        if avg_latency > self.target_latency_ms * 1.5:
            if self.current_chunk_ms < 4000:
                self.current_chunk_ms += 500
                logger.info(f"Increasing chunk size to {self.current_chunk_ms}ms")
        
        # If consistently fast, could decrease (optional)
        elif avg_latency < self.target_latency_ms * 0.5:
            if self.current_chunk_ms > 500:
                self.current_chunk_ms -= 500
                logger.info(f"Decreasing chunk size to {self.current_chunk_ms}ms")
```

### 5. Audio Pipeline Optimizations

**Minimize buffer copies**:
- Use ring buffer with zero-copy where possible
- Pre-allocate buffers, reuse them
- Avoid unnecessary format conversions

**Efficient resampling**:
- Use high-quality but fast resampler (e.g., libsamplerate or rubato)
- Cache resampler state between chunks
- Only resample if device rate ≠ 16kHz

**VAD tuning**:
- Run VAD on downsampled audio (8kHz is sufficient)
- Tune thresholds for your use case
- Consider hardware VAD if available (some mics provide it)

### 6. UI Responsiveness

**Don't block the main thread**:
- All audio processing in separate thread/process
- IPC messages handled asynchronously
- UI updates via message passing, not direct calls

**Debounced updates**:
- For partial results, don't update UI every frame
- Throttle to 10-15 updates per second max
- User can't perceive faster anyway

**Lazy loading**:
- Load heavy UI components (settings panels) only when needed
- Keep overlay minimal

---

## Reliability Work

### 1. Worker Crash Recovery

From [Error Handling](16-ERROR-HANDLING.md):

```rust
// Rust helper
fn handle_worker_exit(&mut self, exit_code: i32) {
    log::error!("Python worker exited with code {}", exit_code);
    
    if self.restart_count < MAX_RESTARTS {
        log::info!("Attempting restart {}/{}", self.restart_count + 1, MAX_RESTARTS);
        
        // Exponential backoff
        let delay = Duration::from_secs(2_u64.pow(self.restart_count));
        thread::sleep(delay);
        
        match self.restart_worker() {
            Ok(()) => {
                self.restart_count += 1;
                self.notify_electron("worker.recovered");
            }
            Err(e) => {
                self.notify_electron("worker.restart_failed", e);
            }
        }
    } else {
        self.notify_electron("worker.max_restarts");
        self.pause_dictation();
    }
}
```

### 2. Audio Device Recovery

```rust
fn handle_audio_error(&mut self, error: AudioError) {
    match error {
        AudioError::DeviceDisconnected => {
            self.notify_electron("audio.device_disconnected");
            self.pause_dictation();
            
            // Start monitoring for device reconnect
            self.start_device_monitor();
        }
        AudioError::DeviceInUse => {
            // Retry with backoff
            self.retry_with_backoff();
        }
        AudioError::BufferOverrun => {
            log::warn!("Audio buffer overrun - processing too slow");
            // Could increase chunk size or warn user
        }
    }
}
```

### 3. Graceful Degradation

**When things go wrong, don't crash**:
- GPU OOM → Fall back to CPU
- Fast mode too slow → Auto-switch to Balanced
- Audio device fails → Pause, don't crash
- Model inference timeout → Abort chunk, continue

### 4. Resource Cleanup

**Always clean up**:
```rust
impl Drop for PythonWorker {
    fn drop(&mut self) {
        // Send shutdown message
        let _ = self.send(r#"{"type": "shutdown"}"#);
        
        // Wait for graceful exit (with timeout)
        let _ = self.process.wait_timeout(Duration::from_secs(5));
        
        // Force kill if necessary
        if self.process.try_wait().is_none() {
            let _ = self.process.kill();
        }
    }
}
```

### 5. Logging for Diagnostics

From [Error Handling](16-ERROR-HANDLING.md):

```rust
// Always log:
// - App start/stop
// - Dictation start/stop
// - Model load/unload
// - Worker restart
// - Audio device changes
// - Errors with full context

log::info!("Dictation started");
log::warn!("Worker crashed, restarting");
log::error!("Failed to load model: {}", e);

// Include context:
log::error!(
    "Injection failed: target={}, error={}",
    window_info.process_name,
    e
);
```

---

## Targets (From Performance Budget)

### Latency Targets

| Mode | Chunk Size | Target | Maximum |
|------|-----------|--------|---------|
| Fast | 0.5s | <500ms | <800ms |
| Balanced | 2s | <1000ms | <1500ms |
| Accurate | 4s | <1500ms | <2500ms |

**Measurement**: Time from speech end to text appearing in target app.

### Memory Targets

| Component | Target | Maximum |
|-----------|--------|---------|
| Electron Main | 150MB | 300MB |
| Electron Renderer | 100MB | 200MB |
| Rust Helper | 50MB | 100MB |
| Python + selected ASR | 2000MB | 3000MB |
| Audio Buffers | 50MB | 100MB |
| **Total** | **1550MB** | **2700MB** |

### Stability Targets

- **Uptime**: 99.9% (less than 1 crash per 1000 hours)
- **Recovery time**: <5 seconds after recoverable error
- **Graceful degradation**: All features work in CPU-only mode (slower)

---

## Benchmarking

### Development Benchmarks

Run after any optimization:

```bash
# Single-file benchmark
cd services/asr
uv run python scripts/benchmark.py \
    --input test.wav \
    --mode chunk-2s \
    --iterations 50

# Full pipeline benchmark
scripts/benchmark_e2e.py --duration 300 --report
```

### Performance Regression Test

```bash
# Before release, run full suite
cd scripts/performance
./run_all_benchmarks.sh

# Generates report:
# - Latency distribution
# - Memory usage over time
# - Crash/Recovery stats
# - Comparison to baseline
```

---

## Success Criteria

- [ ] Latency benchmarks for each mode (Fast, Balanced, Accurate) meet targets
- [ ] Recovery from simulated worker crash handled gracefully (auto-restart works)
- [ ] Recovery from simulated audio disconnect handled gracefully
- [ ] UI shows clear, actionable error messages
- [ ] Logs are persisted and readable in standard format
- [ ] Memory usage logged and reviewed - stays under target
- [ ] 30-minute continuous dictation test passes without issues
- [ ] Rapid start/stop test (20 cycles) passes without resource leaks
- [ ] CPU-only mode works (slower but functional)
- [ ] GPU fallback to CPU works when OOM

---

## Performance Checklist

Before considering this phase complete:

### PyTorch
- [ ] Using `inference_mode()`, not just `no_grad`
- [ ] Float16 on CUDA where supported
- [ ] Model warmed up on load
- [ ] Model instance reused (not reloaded)

### Audio
- [ ] Minimal buffer copies
- [ ] Efficient resampling
- [ ] VAD tuned appropriately
- [ ] No buffer overruns under normal use

### IPC
- [ ] Asynchronous message handling
- [ ] No blocking calls on main threads
- [ ] Message batching where appropriate

### UI
- [ ] Debounced updates
- [ ] No main thread blocking
- [ ] Lazy loading of heavy components

### Error Handling
- [ ] All errors logged with context
- [ ] Recovery strategies tested
- [ ] Graceful degradation verified
- [ ] Resource cleanup on exit

---

## Related Documents

- **Performance Budget**: [Document 17](17-PERFORMANCE-BUDGET.md) - Detailed latency and memory budgets
- **Error Handling**: [Document 16](16-ERROR-HANDLING.md) - Recovery strategies and error scenarios
- **ASR Engine**: [Phase 3](07-PHASE-03-ASR-ENGINE.md) - Model optimization details
- **Audio Pipeline**: [Phase 6](10-PHASE-06-AUDIO.md) - Audio optimization details
- **Previous Phase**: [Phase 7](11-PHASE-07-END-TO-END.md) - Working baseline
- **Next Phase**: [Phase 9](13-PHASE-09-PACKAGING.md) - Distribution

---

## Common Pitfalls to Avoid

1. **Don't optimize too early** - Measure first, then optimize
2. **Don't sacrifice reliability for speed** - Recovery is more important than speed
3. **Test on minimum hardware** - Your dev machine is probably faster than users'
4. **Profile in release mode** - Debug builds are much slower
5. **Watch memory in long tests** - Leaks show up over time, not immediately
