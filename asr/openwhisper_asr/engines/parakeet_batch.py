"""Parakeet batch transcription spike.

This module intentionally imports heavyweight model dependencies inside functions so
basic protocol tests can run without installing PyTorch or NeMo.
"""

from __future__ import annotations

import argparse
import os
import sys
import time
from pathlib import Path


def resolve_device(preference: str) -> str:
    """Resolve auto to cuda or cpu."""
    if preference != "auto":
        return preference

    import torch

    return "cuda" if torch.cuda.is_available() else "cpu"


def get_audio_duration(audio_path: Path) -> float | None:
    """Return audio duration in seconds."""
    try:
        import torchaudio

        info = torchaudio.info(str(audio_path))
        return info.num_frames / info.sample_rate
    except Exception:
        pass

    try:
        from scipy.io import wavfile

        sample_rate, data = wavfile.read(str(audio_path))
        return data.shape[0] / sample_rate
    except Exception:
        return None


def format_bytes(n: int) -> str:
    """Return human-readable bytes."""
    if n < 1024:
        return f"{n} B"
    if n < 1024**2:
        return f"{n / 1024:.1f} KB"
    if n < 1024**3:
        return f"{n / 1024 ** 2:.1f} MB"
    return f"{n / 1024 ** 3:.2f} GB"


def get_peak_ram_mb() -> float | None:
    """Return peak RAM in MB if psutil is available."""
    try:
        import psutil

        process = psutil.Process(os.getpid())
        return process.memory_info().peak_wset / (1024 * 1024)
    except Exception:
        return None


def get_hf_cache_path() -> Path:
    """Return the Hugging Face hub cache directory."""
    return Path(os.environ.get("HF_HOME", Path.home() / ".cache" / "huggingface" / "hub"))


def estimate_model_cache_size(model_name: str, cache_root: Path) -> int | None:
    """Estimate on-disk cache size for a model under the HF hub cache."""
    safe_name = model_name.replace("/", "--")
    cache_dir = cache_root / f"models--{safe_name}"
    if not cache_dir.exists():
        return None

    total = 0
    for path in cache_dir.rglob("*"):
        if path.is_file():
            total += path.stat().st_size
    return total


def load_model(model_name: str, device: str) -> tuple[object, float]:
    """Load a NeMo ASR model and return the model plus load time."""
    import nemo.collections.asr as nemo_asr

    start = time.perf_counter()
    model = nemo_asr.models.ASRModel.from_pretrained(model_name=model_name, map_location=device)
    load_time = time.perf_counter() - start
    return model, load_time


def transcribe(model: object, audio_path: Path) -> tuple[str, float]:
    """Transcribe a WAV file and return transcript plus elapsed seconds."""
    import torch

    start = time.perf_counter()
    with torch.no_grad():
        raw_results = model.transcribe([str(audio_path)], batch_size=1)
    transcribe_time = time.perf_counter() - start

    if isinstance(raw_results, list):
        result = raw_results[0] if raw_results else ""
        text = result.text if hasattr(result, "text") else result
    else:
        text = raw_results

    return str(text).strip(), transcribe_time


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Parakeet batch transcription spike")
    parser.add_argument("--audio", required=True, help="Path to a local WAV file")
    parser.add_argument(
        "--device",
        choices=["auto", "cpu", "cuda"],
        default="auto",
        help="Inference device",
    )
    parser.add_argument(
        "--model",
        default="nvidia/parakeet-tdt-0.6b-v3",
        help="NeMo ASR model name or Hugging Face identifier",
    )
    args = parser.parse_args(argv)

    audio_path = Path(args.audio)
    if not audio_path.exists():
        print(f"ERROR: Audio file not found: {audio_path}", file=sys.stderr)
        return 1

    try:
        import torch
    except ImportError as exc:
        print("ERROR: Install Parakeet dependencies with `uv sync --extra nemo`.", file=sys.stderr)
        print(str(exc), file=sys.stderr)
        return 1

    device = resolve_device(args.device)
    print("=" * 60)
    print("OpenWhisper Parakeet Batch Spike")
    print("=" * 60)
    print(f"Model name : {args.model}")
    print(f"Device pref: {args.device}")
    print(f"Resolved   : {device}")
    print(f"CUDA avail : {torch.cuda.is_available()}")
    print(f"Audio file : {audio_path.resolve()}")

    duration = get_audio_duration(audio_path)
    print(f"Audio dur  : {duration:.2f} s" if duration is not None else "Audio dur  : unknown")
    print("-" * 60)

    print("Loading model...")
    try:
        model, load_time = load_model(args.model, device)
    except Exception as exc:
        print(f"ERROR: Model load failed: {exc}", file=sys.stderr)
        return 1

    print(f"Load time  : {load_time:.2f} s")

    cache_root = get_hf_cache_path()
    cache_size = estimate_model_cache_size(args.model, cache_root)
    if cache_size is not None:
        print(f"Cache size : {format_bytes(cache_size)} ({cache_size:,} bytes)")
        print(f"Cache path : {cache_root}")
    else:
        print("Cache size : not found")

    print("-" * 60)
    print("Transcribing...")
    try:
        transcript, transcribe_time = transcribe(model, audio_path)
    except Exception as exc:
        print(f"ERROR: Transcription failed: {exc}", file=sys.stderr)
        return 1

    print(f"Transcribe : {transcribe_time:.2f} s")
    if duration and duration > 0:
        print(f"RT factor  : {transcribe_time / duration:.3f}x")
    print("-" * 60)

    peak_ram = get_peak_ram_mb()
    print(f"Peak RAM   : {peak_ram:.1f} MB" if peak_ram is not None else "Peak RAM   : not available")

    if torch.cuda.is_available():
        try:
            peak_vram = torch.cuda.max_memory_allocated(device) / (1024 * 1024)
            print(f"Peak VRAM  : {peak_vram:.1f} MB")
        except Exception:
            print("Peak VRAM  : not available")

    print("=" * 60)
    print("TRANSCRIPT")
    print("=" * 60)
    print(transcript)
    print("=" * 60)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
