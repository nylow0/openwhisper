use std::process::Stdio;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::protocol::{FromPython, ToPython};

const HEALTH_CHECK_INTERVAL_SECS: u64 = 5;

fn find_service_dir() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("asr");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    Err(anyhow::anyhow!("Could not find asr from current directory"))
}

pub struct PythonWorker {
    #[allow(dead_code)]
    child: Child,
    tx: mpsc::Sender<ToPython>,
    rx: mpsc::Receiver<FromPython>,
}

impl PythonWorker {
    pub async fn spawn() -> Result<Self> {
        // Try venv python first, then uv, then system python
        let service_dir = find_service_dir()?;
        let venv_python = service_dir.join(".venv").join("Scripts").join("python.exe");
        let (program, args) = if venv_python.exists() {
            (venv_python.to_string_lossy().to_string(), vec!["-m", "openwhisper_asr"])
        } else {
            ("uv".to_string(), vec!["run", "python", "-m", "openwhisper_asr"])
        };

        let mut child = Command::new(&program)
            .args(&args)
            .current_dir(service_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");

        let (to_python_tx, mut to_python_rx) = mpsc::channel::<ToPython>(100);
        let (from_python_tx, from_python_rx) = mpsc::channel::<FromPython>(100);
        let (health_tx, _health_rx) = mpsc::channel::<bool>(10);

        // Writer task
        let writer_tx = to_python_tx.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = to_python_rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(e) => {
                        log::error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };
                if let Err(e) = stdin.write_all(json.as_bytes()).await {
                    log::error!("Failed to write to Python stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    log::error!("Failed to write newline to Python stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    log::error!("Failed to flush Python stdin: {}", e);
                    break;
                }
            }
            log::info!("Python writer task shutting down");
        });

        // Reader task
        let health_tx_reader = health_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        log::info!("Python stdout closed");
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<FromPython>(trimmed) {
                            Ok(msg) => {
                                if let FromPython::HealthOk { .. } = &msg {
                                    let _ = health_tx_reader.send(true).await;
                                }
                                if from_python_tx.send(msg).await.is_err() {
                                    log::info!("Python event receiver dropped");
                                    break;
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to parse Python message: {} | raw: {}", e, trimmed);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read from Python stdout: {}", e);
                        break;
                    }
                }
            }
            log::info!("Python reader task shutting down");
        });

        // Health check task
        let health_check_tx = writer_tx;
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS));
            ticker.tick().await; // First tick is immediate, skip it
            loop {
                ticker.tick().await;
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let msg = ToPython::HealthCheck { timestamp: ts };
                if health_check_tx.send(msg).await.is_err() {
                    log::info!("Health check: writer channel closed");
                    break;
                }
            }
        });

        Ok(PythonWorker {
            child,
            tx: to_python_tx.clone(),
            rx: from_python_rx,
        })
    }

    pub async fn send(&self, msg: ToPython) -> Result<()> {
        self.tx.send(msg).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Option<FromPython> {
        self.rx.recv().await
    }
}
