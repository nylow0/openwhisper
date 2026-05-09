# Error Handling & Diagnostics Strategy

## Context

This document defines how OpenWhisper handles failures, errors, and provides diagnostic capabilities. OpenWhisper is a Windows-first offline dictation desktop app with three layers: Electron (UI), Rust (native helper), and Python (ASR worker). Each layer can fail independently, and this strategy ensures graceful degradation and clear user communication.

---

## Error Classification

### 1. Fatal Errors (App Cannot Continue)

These errors require app restart or reinstallation:

| Error | User Message | Action |
|-------|--------------|--------|
| Rust helper binary missing/corrupt | "OpenWhisper needs to be reinstalled. Please download the latest version." | Exit gracefully, offer reinstall link |
| Python runtime missing/corrupt | "Installation is incomplete. Please reinstall OpenWhisper." | Exit gracefully |
| Config file corrupt | "Settings were reset to defaults due to an error." | Reset config, continue |
| Model files corrupt | "The speech model needs to be re-downloaded." | Clear model cache, trigger re-download |

### 2. Recoverable Errors (App Can Continue)

These errors should be handled without user intervention where possible:

| Error | Recovery Strategy | User Notification |
|-------|-------------------|-------------------|
| Python worker crash | Auto-restart with exponential backoff (max 3 attempts) | Tray notification: "Dictation paused. Retrying..." |
| Audio device disconnected | Pause dictation, wait for device reconnect | Overlay: "Microphone disconnected" |
| Audio device in use by another app | Retry with backoff, offer settings | Settings notification |
| Model inference timeout | Abort chunk, continue with next | None (internal telemetry) |
| Text injection blocked | Fallback to clipboard | None (seamless fallback) |
| Target window elevated | Report limitation, use clipboard | Overlay: "Some apps require clipboard mode" |

### 3. User Errors (User Action Required)

| Error | User Message | Guidance |
|-------|--------------|----------|
| No microphone detected | "No microphone found. Please connect a microphone and try again." | Link to audio settings |
| Hotkey conflict | "Your chosen hotkey is used by another app." | Offer alternative hotkeys |
| Model download failed | "Could not download the speech model." | Retry button, manual download option |
| Insufficient disk space | "Not enough space to download the speech model." | Show required space, offer cleanup |
| GPU out of memory | "Your graphics card ran out of memory. Switching to CPU mode." | Auto-fallback to CPU |

---

## Error Propagation Flow

```
Python Worker Error
  -> Send NDJSON error message to Rust
  -> Rust decides: recoverable vs fatal
  -> If recoverable: attempt recovery
  -> If fatal or recovery fails: forward to Electron
  -> Electron displays appropriate UI
  -> Log full error details for diagnostics

Rust Helper Error
  -> Attempt local recovery
  -> If unrecoverable: send to Electron
  -> Electron displays error

Electron Error
  -> Display directly in UI
  -> Log to file
```

---

## Diagnostics & Logging

### Log Levels

| Level | Use Case | Default Setting |
|-------|----------|-----------------|
| ERROR | Failures that affect functionality | Always enabled |
| WARN | Recoverable issues, fallbacks | Always enabled |
| INFO | Normal operation (start/stop dictation, model loaded) | Always enabled |
| DEBUG | Detailed operation logs (chunk timing, IPC messages) | Off by default |
| TRACE | Very verbose (audio buffer state, raw IPC) | Off by default |

### Log Locations

```
%APPDATA%\OpenWhisper\logs\
  openwhisper.log          (current session)
  openwhisper.log.1        (previous session)
  openwhisper.log.2        (session before that)
```

**Log rotation**: Keep last 5 sessions, max 10MB per file.

### Diagnostic Mode

Enable via settings or command-line flag `--diagnostics`:

- Sets log level to DEBUG
- Captures audio samples (with user consent)
- Records timing metrics for every stage
- Exports detailed system info

### Diagnostics Export

User can export diagnostics package:

```
Open Settings -> Advanced -> Export Diagnostics

Generates: %APPDATA%\OpenWhisper\diagnostics\diagnostics-{timestamp}.zip
Contents:
  - logs/ (last 5 sessions)
  - config.json (sanitized)
  - system-info.json (OS, CPU, RAM, GPU)
  - crash-reports/ (if any)
  - audio-samples/ (if user opted in)
```

---

## Specific Error Scenarios

### Scenario 1: Model Loading Failure

```python
# Python worker
try:
    model = load_model(model_path)
except ModelCorruptError:
    send_error({
        "type": "model.corrupt",
        "recoverable": True,
        "action": "clear_cache_and_redownload",
        "userMessage": "The speech model appears to be damaged. It will be re-downloaded."
    })
except OutOfMemoryError:
    send_error({
        "type": "gpu.out_of_memory",
        "recoverable": True,
        "action": "fallback_to_cpu",
        "userMessage": "Your GPU ran out of memory. Switching to CPU mode (may be slower)."
    })
```

### Scenario 2: Audio Capture Failure

```rust
// Rust helper (after Phase 6 migration)
match start_audio_capture() {
    Ok(stream) => stream,
    Err(AudioDeviceDisconnected) => {
        notify_electron("audio.device_disconnected");
        wait_for_device_reconnect();
    },
    Err(AudioDeviceInUse) => {
        notify_electron("audio.device_in_use");
        retry_with_backoff();
    }
}
```

### Scenario 3: Text Injection Failure

```rust
// Rust helper
match inject_text(&text) {
    Ok(_) => {},
    Err(BlockedByTarget) => {
        // Fallback to clipboard
        clipboard::set_text(&text);
        notify_electron("injection.fallback_to_clipboard");
    },
    Err(TargetElevated) => {
        notify_electron("injection.target_elevated");
    }
}
```

---

## Health Check Protocol

Every 5 seconds, Rust sends health check to Python:

```json
{"type": "health.check", "timestamp": 1234567890}
```

Python must respond within 2 seconds:

```json
{"type": "health.ok", "timestamp": 1234567890, "status": "ready"}
```

**Failure handling**:
- No response after 2s: retry once
- Still no response: mark worker as unhealthy
- Attempt graceful restart
- If restart fails 3 times: notify user, pause dictation

---

## User-Facing Error UI

### Toast Notifications (Tray)

For non-blocking errors:
- "Dictation paused - microphone disconnected"
- "Switched to clipboard mode"
- "Model downloading..."

### Overlay Messages

For errors during dictation:
- "Listening..." (normal)
- "Processing..." (inference running)
- "Microphone issue detected" (warning)
- "Please check your microphone" (error)

### Settings Dialog

For persistent errors:
- Show error banner at top
- Provide actionable buttons (Retry, Reset, Help)
- Link to diagnostics export

---

## Crash Recovery

### Python Worker Crash

1. Rust detects process exit
2. Log crash details (exit code, stderr tail)
3. Attempt restart (max 3 attempts)
4. Backoff: 1s, 5s, 30s between attempts
5. If all fail: notify user, pause dictation

### Rust Helper Crash

1. Electron detects IPC disconnect
2. Show error: "OpenWhisper encountered an error and needs to restart."
3. Offer "Restart" button
4. Preserve settings and model cache

### Electron Crash

1. User restarts app
2. On startup, detect unclean shutdown
3. Offer to send crash report (opt-in)
4. Restore previous settings

---

## Success Criteria

- [ ] All fatal errors show clear user messages and recovery steps
- [ ] Recoverable errors are handled without user intervention where possible
- [ ] Logs are written and rotated correctly
- [ ] Diagnostics can be exported from settings
- [ ] Python worker crashes are auto-recovered (up to 3 times)
- [ ] Audio device disconnections are handled gracefully
- [ ] Text injection fallbacks work seamlessly
- [ ] Health check protocol detects worker failures within 5 seconds

---

## Related Documents

- **Protocol Specification**: See Phase 2 document for message format details
- **ASR Engine**: See Phase 3 document for model error scenarios
- **Audio Pipeline**: See Phase 6 document for audio error handling
- **Performance**: See Phase 8 document for timeout thresholds
