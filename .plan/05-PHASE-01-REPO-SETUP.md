# Phase 1: Repository Setup

> Status: Complete for current purposes. Do not rerun this document as a script against the existing repo. Use it as historical context only.

## Context

This is **Phase 1** of 9 in building OpenWhisper, a Windows-first offline dictation desktop app with a three-tier architecture:
- **Electron + Svelte + TypeScript** (UI layer)
- **Rust** (native Windows helper)
- **Python + NeMo** (ASR worker)

**Purpose of this phase**: Create the foundational monorepo structure where all three layers can be developed, built, and run independently. This sets up the development environment for all subsequent phases.

---

## Goal

Create a working monorepo with:
1. Electron desktop app building and running
2. Rust native helper compiling
3. Python ASR service running
4. Shared protocol definitions
5. Root-level orchestration scripts

**Success means**: A developer can clone the repo, run setup commands, and have all three layers working within 10 minutes.

---

## Prerequisites

- Windows 10/11 development machine
- Git installed
- **Bun** installed (https://bun.sh) - JavaScript package manager
- **Rust** installed (https://rustup.rs) - `cargo` and `rustc`
- **Python** 3.10 or 3.11 installed - System Python is fine
- **uv** installed (https://github.com/astral-sh/uv) - Python package manager

---

## Tasks

### 1. Initialize Repository Root

Create the basic repository structure:

```bash
# From repository root
git init  # If not already initialized

# Create root package.json for Bun workspaces
cat > package.json << 'EOF'
{
  "name": "openwhisper",
  "private": true,
  "workspaces": [
    "apps/*",
    "packages/*"
  ],
  "scripts": {
    "dev": "bun run --cwd apps/desktop dev",
    "build": "bun run --cwd apps/desktop build",
    "test": "bun run --cwd apps/desktop test",
    "cargo:build": "cargo build --manifest-path crates/openwhisper-native/Cargo.toml",
    "cargo:dev": "cargo run --manifest-path crates/openwhisper-native/Cargo.toml",
    "python:run": "cd services/asr && uv run python -m openwhisper_asr",
    "setup": "bun install && cargo fetch && cd services/asr && uv sync"
  },
  "devDependencies": {
    "@types/node": "^20.0.0"
  }
}
EOF
```

### 2. Initialize Bun Workspace for Desktop App

```bash
mkdir -p apps/desktop/src/{main,renderer,shared}

# Create desktop package.json
cat > apps/desktop/package.json << 'EOF'
{
  "name": "@openwhisper/desktop",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "electron:dev": "vite build && electron .",
    "electron:build": "vite build && electron-builder"
  },
  "dependencies": {
    "electron": "^28.0.0"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.0.0",
    "@tsconfig/svelte": "^5.0.0",
    "svelte": "^4.0.0",
    "svelte-check": "^3.0.0",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "electron-builder": "^24.0.0"
  },
  "main": "dist/main/index.js"
}
EOF

# Create basic Electron main process
cat > apps/desktop/src/main/index.ts << 'EOF'
import { app, BrowserWindow, ipcMain } from 'electron';
import path from 'path';

let mainWindow: BrowserWindow | null = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      preload: path.join(__dirname, '../preload/index.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(process.env.VITE_DEV_SERVER_URL);
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
  }
}

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// Health check handler
ipcMain.handle('health-check', () => {
  return { status: 'ok', timestamp: Date.now() };
});
EOF

# Create preload script
cat > apps/desktop/src/main/preload.ts << 'EOF'
import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('api', {
  healthCheck: () => ipcRenderer.invoke('health-check')
});
EOF

# Create basic Svelte app
cat > apps/desktop/src/renderer/App.svelte << 'EOF'
<script>
  let status = 'Initializing...';
  
  async function checkHealth() {
    try {
      const result = await window.api.healthCheck();
      status = `Healthy: ${new Date(result.timestamp).toLocaleTimeString()}`;
    } catch (e) {
      status = 'Error: ' + e.message;
    }
  }
  
  checkHealth();
</script>

<main class="p-8">
  <h1 class="text-3xl font-bold mb-4">OpenWhisper</h1>
  <p class="text-gray-600 mb-4">Offline dictation for Windows</p>
  <div class="bg-gray-100 p-4 rounded">
    <p>Status: {status}</p>
  </div>
  <button 
    class="mt-4 px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
    on:click={checkHealth}>
    Check Health
  </button>
</main>
EOF

cat > apps/desktop/src/renderer/main.ts << 'EOF'
import App from './App.svelte';
import './app.css';

const app = new App({
  target: document.getElementById('app')
});

export default app;
EOF

cat > apps/desktop/src/renderer/app.css << 'EOF'
@tailwind base;
@tailwind components;
@tailwind utilities;
EOF

cat > apps/desktop/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>OpenWhisper</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/renderer/main.ts"></script>
  </body>
</html>
EOF

# Create Vite config
cat > apps/desktop/vite.config.ts << 'EOF'
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

export default defineConfig({
  plugins: [svelte()],
  root: __dirname,
  base: './',
  build: {
    outDir: 'dist/renderer',
    emptyOutDir: true
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  }
});
EOF

# Create TypeScript config
cat > apps/desktop/tsconfig.json << 'EOF'
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src",
    "types": ["svelte", "node"]
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules"]
}
EOF

# Create Tailwind config
cat > apps/desktop/tailwind.config.js << 'EOF'
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/renderer/**/*.{svelte,html}'],
  theme: {
    extend: {}
  },
  plugins: []
};
EOF
```

### 3. Initialize Rust Crate

```bash
mkdir -p crates/openwhisper-native/src

# Create Cargo.toml
cat > crates/openwhisper-native/Cargo.toml << 'EOF'
[package]
name = "openwhisper-native"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "openwhisper-native"
path = "src/main.rs"

[dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
windows = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
    "Win32_System_Threading",
    "Win32_System_DataExchange"
] }
log = "0.4"
env_logger = "0.11"
anyhow = "1.0"

[dev-dependencies]
tempfile = "3.8"
EOF

# Create basic main.rs
cat > crates/openwhisper-native/src/main.rs << 'EOF'
use std::io::{self, BufRead, Write};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Message {
    #[serde(rename = "health.check")]
    HealthCheck,
    #[serde(rename = "health.ok")]
    HealthOk { timestamp: u64 },
    #[serde(rename = "echo")]
    Echo { text: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    log::info!("OpenWhisper Native Helper starting...");

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        
        match serde_json::from_str::<Message>(&line) {
            Ok(Message::HealthCheck) => {
                let response = Message::HealthOk { 
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs() 
                };
                let json = serde_json::to_string(&response)?;
                writeln!(stdout_lock, "{}", json)?;
                stdout_lock.flush()?;
            }
            Ok(Message::Echo { text }) => {
                log::info!("Echo: {}", text);
                let response = Message::Echo { text };
                let json = serde_json::to_string(&response)?;
                writeln!(stdout_lock, "{}", json)?;
                stdout_lock.flush()?;
            }
            Err(e) => {
                log::error!("Failed to parse message: {}", e);
            }
        }
    }

    log::info!("OpenWhisper Native Helper shutting down...");
    Ok(())
}
EOF
```

### 4. Initialize Python ASR Service

```bash
mkdir -p services/asr/src/openwhisper_asr

# Create pyproject.toml with uv
cat > services/asr/pyproject.toml << 'EOF'
[project]
name = "openwhisper-asr"
version = "0.1.0"
description = "OpenWhisper ASR Worker"
requires-python = ">=3.10,<3.12"
dependencies = [
    "numpy>=1.24.0",
    "sounddevice>=0.4.6",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0",
    "black>=23.0",
    "mypy>=1.0",
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src/openwhisper_asr"]

[tool.black]
line-length = 100

[tool.mypy]
python_version = "3.10"
strict = true
EOF

# Create Python package init
cat > services/asr/src/openwhisper_asr/__init__.py << 'EOF'
"""OpenWhisper ASR Worker."""

__version__ = "0.1.0"
EOF

# Create main entry point
cat > services/asr/src/openwhisper_asr/__main__.py << 'EOF'
"""ASR Worker entry point."""
import sys
import json
import logging
from typing import Dict, Any

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr
)
logger = logging.getLogger(__name__)


def handle_message(msg: Dict[str, Any]) -> Dict[str, Any]:
    """Handle incoming message."""
    msg_type = msg.get("type")
    
    if msg_type == "health.check":
        return {
            "type": "health.ok",
            "timestamp": msg.get("timestamp"),
            "status": "ready",
            "version": "0.1.0"
        }
    elif msg_type == "echo":
        return {
            "type": "echo.response",
            "text": msg.get("text", "")
        }
    else:
        return {
            "type": "error",
            "message": f"Unknown message type: {msg_type}"
        }


def main():
    """Main entry point."""
    logger.info("ASR Worker starting...")
    
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        
        try:
            msg = json.loads(line)
            logger.debug(f"Received: {msg}")
            
            response = handle_message(msg)
            print(json.dumps(response), flush=True)
            
        except json.JSONDecodeError as e:
            logger.error(f"Invalid JSON: {e}")
            print(json.dumps({"type": "error", "message": "Invalid JSON"}), flush=True)
        except Exception as e:
            logger.exception("Error handling message")
            print(json.dumps({"type": "error", "message": str(e)}), flush=True)
    
    logger.info("ASR Worker shutting down...")


if __name__ == "__main__":
    main()
EOF
```

### 5. Add Shared Protocol Package

```bash
mkdir -p packages/protocol/schemas

# Create protocol README
cat > packages/protocol/README.md << 'EOF'
# OpenWhisper Protocol

Shared protocol definitions for OpenWhisper.

## Message Format

All messages between Rust and Python use NDJSON (newline-delimited JSON).

## Schemas

See `schemas/` directory for JSON Schema definitions.
EOF

# Create basic message schema
cat > packages/protocol/schemas/messages.json << 'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "definitions": {
    "HealthCheck": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": { "const": "health.check" },
        "timestamp": { "type": "integer" }
      }
    },
    "HealthOk": {
      "type": "object",
      "required": ["type", "status"],
      "properties": {
        "type": { "const": "health.ok" },
        "timestamp": { "type": "integer" },
        "status": { "type": "string" }
      }
    },
    "AudioChunk": {
      "type": "object",
      "required": ["type", "data"],
      "properties": {
        "type": { "const": "audio.chunk" },
        "data": { "type": "string", "description": "Base64-encoded PCM audio" },
        "timestamp": { "type": "integer" }
      }
    },
    "TranscriptPartial": {
      "type": "object",
      "required": ["type", "text"],
      "properties": {
        "type": { "const": "transcript.partial" },
        "text": { "type": "string" },
        "isFinal": { "type": "boolean", "const": false }
      }
    },
    "TranscriptFinal": {
      "type": "object",
      "required": ["type", "text"],
      "properties": {
        "type": { "const": "transcript.final" },
        "text": { "type": "string" },
        "words": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "text": { "type": "string" },
              "startMs": { "type": "integer" },
              "endMs": { "type": "integer" },
              "confidence": { "type": "number" }
            }
          }
        }
      }
    }
  }
}
EOF
```

### 6. Create Root README

```bash
cat > README.md << 'EOF'
# OpenWhisper

Windows-first offline dictation desktop app.

## Architecture

- **Electron + Svelte + TypeScript**: UI layer
- **Rust**: Native Windows helper (hotkeys, injection)
- **Python + NeMo**: ASR worker (speech recognition)

## Quick Start

### Prerequisites

- Windows 10/11
- Bun (`curl -fsSL https://bun.sh/install | bash`)
- Rust (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Python 3.10 or 3.11
- uv (`pip install uv`)

### Setup

```bash
# Clone and setup
git clone <repo-url>
cd openwhisper
bun run setup
```

### Development

```bash
# Run Electron app
bun dev

# Run Rust helper
cargo run --manifest-path crates/openwhisper-native/Cargo.toml

# Run Python ASR worker
cd services/asr && uv run python -m openwhisper_asr
```

## Project Structure

```
apps/desktop/          # Electron app
crates/openwhisper-native/  # Rust helper
services/asr/          # Python ASR worker
packages/protocol/     # Shared schemas
.plan/                 # Planning documents
```

## Documentation

See `.plan/` directory for detailed planning documents.

Start with: `.plan/00-INDEX.md`
EOF
```

---

## Defaults

- **Bun** for JavaScript package management (fast, modern)
- **Cargo** for Rust (standard)
- **uv** for Python (fast, modern alternative to pip/poetry)
- Python version is selected by the validated ASR stack in Phase 3
- **All layers build independently** - no cross-dependencies in build
- **Keep build times fast** - avoid heavy dependencies at root
- **Electron and Python dependency trees separate** - no shared packages

---

## Success Criteria

- [ ] `bun install` completes without errors
- [ ] `bun dev` launches Electron window
- [ ] `cargo build` in `crates/openwhisper-native` builds successfully
- [ ] `uv run python -m openwhisper_asr` in `services/asr` starts and responds to health check:
  ```bash
  cd services/asr
  echo '{"type": "health.check"}' | uv run python -m openwhisper_asr
  # Should see: {"type": "health.ok", ...}
  ```
- [ ] Root README explains local dev flow clearly
- [ ] Setup takes <10 minutes on fresh machine

---

## Verification

Test your setup:

```bash
# 1. Install dependencies
bun run setup

# 2. Test Electron
bun dev
# Should see window with "OpenWhisper" title

# 3. Test Rust
cd crates/openwhisper-native
cargo build
echo '{"type": "health.check"}' | cargo run
# Should see: {"type": "health.ok", ...}

# 4. Test Python
cd services/asr
uv sync
echo '{"type": "health.check"}' | uv run python -m openwhisper_asr
# Should see: {"type": "health.ok", ...}
```

---

## Common Issues

### Bun install fails
- Ensure Bun is installed: `bun --version`
- Try: `bun install --force`

### Cargo build fails
- Ensure Rust is installed: `rustc --version`
- Ensure you have Windows SDK installed

### uv sync fails
- Ensure Python 3.10 or 3.11 is installed
- Ensure uv is installed: `uv --version`

---

## Next Phase

After this phase completes, proceed to [Phase 2: Protocol](06-PHASE-02-PROTOCOL.md) to define communication between layers.

---

## Related Documents

- **Index**: [00-INDEX.md](00-INDEX.md) - Navigation
- **Architecture**: [01-SUMMARY.md](01-SUMMARY.md) - System overview
- **Next Phase**: [Phase 2](06-PHASE-02-PROTOCOL.md) - IPC protocol
