# Phase 7: End-to-End Dictation Prototype

## Context

This is **Phase 7** of 9 in building OpenWhisper, a Windows-first offline dictation desktop app.

**Purpose**: Wire everything together into the complete product experience. This is where we prove the entire system works - from hotkey press to text appearing in another application.

**This is the moment of truth** - if this phase works, we have a product. If not, we debug and fix.

---

## Prerequisites

- [ ] Phase 1-6 complete
- [ ] Electron UI works (settings, overlay, tray)
- [ ] Rust helper runs (hotkeys, injection)
- [ ] Python worker runs (ASR)
- [ ] Audio capture produces valid audio
- [ ] ASR model transcribes audio correctly
- [ ] Protocol passes messages correctly
- [ ] Read [Error Handling](16-ERROR-HANDLING.md) for error scenarios

---

## Goal

Prove the complete dictation loop:

```
User presses hotkey
    ↓
Rust detects hotkey, starts dictation
    ↓
Rust starts audio capture
    ↓
Audio flows: Mic -> Rust WASAPI -> Python ASR
    ↓
Transcripts flow: Python → Rust
    ↓
Rust injects text into focused app + sends to Electron
    ↓
Electron updates overlay
    ↓
User sees text appear in real-time
    ↓
User presses hotkey again to stop
```

**Success means**: A user can open Notepad, press a hotkey, speak, and see their words appear in Notepad.

---

## Integration Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│ User Actions                                                         │
│  - Presses hotkey (global)                                          │
│  - Speaks into microphone                                           │
│  - Sees text appear                                                 │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│ Electron Layer (UI)                                                  │
│  - Displays overlay with live transcription                         │
│  - Shows status in tray                                             │
│  - Provides settings for configuration                              │
└─────────────────────────────────────────────────────────────────────┘
                                │ IPC (local socket)
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│ Rust Layer (Native Helper)                                           │
│  - Registers global hotkey                                          │
│  - Detects hotkey press → starts/stops dictation                    │
│  - Spawns/supervises Python worker                                  │
│  - Receives transcripts from Python                                 │
│  - Injects text into focused window (SendInput)                     │
│  - Falls back to clipboard if needed                                │
│  - Sends status/transcripts to Electron                             │
└─────────────────────────────────────────────────────────────────────┘
                                │ NDJSON over stdio
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│ Python Layer (ASR Worker)                                            │
│  - Receives audio chunks from Rust                                  │
│  - (Optional) VAD to detect speech                                  │
│  - Streams audio chunks to ASR model                                │
│  - Returns partial and final transcripts                            │
│  - Sends transcripts to Rust via stdout                             │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Flow

### 1. Dictation Start Flow

```rust
// crates/openwhisper-native/src/dictation.rs

impl DictationManager {
    pub fn start_dictation(&mut self) -> Result<()> {
        // 1. Check if already dictating
        if self.is_dictating {
            return Ok(());
        }
        
        // 2. Check Python worker health
        if !self.worker.is_healthy() {
            self.worker.restart()?;
        }
        
        // 3. Send start command to Python
        self.python.send(ToPython::DictationStart {
            language: self.settings.language.clone(),
            chunk_duration_ms: self.settings.chunk_duration_ms,
        })?;
        
        // 4. Update state
        self.is_dictating = true;
        self.partial_text.clear();
        
        // 5. Notify Electron
        self.electron.send(Event::DictationStarted {
            timestamp: now(),
        })?;
        
        log::info!("Dictation started");
        Ok(())
    }
}
```

### 2. Audio Processing Flow

```python
# services/asr/src/openwhisper_asr/__main__.py

class ASRWorker:
    def handle_dictation_start(self, settings: dict):
        """Start dictation."""
        # Rust owns audio capture. Python prepares to receive audio.chunk messages.
        self.is_dictating = True
        
    def handle_audio_chunk(self, chunk: np.ndarray):
        """Process one audio chunk received from Rust."""
        if not self.is_dictating:
            return

        start_time = time.perf_counter()
        result = self.engine.transcribe_chunk(chunk)
        latency_ms = int((time.perf_counter() - start_time) * 1000)

        if result.is_partial:
            self.send({
                "type": "transcript.partial",
                "text": result.text,
                "is_final": False,
                "processing_latency_ms": latency_ms
            })
        else:
            self.send({
                "type": "transcript.final",
                "text": result.text,
                "words": [w.to_dict() for w in result.words],
                "processing_latency_ms": latency_ms
            })
```

### 3. Text Injection Flow

```rust
// crates/openwhisper-native/src/dictation.rs

impl DictationManager {
    pub fn handle_transcript(&mut self, transcript: Transcript) -> Result<()> {
        match transcript {
            Transcript::Partial { text } => {
                // Inject partial text (optimistic)
                self.injection.update(&text)?;
                
                // Send to Electron for overlay
                self.electron.send(Event::TranscriptPartial { text })?;
            }
            Transcript::Final { text, words } => {
                // Commit final text
                self.injection.commit(&text)?;
                
                // Send to Electron
                self.electron.send(Event::TranscriptFinal { 
                    text, 
                    words 
                })?;
                
                // Clear partial state
                self.partial_text.clear();
            }
        }
        Ok(())
    }
}

// crates/openwhisper-native/src/injection.rs

impl TextInjection {
    pub fn update(&mut self, new_text: &str) -> Result<()> {
        // Find common prefix with previous text
        let common = common_prefix(&self.last_text, new_text);
        let to_delete = self.last_text.len() - common.len();
        let to_type = &new_text[common.len()..];
        
        // Send backspaces
        for _ in 0..to_delete {
            send_key(VK_BACK)?;
        }
        
        // Type new text
        if to_type.len() > 10 {
            // Use clipboard for long updates
            clipboard::set_and_paste(to_type)?;
        } else {
            // Type character by character
            for ch in to_type.chars() {
                send_unicode(ch)?;
            }
        }
        
        self.last_text = new_text.to_string();
        Ok(())
    }
    
    pub fn commit(&mut self, final_text: &str) -> Result<()> {
        // Ensure final text is correct
        self.update(final_text)?;
        self.last_text.clear();
        Ok(())
    }
}
```

### 4. UI Update Flow

```svelte
<!-- apps/desktop/src/renderer/components/Overlay.svelte -->

<script>
  import { onMount } from 'svelte';
  
  let status = 'idle';
  let currentText = '';
  let history = [];
  
  onMount(() => {
    // Listen for dictation start
    window.api.on('dictation:started', () => {
      status = 'listening';
      currentText = '';
    });
    
    // Listen for partial transcripts
    window.api.on('transcript:partial', (text) => {
      currentText = text;
      status = 'listening';
    });
    
    // Listen for final transcripts
    window.api.on('transcript:final', (text) => {
      history = [...history, text];
      currentText = '';
    });
    
    // Listen for dictation stop
    window.api.on('dictation:stopped', () => {
      status = 'idle';
    });
    
    // Listen for errors
    window.api.on('dictation:error', (error) => {
      status = 'error';
      console.error('Dictation error:', error);
    });
  });
</script>

<div class="overlay" class:active={status === 'listening'}>
  {#if status === 'idle'}
    <p>Press {hotkey} to start dictating</p>
  {:else if status === 'listening'}
    <div class="history">
      {#each history as line}
        <p>{line}</p>
      {/each}
    </div>
    <p class="current">{currentText}</p>
    <AudioLevelIndicator />
  {:else if status === 'error'}
    <p class="error">Error occurred. Check settings.</p>
  {/if}
</div>
```

---

## Error Handling in the Loop

See [Error Handling](16-ERROR-HANDLING.md) for complete details.

### Critical Errors (Stop Dictation)

| Error | Action | User Sees |
|-------|--------|-----------|
| Python worker crashes | Restart worker, pause dictation | "Dictation paused, retrying..." |
| Audio device disconnects | Pause, wait for reconnect | "Microphone disconnected" |
| Model inference fails | Log error, skip chunk | No visible effect (skip) |

### Non-Critical Errors (Continue)

| Error | Action | User Sees |
|-------|--------|-----------|
| Injection blocked | Fallback to clipboard | No visible effect |
| Target window elevated | Use clipboard + warning | "Clipboard mode active" |
| Chunk timeout | Skip chunk | Brief pause in transcription |

---

## Required Behavior

### Functional Requirements

- [ ] Hotkey starts/stops dictation globally (any window focused)
- [ ] Overlay reflects recording state (listening/processing/idle)
- [ ] Partial text updates naturally (smooth typing effect)
- [ ] Final text is committed cleanly
- [ ] Errors do not crash the app
- [ ] Model loading state is visible in UI

### Performance Requirements

- [ ] Hotkey response < 100ms
- [ ] First partial result within 1 second of speech
- [ ] Overlay updates at 10+ FPS during dictation
- [ ] No UI freezing during dictation

### Reliability Requirements

- [ ] Can dictate for 5+ minutes without issues
- [ ] Can start/stop dictation 20+ times without issues
- [ ] Recovers from Python worker crash
- [ ] Handles audio device disconnect gracefully

---

## Success Criteria

- [ ] Open Notepad (or any text editor)
- [ ] Press hotkey → Dictation starts (overlay appears)
- [ ] Speak → See text appear in real-time in overlay
- [ ] See text injected into Notepad
- [ ] Press hotkey again → Dictation stops
- [ ] App remains running in tray
- [ ] Can start dictation again
- [ ] No crashes during normal use
- [ ] Errors are handled gracefully

---

## Testing Protocol

### Test 1: Happy Path

1. Open Notepad
2. Press hotkey
3. Speak: "Hello world, this is a test"
4. Press hotkey to stop

**Expected**:
- Overlay appeared
- Text appeared in overlay as you spoke
- Same text appeared in Notepad
- Overlay closed or returned to idle

### Test 2: Long Dictation

1. Dictate continuously for 5 minutes
2. Speak naturally with pauses

**Expected**:
- No crashes
- Memory stable
- Quality consistent
- All text captured

### Test 3: Rapid Toggle

1. Press hotkey 10 times rapidly

**Expected**:
- No crashes
- State remains consistent
- No resource leaks

### Test 4: Error Recovery

1. Start dictation
2. Unplug microphone
3. Plug microphone back in
4. Continue dictation

**Expected**:
- Error shown when unplugged
- Recovery when plugged back in
- Can continue dictation

### Test 5: App Switching

1. Start dictation in Notepad
2. Without stopping, click on Word
3. Continue speaking

**Expected**:
- Text now appears in Word
- Dictation continues seamlessly

---

## Debugging Tips

### Logs to Check

```
%APPDATA%\OpenWhisper\logs\openwhisper.log
```

Look for:
- `Dictation started`
- `Audio chunk received`
- `Transcript received`
- `Text injected`
- Any ERROR or WARN lines

### Enable Debug Logging

Settings → Advanced → Log Level → DEBUG

### Test Components Individually

```bash
# Test audio capture in Rust, then send recorded chunks to Python
cd crates/openwhisper-native
cargo run -- --audio-smoke-test

# Test ASR only (with test WAV)
python scripts/benchmark.py --input test.wav

# Test injection only
cd crates/openwhisper-native
cargo run -- inject "test text"
```

---

## Common Issues

### No text appears

1. Check if audio is being captured (DEBUG logs)
2. Check if ASR is producing transcripts
3. Check if messages are flowing through protocol
4. Check if injection is working (try clipboard mode)

### Text appears in overlay but not in app

1. Check if target app blocks injection (try clipboard mode)
2. Check if target app is elevated (run OpenWhisper as admin)
3. Check injection logs for errors

### High latency

1. Check chunk size (larger = slower)
2. Check if using GPU (CPU is slower)
3. Check [Performance Budget](17-PERFORMANCE-BUDGET.md)

---

## Related Documents

- **Error Handling**: [Document 16](16-ERROR-HANDLING.md) - Comprehensive error scenarios
- **Performance Budget**: [Document 17](17-PERFORMANCE-BUDGET.md) - Latency targets
- **Rust Helper**: [Phase 5](09-PHASE-05-RUST-HELPER.md) - Injection details
- **Audio Pipeline**: [Phase 6](10-PHASE-06-AUDIO.md) - Audio capture details
- **Previous Phase**: [Phase 6](10-PHASE-06-AUDIO.md) - Audio working
- **Next Phase**: [Phase 8](12-PHASE-08-PERFORMANCE.md) - Optimization
