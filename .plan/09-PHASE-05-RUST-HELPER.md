# Phase 5: Rust Native Helper v1

## Context

This is **Phase 5** of 9 in building OpenWhisper, a Windows-first offline dictation desktop app with three-tier architecture (Electron UI, Rust native helper, Python ASR worker).

**Purpose of this phase**: Implement the Rust native helper that handles Windows-specific functionality that Electron and Python cannot do well: global hotkeys, text injection, clipboard operations, and Python worker supervision.

**Current repo note**: Some helper foundation already exists: named-pipe IPC, Python worker spawn, health checks, and mock transcript forwarding. This phase should focus on the missing native Windows pieces: hotkeys, injection, clipboard, active-window/elevation checks, and robust supervision.

**Why Rust?**:
- Direct Windows API access without FFI complexity
- Memory safety for system-level operations
- Small binary size
- Easier distribution than C++

---

## Prerequisites

- [ ] Phase 1 (Repo Setup) complete
- [ ] Phase 2 (Protocol) complete - NDJSON messaging works
- [ ] Phase 3 (ASR Engine) complete and PASSED Go/No-Go
- [ ] Phase 4 (Product Shell) complete - UI exists to show status
- [ ] Rust toolchain installed (`cargo`, `rustc`)
- [ ] Read [Error Handling](16-ERROR-HANDLING.md) for error scenarios

---

## Goal

Move fragile Windows-specific behavior out of Electron and Python into a dedicated Rust native helper that is:
- Reliable (handles errors gracefully)
- Efficient (minimal resource usage)
- Secure (proper Windows API usage)
- Observable (reports status to Electron)

---

## Core Responsibilities

### 1. Global Hotkey Registration

Register system-wide hotkey that works even when app is not focused.

**Requirements**:
- Register configurable hotkey (default: Ctrl+Shift+D)
- Work globally (any window focused)
- Handle hotkey conflicts gracefully
- Support modifiers: Ctrl, Shift, Alt, Win

**Implementation**:
```rust
// crates/openwhisper-native/src/hotkeys.rs
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_CONTROL, MOD_SHIFT};

pub struct HotkeyManager {
    id: i32,
    callback: Box<dyn Fn()>,
}

impl HotkeyManager {
    pub fn register(modifiers: u32, key: u32, callback: impl Fn() + 'static) -> Result<Self> {
        // Register with Windows
        unsafe {
            RegisterHotKey(None, id, modifiers, key)
                .ok()
                .map_err(|e| HotkeyError::RegistrationFailed(e))?;
        }
        // Set up message loop to receive hotkey events
    }
}
```

**Error Handling**:
- Hotkey conflict: Report to Electron, suggest alternatives
- Registration failure: Retry with different key, notify user

### 2. Text Injection (SendInput)

Inject text into the currently focused window using Windows `SendInput` API.

**Requirements**:
- Inject text as keystrokes
- Support Unicode (international characters)
- Handle special keys reasonably
- Work with most applications

**Implementation**:
```rust
// crates/openwhisper-native/src/injection.rs
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_UNICODE};

pub fn inject_text(text: &str) -> Result<(), InjectionError> {
    // Convert text to INPUT structures
    let inputs: Vec<INPUT> = text.chars()
        .map(|c| create_unicode_input(c))
        .collect();
    
    // Send to active window
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn create_unicode_input(c: char) -> INPUT {
    // Create INPUT for Unicode character
}
```

**Limitations**:
- Cannot inject into elevated (admin) apps if not elevated
- Some apps block synthetic input (games, secure fields)
- Complex formatting (rich text) not supported

### 3. Clipboard Fallback

When injection fails or is inappropriate, use clipboard.

**Requirements**:
- Save current clipboard content
- Copy text to clipboard
- Paste into target (Ctrl+V)
- Restore original clipboard content
- Handle large clipboard data

**Implementation**:
```rust
// crates/openwhisper-native/src/clipboard.rs
use windows::Win32::System::DataExchange::{OpenClipboard, GetClipboardData, SetClipboardData};

pub struct ClipboardManager;

impl ClipboardManager {
    pub fn save_clipboard() -> ClipboardData {
        // Save current clipboard content
    }
    
    pub fn set_text(text: &str) {
        // Copy text to clipboard
    }
    
    pub fn paste() {
        // Simulate Ctrl+V
        send_key_combination(VK_CONTROL, VK_V);
    }
    
    pub fn restore(data: ClipboardData) {
        // Restore original clipboard
    }
}
```

**Usage Flow**:
```rust
let saved = clipboard.save_clipboard();
clipboard.set_text(&text);
clipboard.paste();
// Small delay
clipboard.restore(saved);
```

### 4. Active Window Detection

Detect which window is currently focused and whether we can inject into it.

**Requirements**:
- Get foreground window handle
- Get process name and window title
- Check if elevated (admin)
- Check if injection likely to work

**Implementation**:
```rust
// crates/openwhisper-native/src/window.rs
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

pub struct WindowInfo {
    pub handle: HWND,
    pub title: String,
    pub process_name: String,
    pub is_elevated: bool,
}

pub fn get_active_window() -> Option<WindowInfo> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == 0 {
            return None;
        }
        // Get process ID
        // Open process, get executable name
        // Check elevation via token
    }
}
```

### 5. Python Worker Supervision

Launch, monitor, and restart the Python ASR worker process.

**Requirements**:
- Spawn Python process with ASR worker
- Monitor health via periodic checks
- Auto-restart on crash (max 3 attempts)
- Forward stdout/stderr to logs
- Graceful shutdown on app exit

**Implementation**:
```rust
// crates/openwhisper-native/src/python_worker.rs
use std::process::{Command, Child, Stdio};
use std::io::{BufRead, BufReader, Write};

pub struct PythonWorker {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    restart_count: u32,
}

impl PythonWorker {
    pub fn spawn(python_path: &str, script_path: &str) -> Result<Self> {
        let mut child = Command::new(python_path)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        // Set up stdout reader for NDJSON
        // Set up stderr reader for logging
    }
    
    pub fn send(&mut self, message: &str) -> Result<()> {
        writeln!(self.stdin, "{}", message)?;
        self.stdin.flush()?;
    }
    
    pub fn health_check(&mut self) -> Result<bool> {
        self.send(r#"{"type": "health.check"}"#)?;
        // Wait for response with timeout
    }
    
    pub fn check_alive(&mut self) -> bool {
        // Check if process still running
    }
    
    pub fn restart(&mut self) -> Result<()> {
        if self.restart_count >= 3 {
            return Err(WorkerError::MaxRestartsReached);
        }
        self.kill()?;
        *self = Self::spawn(...)?;
        self.restart_count += 1;
    }
}
```

### 6. Partial Result Handling

Handle streaming partial results - update injected text as transcription improves.

**Algorithm**:
```rust
pub struct PartialResultHandler {
    last_injected_text: String,
}

impl PartialResultHandler {
    pub fn update(&mut self, new_text: &str) {
        // Find common prefix
        let common_len = self.last_injected_text.chars()
            .zip(new_text.chars())
            .take_while(|(a, b)| a == b)
            .count();
        
        // Backspace changed suffix
        let to_delete = self.last_injected_text.chars().count() - common_len;
        send_backspaces(to_delete);
        
        // Type new suffix
        let new_suffix: String = new_text.chars().skip(common_len).collect();
        inject_text(&new_suffix);
        
        self.last_injected_text = new_text.to_string();
    }
    
    pub fn commit(&mut self, final_text: &str) {
        // Ensure final text is correct
        self.update(final_text);
        self.last_injected_text.clear();
    }
}
```

**Strategy**:
- For short updates (<10 chars): Use keystrokes
- For long updates: Use clipboard (delete all, paste new)

---

## Injection Strategy Defaults

When to use each injection method:

| Scenario | Method | Rationale |
|----------|--------|-----------|
| Final text, any length | Clipboard | Reliable, preserves formatting |
| Partial update, <10 chars | Keystrokes | Minimizes clipboard flicker |
| Partial update, ≥10 chars | Clipboard | Faster than typing |
| Target blocks injection | Clipboard fallback | Seamless recovery |
| Target is elevated | Clipboard + warning | Cannot inject, must paste |

---

## Error Handling

Per [Error Handling](16-ERROR-HANDLING.md):

### Fatal Errors (Report to Electron, Exit)

- Rust helper binary corrupt
- Cannot create IPC channel
- Python runtime missing

### Recoverable Errors (Auto-Retry)

- Python worker crash: Restart (max 3 times)
- Hotkey conflict: Try alternative, notify user
- Injection blocked: Fallback to clipboard

### User Notifications (Via Electron)

- Target window elevated → "Some apps require clipboard mode"
- Clipboard large → "Large text copied to clipboard"
- Worker restarted → "Dictation service recovered"

---

## Default Configuration

```rust
pub const DEFAULTS = Config {
    hotkey: Hotkey {
        modifiers: MOD_CONTROL | MOD_SHIFT,
        key: VK_D,
    },
    injection: InjectionConfig {
        method: InjectionMethod::Auto,
        clipboard_threshold: 10,  // chars
        restore_clipboard: true,
        clipboard_restore_delay_ms: 500,
    },
    worker: WorkerConfig {
        max_restarts: 3,
        health_check_interval_ms: 5000,
        startup_timeout_ms: 30000,
    },
};
```

---

## Success Criteria

- [ ] Hotkey toggles dictation while another app is focused
- [ ] Text can be injected into Notepad
- [ ] Text can be injected into Chrome address bar
- [ ] Text can be injected into VS Code
- [ ] Clipboard fallback works when injection blocked
- [ ] Clipboard content is saved and restored
- [ ] Python worker auto-restarts after crash (max 3 times)
- [ ] Electron receives native helper status updates
- [ ] Elevation detection works (reports limitation)
- [ ] Partial result updates work smoothly

---

## Testing Approach

**Unit Tests**:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_common_prefix() {
        let old = "Hello world";
        let new = "Hello there";
        assert_eq!(common_prefix(old, new), "Hello ");
    }
}
```

**Integration Tests**:
- Start helper, verify health check
- Send hotkey, verify event received
- Inject text into test window
- Kill worker, verify restart

**Manual Tests**:
- Test in 10+ real applications
- Test with various text (short, long, Unicode)
- Test edge cases (empty text, special chars)

---

## Related Documents

- **Error Handling**: [Document 16](16-ERROR-HANDLING.md) - Comprehensive error handling strategy
- **Protocol**: [Phase 2](06-PHASE-02-PROTOCOL.md) - IPC message format
- **ASR Engine**: [Phase 3](07-PHASE-03-ASR-ENGINE.md) - Worker being supervised
- **Audio Pipeline**: [Phase 6](10-PHASE-06-AUDIO.md) - Future WASAPI integration
- **Previous Phase**: [Phase 4](08-PHASE-04-PRODUCT-SHELL.md) - UI to display status
- **Next Phase**: [Phase 6](10-PHASE-06-AUDIO.md) - Audio capture

---

## Implementation Notes

1. **Use windows-rs crate**: Microsoft's official Rust bindings for Windows API
2. **Handle Unicode properly**: Windows uses UTF-16, convert carefully
3. **Thread safety**: Windows APIs may have thread affinity requirements
4. **Error context**: Always include Windows error codes in error messages
5. **Logging**: Log all Windows API calls in DEBUG mode for diagnostics
