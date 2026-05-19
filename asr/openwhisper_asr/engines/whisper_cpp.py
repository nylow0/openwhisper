"""whisper.cpp engine integration."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import time
from dataclasses import dataclass, replace
from pathlib import Path
from typing import cast

from openwhisper_asr.engine import ASREngine
from openwhisper_asr.types import TranscriptionResult

JsonDict = dict[str, object]

SUPPORTED_SPOKEN_LANGUAGES = frozenset(
    {
        "en",
        "zh",
        "de",
        "es",
        "ru",
        "ko",
        "fr",
        "ja",
        "pt",
        "tr",
        "pl",
        "ca",
        "nl",
        "ar",
        "sv",
        "it",
        "id",
        "hi",
        "fi",
        "vi",
        "he",
        "uk",
        "el",
        "ms",
        "cs",
        "ro",
        "da",
        "hu",
        "ta",
        "no",
        "th",
        "ur",
        "hr",
        "bg",
        "lt",
        "la",
        "mi",
        "ml",
        "cy",
        "sk",
        "te",
        "fa",
        "lv",
        "bn",
        "sr",
        "az",
        "sl",
        "kn",
        "et",
        "mk",
        "br",
        "eu",
        "is",
        "hy",
        "ne",
        "mn",
        "bs",
        "kk",
        "sq",
        "sw",
        "gl",
        "mr",
        "pa",
        "si",
        "km",
        "sn",
        "yo",
        "so",
        "af",
        "oc",
        "ka",
        "be",
        "tg",
        "sd",
        "gu",
        "am",
        "yi",
        "lo",
        "uz",
        "fo",
        "ht",
        "ps",
        "tk",
        "nn",
        "mt",
        "sa",
        "lb",
        "my",
        "bo",
        "tl",
        "mg",
        "as",
        "tt",
        "haw",
        "ln",
        "ha",
        "ba",
        "jw",
        "su",
        "yue",
    }
)

_MODEL_ALIASES = {
    "medium": "medium_en_q8",
    "medium-q8": "medium_en_q8",
    "medium_en_q8": "medium_en_q8",
    "turbo": "large_v3_turbo_q8",
    "large-v3-turbo": "large_v3_turbo_q8",
    "large-v3-turbo-q8": "large_v3_turbo_q8",
    "large_v3_turbo_q8": "large_v3_turbo_q8",
}

_LOCAL_BINARY_CANDIDATES = {
    "cpu_avx_vnni": (
        ".local/whispercpp-src/build-cpu-avxvnni-local/bin/whisper-cli.exe",
        ".local/whispercpp/bin/Release/whisper-cli.exe",
    ),
    "gpu_cuda_sm120": (
        ".local/whispercpp-src/build-cuda-sm120-local/bin/whisper-cli.exe",
        ".local/whispercpp/cuda-bin/Release/whisper-cli.exe",
    ),
}

_LOCAL_PATH_CANDIDATES = {
    "gpu_cuda_sm120": (
        ".local/tools/cuda-13.2/toolkit/bin",
        ".local/tools/cuda-13.2/toolkit/bin/x64",
        ".local/whispercpp-src/build-cuda-sm120-local/bin",
        ".local/whispercpp/cuda-bin/Release",
    ),
}


class WhisperCppError(RuntimeError):
    """Raised when whisper.cpp cannot be configured or run."""


@dataclass(frozen=True)
class WhisperCppProfile:
    name: str
    device: str
    binary_hint: str
    model_key: str
    args: tuple[str, ...]


@dataclass(frozen=True)
class WhisperCppSelection:
    profile: WhisperCppProfile
    exe_path: Path
    model_path: Path
    memory_mb: float


def default_config_path() -> Path:
    return Path(__file__).resolve().parents[2] / "config" / "whispercpp-profiles.json"


def list_profile_names(config_path: Path | None = None) -> list[str]:
    config = _load_json(config_path or default_config_path())
    profiles = _as_object(config.get("profiles"), "profiles")
    return sorted(profiles.keys())


def parse_whisper_cpp_payload(payload: JsonDict, processing_latency_ms: int) -> TranscriptionResult:
    result = _as_object(payload.get("result", {}), "result")
    language = _optional_string(result.get("language"))
    transcription = _as_list(payload.get("transcription", []), "transcription")

    parts: list[str] = []
    for item in transcription:
        segment = _as_object(item, "transcription item")
        text = _optional_string(segment.get("text"))
        if text:
            stripped = text.strip()
            if stripped:
                parts.append(stripped)

    return TranscriptionResult(
        text=" ".join(parts),
        words=[],
        language=language,
        is_partial=False,
        processing_latency_ms=processing_latency_ms,
    )


def calculate_whisper_cpp_language_score(payload: JsonDict) -> float | None:
    probabilities: list[float] = []
    _collect_token_probabilities(payload, probabilities)
    if not probabilities:
        return None
    return sum(probabilities) / len(probabilities)


def remove_whisper_cpp_language_args(args: tuple[str, ...] | list[str]) -> tuple[str, ...]:
    cleaned: list[str] = []
    skip_next = False
    for arg in args:
        if skip_next:
            skip_next = False
            continue
        if arg in {"-l", "--language"}:
            skip_next = True
            continue
        cleaned.append(arg)
    return tuple(cleaned)


def force_whisper_cpp_language_args(args: tuple[str, ...] | list[str], language: str) -> tuple[str, ...]:
    return (*remove_whisper_cpp_language_args(args), "-l", language)


def short_whisper_cpp_scoring_args(args: tuple[str, ...] | list[str], language: str) -> tuple[str, ...]:
    cleaned = _remove_whisper_cpp_duration_args(force_whisper_cpp_language_args(args, language))
    return (*cleaned, "-d", "6000", "-ojf")


class WhisperCppEngine(ASREngine):
    """Batch transcription engine backed by whisper.cpp's CLI."""

    def __init__(
        self,
        config_path: str | Path | None = None,
        profile: str | None = None,
        timeout_seconds: int = 300,
        spoken_languages: tuple[str, ...] = ("en",),
        auto_detect_language: bool = False,
    ) -> None:
        self._config_path = Path(config_path).resolve() if config_path is not None else default_config_path()
        self._asr_root = self._config_path.parents[1]
        self._config = _load_json(self._config_path)
        self._profile_name = profile
        self._timeout_seconds = timeout_seconds
        self._spoken_languages = _normalize_spoken_languages(spoken_languages)
        self._auto_detect_language = auto_detect_language
        self._selection: WhisperCppSelection | None = None

    @property
    def loaded_profile_name(self) -> str | None:
        return self._selection.profile.name if self._selection is not None else None

    @property
    def loaded_device(self) -> str | None:
        return self._selection.profile.device if self._selection is not None else None

    @property
    def loaded_memory_mb(self) -> float:
        return self._selection.memory_mb if self._selection is not None else 0.0

    def load_model(self, model: str | None = None, device: str = "auto") -> None:
        if self._profile_name:
            profile_names = [self._profile_name]
        else:
            profile_names = self._candidate_profile_names(model, device)
        last_error: WhisperCppError | None = None

        for profile_name in profile_names:
            profile = self._read_profile(profile_name)
            try:
                model_path = self._resolve_model_path(profile.model_key)
                exe_path = self._resolve_exe_path(profile.binary_hint)
            except WhisperCppError as exc:
                last_error = exc
                if self._profile_name:
                    raise
                continue

            memory_mb = self._profile_memory_mb(profile.name)
            self._selection = WhisperCppSelection(
                profile=profile,
                exe_path=exe_path,
                model_path=model_path,
                memory_mb=memory_mb,
            )
            return

        if last_error is not None:
            raise last_error
        raise WhisperCppError(f"No whisper.cpp profile found for model={model} device={device}")

    def transcribe_batch(self, audio_path: str) -> TranscriptionResult:
        if self._selection is None:
            self.load_model()

        selection = self._selection
        if selection is None:
            raise WhisperCppError("whisper.cpp engine was not loaded")

        audio = Path(audio_path).resolve()
        if not audio.is_file():
            raise WhisperCppError(f"Audio file does not exist: {audio}")

        with tempfile.TemporaryDirectory(prefix="openwhisper-asr-") as temp_dir:
            output_base = Path(temp_dir) / "transcript"
            start = time.perf_counter()
            if self._auto_detect_language:
                forced_language: str | None = None
                profile_args = force_whisper_cpp_language_args(selection.profile.args, "auto")
            else:
                forced_language = self._choose_spoken_language(selection, audio, Path(temp_dir))
                profile_args = force_whisper_cpp_language_args(selection.profile.args, forced_language)
            command = [
                str(selection.exe_path),
                "-m",
                str(selection.model_path),
                "-f",
                str(audio),
                *profile_args,
                "-of",
                str(output_base),
            ]

            completed = self._run_whisper_cpp(command, selection, self._timeout_seconds)
            elapsed_ms = int((time.perf_counter() - start) * 1000)

            if completed.returncode != 0:
                raise WhisperCppError(
                    "whisper.cpp failed with exit code "
                    f"{completed.returncode}: {_tail(completed.stderr or completed.stdout)}"
                )

            json_path = output_base.with_suffix(".json")
            if not json_path.is_file():
                raise WhisperCppError(
                    "whisper.cpp did not create JSON output. "
                    f"stdout/stderr tail: {_tail(completed.stdout + completed.stderr)}"
                )

            payload = _load_json(json_path)
            result = parse_whisper_cpp_payload(payload, processing_latency_ms=elapsed_ms)
            if result.language is None and forced_language is not None:
                result = replace(result, language=forced_language)
            if self._auto_detect_language and result.language is not None:
                if result.language not in SUPPORTED_SPOKEN_LANGUAGES:
                    raise WhisperCppError(f"whisper.cpp returned unsupported language: {result.language}")
            elif result.language is not None and result.language not in self._spoken_languages:
                raise WhisperCppError(
                    f"whisper.cpp returned language outside configured spoken languages: {result.language}"
                )
            return result

    def transcribe_chunk(self, audio_pcm: bytes, sample_rate_hz: int) -> TranscriptionResult | None:
        return None

    def _choose_spoken_language(self, selection: WhisperCppSelection, audio: Path, temp_dir: Path) -> str:
        if len(self._spoken_languages) == 1:
            return self._spoken_languages[0]

        best_language = self._spoken_languages[0]
        best_score: float | None = None
        for language in self._spoken_languages:
            score = self._score_spoken_language(selection, audio, temp_dir, language)
            if score is not None and (best_score is None or score > best_score):
                best_language = language
                best_score = score
        return best_language

    def _score_spoken_language(
        self,
        selection: WhisperCppSelection,
        audio: Path,
        temp_dir: Path,
        language: str,
    ) -> float | None:
        output_base = temp_dir / f"language-score-{language}"
        command = [
            str(selection.exe_path),
            "-m",
            str(selection.model_path),
            "-f",
            str(audio),
            *short_whisper_cpp_scoring_args(selection.profile.args, language),
            "-of",
            str(output_base),
        ]
        try:
            completed = self._run_whisper_cpp(command, selection, min(self._timeout_seconds, 60))
        except subprocess.TimeoutExpired:
            return None
        if completed.returncode != 0:
            return None

        json_path = output_base.with_suffix(".json")
        if not json_path.is_file():
            return None
        return calculate_whisper_cpp_language_score(_load_json(json_path))

    def _run_whisper_cpp(
        self,
        command: list[str],
        selection: WhisperCppSelection,
        timeout_seconds: int,
    ) -> subprocess.CompletedProcess[str]:
        if os.name == "nt":
            # Prevent console window flash on Windows when the worker is
            # running as a background / GUI-launched process.
            return subprocess.run(
                command,
                cwd=str(self._asr_root),
                env=self._subprocess_env(selection),
                capture_output=True,
                text=True,
                timeout=timeout_seconds,
                check=False,
                creationflags=0x08000000,  # CREATE_NO_WINDOW
            )
        return subprocess.run(
            command,
            cwd=str(self._asr_root),
            env=self._subprocess_env(selection),
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            check=False,
        )

    def _candidate_profile_names(self, model: str | None, device: str) -> list[str]:
        model_key = _normalize_model_key(model)
        profiles = self._profiles()
        devices: tuple[str, ...]
        if device == "auto":
            devices = ("gpu", "cpu")
        elif device in {"cpu", "gpu"}:
            devices = (device,)
        else:
            raise WhisperCppError(f"Unsupported whisper.cpp device: {device}")

        candidates: list[str] = []
        for wanted_device in devices:
            for name in profiles:
                profile = self._read_profile(name)
                if profile.device == wanted_device and profile.model_key == model_key:
                    candidates.append(name)

        return candidates

    def _profiles(self) -> JsonDict:
        return _as_object(self._config.get("profiles"), "profiles")

    def _read_profile(self, profile_name: str) -> WhisperCppProfile:
        profiles = self._profiles()
        raw = _as_object(profiles.get(profile_name), f"profile {profile_name}")
        return WhisperCppProfile(
            name=profile_name,
            device=_required_string(raw.get("device"), f"{profile_name}.device"),
            binary_hint=_required_string(raw.get("binary_hint"), f"{profile_name}.binary_hint"),
            model_key=_required_string(raw.get("model"), f"{profile_name}.model"),
            args=tuple(_as_string_list(raw.get("args"), f"{profile_name}.args")),
        )

    def _profile_memory_mb(self, profile_name: str) -> float:
        raw = _as_object(self._profiles().get(profile_name), f"profile {profile_name}")
        benchmark = _as_object(raw.get("benchmark", {}), f"{profile_name}.benchmark")
        value = benchmark.get("peak_ram_mb")
        return float(value) if isinstance(value, int | float) else 0.0

    def _resolve_model_path(self, model_key: str) -> Path:
        models = _as_object(self._config.get("models"), "models")
        raw = _as_object(models.get(model_key), f"model {model_key}")
        model_file = _required_string(raw.get("file"), f"{model_key}.file")
        expected_size = _required_int(raw.get("size_bytes"), f"{model_key}.size_bytes")
        model_path = self._model_dir() / model_file

        if not model_path.is_file():
            raise WhisperCppError(f"Model file does not exist: {model_path}")

        actual_size = model_path.stat().st_size
        if actual_size != expected_size:
            raise WhisperCppError(
                f"Model file size mismatch for {model_path}: expected {expected_size}, got {actual_size}"
            )

        return model_path

    def _model_dir(self) -> Path:
        raw = _as_object(self._config.get("model_dir"), "model_dir")
        env_name = _required_string(raw.get("env"), "model_dir.env")
        env_value = os.environ.get(env_name)
        if env_value:
            return Path(env_value).expanduser().resolve()

        default_relative = _required_string(
            raw.get("default_relative_to_config"),
            "model_dir.default_relative_to_config",
        )
        return (self._config_path.parent / default_relative).resolve()

    def _resolve_exe_path(self, binary_hint: str) -> Path:
        hints = _as_object(self._config.get("binary_hints"), "binary_hints")
        hint = _as_object(hints.get(binary_hint), binary_hint)
        env_name = _required_string(hint.get("env"), f"{binary_hint}.env")
        env_value = os.environ.get(env_name)
        if env_value:
            exe_path = Path(env_value).expanduser().resolve()
            if exe_path.is_file():
                return exe_path
            raise WhisperCppError(f"{env_name} points to a missing whisper.cpp binary: {exe_path}")

        for candidate in _LOCAL_BINARY_CANDIDATES.get(binary_hint, ()):
            exe_path = (self._asr_root / candidate).resolve()
            if exe_path.is_file():
                return exe_path

        path_name = "whisper-cli.exe" if os.name == "nt" else "whisper-cli"
        path_match = shutil.which(path_name)
        if path_match is not None:
            return Path(path_match).resolve()

        raise WhisperCppError(f"Could not find whisper.cpp binary for {binary_hint}; set {env_name}")

    def _subprocess_env(self, selection: WhisperCppSelection) -> dict[str, str]:
        env = os.environ.copy()
        hints = _as_object(self._config.get("binary_hints"), "binary_hints")
        hint = _as_object(hints.get(selection.profile.binary_hint), selection.profile.binary_hint)
        path_parts = [str(selection.exe_path.parent)]
        prepend_path_env = _optional_string(hint.get("prepend_path_env"))
        if prepend_path_env:
            prepend_path = os.environ.get(prepend_path_env)
            if prepend_path:
                path_parts.insert(0, prepend_path)

        for candidate in _LOCAL_PATH_CANDIDATES.get(selection.profile.binary_hint, ()):
            path = (self._asr_root / candidate).resolve()
            if path.is_dir():
                path_parts.append(str(path))

        existing_path = env.get("PATH")
        if existing_path:
            path_parts.append(existing_path)
        env["PATH"] = os.pathsep.join(path_parts)
        if selection.profile.device == "cpu":
            # Prevent OpenBLAS from spawning its own thread pool and competing
            # with whisper.cpp's OpenMP threading on CPU inference.
            env["OPENBLAS_NUM_THREADS"] = "1"
        return env


def _normalize_model_key(model: str | None) -> str:
    if model is None:
        return "medium_en_q8"
    key = _MODEL_ALIASES.get(model)
    if key is None:
        raise WhisperCppError(f"Unsupported whisper.cpp model: {model}")
    return key


def _normalize_spoken_languages(spoken_languages: tuple[str, ...]) -> tuple[str, ...]:
    languages: list[str] = []
    for item in spoken_languages:
        language = item.strip().lower()
        if language not in SUPPORTED_SPOKEN_LANGUAGES or language in languages:
            continue
        languages.append(language)
    return tuple(languages) if languages else ("en",)


def _remove_whisper_cpp_duration_args(args: tuple[str, ...]) -> tuple[str, ...]:
    cleaned: list[str] = []
    skip_next = False
    for arg in args:
        if skip_next:
            skip_next = False
            continue
        if arg in {"-d", "--duration"}:
            skip_next = True
            continue
        cleaned.append(arg)
    return tuple(cleaned)


def _collect_token_probabilities(value: object, probabilities: list[float]) -> None:
    if isinstance(value, dict):
        for key, item in value.items():
            if key in {"p", "prob", "probability"} and isinstance(item, int | float):
                probabilities.append(float(item))
            else:
                _collect_token_probabilities(item, probabilities)
    elif isinstance(value, list):
        for item in value:
            _collect_token_probabilities(item, probabilities)


def _load_json(path: Path) -> JsonDict:
    try:
        with path.open("r", encoding="utf-8") as file:
            payload = json.load(file)
    except OSError as exc:
        raise WhisperCppError(f"Failed to read {path}: {exc}") from exc
    return _as_object(payload, str(path))


def _as_object(value: object, label: str) -> JsonDict:
    if not isinstance(value, dict):
        raise WhisperCppError(f"Expected object for {label}")
    return cast(JsonDict, value)


def _as_list(value: object, label: str) -> list[object]:
    if not isinstance(value, list):
        raise WhisperCppError(f"Expected list for {label}")
    return value


def _as_string_list(value: object, label: str) -> list[str]:
    items = _as_list(value, label)
    strings: list[str] = []
    for item in items:
        if not isinstance(item, str):
            raise WhisperCppError(f"Expected string item for {label}")
        strings.append(item)
    return strings


def _required_string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise WhisperCppError(f"Expected non-empty string for {label}")
    return value


def _optional_string(value: object) -> str | None:
    return value if isinstance(value, str) else None


def _required_int(value: object, label: str) -> int:
    if not isinstance(value, int):
        raise WhisperCppError(f"Expected integer for {label}")
    return value


def _tail(text: str, max_lines: int = 20) -> str:
    lines = text.strip().splitlines()
    return "\n".join(lines[-max_lines:]) if lines else "(no output)"
