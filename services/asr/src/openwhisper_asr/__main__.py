"""ASR Worker entry point."""

import sys
import json
import logging
import time
import threading
from typing import Dict, Any, Optional

from openwhisper_asr.protocol import (
    read_messages,
    send_message,
    send_error,
    send_health_ok,
    send_transcript_partial,
    send_transcript_final,
    WordResult,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger(__name__)

# ─── Mock Transcript Generator ───

MOCK_SENTENCES = [
    "Hello world this is a test of the OpenWhisper dictation system",
    "The quick brown fox jumps over the lazy dog",
    "Speech recognition is working correctly in offline mode",
    "This is a mock transcript for testing the protocol layer",
    "OpenWhisper runs entirely on your local machine for privacy",
]


class MockTranscriptGenerator:
    """Generates mock transcript events for protocol testing."""

    def __init__(self) -> None:
        self._running = False
        self._thread: Optional[threading.Thread] = None
        self._sentence_idx = 0

    def start(self) -> None:
        """Start generating mock transcripts."""
        if self._running:
            return
        self._running = True
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()
        logger.info("Mock transcript generator started")

    def stop(self) -> None:
        """Stop generating mock transcripts."""
        self._running = False
        if self._thread:
            self._thread.join(timeout=1.0)
        logger.info("Mock transcript generator stopped")

    def _run(self) -> None:
        """Background thread that sends mock transcripts."""
        while self._running:
            sentence = MOCK_SENTENCES[self._sentence_idx % len(MOCK_SENTENCES)]
            words = sentence.split()

            # Send partial transcripts word by word
            for i in range(1, len(words) + 1):
                if not self._running:
                    return
                partial_text = " ".join(words[:i])
                send_transcript_partial(partial_text, processing_latency_ms=50)
                time.sleep(0.3)

            # Send final transcript
            if self._running:
                word_results = []
                start_ms = 0
                for w in words:
                    duration = max(100, len(w) * 80)
                    word_results.append(
                        WordResult(
                            text=w,
                            start_ms=start_ms,
                            end_ms=start_ms + duration,
                            confidence=0.92,
                        )
                    )
                    start_ms += duration + 50

                send_transcript_final(
                    text=sentence,
                    words=word_results,
                    language="en",
                    processing_latency_ms=120,
                )
                time.sleep(1.0)

            self._sentence_idx += 1


# ─── Message Handler ───

def handle_message(msg: Dict[str, Any], generator: MockTranscriptGenerator) -> Optional[Dict[str, Any]]:
    """Handle incoming message from Rust."""
    msg_type = msg.get("type")

    if msg_type == "health.check":
        send_health_ok(
            timestamp=msg.get("timestamp", int(time.time())),
            status="ready",
        )
        return None

    if msg_type == "dictation.start":
        logger.info("Dictation start requested")
        generator.start()
        return None

    if msg_type == "dictation.stop":
        logger.info("Dictation stop requested")
        generator.stop()
        return None

    if msg_type == "shutdown":
        logger.info("Shutdown requested")
        generator.stop()
        return None

    if msg_type == "model.load":
        logger.info("Model load requested: %s", msg)
        # Mock: pretend model loaded successfully
        send_message({
            "type": "model.loaded",
            "device": msg.get("device", "cpu"),
            "memory_mb": 512.0,
        })
        return None

    if msg_type == "settings.update":
        logger.info("Settings update: %s", msg)
        return None

    if msg_type == "audio.chunk":
        # In mock mode, we ignore audio chunks
        return None

    logger.warning("Unknown message type: %s", msg_type)
    send_error(
        message=f"Unknown message type: {msg_type}",
        recoverable=True,
        code="PROTOCOL_ERROR",
    )
    return None


def main() -> None:
    """Main entry point."""
    logger.info("ASR Worker starting...")

    generator = MockTranscriptGenerator()

    try:
        for msg in read_messages():
            handle_message(msg, generator)
            if msg.get("type") == "shutdown":
                break
    except KeyboardInterrupt:
        logger.info("Interrupted by user")
    finally:
        generator.stop()

    logger.info("ASR Worker shutting down...")


if __name__ == "__main__":
    main()
