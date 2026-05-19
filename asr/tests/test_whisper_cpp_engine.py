from __future__ import annotations

import json
from pathlib import Path

import pytest

from openwhisper_asr.engines.whisper_cpp import (
    WhisperCppEngine,
    WhisperCppProfile,
    WhisperCppSelection,
    force_whisper_cpp_language_args,
    parse_whisper_cpp_payload,
)


def test_parse_whisper_cpp_payload_returns_final_text() -> None:
    payload: dict[str, object] = {
        "result": {"language": "en"},
        "transcription": [
            {"text": " Concorde returned"},
            {"text": " to its place."},
        ],
    }

    result = parse_whisper_cpp_payload(payload, processing_latency_ms=1234)

    assert result.text == "Concorde returned to its place."
    assert result.language == "en"
    assert result.is_partial is False
    assert result.processing_latency_ms == 1234


def test_force_whisper_cpp_language_args_replaces_short_language() -> None:
    args = force_whisper_cpp_language_args(["-l", "auto", "-nt"], "pl")

    assert args == ("-nt", "-l", "pl")


def test_force_whisper_cpp_language_args_replaces_long_language_without_auto() -> None:
    args = force_whisper_cpp_language_args(["--language", "auto", "-oj"], "en")

    assert args == ("-oj", "-l", "en")
    assert "auto" not in args


def test_force_whisper_cpp_language_args_adds_missing_language() -> None:
    args = force_whisper_cpp_language_args(["-nt", "-oj"], "de")

    assert args == ("-nt", "-oj", "-l", "de")


def test_cpu_subprocess_env_caps_openblas_threads(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    config_path = tmp_path / "config" / "whispercpp-profiles.json"
    config_path.parent.mkdir()
    config_path.write_text(
        json.dumps(
            {
                "binary_hints": {
                    "cpu_avx_vnni": {
                        "env": "OPENWHISPER_WHISPERCPP_CPU_EXE",
                    },
                },
                "profiles": {},
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setenv("OPENBLAS_NUM_THREADS", "8")
    profile = WhisperCppProfile(
        name="cpu_avx_vnni_medium_en_q8",
        device="cpu",
        binary_hint="cpu_avx_vnni",
        model_key="medium_en_q8",
        args=(),
    )
    selection = WhisperCppSelection(
        profile=profile,
        exe_path=tmp_path / "whisper-cli.exe",
        model_path=tmp_path / "model.bin",
        memory_mb=0.0,
    )

    env = WhisperCppEngine(config_path=config_path)._subprocess_env(selection)

    assert env["OPENBLAS_NUM_THREADS"] == "1"
