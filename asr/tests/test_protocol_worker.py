from __future__ import annotations

import json
from io import StringIO

from openwhisper_asr.protocol import send_health_ok
from openwhisper_asr.worker.stdio import run_worker


def test_send_health_ok_writes_ndjson() -> None:
    output = StringIO()

    send_health_ok(timestamp=123, output_stream=output)

    payload = json.loads(output.getvalue())
    assert payload == {"type": "health.ok", "timestamp": 123, "status": "ready"}


def test_worker_responds_to_health_check() -> None:
    input_stream = StringIO('{"type":"health.check","timestamp":7}\n{"type":"shutdown"}\n')
    output_stream = StringIO()

    exit_code = run_worker(input_stream=input_stream, output_stream=output_stream)

    assert exit_code == 0
    lines = [json.loads(line) for line in output_stream.getvalue().splitlines()]
    assert lines[0] == {"type": "health.ok", "timestamp": 7, "status": "ready"}
