"""Command line interface for OpenWhisper ASR core."""

from __future__ import annotations

import argparse
import json
import sys
from typing import Sequence

from openwhisper_asr.engine import ASREngine, MockASREngine
from openwhisper_asr.engines.whisper_cpp import WhisperCppEngine, list_profile_names
from openwhisper_asr.protocol import send_transcript_final
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

    if args.command == "transcribe":
        asr_engine: ASREngine
        if args.engine == "mock":
            asr_engine = MockASREngine()
        else:
            asr_engine = WhisperCppEngine(profile=args.profile, timeout_seconds=args.timeout)
        asr_engine.load_model(model=args.model, device=args.device)
        result = asr_engine.transcribe_batch(args.audio)
        if args.json:
            payload = {
                "text": result.text,
                "language": result.language,
                "is_partial": result.is_partial,
                "processing_latency_ms": result.processing_latency_ms,
                "words": [word.__dict__ for word in result.words],
            }
            print(json.dumps(payload), flush=True)
        else:
            print(result.text, flush=True)
        return 0

    parser.print_help(sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
