# Performance Budget & Latency Targets

## Context

This document defines the performance requirements and latency budget for OpenWhisper, a Windows-first offline dictation desktop app. Performance is critical for a good user experience - dictation should feel instantaneous. This budget breaks down the maximum acceptable time for each stage of the pipeline.

**Review note 2026-05-09**: These are product targets, not proven measurements. Phase 3 must replace example ASR numbers with real benchmark results for the selected model/runtime.

---

## Perceived Latency Definition

**Perceived latency** = Time from user stopping speech to text appearing in target application.

This is what users actually feel. It's different from processing latency because it includes:
- Time for the audio buffer to fill
- Time to detect speech end (VAD)
- Time to process final chunk
- Time to inject text

---

## Latency Budget Breakdown

### Target: 500ms Perceived Latency (Fast Mode)

| Stage | Budget | Description | Measurement Point |
|-------|--------|-------------|-------------------|
| Audio Buffer | 100ms | 0.5s chunk @ 20% overlap | First byte in buffer |
| Audio Preprocessing | 20ms | Resample, normalize | After preprocessing |
| VAD (if enabled) | 30ms | Voice activity detection | After VAD decision |
| Network/IPC | 10ms | Rust -> Python NDJSON | Message sent |
| Model Inference | 250ms | Parakeet forward pass | Inference complete |
| Post-processing | 20ms | Decode, format result | Result ready |
| Text Injection | 50ms | SendInput or clipboard | Text in target app |
| **Total Budget** | **480ms** | | |
| **Buffer** | **20ms** | For variability | |
| **Target** | **500ms** | | |

### Balanced Mode (2s chunks)

| Stage | Budget | Notes |
|-------|--------|-------|
| Audio Buffer | 400ms | 2s chunk @ 20% overlap |
| Audio Preprocessing | 20ms | |
| VAD | 30ms | |
| Network/IPC | 10ms | |
| Model Inference | 300ms | Larger chunk = slightly slower |
| Post-processing | 20ms | |
| Text Injection | 50ms | |
| **Total Budget** | **830ms** | Target: <1000ms perceived |

### Accurate Mode (4s chunks)

| Stage | Budget | Notes |
|-------|--------|-------|
| Audio Buffer | 800ms | 4s chunk @ 20% overlap |
| Audio Preprocessing | 20ms | |
| VAD | 30ms | |
| Network/IPC | 10ms | |
| Model Inference | 400ms | Largest chunk |
| Post-processing | 20ms | |
| Text Injection | 50ms | |
| **Total Budget** | **1330ms** | Target: <1500ms perceived |

---

## Hardware Assumptions

### Fast Mode Target Hardware

- **CPU**: Intel i5-8400 / AMD Ryzen 5 2600 or better
- **GPU**: NVIDIA RTX-class GPU preferred for low latency; exact minimum must be validated in Phase 3
- **RAM**: 8GB system RAM
- **Storage**: SSD (for model loading)

### Minimum Supported Hardware

- **CPU**: Intel i3-6100 / AMD Ryzen 3 1200
- **GPU**: None (CPU-only mode)
- **RAM**: 8GB system RAM
- **Storage**: Any

**On minimum hardware**: Expect 2-3x latency. Auto-switch to larger chunks for stability.

---

## Partial Result Latency

Partial results should appear faster to give user feedback:

| Mode | Target Partial Latency | Strategy |
|------|------------------------|----------|
| Fast | 200ms | Stream every 0.5s, show immediately |
| Balanced | 500ms | Stream every 1s, show immediately |
| Accurate | 800ms | Stream every 2s, show immediately |

**Partial handling**: 
- Show in overlay immediately
- Inject optimistically (keystrokes for short updates)
- Backspace and retype if revision occurs

---

## Memory Budget

| Component | Target | Maximum | Notes |
|-----------|--------|---------|-------|
| Electron Main | 150MB | 300MB | Main process |
| Electron Renderer | 100MB | 200MB | UI windows |
| Rust Helper | 50MB | 100MB | Native code |
| Python Worker (selected ASR) | 2000MB | 3000MB | Tighten after Phase 3 measurements |
| Python Worker (runtime) | 200MB | 400MB | PyTorch overhead |
| Audio Buffers | 50MB | 100MB | Ring buffers |
| **Total Target** | **1750MB** | **3100MB** | |

**Memory management**:
- Auto-unload model after 30min idle (optional, off by default)
- Clear audio buffers when dictation stops
- Monitor and log memory every 60 seconds in DEBUG mode

---

## Throughput Requirements

### Sustained Dictation

User may dictate continuously for 30+ minutes:

| Metric | Requirement |
|--------|-------------|
| Max consecutive dictation | 60 minutes |
| Audio buffer overflow | Never |
| Memory growth | <10% over 30min |
| Transcript accuracy maintained | Yes |

### Burst Handling

Rapid start/stop dictation (user testing hotkey):

| Scenario | Requirement |
|----------|-------------|
| 5 rapid toggles | Handle gracefully |
| Toggle latency | <100ms |
| No resource leaks | Verified by test |

---

## Timing Measurement Implementation

### Instrumentation Points

```python
# Python worker
timings = {
    "audio_received_at": timestamp(),
    "preprocessing_done_at": None,
    "inference_start_at": None,
    "inference_done_at": None,
    "result_sent_at": None
}

def process_chunk(chunk):
    timings["preprocessing_done_at"] = timestamp()
    timings["inference_start_at"] = timestamp()
    result = model.infer(chunk)
    timings["inference_done_at"] = timestamp()
    send_result(result, timings)
```

```rust
// Rust helper
struct TimingTracker {
    hotkey_pressed_at: Instant,
    audio_started_at: Option<Instant>,
    partial_received_at: Vec<Instant>,
    final_received_at: Option<Instant>,
    injection_done_at: Option<Instant>,
}
```

### Metrics to Log (DEBUG level)

Every dictation session:
```
[PERF] Session {id}: Duration={X}s, Chunks={N}, 
       AvgInference={X}ms, MaxInference={X}ms,
       FirstPartial={X}ms, FinalLatency={X}ms
```

---

## Adaptive Chunk Sizing

Based on measured performance, automatically adjust:

```
IF avg_inference_time > chunk_size * 0.5:
    # Risk of falling behind
    INCREASE chunk_size by 0.5s (up to max 4s)
    NOTIFY user: "Adjusting for your hardware..."

IF avg_inference_time < chunk_size * 0.2 AND chunk_size > 0.5s:
    # Headroom available
    DECREASE chunk_size by 0.5s (down to min 0.5s)
```

---

## Success Criteria

- [ ] Fast mode achieves <500ms average final latency on target hardware
- [ ] Balanced mode achieves <1000ms average final latency
- [ ] Partial results appear within 200ms (Fast) / 500ms (Balanced)
- [ ] Memory preferably stays under 2GB during normal operation; hard gate is defined in Phase 3 assumptions
- [ ] No audio buffer overflows during 30min continuous dictation
- [ ] System remains responsive during dictation (no UI freezing)
- [ ] Timing metrics are logged in DEBUG mode
- [ ] Adaptive chunk sizing responds within 3 chunks

---

## Benchmarking Checklist

### Development Benchmarks

Run these after Phase 3 (ASR validation):

```bash
# Test WAV transcription (batch vs chunked)
cd services/asr
uv run python scripts/benchmark.py --file test.wav --mode chunk --size 0.5
uv run python scripts/benchmark.py --file test.wav --mode chunk --size 2.0
uv run python scripts/benchmark.py --file test.wav --mode chunk --size 4.0

# Record actual output in the Phase 3 result document.
```

### Integration Benchmarks

Run these after Phase 7 (End-to-End):

1. **Latency test**: Measure time from speech end to text appearance
2. **Stress test**: 30min continuous dictation, measure memory
3. **Burst test**: 20 rapid start/stop cycles
4. **Recovery test**: Measure restart time after crash

---

## Related Documents

- **ASR Engine**: See Phase 3 for model performance validation
- **Audio Pipeline**: See Phase 6 for buffer and VAD timing
- **Performance Phase**: See Phase 8 for optimization strategies
