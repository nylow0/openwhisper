use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

use crate::protocol::WordResult;

#[derive(Debug, Clone, PartialEq)]
pub struct Transcription {
    pub text: String,
    pub words: Vec<WordResult>,
    pub language: Option<String>,
    pub processing_latency_ms: u32,
}

pub trait AsrEngine: Send {
    fn load_model(&mut self, model: &str, device: &str) -> Result<ModelLoadInfo>;
    fn transcribe_file(&mut self, audio_path: &Path) -> Result<Transcription>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelLoadInfo {
    pub device: String,
    pub memory_mb: f64,
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct MockEngine {
    loaded: bool,
}

#[cfg(test)]
impl AsrEngine for MockEngine {
    fn load_model(&mut self, _model: &str, device: &str) -> Result<ModelLoadInfo> {
        self.loaded = true;
        Ok(ModelLoadInfo {
            device: if device.is_empty() {
                "cpu".to_string()
            } else {
                device.to_string()
            },
            memory_mb: 512.0,
        })
    }

    fn transcribe_file(&mut self, audio_path: &Path) -> Result<Transcription> {
        if !self.loaded {
            self.load_model("mock", "cpu")?;
        }

        let name = audio_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("audio");
        let text = format!("Mock transcript for {name}");

        Ok(Transcription {
            words: mock_words(&text),
            text,
            language: Some("en".to_string()),
            processing_latency_ms: 1,
        })
    }
}

#[cfg(test)]
pub fn mock_words(text: &str) -> Vec<WordResult> {
    let mut start_ms = 0;
    text.split_whitespace()
        .map(|word| {
            let duration_ms = (word.len() as u32 * 60).max(120);
            let result = WordResult {
                text: word.to_string(),
                start_ms,
                end_ms: start_ms + duration_ms,
                confidence: Some(0.92),
            };
            start_ms += duration_ms + 40;
            result
        })
        .collect()
}

#[derive(Debug)]
pub struct WhisperCppEngine {
    config_path: PathBuf,
    selection: Option<WhisperCppSelection>,
    language_config: AsrLanguageConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AsrLanguageConfig {
    spoken_languages: Vec<String>,
    auto_detect_language: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct WhisperCppSelection {
    device: String,
    exe_path: PathBuf,
    model_path: PathBuf,
    args: Vec<String>,
    memory_mb: f64,
}

pub(crate) fn env_value_is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub fn apply_asr_language_env(languages: &str, auto_detect_language: bool) {
    std::env::set_var("OPENWHISPER_ASR_LANGUAGES", languages);
    if auto_detect_language {
        std::env::set_var("OPENWHISPER_ASR_AUTO_DETECT_LANGUAGE", "1");
    } else {
        std::env::remove_var("OPENWHISPER_ASR_AUTO_DETECT_LANGUAGE");
    }
}

fn read_asr_language_config() -> AsrLanguageConfig {
    let spoken_raw =
        std::env::var("OPENWHISPER_ASR_LANGUAGES").unwrap_or_else(|_| "en".to_string());
    let mut spoken_languages = Vec::new();
    for item in spoken_raw.split(',') {
        let language = item.trim().to_lowercase();
        if language.is_empty() || spoken_languages.iter().any(|l| l == &language) {
            continue;
        }
        spoken_languages.push(language);
    }
    if spoken_languages.is_empty() {
        spoken_languages.push("en".to_string());
    }

    let auto_detect_language = std::env::var("OPENWHISPER_ASR_AUTO_DETECT_LANGUAGE")
        .map(|value| env_value_is_truthy(&value))
        .unwrap_or(false);

    AsrLanguageConfig {
        spoken_languages,
        auto_detect_language,
    }
}

fn resolve_whisper_language_flag(config: &AsrLanguageConfig) -> &str {
    if config.auto_detect_language || config.spoken_languages.len() > 1 {
        "auto"
    } else {
        &config.spoken_languages[0]
    }
}

fn replace_whisper_language_arg(args: &[String], language: &str) -> Vec<String> {
    let mut cleaned = Vec::with_capacity(args.len() + 2);
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "-l" || arg == "--language" {
            skip_next = true;
            continue;
        }
        cleaned.push(arg.clone());
    }
    cleaned.push("-l".to_string());
    cleaned.push(language.to_string());
    cleaned
}

fn validate_transcription_language(
    detected: &Option<String>,
    config: &AsrLanguageConfig,
) -> Result<()> {
    let Some(language) = detected else {
        return Ok(());
    };
    if config.auto_detect_language {
        return Ok(());
    }
    if config.spoken_languages.len() == 1 && language != &config.spoken_languages[0] {
        anyhow::bail!(
            "whisper.cpp returned language {language} but configured spoken language is {}",
            config.spoken_languages[0]
        );
    }
    if config.spoken_languages.len() > 1
        && !config
            .spoken_languages
            .iter()
            .any(|spoken| spoken == language)
    {
        anyhow::bail!(
            "whisper.cpp returned language {language} outside configured spoken languages: {:?}",
            config.spoken_languages
        );
    }
    Ok(())
}

fn canonicalize_existing_path(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("failed to resolve path {}", path.display()))
}

#[derive(Debug, Deserialize)]
struct WhisperCppConfig {
    model_dir: ModelDirConfig,
    models: serde_json::Map<String, Value>,
    profiles: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct ModelDirConfig {
    env: String,
    default_relative_to_config: String,
}

#[derive(Debug, Deserialize)]
struct ModelConfig {
    file: String,
    size_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct ProfileConfig {
    device: String,
    binary_hint: String,
    model: String,
    args: Vec<String>,
    #[serde(default)]
    benchmark: BenchmarkConfig,
}

#[derive(Debug, Default, Deserialize)]
struct BenchmarkConfig {
    peak_ram_mb: Option<f64>,
}

impl Default for WhisperCppEngine {
    fn default() -> Self {
        Self {
            config_path: default_config_path(),
            selection: None,
            language_config: read_asr_language_config(),
        }
    }
}

impl WhisperCppEngine {
    #[cfg(test)]
    fn with_config_path(config_path: PathBuf) -> Self {
        Self {
            config_path,
            selection: None,
            language_config: read_asr_language_config(),
        }
    }

    fn load_config(&self) -> Result<WhisperCppConfig> {
        let file = std::fs::File::open(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        serde_json::from_reader(file)
            .with_context(|| format!("failed to parse {}", self.config_path.display()))
    }

    fn select_profile(
        &self,
        config: &WhisperCppConfig,
        model: &str,
        device: &str,
    ) -> Result<(String, ProfileConfig, PathBuf)> {
        let model_key = normalize_model_key(model)?;
        let wanted_devices: &[&str] = match device {
            "auto" => &["gpu", "cpu"],
            "gpu" => &["gpu"],
            "cpu" => &["cpu"],
            other => anyhow::bail!("unsupported whisper.cpp device: {other}"),
        };
        let allow_fallback = device == "auto";
        let mut skipped_profiles = Vec::new();

        for wanted_device in wanted_devices {
            for (profile_name, raw_profile) in &config.profiles {
                let profile: ProfileConfig = serde_json::from_value(raw_profile.clone())
                    .with_context(|| {
                        format!("failed to parse whisper.cpp profile {profile_name}")
                    })?;
                if profile.device == *wanted_device && profile.model == model_key {
                    match self.resolve_exe_path(&profile.binary_hint) {
                        Ok(exe_path) => return Ok((profile_name.clone(), profile, exe_path)),
                        Err(error) if allow_fallback => {
                            skipped_profiles.push(format!("{profile_name}: {error}"));
                            continue;
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        }

        if !skipped_profiles.is_empty() {
            anyhow::bail!(
                "no usable whisper.cpp profile found for model={model} device={device}; skipped {}",
                skipped_profiles.join("; ")
            );
        }

        anyhow::bail!("no whisper.cpp profile found for model={model} device={device}")
    }

    fn resolve_model_path(&self, config: &WhisperCppConfig, model_key: &str) -> Result<PathBuf> {
        let raw_model = config
            .models
            .get(model_key)
            .ok_or_else(|| anyhow::anyhow!("unknown whisper.cpp model: {model_key}"))?;
        let model: ModelConfig = serde_json::from_value(raw_model.clone())
            .with_context(|| format!("failed to parse whisper.cpp model {model_key}"))?;
        let model_dir = self.model_dir(&config.model_dir);
        let model_path = model_dir.join(&model.file);

        if !model_path.is_file() {
            anyhow::bail!(
                "model file does not exist: {}. Set {} to the OpenWhisper model directory or reinstall with model assets",
                model_path.display(),
                config.model_dir.env
            );
        }

        let actual_size = model_path
            .metadata()
            .with_context(|| format!("failed to stat {}", model_path.display()))?
            .len();
        if actual_size != model.size_bytes {
            anyhow::bail!(
                "model file size mismatch for {}: expected {}, got {}",
                model_path.display(),
                model.size_bytes,
                actual_size
            );
        }

        canonicalize_existing_path(&model_path)
    }

    fn model_dir(&self, config: &ModelDirConfig) -> PathBuf {
        let dir = if let Ok(value) = std::env::var(&config.env) {
            if !value.trim().is_empty() {
                PathBuf::from(value)
            } else {
                self.config_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&config.default_relative_to_config)
            }
        } else {
            self.config_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&config.default_relative_to_config)
        };
        dir.canonicalize().unwrap_or(dir)
    }

    fn resolve_exe_path(&self, binary_hint: &str) -> Result<PathBuf> {
        let env_name = match binary_hint {
            "cpu_avx_vnni" => "OPENWHISPER_WHISPERCPP_CPU_EXE",
            "gpu_cuda_sm120" => "OPENWHISPER_WHISPERCPP_GPU_EXE",
            other => anyhow::bail!("unknown whisper.cpp binary hint: {other}"),
        };

        if let Ok(value) = std::env::var(env_name) {
            if !value.trim().is_empty() {
                let path = PathBuf::from(value);
                if path.is_file() {
                    return canonicalize_existing_path(&path);
                }
                anyhow::bail!(
                    "{env_name} points to a missing whisper.cpp binary: {}",
                    path.display()
                );
            }
        }

        let asr_root = whisper_tooling_root(self.config_path.parent().unwrap_or(Path::new(".")));
        for candidate in local_binary_candidates(binary_hint) {
            let path = asr_root.join(candidate);
            if path.is_file() {
                return canonicalize_existing_path(&path);
            }
        }

        anyhow::bail!("could not find whisper.cpp binary for {binary_hint}; set {env_name}")
    }
}

impl AsrEngine for WhisperCppEngine {
    fn load_model(&mut self, model: &str, device: &str) -> Result<ModelLoadInfo> {
        let config = self.load_config()?;
        let (_profile_name, profile, exe_path) = self.select_profile(&config, model, device)?;
        let model_path = self.resolve_model_path(&config, &profile.model)?;
        let memory_mb = profile.benchmark.peak_ram_mb.unwrap_or(0.0);
        let language_config = read_asr_language_config();
        let language_flag = resolve_whisper_language_flag(&language_config);
        let whisper_args = replace_whisper_language_arg(&profile.args, language_flag);
        self.language_config = language_config;

        self.selection = Some(WhisperCppSelection {
            device: profile.device.clone(),
            exe_path,
            model_path,
            args: whisper_args,
            memory_mb,
        });

        Ok(ModelLoadInfo {
            device: profile.device,
            memory_mb,
        })
    }

    fn transcribe_file(&mut self, audio_path: &Path) -> Result<Transcription> {
        if !audio_path.is_file() {
            anyhow::bail!("audio file does not exist: {}", audio_path.display());
        }

        let decode_path = canonicalize_existing_path(audio_path)?;

        if self.selection.is_none() {
            let model = std::env::var("OPENWHISPER_ASR_MODEL")
                .unwrap_or_else(|_| "large_v3_turbo_q8".to_string());
            let device =
                std::env::var("OPENWHISPER_ASR_DEVICE").unwrap_or_else(|_| "auto".to_string());
            self.load_model(&model, &device)?;
        }
        let selection = self
            .selection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("whisper.cpp engine was not loaded"))?;
        let output_base = std::env::temp_dir().join(format!(
            "openwhisper-asr-rs-transcript-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));

        log::debug!(
            "whisper.cpp decode: spoken_languages={:?} auto_detect={}",
            self.language_config.spoken_languages,
            self.language_config.auto_detect_language
        );

        let start = Instant::now();
        let asr_root = whisper_tooling_root(self.config_path.parent().unwrap_or(Path::new(".")));
        let completed = run_whisper_cpp(selection, &decode_path, &output_base, &asr_root)?;
        let elapsed_ms = start.elapsed().as_millis().min(u32::MAX as u128) as u32;

        if !completed.status.success() {
            anyhow::bail!(
                "whisper.cpp failed with exit code {:?}: {}",
                completed.status.code(),
                tail(&String::from_utf8_lossy(&completed.stderr))
            );
        }

        let json_path = output_base.with_extension("json");
        let file = std::fs::File::open(&json_path).with_context(|| {
            format!(
                "whisper.cpp did not create JSON output at {}. stdout/stderr tail: {}",
                json_path.display(),
                tail(&format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&completed.stdout),
                    String::from_utf8_lossy(&completed.stderr)
                ))
            )
        })?;
        let payload: Value = serde_json::from_reader(file).with_context(|| {
            format!("failed to parse whisper.cpp output {}", json_path.display())
        })?;
        let _ = std::fs::remove_file(&json_path);

        let transcription = parse_whisper_cpp_payload(&payload, elapsed_ms)?;
        validate_transcription_language(&transcription.language, &self.language_config)?;
        Ok(transcription)
    }
}

fn run_whisper_cpp(
    selection: &WhisperCppSelection,
    audio_path: &Path,
    output_base: &Path,
    asr_root: &Path,
) -> Result<std::process::Output> {
    let mut command = Command::new(&selection.exe_path);
    command
        .current_dir(whisper_cpp_working_dir(&selection.exe_path, asr_root))
        .arg("-m")
        .arg(&selection.model_path)
        .arg("-f")
        .arg(audio_path)
        .args(&selection.args)
        .arg("-of")
        .arg(output_base);

    for (key, value) in whisper_cpp_subprocess_env(selection, asr_root) {
        command.env(key, value);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    command
        .output()
        .with_context(|| format!("failed to run {}", selection.exe_path.display()))
}

fn whisper_cpp_working_dir(exe_path: &Path, fallback: &Path) -> PathBuf {
    exe_path
        .parent()
        .filter(|path| path.is_dir())
        .unwrap_or(fallback)
        .to_path_buf()
}

fn whisper_cpp_subprocess_env(
    selection: &WhisperCppSelection,
    asr_root: &Path,
) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = std::env::vars().collect();
    let separator = if cfg!(windows) { ";" } else { ":" };
    let mut path_parts = vec![selection
        .exe_path
        .parent()
        .unwrap_or(asr_root)
        .display()
        .to_string()];

    if selection.device == "gpu" {
        if let Ok(prepend) = std::env::var("OPENWHISPER_WHISPERCPP_GPU_PATH") {
            if !prepend.trim().is_empty() {
                path_parts.insert(0, prepend);
            }
        }
        for candidate in gpu_path_candidates() {
            let path = asr_root.join(candidate);
            if path.is_dir() {
                path_parts.push(path.display().to_string());
            }
        }
    }

    if let Ok(existing) = std::env::var("PATH") {
        path_parts.push(existing);
    }
    env.insert("PATH".to_string(), path_parts.join(separator));

    if selection.device == "cpu" {
        env.insert("OPENBLAS_NUM_THREADS".to_string(), "1".to_string());
    }

    env
}

fn gpu_path_candidates() -> &'static [&'static str] {
    &[
        ".local/tools/cuda-13.2/toolkit/bin",
        ".local/tools/cuda-13.2/toolkit/bin/x64",
        ".local/whispercpp-src/build-cuda-sm120-local/bin",
        ".local/whispercpp/cuda-bin/Release",
    ]
}

pub fn parse_whisper_cpp_payload(
    payload: &Value,
    processing_latency_ms: u32,
) -> Result<Transcription> {
    let language = payload
        .get("result")
        .and_then(|result| result.get("language"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let transcription = payload
        .get("transcription")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("whisper.cpp JSON output is missing transcription list"))?;
    let text = transcription
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<String>();

    Ok(Transcription {
        text,
        words: Vec::new(),
        language,
        processing_latency_ms,
    })
}

fn normalize_model_key(model: &str) -> Result<&'static str> {
    match model {
        "" | "medium" | "medium-q8" | "medium_en_q8" => Ok("medium_en_q8"),
        "turbo" | "large-v3-turbo" | "large-v3-turbo-q8" | "large_v3_turbo_q8" => {
            Ok("large_v3_turbo_q8")
        }
        other => anyhow::bail!("unsupported whisper.cpp model: {other}"),
    }
}

fn local_binary_candidates(binary_hint: &str) -> &'static [&'static str] {
    match binary_hint {
        "cpu_avx_vnni" => &[
            ".local/whispercpp-src/build-cpu-avxvnni-local/bin/whisper-cli.exe",
            ".local/whispercpp/bin/Release/whisper-cli.exe",
        ],
        "gpu_cuda_sm120" => &[
            ".local/whispercpp-src/build-cuda-sm120-local/bin/whisper-cli.exe",
            ".local/whispercpp/cuda-bin/Release/whisper-cli.exe",
        ],
        _ => &[],
    }
}

fn default_config_path() -> PathBuf {
    if let Ok(value) = std::env::var("OPENWHISPER_WHISPERCPP_CONFIG") {
        if !value.trim().is_empty() {
            return PathBuf::from(value);
        }
    }

    find_workspace_root()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config")
        .join("whispercpp-profiles.json")
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("config").join("whispercpp-profiles.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn whisper_tooling_root(workspace_or_config_parent: &Path) -> PathBuf {
    if let Some(workspace) = find_workspace_root() {
        return workspace.join("asr");
    }
    workspace_or_config_parent.join("asr")
}

fn unix_timestamp_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn tail(text: &str) -> String {
    let lines = text.trim().lines().rev().take(20).collect::<Vec<_>>();
    lines.into_iter().rev().collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::{
        parse_whisper_cpp_payload, read_asr_language_config, replace_whisper_language_arg,
        resolve_whisper_language_flag, AsrEngine, AsrLanguageConfig, MockEngine, WhisperCppEngine,
    };

    fn restore_env(name: &str, previous: Option<String>) {
        match previous {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }

    #[test]
    fn resolve_language_flag_uses_auto_for_auto_detect_or_multiple_languages() {
        let auto = AsrLanguageConfig {
            spoken_languages: vec!["en".to_string()],
            auto_detect_language: true,
        };
        assert_eq!(resolve_whisper_language_flag(&auto), "auto");

        let multi = AsrLanguageConfig {
            spoken_languages: vec!["en".to_string(), "de".to_string()],
            auto_detect_language: false,
        };
        assert_eq!(resolve_whisper_language_flag(&multi), "auto");
    }

    #[test]
    fn resolve_language_flag_uses_single_spoken_language_without_auto_detect() {
        let single = AsrLanguageConfig {
            spoken_languages: vec!["de".to_string()],
            auto_detect_language: false,
        };
        assert_eq!(resolve_whisper_language_flag(&single), "de");
    }

    #[test]
    fn replace_whisper_language_arg_overrides_profile_language() {
        let args = replace_whisper_language_arg(
            &["-l".to_string(), "auto".to_string(), "-nt".to_string()],
            "en",
        );
        assert_eq!(args, vec!["-nt".to_string(), "-l".to_string(), "en".to_string()]);
    }

    #[test]
    fn read_asr_language_config_parses_env() {
        std::env::set_var("OPENWHISPER_ASR_LANGUAGES", "de,en,de");
        std::env::set_var("OPENWHISPER_ASR_AUTO_DETECT_LANGUAGE", "1");
        let config = read_asr_language_config();
        assert_eq!(config.spoken_languages, vec!["de".to_string(), "en".to_string()]);
        assert!(config.auto_detect_language);
    }

    #[test]
    fn mock_file_transcription_includes_source_name() {
        let mut engine = MockEngine::default();
        let result = engine.transcribe_file(Path::new("sample.wav")).unwrap();

        assert_eq!(result.text, "Mock transcript for sample.wav");
        assert_eq!(result.language.as_deref(), Some("en"));
        assert!(!result.words.is_empty());
    }

    #[test]
    fn parses_whisper_cpp_json_segments_into_final_text_without_cleanup() {
        let result = parse_whisper_cpp_payload(
            &json!({
                "result": {"language": "en"},
                "transcription": [
                    {"text": " hello"},
                    {"text": " world "}
                ]
            }),
            123,
        )
        .unwrap();

        assert_eq!(result.text, " hello world ");
        assert_eq!(result.language.as_deref(), Some("en"));
        assert_eq!(result.processing_latency_ms, 123);
    }

    #[test]
    fn whisper_cpp_load_reports_missing_model_as_model_configuration_error() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("whispercpp-profiles.json");
        std::fs::write(
            &config_path,
            r#"{
                "model_dir": {"env": "OPENWHISPER_MODEL_DIR_TEST_ONLY", "default_relative_to_config": "../models"},
                "models": {"medium_en_q8": {"file": "missing.bin", "size_bytes": 1}},
                "profiles": {
                    "cpu": {
                        "device": "cpu",
                        "binary_hint": "cpu_avx_vnni",
                        "model": "medium_en_q8",
                        "args": ["-l", "en", "-oj"],
                        "benchmark": {"peak_ram_mb": 1.0}
                    }
                }
            }"#,
        )
        .unwrap();
        let mut engine = WhisperCppEngine::with_config_path(config_path);

        let error = engine.load_model("medium_en_q8", "cpu").unwrap_err();

        assert!(error.to_string().contains("model file does not exist"));
    }

    #[test]
    fn auto_device_falls_back_to_cpu_when_gpu_binary_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let model_dir = dir.path().join("models");
        let cpu_dir = dir.path().join("cpu");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&model_dir).unwrap();
        std::fs::create_dir_all(&cpu_dir).unwrap();
        let model_path = model_dir.join("model.bin");
        let cpu_exe = cpu_dir.join("whisper-cli.exe");
        std::fs::write(&model_path, b"model").unwrap();
        std::fs::write(&cpu_exe, b"").unwrap();

        let config_path = config_dir.join("whispercpp-profiles.json");
        std::fs::write(
            &config_path,
            r#"{
                "model_dir": {"env": "OPENWHISPER_MODEL_DIR_AUTO_FALLBACK_TEST", "default_relative_to_config": "../models"},
                "models": {"medium_en_q8": {"file": "model.bin", "size_bytes": 5}},
                "profiles": {
                    "gpu": {
                        "device": "gpu",
                        "binary_hint": "gpu_cuda_sm120",
                        "model": "medium_en_q8",
                        "args": ["-l", "en"],
                        "benchmark": {"peak_ram_mb": 2.0}
                    },
                    "cpu": {
                        "device": "cpu",
                        "binary_hint": "cpu_avx_vnni",
                        "model": "medium_en_q8",
                        "args": ["-l", "en"],
                        "benchmark": {"peak_ram_mb": 1.0}
                    }
                }
            }"#,
        )
        .unwrap();

        let old_model_dir = std::env::var("OPENWHISPER_MODEL_DIR_AUTO_FALLBACK_TEST").ok();
        let old_cpu = std::env::var("OPENWHISPER_WHISPERCPP_CPU_EXE").ok();
        let old_gpu = std::env::var("OPENWHISPER_WHISPERCPP_GPU_EXE").ok();
        std::env::set_var("OPENWHISPER_MODEL_DIR_AUTO_FALLBACK_TEST", &model_dir);
        std::env::set_var("OPENWHISPER_WHISPERCPP_CPU_EXE", &cpu_exe);
        std::env::set_var(
            "OPENWHISPER_WHISPERCPP_GPU_EXE",
            dir.path().join("missing").join("whisper-cli.exe"),
        );

        let mut engine = WhisperCppEngine::with_config_path(config_path);
        let info = engine.load_model("medium_en_q8", "auto").unwrap();

        assert_eq!(info.device, "cpu");
        assert_eq!(info.memory_mb, 1.0);

        restore_env("OPENWHISPER_MODEL_DIR_AUTO_FALLBACK_TEST", old_model_dir);
        restore_env("OPENWHISPER_WHISPERCPP_CPU_EXE", old_cpu);
        restore_env("OPENWHISPER_WHISPERCPP_GPU_EXE", old_gpu);
    }

    #[test]
    fn whisper_cpp_working_dir_uses_executable_folder_for_packaged_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let exe_dir = dir.path().join("resources").join("whispercpp").join("gpu");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let exe_path = exe_dir.join("whisper-cli.exe");
        let nonexistent_asr_root = dir.path().join("resources").join("asr");

        let working_dir = super::whisper_cpp_working_dir(&exe_path, &nonexistent_asr_root);

        assert_eq!(working_dir, exe_dir);
    }
}
