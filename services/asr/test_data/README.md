# ASR Test Data Contract

This directory defines the reproducible local test inputs for Phase 3 ASR spikes and benchmarks.

**Rule**: Large WAV files are NOT committed to the repo. This folder contains only the contract, instructions, and (optionally) tiny license-safe placeholders.

---

## Directory Layout

```text
services/asr/test_data/
  README.md              # this file
  .gitignore             # blocks *.wav, *.mp3, *.flac from git
  samples/
    sample-clean-short.wav    # local only (not committed)
    sample-clean-short.txt    # reference transcript
    sample-clean-long.wav     # local only (not committed)
    sample-clean-long.txt     # reference transcript
    sample-noisy-short.wav    # optional: local only
    sample-noisy-short.txt    # optional: reference transcript
    sample-non-english.wav    # optional: local only
    sample-non-english.txt    # optional: reference transcript
```

### For Benchmark Agents

Any script that needs test data should look here:

| Path | Purpose |
|------|---------|
| `services/asr/test_data/samples/*.wav` | Audio input files |
| `services/asr/test_data/samples/*.txt` | Reference transcripts (same basename) |

---

## Audio Requirements

Every test WAV must meet these criteria for consistent benchmarking:

| Property | Requirement |
|----------|-------------|
| Format | WAV (PCM) |
| Sample rate | 16 kHz preferred; 8 kHz or 22.05 kHz acceptable if noted in filename |
| Channels | Mono preferred; stereo must be noted in filename |
| Content | Clean speech (English default) |
| Minimum | At least one 10-30 second sample |
| Recommended | At least one 60+ second sample for long-form tests |
| Optional | One noisy/reverberant sample for robustness checks |
| Optional | One non-English sample if multilingual evaluation is planned |

### Filename Conventions

Use descriptive names so benchmark logs are readable:

```text
<content>-<condition>-<duration>.wav
sample-clean-short.wav       # ~15s, studio speech
sample-clean-long.wav        # ~90s, studio speech
sample-noisy-short.wav       # ~15s, cafe background noise
sample-en-male-30s.wav       # 30s, specific speaker tag
```

---

## Reference Transcript Format

Each `.txt` file must share the same basename as its matching `.wav`:

```text
sample-clean-short.wav
sample-clean-short.txt
```

### `.txt` Content Rules

1. **Plain text only** — no markdown, no HTML.
2. **One line** — the entire reference transcript on a single line. Line breaks in the source audio should be replaced with a single space.
3. **Normalize lightly**:
   - Lowercase everything (the benchmark normalizes before WER).
   - Remove punctuation except apostrophes in contractions.
   - Expand common contractions if you want strict matching, or leave them and let the benchmark handle normalization.
4. **No timestamps** — this is a reference file, not an alignment label.

### Example

`sample-clean-short.txt`:

```text
the quick brown fox jumps over the lazy dog
```

---

## How to Add Local Samples

1. Place `.wav` and matching `.txt` files into `services/asr/test_data/samples/`.
2. Verify the audio meets the requirements above.
3. **Do not** `git add` the WAV. The `.gitignore` already blocks them.
4. If you want to record what samples you used for a specific run, add a line to your benchmark output or a local `run-notes.md` (also gitignored).

### Where to Get Safe Audio

- Record yourself reading public-domain text (e.g., Project Gutenberg).
- Use CC-0 or CC-BY speech datasets that allow redistribution, but **still do not commit the raw audio** unless it is under ~100 KB and explicitly license-safe.
- Open-source ASR evaluation sets (LibriSpeech test-clean, Mozilla Common Voice validation clips) are good candidates for local use.

---

## Benchmark Discovery Contract

Future benchmark scripts should discover inputs with this logic (pseudocode):

```python
from pathlib import Path

TEST_DATA_DIR = Path(__file__).parent.parent / "test_data" / "samples"

def discover_samples():
    wavs = sorted(TEST_DATA_DIR.glob("*.wav"))
    pairs = []
    for wav in wavs:
        txt = wav.with_suffix(".txt")
        if txt.exists():
            pairs.append({"audio": wav, "reference": txt.read_text(encoding="utf-8").strip()})
    return pairs
```

If no samples are found, the benchmark must print:

```text
No test samples found in services/asr/test_data/samples/
Add .wav + .txt pairs locally. See test_data/README.md
```

---

## Git Policy

- **Committed**: `README.md`, `.gitignore`
- **Ignored**: `*.wav`, `*.mp3`, `*.flac`, `*.ogg`, `run-notes.md`, `*.json` (per-run results)

---

## Related Documents

- Phase 3 ASR overview: `.plan/07-PHASE-03-ASR-ENGINE.md`
- Performance budget: `.plan/17-PERFORMANCE-BUDGET.md`
- Benchmark CLI update: `.plan/phase-03-feature-updates/06-benchmark-cli.md`
