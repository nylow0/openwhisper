# Contingency Plan: When Parakeet Fails

## Context

This document outlines the fallback strategy if Parakeet TDT 0.6B v3 does not meet our performance or quality requirements. OpenWhisper is a Windows-first offline dictation desktop app, and the ASR model is the core technology risk. We validate Parakeet in Phase 3, and if it fails, we execute this contingency plan instead of continuing with a broken foundation.

---

## Go/No-Go Decision Criteria

After Phase 3 (ASR Engine Validation), evaluate Parakeet against these criteria:

### Go Criteria (All must pass)

| Criterion | Threshold | Measurement |
|-----------|-----------|-------------|
| Word Error Rate (WER) | <15% on clean speech | Librispeech test-clean |
| Latency (Fast mode) | <500ms average | See Performance Budget doc |
| Streaming quality | <5% WER degradation vs batch | Same audio, chunk vs batch |
| Memory usage | <2GB peak | During inference |
| CPU fallback | Functional (not fast) | Works on CPU, even if slow |

### No-Go Triggers (Any triggers contingency)

- WER >20% on clean speech (unusable accuracy)
- Streaming WER >25% (chunking breaks the model)
- Latency >1000ms in Fast mode on target hardware (too slow)
- Memory >3GB (excessive)
- Model fails to load on supported hardware (compatibility issue)

---

## Contingency Options

### Option A: Smaller Parakeet Model (Preferred)

If 0.6B is too slow but quality is good:

**Candidate**: `nvidia/parakeet-tdt-0.1b-v3` (if available) or similar

| Aspect | Change |
|--------|--------|
| Model size | Much smaller than Parakeet 0.6B |
| Quality | Slightly lower (~2-3% WER increase) |
| Speed | 2-3x faster inference |
| Memory | Lower than Parakeet 0.6B |

**Decision**: Use if 0.6B quality is good but speed/memory issues

### Option B: Faster-Whisper (CTranslate2)

If Parakeet quality is insufficient:

**Candidate**: `faster-whisper` with `distil-medium.en` or `base` model

| Aspect | Details |
|--------|---------|
| Framework | CTranslate2 (optimized inference) |
| Quality | Comparable to OpenAI Whisper |
| Speed | 4x faster than original Whisper |
| Memory | ~500MB-1GB depending on model |
| Streaming | Requires VAD + chunking (not native streaming) |

**Pros**:
- Battle-tested, many production deployments
- Good multilingual support
- Active community

**Cons**:
- Not truly streaming (need VAD-based segmentation)
- Different API than NeMo
- Requires reimplementation of ASR worker

**Decision**: Use if Parakeet quality is unacceptable

### Option C: Whisper.cpp

If we need maximum efficiency:

**Candidate**: `whisper.cpp` with quantized models

| Aspect | Details |
|--------|---------|
| Framework | C++ implementation |
| Quality | Good with medium+ models |
| Speed | Very fast, especially with GPU/GGML |
| Memory | Very efficient |
| Streaming | Via VAD + chunking |

**Pros**:
- Extremely fast and efficient
- Can run on CPU very well
- Can integrate via Rust bindings

**Cons**:
- Requires significant rearchitecture (might replace Python worker)
- Different model format
- More complex integration

**Decision**: Use if both Parakeet and Faster-Whisper fail

### Option D: Hybrid Cloud Fallback (Last Resort)

If no local model works acceptably:

**Approach**: Offer optional cloud ASR with local fallback

| Aspect | Details |
|--------|---------|
| Online | Use OpenAI Whisper API or similar |
| Offline | Use tiny local model (quality sacrificed) |
| Privacy | User must opt-in to cloud |

**Decision**: Only if all local options fail AND market research shows users accept cloud

---

## Contingency Execution Plan

### Timeline

If Parakeet fails validation in Phase 3:

```
Day 1-2:   Root cause analysis - is it speed, quality, or both?
Day 3-4:   Evaluate Option A (smaller Parakeet) if applicable
Day 5-10:  If Option A fails, evaluate Option B (Faster-Whisper)
Day 11-15: If Option B fails, evaluate Option C (Whisper.cpp)
Day 16:    Decision point - proceed with best option or Option D
```

**Maximum delay**: 2 weeks before reverting to original schedule with chosen alternative.

### Phase Adjustments

| Phase | Parakeet Path | Contingency Path |
|-------|---------------|------------------|
| Phase 3 | Validate Parakeet | Validate alternative |
| Phase 4 | UI with Parakeet | UI with alternative |
| Phase 6 | Audio + Parakeet | Audio + alternative |
| Phase 9 | Package Parakeet | Package alternative |

### Documentation Updates

If contingency is triggered:

1. Update `03-TECH-STACK.md` with new model choice
2. Update `07-PHASE-03-ASR-ENGINE.md` with new validation criteria
3. Update `15-ASSUMPTIONS.md` with new defaults
4. Update `17-PERFORMANCE-BUDGET.md` with new latency targets

---

## Validation Criteria for Alternatives

Any alternative model must meet:

| Criterion | Minimum | Preferred |
|-----------|---------|-----------|
| WER (clean speech) | <18% | <12% |
| Latency (Fast mode) | <800ms | <500ms |
| Memory | <2GB | <1.5GB |
| Streaming | Supported | Native |
| Windows support | Yes | Yes |
| Offline capable | Required | Required |
| License | Permissive | Permissive |

---

## Pre-Validation Preparation

Do not create unused alternative directories before Parakeet fails. That would clutter the repo.

Before Phase 3 begins, prepare only:

- The objective Go/No-Go criteria.
- A short list of fallback candidates.
- Notes on likely setup commands.

If Parakeet fails, create the smallest possible alternative spike under `services/asr/` or a clearly named experimental branch, then delete failed experiments once the decision is made.

---

## Risk Mitigation During Phase 3

Even if Parakeet looks good, minimize lock-in:

1. **Abstract the ASR interface**:
   ```python
   class ASREngine(ABC):
       def load_model(self, path): ...
       def transcribe_chunk(self, audio): ...
       def transcribe_batch(self, audio): ...
   
   class ParakeetEngine(ASREngine): ...
   class FasterWhisperEngine(ASREngine): ...
   ```

2. **Keep protocol generic**: Don't encode Parakeet-specific messages in protocol

3. **Document assumptions**: Note which behaviors are Parakeet-specific

---

## Decision Log Template

If contingency is triggered, document:

```markdown
## ASR Model Decision - [Date]

### Evaluated Options
- Parakeet 0.6B: FAILED [reason]
- Parakeet 0.1B: [result]
- Faster-Whisper: [result]
- Whisper.cpp: [result]

### Final Decision
[Chosen model] because [reasoning]

### Impact
- Schedule delay: [X] days
- Quality impact: [higher/lower/similar]
- Performance impact: [better/worse/similar]
- Implementation changes: [list]

### Updated Timeline
[New phase dates]
```

---

## Success Criteria

- [ ] Alternative model candidates and setup notes are ready before Phase 3 completes
- [ ] ASR interface is abstracted (not Parakeet-specific)
- [ ] Decision criteria are objective and measurable
- [ ] Go/No-Go decision is made within 3 days of Phase 3 completion
- [ ] If contingency triggered, alternative is validated within 2 weeks
- [ ] All documentation is updated within 1 day of decision

---

## Related Documents

- **ASR Engine**: See Phase 3 document for Parakeet validation plan
- **Tech Stack**: See document for alternative model requirements
- **Performance Budget**: See document for latency targets applied to all models
