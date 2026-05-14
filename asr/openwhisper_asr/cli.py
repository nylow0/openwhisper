"""Command line interface for OpenWhisper ASR core."""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Callable, Sequence

from openwhisper_asr.engine import ASREngine, MockASREngine
from openwhisper_asr.engines.whisper_cpp import WhisperCppEngine, list_profile_names
from openwhisper_asr.protocol import send_transcript_final
from openwhisper_asr.recording import (
    DEFAULT_CHANNELS,
    DEFAULT_SAMPLE_RATE_HZ,
    default_recording_path,
    list_input_devices,
    record_wav,
)
from openwhisper_asr.types import TranscriptionResult
from openwhisper_asr.worker.stdio import run_worker


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="ow-asr", description="OpenWhisper ASR core tools")
    subparsers = parser.add_subparsers(dest="command")

    subparsers.add_parser("serve", help="Run the NDJSON stdio worker")

    mock_parser = subparsers.add_parser("mock", help="Emit one mock transcript JSON event")
    mock_parser.add_argument(
        "--text",
        default="Hello world this is a test of the OpenWhisper dictation system",
        help="Transcript text to emit",
    )

    subparsers.add_parser("profiles", help="List configured whisper.cpp profiles")
    subparsers.add_parser("devices", help="List audio input devices")

    record_parser = subparsers.add_parser("record", help="Record microphone audio to a WAV file")
    record_parser.add_argument("output", nargs="?", help="Output WAV path")
    record_parser.add_argument("--seconds", type=float, default=5.0, help="Recording duration")
    record_parser.add_argument("--sample-rate", type=int, default=DEFAULT_SAMPLE_RATE_HZ)
    record_parser.add_argument("--channels", type=int, default=DEFAULT_CHANNELS)
    record_parser.add_argument("--device", help="Input device index or name")
    record_parser.add_argument("--json", action="store_true", help="Print JSON result")
    record_parser.add_argument("--transcribe", action="store_true", help="Transcribe after recording")
    record_parser.add_argument("--model", default="medium_en_q8", help="Model key or alias: base, medium")
    record_parser.add_argument("--asr-device", choices=["auto", "cpu", "gpu"], default="auto")
    record_parser.add_argument("--profile", help="Exact whisper.cpp profile name")
    record_parser.add_argument("--timeout", type=int, default=300, help="whisper.cpp timeout in seconds")

    transcribe_parser = subparsers.add_parser("transcribe", help="Transcribe an audio file")
    transcribe_parser.add_argument("audio", help="Path to the audio file")
    transcribe_parser.add_argument("--engine", choices=["whispercpp", "mock"], default="whispercpp")
    transcribe_parser.add_argument("--model", default="medium_en_q8", help="Model key or alias: base, medium")
    transcribe_parser.add_argument("--device", choices=["auto", "cpu", "gpu"], default="auto")
    transcribe_parser.add_argument("--profile", help="Exact whisper.cpp profile name")
    transcribe_parser.add_argument("--timeout", type=int, default=300, help="whisper.cpp timeout in seconds")
    transcribe_parser.add_argument("--json", action="store_true", help="Print full JSON result")

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.command is None or args.command == "serve":
        return run_worker()

    if args.command == "mock":
        mock_engine = MockASREngine(text=args.text)
        mock_engine.load_model()
        result = mock_engine.transcribe_batch("mock.wav")
        send_transcript_final(
            text=result.text,
            words=result.words,
            language=result.language,
            processing_latency_ms=result.processing_latency_ms,
        )
        return 0

    if args.command == "profiles":
        print(json.dumps({"profiles": list_profile_names()}, indent=2), flush=True)
        return 0

    if args.command == "devices":
        return _run_cli_action(lambda: print(json.dumps({"input_devices": list_input_devices()}, indent=2), flush=True))

    if args.command == "record":
        return _run_cli_action(lambda: _record_command(args))

    if args.command == "transcribe":
        return _run_cli_action(lambda: _transcribe_command(args))

    parser.print_help(sys.stderr)
    return 2


def _record_command(args: argparse.Namespace) -> None:
    output = args.output or default_recording_path()
    device = _device_arg(args.device)
    recording = record_wav(
        output_path=output,
        duration_seconds=args.seconds,
        sample_rate_hz=args.sample_rate,
        channels=args.channels,
        device=device,
    )

    payload: dict[str, object] = {
        "path": str(recording.path),
        "duration_seconds": recording.duration_seconds,
        "sample_rate_hz": recording.sample_rate_hz,
        "channels": recording.channels,
        "frames": recording.frames,
    }

    if args.transcribe:
        result = _transcribe(
            audio=str(recording.path),
            engine_name="whispercpp",
            model=args.model,
            device=args.asr_device,
            profile=args.profile,
            timeout=args.timeout,
        )
        payload["transcript"] = _transcription_payload(result)

    if args.json or args.transcribe:
        print(json.dumps(payload), flush=True)
    else:
        print(f"Recorded {recording.path}", flush=True)


def _transcribe_command(args: argparse.Namespace) -> None:
    result = _transcribe(
        audio=args.audio,
        engine_name=args.engine,
        model=args.model,
        device=args.device,
        profile=args.profile,
        timeout=args.timeout,
    )
    if args.json:
        print(json.dumps(_transcription_payload(result)), flush=True)
    else:
        print(result.text, flush=True)


def _transcribe(
    audio: str,
    engine_name: str,
    model: str,
    device: str,
    profile: str | None,
    timeout: int,
) -> TranscriptionResult:
    asr_engine: ASREngine
    if engine_name == "mock":
        asr_engine = MockASREngine()
    else:
        asr_engine = WhisperCppEngine(profile=profile, timeout_seconds=timeout)
    asr_engine.load_model(model=model, device=device)
    return asr_engine.transcribe_batch(audio)


def _transcription_payload(result: TranscriptionResult) -> dict[str, object]:
    return {
        "text": result.text,
        "language": result.language,
        "is_partial": result.is_partial,
        "processing_latency_ms": result.processing_latency_ms,
        "words": [word.__dict__ for word in result.words],
    }


def _device_arg(value: str | None) -> int | str | None:
    if value is None:
        return None
    try:
        return int(value)
    except ValueError:
        return value


def _run_cli_action(action: Callable[[], None]) -> int:
    try:
        action()
    except Exception as exc:
        print(f"ERROR: {exc}", file=sys.stderr, flush=True)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
