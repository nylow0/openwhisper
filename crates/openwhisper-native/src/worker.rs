use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::protocol::{FromWorker, ToWorker};

const HEALTH_CHECK_INTERVAL_SECS: u64 = 5;
const WORKER_ENV: &str = "OPENWHISPER_ASR_WORKER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerKind {
    Python,
    Rust,
}

impl WorkerKind {
    fn from_value(value: Option<&str>) -> Result<Self> {
        match value {
            None | Some("") | Some("python") => Ok(Self::Python),
            Some("rust") => Ok(Self::Rust),
            Some(value) => anyhow::bail!("{WORKER_ENV} must be 'python' or 'rust', got '{value}'"),
        }
    }

    fn selected() -> Result<Self> {
        Self::from_value(std::env::var(WORKER_ENV).ok().as_deref())
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::Rust => "Rust",
        }
    }
}

struct WorkerCommand {
    program: PathBuf,
    args: Vec<&'static str>,
    current_dir: Option<PathBuf>,
}

fn find_ancestor_child(child: &str, marker: &str) -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(child);
        if candidate.join(marker).exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn find_service_dir() -> Result<PathBuf> {
    find_ancestor_child("asr", "pyproject.toml")
        .context("Could not find OpenWhisper ASR directory from current directory")
}

fn find_rust_worker_dir() -> Result<PathBuf> {
    find_ancestor_child("crates/openwhisper-asr-rs", "Cargo.toml")
        .context("Could not find OpenWhisper Rust ASR crate from current directory")
}

fn worker_executable_name() -> &'static str {
    if cfg!(windows) {
        "ow-asr-rs.exe"
    } else {
        "ow-asr-rs"
    }
}

fn find_rust_worker_binary() -> Result<PathBuf> {
    let name = worker_executable_name();

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let sibling = exe_dir.join(name);
            if sibling.exists() {
                return Ok(sibling);
            }
        }
    }

    let worker_dir = find_rust_worker_dir()?;
    for candidate in [
        worker_dir.join("target").join("debug").join(name),
        worker_dir
            .join("target")
            .join("x86_64-pc-windows-msvc")
            .join("debug")
            .join(name),
    ] {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Ok(PathBuf::from("ow-asr-rs"))
}

fn python_command() -> Result<WorkerCommand> {
    let service_dir = find_service_dir()?;
    let venv_python = service_dir.join(".venv").join("Scripts").join("python.exe");
    if venv_python.exists() {
        return Ok(WorkerCommand {
            program: venv_python,
            args: vec!["-m", "openwhisper_asr"],
            current_dir: Some(service_dir),
        });
    }

    Ok(WorkerCommand {
        program: PathBuf::from("uv"),
        args: vec!["run", "python", "-m", "openwhisper_asr"],
        current_dir: Some(service_dir),
    })
}

fn rust_command() -> Result<WorkerCommand> {
    Ok(WorkerCommand {
        program: find_rust_worker_binary()?,
        args: vec![],
        current_dir: None,
    })
}

fn worker_command(kind: WorkerKind) -> Result<WorkerCommand> {
    match kind {
        WorkerKind::Python => python_command(),
        WorkerKind::Rust => rust_command(),
    }
}

pub struct AsrWorker {
    _child: Child,
    kind: WorkerKind,
    tx: mpsc::Sender<ToWorker>,
    rx: mpsc::Receiver<FromWorker>,
}

impl AsrWorker {
    pub async fn spawn() -> Result<Self> {
        let kind = WorkerKind::selected()?;
        let worker_command = worker_command(kind)?;
        log::info!(
            "Starting {} ASR worker via {:?}",
            kind.label(),
            worker_command.program
        );

        let mut command = Command::new(&worker_command.program);
        command
            .args(&worker_command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(current_dir) = worker_command.current_dir {
            command.current_dir(current_dir);
        }

        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to spawn {} ASR worker", kind.label()))?;
        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");

        let (to_worker_tx, mut to_worker_rx) = mpsc::channel::<ToWorker>(100);
        let (from_worker_tx, from_worker_rx) = mpsc::channel::<FromWorker>(100);

        let writer_tx = to_worker_tx.clone();
        let worker_label = kind.label();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = to_worker_rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(e) => {
                        log::error!("Failed to serialize ASR worker message: {}", e);
                        continue;
                    }
                };
                if let Err(e) = stdin.write_all(json.as_bytes()).await {
                    log::error!(
                        "Failed to write to {} ASR worker stdin: {}",
                        worker_label,
                        e
                    );
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    log::error!(
                        "Failed to write newline to {} ASR worker stdin: {}",
                        worker_label,
                        e
                    );
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    log::error!("Failed to flush {} ASR worker stdin: {}", worker_label, e);
                    break;
                }
            }
            log::info!("{} ASR writer task shutting down", worker_label);
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        log::info!("{} ASR worker stdout closed", worker_label);
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<FromWorker>(trimmed) {
                            Ok(msg) => {
                                if from_worker_tx.send(msg).await.is_err() {
                                    log::info!("{} ASR event receiver dropped", worker_label);
                                    break;
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to parse {} ASR worker message: {} | raw: {}",
                                    worker_label,
                                    e,
                                    trimmed
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to read from {} ASR worker stdout: {}",
                            worker_label,
                            e
                        );
                        break;
                    }
                }
            }
            log::info!("{} ASR reader task shutting down", worker_label);
        });

        let health_check_tx = writer_tx;
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS));
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if health_check_tx
                    .send(ToWorker::HealthCheck { timestamp: ts })
                    .await
                    .is_err()
                {
                    log::info!("ASR health check: writer channel closed");
                    break;
                }
            }
        });

        Ok(Self {
            _child: child,
            kind,
            tx: to_worker_tx,
            rx: from_worker_rx,
        })
    }

    pub fn kind(&self) -> WorkerKind {
        self.kind
    }

    pub async fn send(&self, msg: ToWorker) -> Result<()> {
        self.tx.send(msg).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Option<FromWorker> {
        self.rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::WorkerKind;

    #[test]
    fn default_worker_selection_keeps_python_fallback() {
        assert_eq!(WorkerKind::from_value(None).unwrap(), WorkerKind::Python);
        assert_eq!(
            WorkerKind::from_value(Some("")).unwrap(),
            WorkerKind::Python
        );
    }

    #[test]
    fn worker_selection_explicitly_opts_into_rust() {
        assert_eq!(
            WorkerKind::from_value(Some("rust")).unwrap(),
            WorkerKind::Rust
        );
    }

    #[test]
    fn worker_selection_rejects_unknown_values_before_spawn() {
        let error = WorkerKind::from_value(Some("whisper")).unwrap_err();

        assert!(error.to_string().contains("OPENWHISPER_ASR_WORKER"));
    }
}
