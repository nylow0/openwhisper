# Phase 4: Electron/Svelte Product Shell

## Context

This is **Phase 4** of 9 in building OpenWhisper, a Windows-first offline dictation desktop app.

**Purpose**: Build the user-facing interface - the visible parts of the application that users interact with. This phase is **blocked until Phase 3 (ASR Engine) passes** - don't build a beautiful UI for a model that doesn't work.

**Key principle**: The UI should feel like a lightweight system utility, not a heavy application. Think WhisperFlow, not Microsoft Word.

---

## Prerequisites

- [ ] Phase 1 (Repo Setup) complete
- [ ] Phase 2 (Protocol) complete
- [ ] Phase 3 (ASR Engine) **PASSED Go/No-Go decision**
- [ ] Can run Electron app
- [ ] Protocol between Electron and Rust works
- [ ] Read [Error Handling](16-ERROR-HANDLING.md) for UI error patterns

---

## Goal

Build all UI surfaces:
1. Main settings window
2. System tray menu
3. Floating transcription overlay
4. Model loading/onboarding screen
5. Diagnostics/log viewer

**Success means**: Users can configure the app, see transcription in real-time, and understand what's happening.

---

## UI Surfaces

### 1. Main Settings Window

**Purpose**: Primary configuration interface

**Design**:
- Single-window application (not multi-window)
- Opens from tray menu or hotkey
- Tabbed or sidebar navigation
- Modern, clean aesthetic (Tailwind)

**Sections**:

| Section | Settings |
|---------|----------|
| **Audio** | Input device, sample rate (16kHz), VAD toggle |
| **Dictation** | Language (Auto/EN/ES/FR/etc.), punctuation toggle, profanity filter |
| **Performance** | Mode (Fast/Balanced/Accurate), GPU toggle |
| **Hotkey** | Global hotkey configuration with conflict detection |
| **Injection** | Method (Auto/Keystrokes/Clipboard), clipboard threshold |
| **Storage** | Model location, disk usage, clear cache button |
| **Advanced** | Log level, diagnostics export, reset settings |
| **About** | Version, links, license, check for updates |

**Key Behaviors**:
- Changes apply immediately (no "Save" button needed)
- Validation shows inline errors
- Restart required indicators where applicable

### 2. System Tray

**Purpose**: Always-visible access point

**Design**:
- Icon in Windows system tray
- Right-click context menu
- Left-click toggles dictation or opens settings

**Menu Items**:
```
OpenWhisper v0.1.0
─────────────────
Start Dictation (Ctrl+Shift+D)
[ ] Mute Microphone
─────────────────
Settings
View Logs
─────────────────
Quit
```

**States**:
- Idle: Default icon
- Dictating: Recording indicator (red dot)
- Error: Warning indicator
- Loading: Animated spinner

### 3. Floating Overlay

**Purpose**: Show transcription in real-time without taking focus

**Design**:
- Frameless window
- Always on top
- Bottom-center of screen (configurable)
- Semi-transparent background
- No window decorations (no close/minimize buttons)

**Layout**:
```
┌─────────────────────────────────────┐
│ 🎤 Listening...          [—] [×]   │  <- Header with status and minimize/close
├─────────────────────────────────────┤
│                                     │
│  This is the live transcription     │  <- Most recent text (scrolls if long)
│  showing what the user is saying    │
│                                     │
│  Previous text appears here and     │  <- History (last 2-3 lines)
│  scrolls up as new text comes in    │
│                                     │
├─────────────────────────────────────┤
│ ████████░░░░░░░░░░  1.2s   ⚡ Fast │  <- Audio level, duration, mode indicator
└─────────────────────────────────────┘
```

**States**:
- Idle: "Press {hotkey} to start dictating"
- Listening: "Listening..." + audio level indicator
- Processing: "Processing..." + spinner (when chunk is being transcribed)
- Error: Error message with retry button

**Behaviors**:
- Does NOT steal focus (WS_EX_NOACTIVATE on Windows)
- Click-through when dictating (optional setting)
- Auto-hides after 5 seconds of inactivity (configurable)
- Can be disabled entirely in settings

### 4. Onboarding / Model Loading

**Purpose**: First-run experience and model management

**Flows**:

**First Run**:
1. Welcome screen - "Welcome to OpenWhisper"
2. Microphone permission request
3. Model download screen - "Downloading speech model"
   - Progress bar
   - Cancel/Skip button
   - Estimated time
4. Ready screen - "You're all set! Press {hotkey} to start"

**Model Download Screen**:
```
Downloading Speech Model

███████████████████░░░  78%

Downloaded: 940 MB / measured total
Speed: 5.2 MB/s
ETA: 45 seconds

[Cancel]  [Skip for now]

The model is required for dictation and will be
downloaded to:
%LOCALAPPDATA%\OpenWhisper\models
```

**Skip Behavior**:
- Can skip download and use app
- Shows persistent notification: "Model required for dictation"
- Retry download button available

### 5. Diagnostics / Log Viewer

**Purpose**: Help users (and developers) debug issues

**Location**: Settings → Advanced → Diagnostics

**Features**:
- Live log tail (last 100 lines)
- Log level filter (Error, Warn, Info, Debug)
- Export diagnostics package button
- Clear logs button
- System info display (OS, CPU, RAM, GPU)

**Export creates**:
```
%APPDATA%\OpenWhisper\diagnostics\diagnostics-20240115-143022.zip
  ├── logs/
  │   ├── openwhisper.log
  │   ├── openwhisper.log.1
  │   └── ...
  ├── config.json (sanitized)
  ├── system-info.json
  └── manifest.json
```

---

## Electron Responsibilities

### Main Process

```typescript
// apps/desktop/src/main/index.ts

// App lifecycle
app.on('ready', createWindows);
app.on('window-all-closed', handleWindowsClosed);

// Tray management
let tray: Tray;
function createTray() {
  tray = new Tray(iconPath);
  tray.setContextMenu(createTrayMenu());
  tray.on('click', toggleDictation);
}

// Window management
function createWindows() {
  // Settings window (hidden initially)
  settingsWindow = createSettingsWindow();
  
  // Overlay window (hidden initially)  
  overlayWindow = createOverlayWindow();
}

// IPC handlers
ipcMain.handle('dictation:start', startDictation);
ipcMain.handle('dictation:stop', stopDictation);
ipcMain.handle('settings:get', getSettings);
ipcMain.handle('settings:set', setSettings);

// Forward events from Rust
rustClient.on('transcript.partial', (data) => {
  overlayWindow.webContents.send('transcript:partial', data);
});

rustClient.on('transcript.final', (data) => {
  mainWindow.webContents.send('transcript:final', data);
  overlayWindow.webContents.send('transcript:final', data);
});
```

### Renderer Process (Svelte)

**Settings Component**:
```svelte
<!-- apps/desktop/src/renderer/routes/Settings.svelte -->
<script>
  import { settings } from '../stores/settings';
  import AudioSettings from '../components/AudioSettings.svelte';
  import DictationSettings from '../components/DictationSettings.svelte';
  
  let activeTab = 'audio';
  
  async function saveSettings() {
    await window.api.setSettings($settings);
  }
</script>

<div class="settings-container">
  <aside class="sidebar">
    <button on:click={() => activeTab = 'audio'}>Audio</button>
    <button on:click={() => activeTab = 'dictation'}>Dictation</button>
    <!-- ... -->
  </aside>
  
  <main>
    {#if activeTab === 'audio'}
      <AudioSettings bind:settings={$settings.audio} />
    {:else if activeTab === 'dictation'}
      <DictationSettings bind:settings={$settings.dictation} />
    {/if}
  </main>
</div>
```

**Overlay Component**:
```svelte
<!-- apps/desktop/src/renderer/components/Overlay.svelte -->
<script>
  import { onMount } from 'svelte';
  
  let status = 'idle'; // 'idle' | 'listening' | 'processing' | 'error'
  let transcript = '';
  let history = [];
  
  onMount(() => {
    window.api.onTranscriptPartial((text) => {
      transcript = text;
      status = 'listening';
    });
    
    window.api.onTranscriptFinal((text) => {
      history = [...history, text];
      transcript = '';
    });
  });
</script>

<div class="overlay" class:listening={status === 'listening'}>
  <div class="header">
    {#if status === 'idle'}
      <span>Press {hotkey} to start</span>
    {:else if status === 'listening'}
      <span class="recording">🎤 Listening...</span>
    {:else if status === 'processing'}
      <span>Processing...</span>
    {/if}
  </div>
  
  <div class="content">
    {#each history as line}
      <p class="history">{line}</p>
    {/each}
    <p class="current">{transcript}</p>
  </div>
  
  <div class="footer">
    <AudioLevelIndicator />
    <span class="mode">⚡ Fast</span>
  </div>
</div>
```

---

## Settings Persistence

**Storage**: `%APPDATA%\OpenWhisper\config.json`

**Schema**:
```typescript
interface UserSettings {
  version: '1.0.0';
  audio: {
    deviceId: string;
    sampleRate: 16000 | 22050 | 44100 | 48000;
    chunkDurationMs: 500 | 2000 | 4000;
    vadEnabled: boolean;
  };
  dictation: {
    hotkey: string;
    language: 'auto' | string;
    punctuation: boolean;
    injectionMethod: 'auto' | 'keystrokes' | 'clipboard';
  };
  ui: {
    overlayEnabled: boolean;
    overlayPosition: 'bottom-center' | 'top-center' | 'custom';
    minimizeToTray: boolean;
    theme: 'light' | 'dark' | 'system';
  };
  system: {
    autoStart: boolean;
    logLevel: 'error' | 'warn' | 'info' | 'debug' | 'trace';
    gpuEnabled: boolean;
  };
}
```

**Migration**: See [Assumptions](15-ASSUMPTIONS.md) for settings migration strategy.

---

## Defaults

- **First run**: Show onboarding
- **Hotkey**: Ctrl+Shift+D
- **Mode**: Balanced (2s chunks)
- **Overlay**: Enabled, bottom-center
- **Tray**: Minimize on close (don't quit)
- **Auto-start**: Disabled
- **Theme**: Follow system

---

## Success Criteria

- [ ] App opens to real settings UI (not placeholder/landing page)
- [ ] System tray icon appears and menu works
- [ ] Tray menu can start/stop dictation
- [ ] Settings window opens from tray
- [ ] All settings sections present and functional
- [ ] Settings changes persist after restart
- [ ] Overlay appears without stealing focus from other apps
- [ ] Overlay shows mock transcript events (live updates)
- [ ] Overlay can be disabled in settings
- [ ] Onboarding shows on first run
- [ ] Model download progress is visible
- [ ] Diagnostics/logs accessible in settings
- [ ] No TypeScript `any` types in codebase

---

## Testing Checklist

- [ ] Open settings, change hotkey, verify it persists
- [ ] Enable/disable overlay, verify behavior
- [ ] Start dictation, verify overlay appears
- [ ] Verify overlay doesn't steal focus (click on another app while overlay is visible)
- [ ] Right-click tray, verify menu appears
- [ ] Quit from tray, verify app exits
- [ ] Restart app, verify settings restored
- [ ] Check all settings sections render without errors
- [ ] Verify responsive design (resize window)

---

## Related Documents

- **Error Handling**: [Document 16](16-ERROR-HANDLING.md) - Error UI patterns
- **Assumptions**: [Document 15](15-ASSUMPTIONS.md) - Settings defaults and migration
- **Previous Phase**: [Phase 3](07-PHASE-03-ASR-ENGINE.md) - Must pass before this phase
- **Next Phase**: [Phase 5](09-PHASE-05-RUST-HELPER.md) - Native Windows integration
