"""Command line interface for OpenWhisper ASR core."""

from __future__ import annotations

import argparse
import json
import sys
from typing import Sequence

from openwhisper_asr.engine import MockASREngine
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

    transcribe_parser = subparsers.add_parser("transcribe", help="Transcribe an audio file with the mock engine")
    transcribe_parser.add_argument("audio", help="Path to the audio file")
    transcribe_parser.add_argument("--json", action="store_true", help="Print full JSON result")

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.command is None or args.command == "serve":
        return run_worker()

    if args.command == "mock":
        engine = MockASREngine(text=args.text)
        engine.load_model()
        result = engine.transcribe_batch("mock.wav")
        send_transcript_final(
            text=result.text,
            words=result.words,
            language=result.language,
            processing_latency_ms=result.processing_latency_ms,
        )
        return 0

    if args.command == "transcribe":
        engine = MockASREngine()
        engine.load_model()
        result = engine.transcribe_batch(args.audio)
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
