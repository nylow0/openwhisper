# Phase 3: ASR Engine Validation

## Context

This is the current phase.

Phase 1 is effectively complete. Phase 2 is complete enough to proceed because Electron, Rust, and Python already communicate through the intended process boundaries. Phase 3 is the critical technical gate: prove the ASR stack before investing in product UI polish.

The original plan assumed Parakeet was a 1.2 GB FP16 download with a direct `model.fp16.bin` URL. That assumption is wrong or at least unproven. The Hugging Face model currently exposes NeMo/Hugging Face artifacts around 2.51 GB each, and the repo is around 5 GB if everything is fetched. Phase 3 must measure the real cache, runtime, and packaging implications.

---

## Goal

Decide whether `nvidia/parakeet-tdt-0.6b-v3` is the right ASR engine for OpenWhisper v1.

Success means:

- Model loads on Windows through the Python worker environment.
- Batch transcription works on WAV files.
- Chunked/streaming-style transcription works well enough for dictation.
- Latency, WER, memory, and disk size are measured.
- A clear Go/No-Go decision is recorded.

Failure means:

- Stop Parakeet work.
- Execute [18-CONTINGENCY.md](18-CONTINGENCY.md).
- Do not start Phase 4 polish until a viable ASR choice exists.

---

## Prerequisites

- [x] Phase 1 repo setup is materially complete.
- [x] Phase 2 protocol is materially complete.
- [ ] Local machine has Python and `uv` available.
- [ ] JS dependencies are installed when UI verification is needed.
- [ ] Read [17-PERFORMANCE-BUDGET.md](17-PERFORMANCE-BUDGET.md).
- [ ] Read [18-CONTINGENCY.md](18-CONTINGENCY.md).

---

## Phase 3 Rules

1. Do the smallest ASR spike first.
2. Use WAV files before live microphone work.
3. Do not build product UI during this phase except minimal status needed for testing.
4. Do not assume a hand-written model download URL. Use NeMo/Hugging Face mechanisms, then measure what actually lands on disk.
5. Keep the ASR interface generic so Faster-Whisper or Whisper.cpp can replace Parakeet.
6. Record the decision in a result document before moving to Phase 4.

---

## Work Items for Delegation

For smaller agent-sized tasks, use the split feature-update briefs in
[phase-03-feature-updates/](phase-03-feature-updates/). Keep this document as the
source phase overview and decision gate.

### 1. Environment Spike

Scope:

- `services/asr/pyproject.toml`
- `services/asr/uv.lock`

Tasks:

- Add only the dependencies needed for the ASR spike.
- Pick Python, PyTorch, torchaudio, and NeMo versions from current compatibility evidence.
- Document whether the environment works on Windows.
- Do not add UI or packaging work.

Acceptance:

- `uv sync` succeeds.
- `uv run python -c "import nemo.collections.asr"` succeeds.
- The exact Python, torch, CUDA, and NeMo versions are printed.

### 2. Parakeet Spike

Scope:

- `services/asr/scripts/parakeet_spike.py`

Tasks:

- Load `nvidia/parakeet-tdt-0.6b-v3`.
- Run batch transcription on a local WAV.
- Run chunked transcription using NVIDIA/NeMo chunked inference guidance where possible.
- Print load time, device, precision, model cache size, memory, and sample output.

Acceptance:

- One command produces a readable spike report.
- The report includes enough data to decide whether to continue.

### 3. ASR Interface

Scope:

- `services/asr/src/openwhisper_asr/engine.py`

Tasks:

- Define a small ASR abstraction.
- Implement `ParakeetEngine` behind that abstraction.
- Keep all Parakeet-specific details inside the Parakeet class.

Required API:

```python
class ASREngine:
    def load(self) -> None: ...
    def transcribe_batch(self, audio_path: str) -> object: ...
    def transcribe_chunk(self, audio: object) -> object: ...
    def get_model_info(self) -> dict[str, object]: ...
```

Acceptance:

- Faster-Whisper or Whisper.cpp could be added later without changing the Rust/Python protocol.

### 4. Benchmark CLI

Scope:

- `services/asr/scripts/benchmark.py`
- `services/asr/test_data/README.md`

Tasks:

- Compare batch vs chunked results.
- Calculate WER against reference text.
- Measure average, p50, p95, p99 latency.
- Measure peak RAM and, if available, GPU memory.
- Measure model/cache size.
- Emit human-readable output and optional JSON.

Acceptance:

- Benchmarks cover `batch`, `chunk-0.5s`, `chunk-2s`, and `chunk-4s`.
- Results are reproducible with local test data.

### 5. Worker Integration

Scope:

- `services/asr/src/openwhisper_asr/__main__.py`
- `services/asr/src/openwhisper_asr/protocol.py`

Tasks:

- Replace mock transcripts with real ASR behind a feature flag or mode.
- `audio.chunk` should feed the selected engine.
- `model.load` should load the real model and emit real metadata.
- Keep mock mode for protocol testing if it remains useful.

Acceptance:

- Rust can still run the worker.
- Real mode sends `model.loaded`, `transcript.partial`, and `transcript.final` events.

### 6. Go/No-Go Report

Scope:

- Create `services/asr/PHASE-03-RESULTS.md` or `.plan/PHASE-03-RESULTS.md`.

Required contents:

- Environment versions
- Hardware tested
- Model cache size
- Runtime package size estimate
- Load time
- Batch WER
- Chunked WER by mode
- Latency distribution by mode
- Peak RAM and GPU memory
- CPU fallback result
- Decision: Go with Parakeet, test a smaller Parakeet, switch to Faster-Whisper, or switch to Whisper.cpp

---

## Go Criteria

All must pass unless Dany explicitly accepts a business tradeoff:

- [ ] Model loads on Windows.
- [ ] Batch transcription works.
- [ ] Chunked transcription works.
- [ ] Chunked WER is within 5 percentage points of batch WER on clean speech.
- [ ] Clean-speech WER is good enough for dictation, preferably under 15%.
- [ ] Fast mode average processing latency is under 500 ms on target GPU hardware.
- [ ] Balanced mode average processing latency is under 1000 ms on target hardware.
- [ ] CPU fallback works, even if slower.
- [ ] Peak memory is under 3 GB, preferably under 2 GB.
- [ ] Model cache can be kept under 3.25 GB after cleanup.
- [ ] Installed app size has a credible path under 4.5 GB, preferably under 3.5 GB.

---

## No-Go Triggers

Any of these should trigger the contingency plan:

- Model does not load reliably on Windows.
- Chunked mode is unusable for dictation.
- WER degradation vs batch is greater than 5 percentage points.
- Fast mode average latency is over 1000 ms on target GPU hardware.
- Balanced mode average latency is over 1500 ms on target hardware.
- Peak memory exceeds 3 GB during normal dictation.
- Model/cache footprint cannot be kept under 3.25 GB.
- Packaging path requires an installer or installed app size that is clearly unacceptable for the target market.

---

## Notes on Model Download

Do not implement a custom downloader around `model.fp16.bin`; that file is not the current reliable contract.

Allowed approaches:

- Let NeMo load by model name during the spike.
- Use `huggingface_hub.snapshot_download` with revision pinning and allow/ignore patterns after you know which files are required.
- Record exactly which files are needed and delete duplicate artifacts before measuring cache size.

---

## Related Documents

- [03-TECH-STACK.md](03-TECH-STACK.md)
- [14-VERIFICATION.md](14-VERIFICATION.md)
- [17-PERFORMANCE-BUDGET.md](17-PERFORMANCE-BUDGET.md)
- [18-CONTINGENCY.md](18-CONTINGENCY.md)
