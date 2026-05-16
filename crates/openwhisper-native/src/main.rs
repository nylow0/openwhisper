use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio::signal;

mod ipc;
mod ipc_protocol;
mod protocol;
mod python;

use ipc::IpcServer;
use ipc_protocol::{IpcCommand, IpcEvent};
use protocol::{FromPython, PythonSettings, ToPython};
use python::PythonWorker;

#[derive(Debug, Clone)]
struct AppState {
    is_dictating: bool,
    is_model_loaded: bool,
    worker_healthy: bool,
}

fn python_settings_from_ipc(settings: Option<&serde_json::Value>) -> PythonSettings {
    let audio = settings
        .and_then(|value| value.get("audio"))
        .or(settings);
    let dictation = settings.and_then(|value| value.get("dictation"));

    let language = dictation
        .and_then(|value| value.get("language"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty() && *value != "auto")
        .map(ToOwned::to_owned);

    let chunk_duration_ms = audio
        .and_then(|value| value.get("chunkDurationMs"))
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(2_000);

    let vad_enabled = audio
        .and_then(|value| value.get("vadEnabled"))
        .and_then(|value| value.as_bool())
        .unwrap_or(true);

    PythonSettings {
        language,
        chunk_duration_ms,
        vad_enabled,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("OpenWhisper Native Helper starting...");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut pipe_pid: Option<u32> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--pipe-pid" && i + 1 < args.len() {
            pipe_pid = args[i + 1].parse().ok();
            i += 2;
        } else {
            i += 1;
        }
    }

    let pid = pipe_pid.unwrap_or_else(std::process::id);
    let (ipc_server, mut ipc_cmd_rx, ipc_event_tx) = IpcServer::new(pid);

    // Spawn IPC server in background
    let ipc_handle = tokio::spawn(async move {
        if let Err(e) = ipc_server.run().await {
            log::error!("IPC server error: {}", e);
        }
    });

    // Spawn Python worker
    let mut python_worker = PythonWorker::spawn().await?;
    log::info!("Python worker spawned");

    // Shared state
    let state = Arc::new(RwLock::new(AppState {
        is_dictating: false,
        is_model_loaded: false,
        worker_healthy: true,
    }));

    // Main bridge loop
    let state_clone = state.clone();
    let bridge_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(cmd) = ipc_cmd_rx.recv() => {
                    let mut s = state_clone.write().await;
                    match cmd {
                        IpcCommand::DictationStart { settings } => {
                            log::info!("Received dictation.start from Electron");
                            let python_settings = python_settings_from_ipc(settings.as_ref());
                            s.is_dictating = true;
                            drop(s);
                            let _ = ipc_event_tx.send(IpcEvent::DictationStarted {
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                            }).await;
                            if let Err(e) = python_worker.send(ToPython::DictationStart {
                                language: python_settings.language,
                                chunk_duration_ms: python_settings.chunk_duration_ms,
                            }).await {
                                log::error!("Failed to send dictation.start to Python: {}", e);
                            }
                        }
                        IpcCommand::DictationStop => {
                            log::info!("Received dictation.stop from Electron");
                            s.is_dictating = false;
                            drop(s);
                            if let Err(e) = python_worker.send(ToPython::DictationStop).await {
                                log::error!("Failed to send dictation.stop to Python: {}", e);
                            }
                            let _ = ipc_event_tx.send(IpcEvent::DictationStopped {
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                            }).await;
                        }
                        IpcCommand::GetStatus => {
                            log::info!("Received status.get from Electron");
                            drop(s);
                            let status = state_clone.read().await;
                            let _ = ipc_event_tx.send(IpcEvent::Status {
                                is_dictating: status.is_dictating,
                                is_model_loaded: status.is_model_loaded,
                                worker_healthy: status.worker_healthy,
                            }).await;
                        }
                        IpcCommand::UpdateSettings { settings } => {
                            log::info!("Received settings.update from Electron");
                            let python_settings = python_settings_from_ipc(Some(&settings));
                            drop(s);
                            if let Err(e) = python_worker.send(ToPython::SettingsUpdate {
                                settings: python_settings,
                            }).await {
                                log::error!("Failed to send settings.update to Python: {}", e);
                            }
                        }
                    }
                }
                Some(event) = python_worker.recv() => {
                    match event {
                        FromPython::HealthOk { timestamp, status } => {
                            log::debug!("Python health.ok: ts={}, status={}", timestamp, status);
                            let mut s = state_clone.write().await;
                            s.worker_healthy = true;
                        }
                        FromPython::ModelLoaded { device, memory_mb } => {
                            log::info!("Python model.loaded: device={}, memory={}MB", device, memory_mb);
                            let mut s = state_clone.write().await;
                            s.is_model_loaded = true;
                        }
                        FromPython::ModelError { error, recoverable } => {
                            log::error!("Python model.error: {} (recoverable={})", error, recoverable);
                            let mut s = state_clone.write().await;
                            s.is_dictating = false;
                            let _ = ipc_event_tx.send(IpcEvent::Error {
                                code: "MODEL_LOAD_FAILED".to_string(),
                                message: error,
                                recoverable,
                            }).await;
                        }
                        FromPython::TranscriptPartial { text, is_final, processing_latency_ms } => {
                            let _ = ipc_event_tx.send(IpcEvent::TranscriptPartial {
                                text,
                                is_final,
                                processing_latency_ms,
                            }).await;
                        }
                        FromPython::TranscriptFinal { text, words, language, processing_latency_ms } => {
                            let _ = ipc_event_tx.send(IpcEvent::TranscriptFinal {
                                text,
                                words,
                                language,
                                processing_latency_ms,
                            }).await;
                        }
                        FromPython::TranscriptError { error, chunk_timestamp } => {
                            log::error!("Python transcript.error at {}: {}", chunk_timestamp, error);
                            let _ = ipc_event_tx.send(IpcEvent::Error {
                                code: "TRANSCRIPT_ERROR".to_string(),
                                message: error,
                                recoverable: true,
                            }).await;
                        }
                        FromPython::AudioError { error, code } => {
                            log::error!("Python audio.error ({}): {}", code, error);
                            let mut s = state_clone.write().await;
                            s.is_dictating = false;
                            let _ = ipc_event_tx.send(IpcEvent::Error {
                                code,
                                message: error,
                                recoverable: true,
                            }).await;
                        }
                        FromPython::Error { code, message, recoverable } => {
                            log::error!("Python error: {} (recoverable={})", message, recoverable);
                            let _ = ipc_event_tx.send(IpcEvent::Error {
                                code: code.unwrap_or_else(|| "WORKER_ERROR".to_string()),
                                message,
                                recoverable,
                            }).await;
                        }
                    }
                }
                else => {
                    log::info!("Bridge channels closed, shutting down");
                    break;
                }
            }
        }
    });

    // Wait for Ctrl+C
    match signal::ctrl_c().await {
        Ok(()) => {
            log::info!("Received Ctrl+C, shutting down...");
        }
        Err(e) => {
            log::error!("Failed to listen for Ctrl+C: {}", e);
        }
    }

    // Abort background tasks to trigger shutdown
    bridge_handle.abort();
    ipc_handle.abort();

    log::info!("OpenWhisper Native Helper shutting down...");
    Ok(())
}
