# Phase 2: Protocol and Process Architecture

> Status: Complete enough for Phase 3. The repo already has Electron-to-Rust named-pipe IPC, Rust-to-Python NDJSON, and mock transcript forwarding. Remaining work should be treated as hardening, not a blocker for ASR validation.

## Context

This is **Phase 2** of 9 in building OpenWhisper, a Windows-first offline dictation desktop app with three layers: Electron (UI), Rust (native helper), and Python (ASR worker).

**Purpose**: Define stable, reliable communication channels between all three layers. This is the nervous system of the application - if communication fails, nothing works.

**Communication flows**:
- Electron ↔ Rust: Local IPC (sockets/pipes)
- Rust ↔ Python: NDJSON over stdio

---

## Prerequisites

- [ ] Phase 1 complete - all three layers build independently
- [ ] Can run Electron app
- [ ] Can run Rust helper
- [ ] Can run Python worker
- [ ] Read [Error Handling](16-ERROR-HANDLING.md) for error message formats

---

## Goal

Establish stable communication:
1. Electron can send commands to Rust and receive events
2. Rust can spawn Python and exchange messages
3. Protocol handles errors gracefully
4. All messages are strongly typed

---

## Communication Architecture

```
Electron Main Process                    Rust Helper                    Python Worker
       │                                      │                               │
       │ 1. IPC (local socket/pipe)           │                               │
       │◄────────────────────────────────────►│                               │
       │   Commands: start/stop dictation     │                               │
       │   Events: transcript, status         │                               │
       │                                      │                               │
       │                               2. NDJSON over stdio                 │
       │                                      │◄─────────────────────────────►│
       │                                      │   Commands: audio.chunk       │
       │                                      │   Events: transcript.*        │
       │                                      │                               │
       ▼                                      ▼                               ▼
  Renderer                              System APIs                    Model Inference
```

---

## Layer 1: Electron ↔ Rust IPC

### Transport

**Windows**: Named pipes (`\\.\pipe\OpenWhisper-{pid}`)

**Implementation**:
```typescript
// apps/desktop/src/main/ipc.ts
import { ipcMain, ipcRenderer } from 'electron';

// Main process (Electron)
ipcMain.handle('dictation:start', async () => {
  return rustClient.send({ type: 'dictation.start' });
});

ipcMain.on('rust:event', (event, data) => {
  // Forward to renderer
  mainWindow.webContents.send('asr:event', data);
});

// Preload (bridge)
contextBridge.exposeInMainWorld('api', {
  startDictation: () => ipcRenderer.invoke('dictation:start'),
  onTranscript: (callback) => ipcRenderer.on('transcript', callback)
});
```

### Message Types (Electron ↔ Rust)

**Commands** (Electron → Rust):
```typescript
// apps/desktop/src/shared/types.ts

interface DictationStartCommand {
  type: 'dictation.start';
  settings?: Partial<AudioSettings>;
}

interface DictationStopCommand {
  type: 'dictation.stop';
}

interface GetStatusCommand {
  type: 'status.get';
}

interface UpdateSettingsCommand {
  type: 'settings.update';
  settings: UserSettings;
}

type Command = 
  | DictationStartCommand 
  | DictationStopCommand 
  | GetStatusCommand 
  | UpdateSettingsCommand;
```

**Events** (Rust → Electron):
```typescript
interface DictationStartedEvent {
  type: 'dictation.started';
  timestamp: number;
}

interface TranscriptPartialEvent {
  type: 'transcript.partial';
  text: string;
  isFinal: false;
}

interface TranscriptFinalEvent {
  type: 'transcript.final';
  text: string;
  words: WordResult[];
}

interface StatusEvent {
  type: 'status';
  isDictating: boolean;
  isModelLoaded: boolean;
  workerHealthy: boolean;
}

interface ErrorEvent {
  type: 'error';
  code: string;
  message: string;
  recoverable: boolean;
}

type Event =
  | DictationStartedEvent
  | TranscriptPartialEvent
  | TranscriptFinalEvent
  | StatusEvent
  | ErrorEvent;
```

---

## Layer 2: Rust ↔ Python Protocol

### Transport: NDJSON over Stdio

**Why NDJSON?**
- Simple: One JSON object per line
- Language-agnostic: Works with any language
- Debuggable: Can inspect with `cat` or `tail`
- Reliable: Line boundaries are clear

**Format**:
```
{"type": "health.check", "timestamp": 1234567890}\n
{"type": "health.ok", "timestamp": 1234567890, "status": "ready"}\n
{"type": "audio.chunk", "data": "base64encoded...", "timestamp": 1234567890}\n
{"type": "transcript.partial", "text": "hello world", "isFinal": false}\n```

### Message Types (Rust ↔ Python)

**Rust → Python**:
```rust
// crates/openwhisper-native/src/protocol.rs

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ToPython {
    #[serde(rename = "health.check")]
    HealthCheck { timestamp: u64 },
    
    #[serde(rename = "dictation.start")]
    DictationStart {
        language: Option<String>,
        chunk_duration_ms: u32,
    },
    
    #[serde(rename = "dictation.stop")]
    DictationStop,
    
    #[serde(rename = "audio.chunk")]
    AudioChunk {
        data: String, // base64
        timestamp: u64,
        is_final: bool,
    },
    
    #[serde(rename = "model.load")]
    ModelLoad {
        model_path: String,
        device: String, // "cuda" or "cpu"
    },
    
    #[serde(rename = "settings.update")]
    SettingsUpdate {
        settings: PythonSettings,
    },
    
    #[serde(rename = "shutdown")]
    Shutdown,
}
```

**Python → Rust**:
```python
# services/asr/src/openwhisper_asr/protocol.py

from typing import Literal, TypedDict, Optional, List
from dataclasses import dataclass

class HealthOk(TypedDict):
    type: Literal["health.ok"]
    timestamp: int
    status: str  # "ready", "loading", "error"

class ModelLoaded(TypedDict):
    type: Literal["model.loaded"]
    device: str  # "cuda" or "cpu"
    memory_mb: float

class ModelError(TypedDict):
    type: Literal["model.error"]
    error: str
    recoverable: bool

class TranscriptPartial(TypedDict):
    type: Literal["transcript.partial"]
    text: str
    is_final: Literal[False]
    processing_latency_ms: int

class WordResult(TypedDict):
    text: str
    start_ms: int
    end_ms: int
    confidence: Optional[float]

class TranscriptFinal(TypedDict):
    type: Literal["transcript.final"]
    text: str
    words: List[WordResult]
    language: Optional[str]
    processing_latency_ms: int

class TranscriptError(TypedDict):
    type: Literal["transcript.error"]
    error: str
    chunk_timestamp: int

class AudioError(TypedDict):
    type: Literal["audio.error"]
    error: str
    code: str  # "device_disconnected", "buffer_overrun", etc.

PythonMessage = (
    HealthOk | ModelLoaded | ModelError | 
    TranscriptPartial | TranscriptFinal | 
    TranscriptError | AudioError
)
```

---

## Shared Data Types

### TranscriptionResult

**TypeScript** (Electron):
```typescript
interface TranscriptionResult {
  text: string;
  words: WordResult[];
  language: string | null;
  isPartial: boolean;
  processingLatencyMs: number;
}

interface WordResult {
  text: string;
  startMs: number;
  endMs: number;
  confidence?: number;
}
```

**Rust** (native helper):
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub words: Vec<WordResult>,
    pub language: Option<String>,
    pub is_partial: bool,
    pub processing_latency_ms: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WordResult {
    pub text: String,
    pub start_ms: u32,
    pub end_ms: u32,
    pub confidence: Option<f32>,
}
```

**Python** (ASR worker):
```python
@dataclass
class TranscriptionResult:
    text: str
    words: List[WordResult]
    language: Optional[str]
    is_partial: bool
    processing_latency_ms: int

@dataclass
class WordResult:
    text: str
    start_ms: int
    end_ms: int
    confidence: Optional[float] = None
```

### Settings

```typescript
// Shared across all layers
interface AudioSettings {
  deviceId: string;
  sampleRate: number;
  chunkDurationMs: number; // 500, 2000, or 4000
  vadEnabled: boolean;
}

interface DictationSettings {
  hotkey: string; // e.g., "Ctrl+Shift+D"
  injectionMethod: 'auto' | 'keystrokes' | 'clipboard';
  language: string; // "auto" or language code
  punctuation: boolean;
}

interface UserSettings {
  audio: AudioSettings;
  dictation: DictationSettings;
  ui: UISettings;
  system: SystemSettings;
}
```

---

## Protocol Defaults

- **Use NDJSON** over stdio between Rust and Python (not gRPC, not custom binary)
- **Use strongly typed TypeScript** interfaces in Electron (avoid `any`)
- **Mirror protocol structs** in Rust and Python (same field names, same semantics)
- **Include timestamps** in all messages for latency tracking
- **Include latency metrics** in transcript events
- **All errors include `recoverable` flag** for automatic retry decisions

---

## Error Handling in Protocol

All error messages follow this format:

```json
{
  "type": "error",
  "code": "WORKER_CRASHED",
  "message": "Python worker process exited unexpectedly",
  "recoverable": true,
  "details": {
    "exit_code": 1,
    "stderr": "..."
  }
}
```

**Error codes**:
- `WORKER_CRASHED` - Python process died
- `MODEL_LOAD_FAILED` - Could not load ASR model
- `AUDIO_DEVICE_ERROR` - Microphone issue
- `INJECTION_BLOCKED` - Cannot inject into target window
- `TIMEOUT` - Operation timed out
- `PROTOCOL_ERROR` - Malformed message

See [Error Handling](16-ERROR-HANDLING.md) for complete error taxonomy.

---

## Health Check Protocol

Every 5 seconds, Rust sends health check:

```json
{"type": "health.check", "timestamp": 1704123456}
```

Python must respond within 2 seconds:

```json
{"type": "health.ok", "timestamp": 1704123456, "status": "ready"}
```

**Timeout handling**:
1. No response after 2s: Log warning
2. No response after 5s: Mark worker unhealthy
3. No response after 10s: Attempt restart

---

## Implementation Example

### Python Side (Reading NDJSON)

```python
# services/asr/src/openwhisper_asr/protocol.py
import sys
import json
from typing import Iterator, Dict, Any

def read_messages() -> Iterator[Dict[str, Any]]:
    """Read NDJSON messages from stdin."""
    for line in sys.stdin:
        line = line.strip()
        if line:
            try:
                yield json.loads(line)
            except json.JSONDecodeError as e:
                send_error(f"Invalid JSON: {e}")

def send_message(msg: Dict[str, Any]) -> None:
    """Send NDJSON message to stdout."""
    print(json.dumps(msg), flush=True)

def send_error(message: str, recoverable: bool = True) -> None:
    """Send error message."""
    send_message({
        "type": "error",
        "message": message,
        "recoverable": recoverable
    })

# Main loop
def main():
    for msg in read_messages():
        handle_message(msg)
```

### Rust Side (Reading NDJSON)

```rust
// crates/openwhisper-native/src/protocol.rs
use std::io::{BufRead, BufReader, Write};
use serde::{Serialize, Deserialize};

pub struct PythonProtocol<R, W> {
    reader: BufReader<R>,
    writer: W,
}

impl<R: std::io::Read, W: std::io::Write> PythonProtocol<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer,
        }
    }
    
    pub fn send(&mut self, msg: &ToPython) -> anyhow::Result<()> {
        let json = serde_json::to_string(msg)?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }
    
    pub fn recv(&mut self) -> anyhow::Result<FromPython> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        let msg = serde_json::from_str(&line)?;
        Ok(msg)
    }
}
```

---

## Success Criteria

- [ ] Electron can request status from Rust and receive response
- [ ] Rust can spawn Python worker process
- [ ] Rust can send messages to Python via NDJSON
- [ ] Python can send messages to Rust via NDJSON
- [ ] Python can send mock `transcript.partial` events
- [ ] Python can send mock `transcript.final` events
- [ ] UI can render live mock transcripts
- [ ] Health check protocol works (5s interval)
- [ ] Error messages are properly formatted and include `recoverable` flag
- [ ] All TypeScript interfaces have no `any` types

---

## Testing the Protocol

```bash
# Terminal 1: Start Python worker
cd services/asr
echo '{"type": "health.check"}' | uv run python -m openwhisper_asr

# Terminal 2: Test with Rust
cd crates/openwhisper-native
echo '{"type": "health.check"}' | cargo run

# Terminal 3: Run Electron
bun dev
# Click "Check Health" button
```

---

## Related Documents

- **Error Handling**: [Document 16](16-ERROR-HANDLING.md) - Error message formats
- **ASR Engine**: [Phase 3](07-PHASE-03-ASR-ENGINE.md) - Real transcripts
- **Rust Helper**: [Phase 5](09-PHASE-05-RUST-HELPER.md) - IPC implementation
- **Previous Phase**: [Phase 1](05-PHASE-01-REPO-SETUP.md) - Prerequisites
- **Next Phase**: [Phase 3](07-PHASE-03-ASR-ENGINE.md) - ASR validation
