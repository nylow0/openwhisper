"""Parakeet batch transcription spike.

Loads nvidia/parakeet-tdt-0.6b-v3 (or another NeMo ASR model) and transcribes
a single WAV file in batch mode. Prints timing, memory, and cache metrics.

Usage:
    uv run python scripts/parakeet_batch_spike.py --audio test_data/samples/sample-clean-short.wav
    uv run python scripts/parakeet_batch_spike.py --audio test.wav --device cpu
    uv run python scripts/parakeet_batch_spike.py --audio test.wav --model nvidia/parakeet-tdt-0.6b-v3
"""

from __future__ import annotations

import argparse
import os
import sys
import time
import warnings
from pathlib import Path
from typing import Optional

import torch
import torchaudio


def resolve_device(preference: str) -> str:
    """Resolve auto -> cuda or cpu."""
    if preference == "auto":
        return "cuda" if torch.cuda.is_available() else "cpu"
    return preference


def get_audio_duration(audio_path: Path) -> Optional[float]:
    """Return audio duration in seconds."""
    try:
        info = torchaudio.info(str(audio_path))
        return info.num_frames / info.sample_rate
    except Exception:
        pass
    # Fallback: scipy
    try:
        from scipy.io import wavfile
        sr, data = wavfile.read(str(audio_path))
        return data.shape[0] / sr
    except Exception:
        return None


def format_bytes(n: int) -> str:
    """Human-readable bytes."""
    if n < 1024:
        return f"{n} B"
    if n < 1024**2:
        return f"{n / 1024:.1f} KB"
    if n < 1024**3:
        return f"{n / 1024 ** 2:.1f} MB"
    return f"{n / 1024 ** 3:.2f} GB"


def get_peak_ram_mb() -> Optional[float]:
    """Return peak RAM in MB if available (Windows fallback via os)."""
    try:
        import psutil
        process = psutil.Process(os.getpid())
        return process.memory_info().peak_wset / (1024 * 1024)
    except Exception:
        return None


def get_hf_cache_path() -> Path:
    """Return the Hugging Face hub cache directory."""
    return Path(os.environ.get("HF_HOME", Path.home() / ".cache" / "huggingface" / "hub"))


def estimate_model_cache_size(model_name: str, cache_root: Path) -> Optional[int]:
    """Estimate on-disk cache size for a model under the HF hub cache."""
    # HF cache stores models under models--<org>--<repo>
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
    """Load a NeMo ASR model and return (model, load_time_seconds)."""
    import nemo.collections.asr as nemo_asr

    start = time.perf_counter()
    model = nemo_asr.models.ASRModel.from_pretrained(model_name=model_name, map_location=device)
    load_time = time.perf_counter() - start
    return model, load_time


def transcribe(model, audio_path: Path, device: str) -> tuple[str, float]:
    """Transcribe a WAV file and return (transcript, transcribe_time_seconds)."""
    # NeMo's transcribe() handles loading and resampling when given file paths.
    # This is more reliable than manual tensor preprocessing.
    start = time.perf_counter()
    with torch.no_grad():
        results = model.transcribe([str(audio_path)], batch_size=1)
    transcribe_time = time.perf_counter() - start

    # Results may be a list of strings or a list of objects
    if isinstance(results, list):
        text = results[0] if len(results) > 0 else ""
        if hasattr(text, "text"):
            text = text.text
    else:
        text = str(results)

    return str(text).strip(), transcribe_time


def main() -> int:
    parser = argparse.ArgumentParser(description="Parakeet batch transcription spike")
    parser.add_argument("--audio", required=True, help="Path to a local WAV file")
    parser.add_argument(
        "--device",
        choices=["auto", "cpu", "cuda"],
        default="auto",
        help="Inference device (default: auto)",
    )
    parser.add_argument(
        "--model",
        default="nvidia/parakeet-tdt-0.6b-v3",
        help="NeMo ASR model name or Hugging Face identifier (default: nvidia/parakeet-tdt-0.6b-v3)",
    )
    args = parser.parse_args()

    audio_path = Path(args.audio)
    if not audio_path.exists():
        print(f"ERROR: Audio file not found: {audio_path}", file=sys.stderr)
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
    if duration is not None:
        print(f"Audio dur  : {duration:.2f} s")
    else:
        print("Audio dur  : unknown")
    print("-" * 60)

    # --- Load model ---
    print("Loading model...")
    try:
        model, load_time = load_model(args.model, device)
    except Exception as exc:
        print(f"ERROR: Model load failed: {exc}", file=sys.stderr)
        return 1

    print(f"Load time  : {load_time:.2f} s")

    # --- Cache size ---
    cache_root = get_hf_cache_path()
    cache_size = estimate_model_cache_size(args.model, cache_root)
    if cache_size is not None:
        print(f"Cache size : {format_bytes(cache_size)} ({cache_size:,} bytes)")
        print(f"Cache path : {cache_root}")
    else:
        print("Cache size : not found (model may not have been downloaded yet)")

    # --- Transcribe ---
    print("-" * 60)
    print("Transcribing...")
    try:
        transcript, transcribe_time = transcribe(model, audio_path, device)
    except Exception as exc:
        print(f"ERROR: Transcription failed: {exc}", file=sys.stderr)
        return 1

    print(f"Transcribe : {transcribe_time:.2f} s")
    if duration and duration > 0:
        rtf = transcribe_time / duration
        print(f"RT factor  : {rtf:.3f}x")
    print("-" * 60)

    # --- Memory ---
    peak_ram = get_peak_ram_mb()
    if peak_ram is not None:
        print(f"Peak RAM   : {peak_ram:.1f} MB")
    else:
        print("Peak RAM   : not available (install psutil for this metric)")

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
    sys.exit(main())
