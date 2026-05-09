use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::ipc_protocol::{IpcCommand, IpcEvent};

pub struct IpcServer {
    pipe_name: String,
    tx: mpsc::Sender<IpcCommand>,
    rx: mpsc::Receiver<IpcEvent>,
    event_tx: mpsc::Sender<IpcEvent>,
}

impl IpcServer {
    pub fn new(pid: u32) -> (Self, mpsc::Receiver<IpcCommand>, mpsc::Sender<IpcEvent>) {
        let pipe_name = format!(r"\\.\pipe\OpenWhisper-{}", pid);
        let (cmd_tx, cmd_rx) = mpsc::channel::<IpcCommand>(100);
        let (event_tx, event_rx) = mpsc::channel::<IpcEvent>(100);

        let server = IpcServer {
            pipe_name: pipe_name.clone(),
            tx: cmd_tx,
            rx: event_rx,
            event_tx: event_tx.clone(),
        };

        (server, cmd_rx, event_tx)
    }

    pub async fn run(self) -> Result<()> {
        let pipe_name = self.pipe_name.clone();
        log::info!("Starting IPC server on {}", pipe_name);
        println!("Starting IPC server on {}", pipe_name);

        // Create initial pipe instance
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe_name)?;

        self.handle_connection(server).await?;
        Ok(())
    }

    async fn handle_connection(self, server: NamedPipeServer) -> Result<()> {
        log::info!("Waiting for Electron to connect...");

        match timeout(Duration::from_secs(30), server.connect()).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                log::error!("Named pipe connect error: {}", e);
                return Err(e.into());
            }
            Err(_) => {
                log::error!("Timeout waiting for Electron connection");
                return Err(anyhow::anyhow!("Timeout waiting for Electron connection"));
            }
        }

        log::info!("Electron connected via named pipe");

        let (reader, mut writer) = tokio::io::split(server);
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        // Spawn event sender task
        let mut event_rx = self.rx;
        let event_tx_for_error = self.event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let json = match serde_json::to_string(&event) {
                    Ok(j) => j,
                    Err(e) => {
                        log::error!("Failed to serialize IPC event: {}", e);
                        continue;
                    }
                };
                if let Err(e) = writer.write_all(json.as_bytes()).await {
                    log::error!("Failed to write IPC event: {}", e);
                    break;
                }
                if let Err(e) = writer.write_all(b"\n").await {
                    log::error!("Failed to write newline: {}", e);
                    break;
                }
                if let Err(e) = writer.flush().await {
                    log::error!("Failed to flush IPC: {}", e);
                    break;
                }
            }
            log::info!("IPC event sender task shutting down");
        });

        // Read commands from Electron
        loop {
            line.clear();
            match timeout(Duration::from_secs(5), buf_reader.read_line(&mut line)).await {
                Ok(Ok(0)) => {
                    log::info!("Electron disconnected");
                    break;
                }
                Ok(Ok(_)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<IpcCommand>(trimmed) {
                        Ok(cmd) => {
                            if self.tx.send(cmd).await.is_err() {
                                log::info!("Command receiver dropped");
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to parse IPC command: {} | raw: {}", e, trimmed);
                            let _ = event_tx_for_error.send(IpcEvent::Error {
                                code: "PROTOCOL_ERROR".to_string(),
                                message: format!("Invalid command: {}", e),
                                recoverable: true,
                            }).await;
                        }
                    }
                }
                Ok(Err(e)) => {
                    log::error!("Failed to read from IPC: {}", e);
                    break;
                }
                Err(_) => {
                    // Timeout on read - this is normal, just continue
                    continue;
                }
            }
        }

        log::info!("IPC server shutting down");
        Ok(())
    }
}
