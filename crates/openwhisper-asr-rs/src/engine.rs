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

pub trait AsrEngine {
    fn load_model(&mut self, model: &str, device: &str) -> Result<ModelLoadInfo>;
    fn transcribe_file(&mut self, audio_path: &Path) -> Result<Transcription>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelLoadInfo {
    pub device: String,
    pub memory_mb: f64,
}

#[derive(Debug, Default)]
pub struct MockEngine {
    loaded: bool,
}

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
}

#[derive(Debug, Clone, PartialEq)]
struct WhisperCppSelection {
    device: String,
    exe_path: PathBuf,
    model_path: PathBuf,
    args: Vec<String>,
    memory_mb: f64,
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
        }
    }
}

impl WhisperCppEngine {
    #[cfg(test)]
    fn with_config_path(config_path: PathBuf) -> Self {
        Self {
            config_path,
            selection: None,
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
    ) -> Result<(String, ProfileConfig)> {
        let model_key = normalize_model_key(model)?;
        let wanted_devices: &[&str] = match device {
            "auto" => &["gpu", "cpu"],
            "gpu" => &["gpu"],
            "cpu" => &["cpu"],
            other => anyhow::bail!("unsupported whisper.cpp device: {other}"),
        };

        for wanted_device in wanted_devices {
            for (profile_name, raw_profile) in &config.profiles {
                let profile: ProfileConfig = serde_json::from_value(raw_profile.clone())
                    .with_context(|| {
                        format!("failed to parse whisper.cpp profile {profile_name}")
                    })?;
                if profile.device == *wanted_device && profile.model == model_key {
                    return Ok((profile_name.clone(), profile));
                }
            }
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
            anyhow::bail!("model file does not exist: {}", model_path.display());
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

        Ok(model_path)
    }

    fn model_dir(&self, config: &ModelDirConfig) -> PathBuf {
        if let Ok(value) = std::env::var(&config.env) {
            if !value.trim().is_empty() {
                return PathBuf::from(value);
            }
        }
        self.config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&config.default_relative_to_config)
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
                    return Ok(path);
                }
                anyhow::bail!(
                    "{env_name} points to a missing whisper.cpp binary: {}",
                    path.display()
                );
            }
        }

        let asr_root = self
            .config_path
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new("."));
        for candidate in local_binary_candidates(binary_hint) {
            let path = asr_root.join(candidate);
            if path.is_file() {
                return Ok(path);
            }
        }

        anyhow::bail!("could not find whisper.cpp binary for {binary_hint}; set {env_name}")
    }
}

impl AsrEngine for WhisperCppEngine {
    fn load_model(&mut self, model: &str, device: &str) -> Result<ModelLoadInfo> {
        let config = self.load_config()?;
        let (_profile_name, profile) = self.select_profile(&config, model, device)?;
        let model_path = self.resolve_model_path(&config, &profile.model)?;
        let exe_path = self.resolve_exe_path(&profile.binary_hint)?;
        let memory_mb = profile.benchmark.peak_ram_mb.unwrap_or(0.0);

        self.selection = Some(WhisperCppSelection {
            device: profile.device.clone(),
            exe_path,
            model_path,
            args: profile.args.clone(),
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

        if self.selection.is_none() {
            let model = std::env::var("OPENWHISPER_ASR_MODEL")
                .unwrap_or_else(|_| "medium_en_q8".to_string());
            let device =
                std::env::var("OPENWHISPER_ASR_DEVICE").unwrap_or_else(|_| "cpu".to_string());
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

        let start = Instant::now();
        let completed = run_whisper_cpp(selection, audio_path, &output_base)?;
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

        parse_whisper_cpp_payload(&payload, elapsed_ms)
    }
}

fn run_whisper_cpp(
    selection: &WhisperCppSelection,
    audio_path: &Path,
    output_base: &Path,
) -> Result<std::process::Output> {
    let mut command = Command::new(&selection.exe_path);
    command
        .arg("-m")
        .arg(&selection.model_path)
        .arg("-f")
        .arg(audio_path)
        .args(&selection.args)
        .arg("-of")
        .arg(output_base);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    command
        .output()
        .with_context(|| format!("failed to run {}", selection.exe_path.display()))
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
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

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
    find_workspace_root()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("asr")
        .join("config")
        .join("whispercpp-profiles.json")
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir
            .join("asr")
            .join("config")
            .join("whispercpp-profiles.json")
            .is_file()
        {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
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

    use super::{parse_whisper_cpp_payload, AsrEngine, MockEngine, WhisperCppEngine};

    #[test]
    fn mock_file_transcription_includes_source_name() {
        let mut engine = MockEngine::default();
        let result = engine.transcribe_file(Path::new("sample.wav")).unwrap();

        assert_eq!(result.text, "Mock transcript for sample.wav");
        assert_eq!(result.language.as_deref(), Some("en"));
        assert!(!result.words.is_empty());
    }

    #[test]
    fn parses_whisper_cpp_json_segments_into_final_text() {
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

        assert_eq!(result.text, "hello world");
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
}
