#!/usr/bin/env python3
import argparse
import json
import os
import re
import subprocess
import threading
import time
import wave
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FLEURS_MANIFEST = ROOT / "asr" / ".local" / "hard_eval" / "fleurs-parakeet" / "manifest.jsonl"
LIBRISPEECH_ROOT = ROOT / "asr" / ".local" / "hard_eval" / "LibriSpeech" / "test-other"
WORK_DIR = ROOT / ".tmp" / "asr-hard-eval"
ASR_EXE_CANDIDATES = [
    ROOT / "crates" / "openwhisper-asr-rs" / "target" / "x86_64-pc-windows-msvc" / "release" / "ow-asr-rs.exe",
    ROOT / "crates" / "openwhisper-asr-rs" / "target" / "x86_64-pc-windows-msvc" / "debug" / "ow-asr-rs.exe",
    ROOT / "crates" / "openwhisper-asr-rs" / "target" / "debug" / "ow-asr-rs.exe",
]


@dataclass
class Sample:
    dataset: str
    language: str
    sample_id: str
    audio_path: Path
    reference: str
    duration_seconds: float | None = None


def normalize_text(text: str) -> str:
    text = repair_mojibake(text)
    text = text.lower()
    text = re.sub(r"[^\w\s']", " ", text, flags=re.UNICODE)
    text = re.sub(r"\s+", " ", text).strip()
    return text


def repair_mojibake(text: str) -> str:
    if "Ð" not in text and "Ñ" not in text and "â" not in text:
        return text
    try:
        return text.encode("latin1").decode("utf-8")
    except UnicodeError:
        return text


def edit_distance(a: list[str] | str, b: list[str] | str) -> int:
    previous = list(range(len(b) + 1))
    for i, ca in enumerate(a, start=1):
        current = [i]
        for j, cb in enumerate(b, start=1):
            current.append(
                min(
                    previous[j] + 1,
                    current[j - 1] + 1,
                    previous[j - 1] + (0 if ca == cb else 1),
                )
            )
        previous = current
    return previous[-1]


def error_rates(reference: str, hypothesis: str) -> tuple[float, float]:
    ref_norm = normalize_text(reference)
    hyp_norm = normalize_text(hypothesis)
    ref_words = ref_norm.split()
    hyp_words = hyp_norm.split()
    wer = edit_distance(ref_words, hyp_words) / max(1, len(ref_words))
    cer = edit_distance(ref_norm, hyp_norm) / max(1, len(ref_norm))
    return wer, cer


def valid_word_timing(word: dict, duration_seconds: float | None) -> bool:
    start_ms = word.get("start_ms")
    end_ms = word.get("end_ms")
    if not isinstance(start_ms, int) or not isinstance(end_ms, int):
        return False
    if not 0 <= start_ms < end_ms:
        return False
    return duration_seconds is None or end_ms <= duration_seconds * 1000 + 200


def wav_duration_seconds(path: Path) -> float:
    with wave.open(str(path), "rb") as reader:
        return reader.getnframes() / max(1, reader.getframerate())


def load_fleurs(languages: set[str]) -> list[Sample]:
    samples = []
    missing = []
    if not FLEURS_MANIFEST.is_file():
        raise FileNotFoundError(
            f"Missing {FLEURS_MANIFEST}. Prepare EN/UK FLEURS test audio with scripts/prepare_fleurs_eval.py."
        )
    with FLEURS_MANIFEST.open("r", encoding="utf-8") as handle:
        for line in handle:
            item = json.loads(line)
            language = {"english": "en", "ukrainian": "uk"}.get(item["language"])
            if language not in languages:
                continue
            audio_path = Path(item["path"])
            if not audio_path.is_file():
                missing.append(str(audio_path))
                continue
            samples.append(
                Sample(
                    dataset=f"fleurs-{language}",
                    language=language,
                    sample_id=item["id"],
                    audio_path=audio_path,
                    reference=repair_mojibake(item["reference"]),
                    duration_seconds=float(item.get("duration_seconds", 0)) or None,
                )
            )
    if missing:
        raise FileNotFoundError(f"Missing {len(missing)} FLEURS WAV files; first missing: {missing[0]}")
    present = {sample.language for sample in samples}
    if not languages.issubset(present):
        raise ValueError(f"FLEURS manifest lacks languages: {sorted(languages - present)}")
    return samples


def load_librispeech(limit: int) -> list[Sample]:
    transcripts = {}
    for trans_path in LIBRISPEECH_ROOT.rglob("*.trans.txt"):
        for line in trans_path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            sample_id, reference = line.split(" ", 1)
            transcripts[sample_id] = reference

    samples = []
    audio_paths = sorted(LIBRISPEECH_ROOT.rglob("*.wav")) + sorted(LIBRISPEECH_ROOT.rglob("*.flac"))
    for audio_path in audio_paths:
        if len(samples) >= limit:
            break
        sample_id = audio_path.stem
        if sample_id not in transcripts:
            continue
        wav_path = audio_path
        if audio_path.suffix.lower() == ".flac":
            wav_path = WORK_DIR / "librispeech-wav" / f"{sample_id}.wav"
            ensure_wav(audio_path, wav_path)
        samples.append(
            Sample(
                dataset="librispeech-test-other",
                language="en",
                sample_id=sample_id,
                audio_path=wav_path,
                reference=transcripts[sample_id],
                duration_seconds=wav_duration_seconds(wav_path),
            )
        )
    return samples


def ensure_wav(source: Path, target: Path) -> None:
    if target.is_file():
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    command = [
        "ffmpeg",
        "-y",
        "-loglevel",
        "error",
        "-i",
        str(source),
        "-ar",
        "16000",
        "-ac",
        "1",
        str(target),
    ]
    subprocess.run(command, cwd=ROOT, check=True)


def run_transcribe(sample: Sample, model: str, device: str, languages: str, mode: str) -> dict:
    command = [
        str(resolve_asr_exe()),
        "transcribe",
        str(sample.audio_path),
        "--model",
        model,
        "--device",
        device,
        "--languages",
        languages,
    ]

    started = time.perf_counter()
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
    )
    elapsed_ms = int((time.perf_counter() - started) * 1000)
    hypothesis = completed.stdout.strip()
    wer, cer = error_rates(sample.reference, hypothesis) if completed.returncode == 0 else (None, None)
    return {
        "mode": mode,
        "runner": "cli",
        "warm": False,
        "dataset": sample.dataset,
        "language": sample.language,
        "sample_id": sample.sample_id,
        "audio_path": str(sample.audio_path),
        "duration_seconds": sample.duration_seconds,
        "model": model,
        "device": device,
        "configured_languages": languages,
        "latency_ms": elapsed_ms,
        "rtf": elapsed_ms / 1000 / sample.duration_seconds if sample.duration_seconds else None,
        "wer": wer,
        "cer": cer,
        "reference": normalize_text(sample.reference),
        "hypothesis": normalize_text(hypothesis),
        "stderr_tail": tail(completed.stderr),
        "returncode": completed.returncode,
        "detected_language": None,
        "word_count": None,
        "timed_word_count": None,
        "confidence_count": None,
        "speech_segment_count": None,
    }


def run_worker_session(samples: list[Sample], model: str, device: str, languages: str, mode: str) -> list[dict]:
    env = os.environ.copy()
    env["OPENWHISPER_ASR_LANGUAGES"] = languages
    env["OPENWHISPER_ASR_AUTO_DETECT_LANGUAGE"] = "0"
    log_path = WORK_DIR / f"worker-{mode}-{languages.replace(',', '-')}.log"
    results = []
    with log_path.open("w", encoding="utf-8") as log:
        worker = subprocess.Popen(
            [str(resolve_asr_exe())], cwd=ROOT, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log,
            text=True, encoding="utf-8", errors="replace", bufsize=1,
        )
        assert worker.stdin is not None and worker.stdout is not None

        def send(command: dict) -> dict:
            watchdog = threading.Timer(180, worker.kill)
            watchdog.start()
            try:
                worker.stdin.write(json.dumps(command) + "\n")
                worker.stdin.flush()
                line = worker.stdout.readline()
            finally:
                watchdog.cancel()
            if not line:
                raise RuntimeError(f"ASR worker exited while handling {command['type']}; see {log_path}")
            return json.loads(line)

        try:
            loaded = send({"type": "model.load", "model": model, "device": device})
            if loaded["type"] != "model.loaded":
                raise RuntimeError(f"ASR model load failed: {loaded}")
            for index, sample in enumerate(samples):
                print(f"{mode} {languages} {index + 1}/{len(samples)} {sample.sample_id}", flush=True)
                started = time.perf_counter()
                event = send({"type": "transcribe.file", "audio_path": str(sample.audio_path)})
                elapsed_ms = int((time.perf_counter() - started) * 1000)
                ok = event["type"] == "transcript.final"
                hypothesis = event.get("text", "") if ok else ""
                wer, cer = error_rates(sample.reference, hypothesis) if ok else (None, None)
                words = event.get("words", []) if ok else []
                results.append({
                    "mode": mode,
                    "runner": "worker",
                    "warm": index > 0,
                    "dataset": sample.dataset,
                    "language": sample.language,
                    "sample_id": sample.sample_id,
                    "audio_path": str(sample.audio_path),
                    "duration_seconds": sample.duration_seconds,
                    "model": model,
                    "device": device,
                    "configured_languages": languages,
                    "latency_ms": elapsed_ms,
                    "processing_latency_ms": event.get("processing_latency_ms"),
                    "rtf": elapsed_ms / 1000 / sample.duration_seconds if sample.duration_seconds else None,
                    "wer": wer,
                    "cer": cer,
                    "reference": normalize_text(sample.reference),
                    "hypothesis": normalize_text(hypothesis),
                    "detected_language": event.get("language"),
                    "word_count": len(words),
                    "timed_word_count": sum(valid_word_timing(word, sample.duration_seconds) for word in words),
                    "zero_duration_word_count": sum(word.get("start_ms") == word.get("end_ms") for word in words),
                    "confidence_count": sum(isinstance(word.get("confidence"), (int, float)) for word in words),
                    "speech_segment_count": len(event.get("speech_segments", [])),
                    "returncode": 0 if ok else 1,
                    "error": event.get("error"),
                })
        finally:
            try:
                send({"type": "shutdown"})
            except RuntimeError:
                pass
            worker.stdin.close()
            worker.wait(timeout=10)
    return results


def resolve_asr_exe() -> Path:
    for candidate in ASR_EXE_CANDIDATES:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError(
        "Missing ow-asr-rs.exe. Build it with: cargo build --manifest-path "
        "crates/openwhisper-asr-rs/Cargo.toml --target x86_64-pc-windows-msvc --release"
    )


def summarize(results: list[dict]) -> list[dict]:
    groups = {}
    for result in results:
        key = (result["mode"], result["runner"], result["warm"], result["dataset"], result["language"], result["model"], result["device"], result["configured_languages"])
        groups.setdefault(key, []).append(result)

    rows = []
    for (mode, runner, warm, dataset, language, model, device, configured_languages), items in sorted(groups.items()):
        ok = [item for item in items if item["returncode"] == 0]
        rows.append(
            {
                "mode": mode,
                "runner": runner,
                "warm": warm,
                "dataset": dataset,
                "language": language,
                "model": model,
                "device": device,
                "configured_languages": configured_languages,
                "samples": len(items),
                "ok": len(ok),
                "language_correct": sum(item["detected_language"] == item["language"] for item in ok if item["detected_language"]),
                "language_reported": sum(item["detected_language"] is not None for item in ok),
                "wer": average([item["wer"] for item in ok]),
                "cer": average([item["cer"] for item in ok]),
                "latency_ms_avg": average([item["latency_ms"] for item in ok]),
                "latency_ms_p50": percentile([item["latency_ms"] for item in ok], 0.50),
                "latency_ms_p90": percentile([item["latency_ms"] for item in ok], 0.90),
                "rtf_avg": average([item["rtf"] for item in ok]),
                "words_avg": average([item["word_count"] for item in ok]),
                "timed_words_avg": average([item["timed_word_count"] for item in ok]),
                "zero_duration_words_avg": average([item.get("zero_duration_word_count") for item in ok]),
                "confident_words_avg": average([item["confidence_count"] for item in ok]),
                "speech_segments_avg": average([item["speech_segment_count"] for item in ok]),
            }
        )
    return rows


def average(values: list[float | int | None]) -> float | None:
    cleaned = [value for value in values if value is not None]
    return sum(cleaned) / len(cleaned) if cleaned else None


def percentile(values: list[float | int | None], q: float) -> float | None:
    cleaned = sorted(value for value in values if value is not None)
    if not cleaned:
        return None
    index = min(len(cleaned) - 1, round((len(cleaned) - 1) * q))
    return cleaned[index]


def tail(text: str, lines: int = 12) -> str:
    return "\n".join(text.strip().splitlines()[-lines:])


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="large_v3_turbo_q8")
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--libri-limit", type=int, default=5)
    parser.add_argument("--sample-limit-per-language", type=int, default=0,
                        help="0 runs every available clip")
    parser.add_argument("--runner", choices=["worker", "cli"], default="worker")
    parser.add_argument("--language-modes", nargs="+", choices=["forced", "en-uk"], default=["forced", "en-uk"])
    parser.add_argument("--output-tag", default=None)
    args = parser.parse_args()

    WORK_DIR.mkdir(parents=True, exist_ok=True)
    samples = load_fleurs({"en", "uk"}) + load_librispeech(args.libri_limit)
    if args.sample_limit_per_language < 0:
        parser.error("--sample-limit-per-language must be non-negative")
    if args.sample_limit_per_language:
        seen = {}
        limited = []
        for sample in samples:
            seen[sample.language] = seen.get(sample.language, 0) + 1
            if seen[sample.language] <= args.sample_limit_per_language:
                limited.append(sample)
        samples = limited
    if not samples:
        parser.error("No EN/UK benchmark audio found")
    results = []
    for mode in args.language_modes:
        language_sets = [
            (language, [sample for sample in samples if sample.language == language])
            for language in sorted({sample.language for sample in samples})
        ] if mode == "forced" else [("en,uk", samples)]
        for languages, selected in language_sets:
            if args.runner == "worker":
                results.extend(run_worker_session(selected, args.model, args.device, languages, mode))
            else:
                results.extend(run_transcribe(sample, args.model, args.device, languages, mode) for sample in selected)

    summary = summarize(results)
    suffix = f"-{args.output_tag}" if args.output_tag else ""
    write_jsonl(WORK_DIR / f"results{suffix}.jsonl", results)
    (WORK_DIR / f"summary{suffix}.json").write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    return 0 if all(row["ok"] == row["samples"] for row in summary) else 1


if __name__ == "__main__":
    raise SystemExit(main())
