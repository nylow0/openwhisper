#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
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


def wav_duration_seconds(path: Path) -> float:
    with wave.open(str(path), "rb") as reader:
        return reader.getnframes() / max(1, reader.getframerate())


def load_fleurs(languages: set[str]) -> list[Sample]:
    samples = []
    missing = []
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
        (WORK_DIR / "missing-audio.txt").parent.mkdir(parents=True, exist_ok=True)
        (WORK_DIR / "missing-audio.txt").write_text("\n".join(missing) + "\n", encoding="utf-8")
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


def run_transcribe(sample: Sample, model: str, device: str, auto_detect: bool) -> dict:
    command = [
        str(resolve_asr_exe()),
        "transcribe",
        str(sample.audio_path),
        "--model",
        model,
        "--device",
        device,
        "--languages",
        sample.language,
    ]
    if auto_detect:
        command.append("--auto-detect-language")

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
        "mode": "final",
        "dataset": sample.dataset,
        "language": sample.language,
        "sample_id": sample.sample_id,
        "audio_path": str(sample.audio_path),
        "duration_seconds": sample.duration_seconds,
        "model": model,
        "device": device,
        "auto_detect_language": auto_detect,
        "latency_ms": elapsed_ms,
        "rtf": elapsed_ms / 1000 / sample.duration_seconds if sample.duration_seconds else None,
        "wer": wer,
        "cer": cer,
        "reference": normalize_text(sample.reference),
        "hypothesis": normalize_text(hypothesis),
        "stderr_tail": tail(completed.stderr),
        "returncode": completed.returncode,
    }


def resolve_asr_exe() -> Path:
    for candidate in ASR_EXE_CANDIDATES:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError(
        "Missing ow-asr-rs.exe. Build it with: cargo build --manifest-path "
        "crates/openwhisper-asr-rs/Cargo.toml --target x86_64-pc-windows-msvc --release"
    )


def run_streaming_pressure(sample: Sample, model: str, device: str, auto_detect: bool, window_seconds: float) -> dict:
    window_path = sample.audio_path
    if sample.duration_seconds and sample.duration_seconds > window_seconds:
        window_path = WORK_DIR / "stream-windows" / f"{sample.sample_id}-{int(window_seconds * 1000)}ms.wav"
        trim_wav(sample.audio_path, window_path, window_seconds)

    window_sample = Sample(
        dataset=sample.dataset,
        language=sample.language,
        sample_id=sample.sample_id,
        audio_path=window_path,
        reference=sample.reference,
        duration_seconds=wav_duration_seconds(window_path),
    )
    result = run_transcribe(window_sample, model, device, auto_detect)
    result["mode"] = "streaming_partial_window"
    result["source_audio_path"] = str(sample.audio_path)
    return result


def trim_wav(source: Path, target: Path, seconds: float) -> None:
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
        "-t",
        str(seconds),
        "-ar",
        "16000",
        "-ac",
        "1",
        str(target),
    ]
    subprocess.run(command, cwd=ROOT, check=True)


def summarize(results: list[dict]) -> list[dict]:
    groups = {}
    for result in results:
        key = (result["mode"], result["dataset"], result["language"], result["model"], result["device"], result["auto_detect_language"])
        groups.setdefault(key, []).append(result)

    rows = []
    for (mode, dataset, language, model, device, auto_detect), items in sorted(groups.items()):
        ok = [item for item in items if item["returncode"] == 0]
        rows.append(
            {
                "mode": mode,
                "dataset": dataset,
                "language": language,
                "model": model,
                "device": device,
                "auto_detect_language": auto_detect,
                "samples": len(items),
                "ok": len(ok),
                "wer": average([item["wer"] for item in ok]),
                "cer": average([item["cer"] for item in ok]),
                "latency_ms_avg": average([item["latency_ms"] for item in ok]),
                "latency_ms_p50": percentile([item["latency_ms"] for item in ok], 0.50),
                "latency_ms_p90": percentile([item["latency_ms"] for item in ok], 0.90),
                "rtf_avg": average([item["rtf"] for item in ok]),
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
    parser.add_argument("--device", default="gpu")
    parser.add_argument("--libri-limit", type=int, default=5)
    parser.add_argument("--stream-limit-per-language", type=int, default=2)
    parser.add_argument("--stream-window-seconds", type=float, default=5.0)
    parser.add_argument("--auto-detect-language", action="store_true")
    parser.add_argument("--output-tag", default=None)
    args = parser.parse_args()

    WORK_DIR.mkdir(parents=True, exist_ok=True)
    samples = load_fleurs({"en", "uk"}) + load_librispeech(args.libri_limit)
    results = []
    for sample in samples:
        results.append(run_transcribe(sample, args.model, args.device, args.auto_detect_language))

    seen = {}
    for sample in samples:
        key = sample.language
        seen[key] = seen.get(key, 0) + 1
        if seen[key] <= args.stream_limit_per_language:
            results.append(
                run_streaming_pressure(
                    sample,
                    args.model,
                    args.device,
                    args.auto_detect_language,
                    args.stream_window_seconds,
                )
            )

    summary = summarize(results)
    suffix = f"-{args.output_tag}" if args.output_tag else ""
    write_jsonl(WORK_DIR / f"results{suffix}.jsonl", results)
    (WORK_DIR / f"summary{suffix}.json").write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    return 0 if all(row["ok"] == row["samples"] for row in summary) else 1


if __name__ == "__main__":
    raise SystemExit(main())
