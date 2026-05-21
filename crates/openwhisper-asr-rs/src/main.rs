mod audio_capture;
mod buffering;
mod engine;
mod protocol;
mod vad;
mod worker;

use std::path::PathBuf;

use anyhow::Context;
use audio_capture::record_default_input_to_wav;
use engine::{AsrEngine, WhisperCppEngine};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .init();

    match parse_args(std::env::args().skip(1).collect())? {
        Command::Worker => worker::run_stdio(),
        Command::Record { path, seconds } => record_default_input_to_wav(&path, seconds),
        Command::Transcribe {
            path,
            model,
            device,
        } => transcribe_file(&path, &model, &device),
    }
}

enum Command {
    Worker,
    Record {
        path: PathBuf,
        seconds: u64,
    },
    Transcribe {
        path: PathBuf,
        model: String,
        device: String,
    },
}

fn parse_args(args: Vec<String>) -> anyhow::Result<Command> {
    if args.is_empty() {
        return Ok(Command::Worker);
    }

    match args[0].as_str() {
        "record" => parse_record_args(&args),
        "transcribe" => parse_transcribe_args(&args),
        other => anyhow::bail!("unsupported command: {other}"),
    }
}

fn parse_record_args(args: &[String]) -> anyhow::Result<Command> {
    let path = args
        .get(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("record requires an output wav path"))?;
    let mut seconds = 5;
    let mut index = 2;

    while index < args.len() {
        match args[index].as_str() {
            "--seconds" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| anyhow::anyhow!("--seconds requires a value"))?;
                seconds = value
                    .parse::<u64>()
                    .context("--seconds must be a positive integer")?;
                if seconds == 0 {
                    anyhow::bail!("--seconds must be greater than zero");
                }
                index += 2;
            }
            other => anyhow::bail!("unsupported record argument: {other}"),
        }
    }

    Ok(Command::Record { path, seconds })
}

fn parse_transcribe_args(args: &[String]) -> anyhow::Result<Command> {
    let path = args
        .get(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("transcribe requires an input audio path"))?;
    let mut model = "medium_en_q8".to_string();
    let mut device = "auto".to_string();
    let mut index = 2;

    while index < args.len() {
        match args[index].as_str() {
            "--model" => {
                model = args
                    .get(index + 1)
                    .ok_or_else(|| anyhow::anyhow!("--model requires a value"))?
                    .to_string();
                index += 2;
            }
            "--device" => {
                device = args
                    .get(index + 1)
                    .ok_or_else(|| anyhow::anyhow!("--device requires a value"))?
                    .to_string();
                index += 2;
            }
            other => anyhow::bail!("unsupported transcribe argument: {other}"),
        }
    }

    Ok(Command::Transcribe {
        path,
        model,
        device,
    })
}

fn transcribe_file(path: &PathBuf, model: &str, device: &str) -> anyhow::Result<()> {
    let mut engine = WhisperCppEngine::default();
    engine.load_model(model, device)?;
    let result = engine.transcribe_file(path)?;
    println!("{}", result.text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{parse_args, Command};

    #[test]
    fn no_args_keeps_stdio_worker_as_default_product_path() {
        let command = parse_args(vec![]).unwrap();

        assert!(matches!(command, Command::Worker));
    }

    #[test]
    fn record_command_requires_explicit_output_and_duration() {
        let command = parse_args(vec![
            "record".to_string(),
            "sample.wav".to_string(),
            "--seconds".to_string(),
            "30".to_string(),
        ])
        .unwrap();

        match command {
            Command::Record { path, seconds } => {
                assert_eq!(path, PathBuf::from("sample.wav"));
                assert_eq!(seconds, 30);
            }
            _ => panic!("expected record command"),
        }
    }

    #[test]
    fn transcribe_command_accepts_model_and_device_options() {
        let command = parse_args(vec![
            "transcribe".to_string(),
            "sample.wav".to_string(),
            "--model".to_string(),
            "turbo".to_string(),
            "--device".to_string(),
            "cpu".to_string(),
        ])
        .unwrap();

        match command {
            Command::Transcribe {
                path,
                model,
                device,
            } => {
                assert_eq!(path, PathBuf::from("sample.wav"));
                assert_eq!(model, "turbo");
                assert_eq!(device, "cpu");
            }
            _ => panic!("expected transcribe command"),
        }
    }
}
