use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::ipc_protocol::{IpcCommand, IpcEvent};

pub struct IpcServer {
    address: String,
    tx: mpsc::Sender<IpcCommand>,
    rx: mpsc::Receiver<IpcEvent>,
    event_tx: mpsc::Sender<IpcEvent>,
}

impl IpcServer {
    pub fn new(pid: u32) -> (Self, mpsc::Receiver<IpcCommand>, mpsc::Sender<IpcEvent>) {
        let address = ipc_address(pid);
        let (cmd_tx, cmd_rx) = mpsc::channel::<IpcCommand>(100);
        let (event_tx, event_rx) = mpsc::channel::<IpcEvent>(100);

        let server = IpcServer {
            address: address.clone(),
            tx: cmd_tx,
            rx: event_rx,
            event_tx: event_tx.clone(),
        };

        (server, cmd_rx, event_tx)
    }

    pub async fn run(self) -> Result<()> {
        log::info!("Starting IPC server on {}", self.address);
        println!("Starting IPC server on {}", self.address);
        self.run_platform().await
    }
}

#[cfg(windows)]
fn ipc_address(pid: u32) -> String {
    format!(r"\\.\pipe\OpenWhisper-{}", pid)
}

#[cfg(unix)]
fn ipc_address(pid: u32) -> String {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/openwhisper-{}.sock", runtime_dir, pid)
}

#[cfg(windows)]
impl IpcServer {
    async fn run_platform(self) -> Result<()> {
        use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&self.address)?;

        self.handle_connection_windows(server).await
    }

    async fn handle_connection_windows(self, server: NamedPipeServer) -> Result<()> {
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

        let (reader, writer) = tokio::io::split(server);
        self.handle_streams(BufReader::new(reader), writer).await
    }
}

#[cfg(unix)]
impl IpcServer {
    async fn run_platform(self) -> Result<()> {
        use tokio::net::UnixListener;

        let socket_path = &self.address;

        // Remove stale socket file if it exists.
        let _ = std::fs::remove_file(socket_path);

        let listener = UnixListener::bind(socket_path)?;
        log::info!("Waiting for Electron to connect...");

        let stream = match timeout(Duration::from_secs(30), listener.accept()).await {
            Ok(Ok((stream, _addr))) => stream,
            Ok(Err(e)) => {
                log::error!("Unix socket accept error: {}", e);
                return Err(e.into());
            }
            Err(_) => {
                log::error!("Timeout waiting for Electron connection");
                return Err(anyhow::anyhow!("Timeout waiting for Electron connection"));
            }
        };

        log::info!("Electron connected via Unix socket");

        let (reader, writer) = tokio::io::split(stream);
        self.handle_streams(BufReader::new(reader), writer).await
    }
}

impl IpcServer {
    async fn handle_streams<R, W>(self, mut buf_reader: BufReader<R>, mut writer: W) -> Result<()>
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
        W: tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let mut event_rx = self.rx;
        let _event_tx_for_error = self.event_tx.clone();

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

        let mut line = String::new();
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
                            let _ = self.event_tx.send(IpcEvent::Error {
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
                    continue;
                }
            }
        }

        log::info!("IPC server shutting down");
        Ok(())
    }
}
