from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

from openwhisper_asr.engines.whisper_cpp import (
    WhisperCppError,
    WhisperCppEngine,
    WhisperCppProfile,
    WhisperCppSelection,
    calculate_whisper_cpp_language_score,
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


def test_calculate_whisper_cpp_language_score_averages_token_probabilities() -> None:
    payload: dict[str, object] = {
        "transcription": [
            {"tokens": [{"text": "hello", "p": 0.25}, {"text": "world", "probability": 0.75}]},
        ],
    }

    assert calculate_whisper_cpp_language_score(payload) == 0.5


def test_transcribe_batch_forces_single_spoken_language_without_auto(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    engine = _loaded_engine(tmp_path, spoken_languages=("pl",), profile_args=("-l", "auto", "-oj"))
    audio_path = _audio_file(tmp_path)
    commands: list[list[str]] = []

    def fake_run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        commands.append(command)
        _write_whisper_output(command, {"result": {"language": "pl"}, "transcription": [{"text": " czesc"}]})
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr("openwhisper_asr.engines.whisper_cpp.subprocess.run", fake_run)

    result = engine.transcribe_batch(str(audio_path))

    assert result.language == "pl"
    assert commands[0][commands[0].index("-l") + 1] == "pl"
    assert "auto" not in commands[0]


def test_transcribe_batch_chooses_best_allowed_language_before_final_transcription(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    engine = _loaded_engine(tmp_path, spoken_languages=("en", "pl"), profile_args=("-l", "auto", "-oj"))
    audio_path = _audio_file(tmp_path)
    scored_languages: list[str] = []
    commands: list[list[str]] = []

    def fake_score(
        selection: WhisperCppSelection,
        audio: Path,
        temp_dir: Path,
        language: str,
    ) -> float | None:
        scored_languages.append(language)
        return {"en": 0.2, "pl": 0.9}[language]

    def fake_run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        commands.append(command)
        _write_whisper_output(command, {"result": {"language": "pl"}, "transcription": [{"text": " czesc"}]})
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr(engine, "_score_spoken_language", fake_score)
    monkeypatch.setattr("openwhisper_asr.engines.whisper_cpp.subprocess.run", fake_run)

    result = engine.transcribe_batch(str(audio_path))

    assert result.language == "pl"
    assert scored_languages == ["en", "pl"]
    assert commands[0][commands[0].index("-l") + 1] == "pl"
    assert all("auto" not in command for command in commands)
    assert commands[0][commands[0].index("-l") + 1] in ("en", "pl")


def test_transcribe_batch_language_scoring_passes_force_candidates_without_auto(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    engine = _loaded_engine(tmp_path, spoken_languages=("en", "pl"), profile_args=("-l", "auto", "-oj"))
    audio_path = _audio_file(tmp_path)
    commands: list[list[str]] = []

    def fake_run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        commands.append(command)
        language = command[command.index("-l") + 1]
        output_base = Path(command[command.index("-of") + 1])
        if output_base.name.startswith("language-score-"):
            probability = 0.9 if language == "pl" else 0.2
            _write_whisper_output(
                command,
                {"transcription": [{"tokens": [{"text": "token", "p": probability}]}]},
            )
        else:
            _write_whisper_output(command, {"result": {"language": language}, "transcription": [{"text": " czesc"}]})
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr("openwhisper_asr.engines.whisper_cpp.subprocess.run", fake_run)

    result = engine.transcribe_batch(str(audio_path))

    languages = [command[command.index("-l") + 1] for command in commands]
    assert result.language == "pl"
    assert languages == ["en", "pl", "pl"]
    assert all(language in ("en", "pl") for language in languages)
    assert all("auto" not in command for command in commands)
    assert all(command[command.index("-d") + 1] == "6000" for command in commands[:2])
    assert all("-ojf" in command for command in commands[:2])


def test_transcribe_batch_falls_back_to_first_language_when_scoring_times_out(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    engine = _loaded_engine(tmp_path, spoken_languages=("en", "pl"), profile_args=("-l", "auto", "-oj"))
    audio_path = _audio_file(tmp_path)
    commands: list[list[str]] = []

    def fake_run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        output_base = Path(command[command.index("-of") + 1])
        if output_base.name.startswith("language-score-"):
            raise subprocess.TimeoutExpired(cmd=command, timeout=60)

        commands.append(command)
        _write_whisper_output(command, {"result": {"language": "en"}, "transcription": [{"text": " hello"}]})
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr("openwhisper_asr.engines.whisper_cpp.subprocess.run", fake_run)

    result = engine.transcribe_batch(str(audio_path))

    assert result.language == "en"
    assert commands[0][commands[0].index("-l") + 1] == "en"
    assert "auto" not in commands[0]


def test_transcribe_batch_uses_forced_language_when_output_language_is_missing(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    engine = _loaded_engine(tmp_path, spoken_languages=("pl",), profile_args=("-l", "auto", "-oj"))
    audio_path = _audio_file(tmp_path)

    def fake_run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        _write_whisper_output(command, {"result": {}, "transcription": [{"text": " czesc"}]})
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr("openwhisper_asr.engines.whisper_cpp.subprocess.run", fake_run)

    result = engine.transcribe_batch(str(audio_path))

    assert result.language == "pl"


def test_transcribe_batch_rejects_output_language_outside_allowed_list(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    engine = _loaded_engine(tmp_path, spoken_languages=("pl",), profile_args=("-l", "auto", "-oj"))
    audio_path = _audio_file(tmp_path)

    def fake_run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        _write_whisper_output(command, {"result": {"language": "ja"}, "transcription": [{"text": " hai"}]})
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr("openwhisper_asr.engines.whisper_cpp.subprocess.run", fake_run)

    with pytest.raises(WhisperCppError, match="outside configured spoken languages: ja"):
        engine.transcribe_batch(str(audio_path))


def test_transcribe_batch_rejects_supported_language_outside_configured_spoken_languages(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    engine = _loaded_engine(tmp_path, spoken_languages=("pl",), profile_args=("-l", "auto", "-oj"))
    audio_path = _audio_file(tmp_path)

    def fake_run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        _write_whisper_output(command, {"result": {"language": "en"}, "transcription": [{"text": " hello"}]})
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr("openwhisper_asr.engines.whisper_cpp.subprocess.run", fake_run)

    with pytest.raises(WhisperCppError, match="outside configured spoken languages: en"):
        engine.transcribe_batch(str(audio_path))


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


def _loaded_engine(
    tmp_path: Path,
    spoken_languages: tuple[str, ...],
    profile_args: tuple[str, ...],
) -> WhisperCppEngine:
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
    engine = WhisperCppEngine(config_path=config_path, spoken_languages=spoken_languages)
    profile = WhisperCppProfile(
        name="cpu_avx_vnni_medium_en_q8",
        device="cpu",
        binary_hint="cpu_avx_vnni",
        model_key="medium_en_q8",
        args=profile_args,
    )
    engine._selection = WhisperCppSelection(
        profile=profile,
        exe_path=tmp_path / "whisper-cli.exe",
        model_path=tmp_path / "model.bin",
        memory_mb=0.0,
    )
    return engine


def _audio_file(tmp_path: Path) -> Path:
    audio_path = tmp_path / "audio.wav"
    audio_path.write_bytes(b"RIFF")
    return audio_path


def _write_whisper_output(command: list[str], payload: dict[str, object]) -> None:
    output_base = Path(command[command.index("-of") + 1])
    output_base.with_suffix(".json").write_text(json.dumps(payload), encoding="utf-8")
