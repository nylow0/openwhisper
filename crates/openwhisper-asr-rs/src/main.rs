mod audio_capture;
mod buffering;
mod engine;
mod protocol;
mod vad;
mod worker;

use std::path::PathBuf;

use anyhow::Context;
use audio_capture::record_default_input_to_wav;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .init();

    match parse_args(std::env::args().skip(1).collect())? {
        Command::Worker => worker::run_stdio(),
        Command::Record { path, seconds } => record_default_input_to_wav(&path, seconds),
    }
}

enum Command {
    Worker,
    Record { path: PathBuf, seconds: u64 },
}

fn parse_args(args: Vec<String>) -> anyhow::Result<Command> {
    if args.is_empty() {
        return Ok(Command::Worker);
    }

    if args[0] != "record" {
        anyhow::bail!("unsupported command: {}", args[0]);
    }

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
            Command::Worker => panic!("expected record command"),
        }
    }
}
