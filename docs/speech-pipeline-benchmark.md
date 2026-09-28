# Speech pipeline investigation and benchmark

2026-09-28. Baseline: `97467a0` on `main`. Test machine: Windows, Intel Core
i5-13450HX, 10 cores / 16 logical processors. The local CPU build used
[whisper.cpp v1.9.4](https://github.com/ggml-org/whisper.cpp/releases/tag/v1.9.4),
`ggml-large-v3-turbo-q8_0.bin`, and the Silero VAD v6.2.0 ggml model. No GPU
results are included.

## What the app did

Electron starts the native helper, which starts the Rust ASR worker. The worker
records a 16 kHz mono WAV, then ran `whisper-cli -m ... -f ...` once per finished
dictation. `model.load` validated paths but did not load model weights into the
worker. The parser returned `words: []` even when whisper.cpp wrote token
offsets and probabilities. All four profiles used `-nt`, which made an
11-second JFK clip's JSON segment span 30 seconds. The worker deleted the WAV
after transcription, and desktop history stored only text, language, and
latency. There was no production VAD.

## Changes and checks

- The worker now uses a local persistent `whisper-server` for selected EN/UK when the
  binary is bundled. It starts on the first decode, reuses the model context,
  and falls back to `whisper-cli` if the server is missing or fails. The server
  runs with `-nlp`; its otherwise unused language-probability table added about
  2.8 seconds to a 5.4-second test clip.
  Explicit detection across all languages stays on the CLI path to preserve
  its language-code behavior.
- Silero VAD detects speech spans before Whisper. A confirmed silent WAV skips
  decoding. Spoken audio still gets one full-context Whisper pass. The VAD
  spans retain original-audio offsets; no overlapping transcription windows
  were added. We avoided whisper.cpp's integrated `--vad` for this path because
  [CLI token offsets can remain on the VAD-compressed timeline](https://github.com/ggml-org/whisper.cpp/issues/4046).
- The CLI profile requests full JSON and no longer uses `-nt`. The worker
  passes decoder token offsets and probabilities through the native bridge.
  Successful desktop dictation keeps its original WAV under the app's
  `recordings` directory. History retains its path, tokens, and VAD spans;
  deleting or evicting history removes the associated recording.
- The benchmark now runs complete FLEURS clips through a persistent worker,
  checks detected language and timing coverage, and can still run separate
  per-file CLI measurements. It no longer scores a truncated 5-second window
  against a full-clip reference, which was not a valid WER comparison.

## EN/UK results

Five [FLEURS test](https://huggingface.co/datasets/google/fleurs) clips per
language, 5.4-18.0 seconds, sampled across each test TSV with
`scripts/prepare_fleurs_eval.py`. Values are mean **per-clip** WER. The latency
comparison pairs exactly the clips used as warm worker requests: four EN and
four UK in forced mode, four EN and five UK in EN+UK mode. Baseline is the old
per-file CLI run; new is the warm persistent worker run, including VAD and
metadata extraction. The baseline's `--auto-detect-language` and the old
EN+UK setting both resolved to whisper.cpp `-l auto`.

| Mode | Language | Baseline latency | New warm latency | Change | WER before / after |
| --- | --- | ---: | ---: | ---: | ---: |
| Forced language | EN | 3.97 s | 3.78 s | 4.8% lower | 9.05% / 9.05% |
| Forced language | UK | 5.41 s | 4.65 s | 14.2% lower | 5.13% / 5.13% |
| EN+UK enabled | EN | 8.38 s | 6.58 s | 21.5% lower | 9.05% / 9.05% |
| EN+UK enabled | UK | 9.62 s | 7.46 s | 22.4% lower | 5.13% / 5.13% |

The first EN+UK request took 8.43 seconds, including server startup and model
load. The persistent context mainly benefits subsequent requests; the table
uses warm requests for that reason.

All ten EN+UK requests reported the expected language. Auto-detection remained
substantially slower than forcing one language, but did not hurt WER on this
sample. That did not justify a second language classifier or forced-language
retry logic yet.

The old parser produced no token records. The new EN+UK run retained 129 EN
and 252 UK token records, all with probabilities. Of those, 127 EN and 240 UK
had positive-duration offsets within the source audio (with 200 ms rounding
tolerance). Whisper returned 2 EN and 12 UK zero-duration token intervals;
the benchmark reports them instead of claiming perfect timestamp quality.
There are no reference word alignments here, so this is structural coverage,
not a measure of timestamp accuracy.

A six-second zero PCM WAV produced `[BLANK_AUDIO]` after 6.4 seconds without
VAD. The worker's real `transcribe.file` path now returned empty text and no
speech segments with 220 ms processing latency. The FLEURS clips all passed
the VAD gate. We also found a server output defect: one UK response repeated
the whole sentence in zero-duration JSON segments. Filtering those segments
restored that clip's WER from 96% to 8%, and the five-clip UK mean from 22.7%
to 5.1%.

## Reproduce

Download `test.tsv` and `audio/test.tar.gz` for `en_us` and `uk_ua` from the
[FLEURS dataset](https://huggingface.co/datasets/google/fleurs/tree/main/data)
into `asr/.local/hard_eval/fleurs-parakeet` as
`{en_us,uk_ua}-test.{tsv,tar.gz}`, then run:

```powershell
uv run --no-project --python 3.12 scripts/prepare_fleurs_eval.py --count-per-language 5
$env:OPENWHISPER_WHISPERCPP_CPU_EXE = '<local whisper-cli.exe path>'
uv run --no-project --python 3.12 scripts/asr_hard_eval_benchmark.py --runner worker --device cpu --libri-limit 0 --output-tag repeat
```

Build `whisper-cli`, `whisper-server`, and `whisper-vad-speech-segments` from
the same whisper.cpp version, with the binaries beside each other. Raw result
rows and summaries go to `.tmp/asr-hard-eval/`; the FLEURS WAVs and archives
remain under ignored `asr/.local/`. The exact local runs used tags `baseline`,
`baseline-auto`, `forced-final`, and `en-uk-metadata`.

## Limits

This is a ten-clip CPU comparison, not an estimate for all speakers or noisy
microphones. GPU latency, VAD false negatives on quiet speech, live microphone
capture, and the installed Electron package were not measured on this checkout.
The package smoke check now requires the server, VAD binary, and model so a
release cannot silently ship without them.
