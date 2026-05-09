# Product Experience

## Context

This document defines the user experience and interaction model for OpenWhisper - a Windows-first offline dictation desktop application.

**Core Principle**: OpenWhisper should feel like a **system utility**, not a traditional application. Think of it like Caps Lock or the volume control - always available, instantly responsive, nearly invisible until needed.

---

## App Identity

### What OpenWhisper Is

- **A dictation utility**: Speak, text appears
- **Always available**: Lives in system tray, responds to global hotkey
- **Offline and private**: No cloud, no data leaves your computer
- **Lightweight**: Minimal resource usage, minimal UI footprint

### What OpenWhisper Is NOT

- **A word processor**: We don't edit text, we insert it
- **A voice assistant**: No "Hey OpenWhisper" wake word, no Q&A
- **A transcription service**: Not for long-form audio files (focus is real-time)
- **Cloud-dependent**: Works entirely offline

### Similar Products

| Product | Similarity | Difference |
|---------|-----------|------------|
| WhisperFlow | Very similar | We aim for offline operation |
| Windows Speech Recognition | Similar goal | We use modern AI models |
| macOS Dictation | Similar UX | We're for Windows |
| Otter.ai | Dictation | We're offline and real-time |
| Dragon NaturallySpeaking | Professional dictation | We're lighter and modern |

---

## Primary User Flow

The entire product is built around this single interaction:

```
┌──────────────────────────────────────────────────────────────┐
│  1. USER PRESSES GLOBAL HOTKEY                                │
│     (e.g., Ctrl+Shift+D - works in ANY application)          │
└──────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────┐
│  2. SMALL POPUP APPEARS                                       │
│     - Frameless, always-on-top                                │
│     - Positioned at bottom-center of screen                   │
│     - Does NOT steal focus from current window                │
│     - Shows "Listening..." with audio level indicator         │
└──────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────┐
│  3. USER SPEAKS                                               │
│     - Live transcription appears in popup as they speak       │
│     - Partial results update continuously                     │
│     - User sees immediate feedback                            │
└──────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────┐
│  4. TEXT IS INSERTED INTO FOCUSED APP                         │
│     - Final text appears in the application user is using     │
│     - Could be Notepad, Word, browser, code editor, etc.      │
│     - Injection happens automatically                         │
└──────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────┐
│  5. POPUP CLOSES OR RETURNS TO IDLE                           │
│     - User presses hotkey again to stop                       │
│     - Or stops speaking and VAD detects silence               │
│     - App returns to tray, waiting for next activation        │
└──────────────────────────────────────────────────────────────┘
```

**Total interaction time**: 5-30 seconds for typical dictation
**Learning curve**: Near zero (just press hotkey and speak)

---

## Detailed Interaction Model

### Starting Dictation

**Methods** (all equivalent):

1. **Global Hotkey** (primary)
   - User presses configured hotkey from anywhere
   - Works even when OpenWhisper is not visible
   - Default: Ctrl+Shift+D

2. **Tray Menu**
   - Right-click tray icon
   - Click "Start Dictation"

3. **Settings Window**
   - Open settings
   - Click "Start Dictation" button

**Immediate Feedback**:
- Overlay appears within 100ms
- Audio level indicator shows microphone is working
- Status text shows "Listening..."

### During Dictation

**Visual Feedback**:
- Overlay shows live transcription
- Partial results appear as user speaks
- Final results committed periodically
- Audio level indicator shows voice activity

**What Happens to the Text**:
1. ASR transcribes speech in real-time
2. Partial results displayed in overlay only
3. When user pauses, final result committed:
   - Injected into focused application
   - Added to overlay history
   - Previous partials cleared

**User Can**:
- Continue speaking (more text added)
- Pause (current chunk finalized)
- Press hotkey (stops dictation)

### Stopping Dictation

**Automatic** (optional, configurable):
- VAD detects silence for 2+ seconds
- Final text committed
- Dictation stops

**Manual** (primary):
- User presses hotkey again
- Any in-progress text is finalized
- Overlay closes or shows idle state

### After Dictation

- App returns to system tray
- No window visible
- Ready for next activation
- Settings persist

---

## Design Goals

### 1. Invisible Until Needed

**Principle**: The app should not demand attention when not in use.

**Implementation**:
- Single tray icon when idle (no window)
- No startup window (starts minimized to tray)
- No notifications except errors
- No "check out our features" popups

**Anti-patterns to avoid**:
- ❌ Splash screen on startup
- ❌ "What's new" dialogs
- ❌ Promotional notifications
- ❌ Window visible on startup

### 2. Instant Availability

**Principle**: When user wants to dictate, there should be no delay.

**Implementation**:
- Model pre-loaded into memory
- Audio capture ready
- Hotkey always registered
- <100ms from hotkey to listening

**Metrics**:
- Hotkey response: <100ms
- First partial result: <1 second
- Model loading (if unloaded): <5 seconds

### 3. Transparent Operation

**Principle**: User should not think about the app, just speak and see text.

**Implementation**:
- No "click here to continue"
- No manual model selection per session
- Automatic quality/speed tradeoffs
- Seamless fallbacks (clipboard when injection blocked)

**What user thinks**: "I'll dictate this email"
**Not**: "I'll open OpenWhisper, wait for it to load, select the model, check settings, then dictate"

### 4. Graceful Degradation

**Principle**: When things don't work perfectly, the experience should still be good.

**Scenarios**:

| Problem | Graceful Response |
|---------|-------------------|
| App blocks injection | Automatically use clipboard |
| GPU out of memory | Fall back to CPU (slower but works) |
| Microphone disconnected | Show error, pause, auto-resume when reconnected |
| Model not downloaded | Show progress, allow skip and retry later |
| Target app elevated | Use clipboard + subtle warning |

**Anti-patterns to avoid**:
- ❌ Crash when injection fails
- ❌ Require manual restart after errors
- ❌ Show technical error codes to users

---

## UI Surfaces

### 1. System Tray Icon

**Always visible** (when app is running)

**Appearance**:
- Idle: Simple microphone icon
- Dictating: Recording indicator (red dot or animation)
- Error: Warning indicator

**Right-click Menu**:
```
OpenWhisper v0.1.0
─────────────────
Start Dictation (Ctrl+Shift+D)
[✓] Show Overlay
─────────────────
Settings...
View Logs
─────────────────
Quit
```

**Left-click Behavior**:
- If not dictating: Start dictation
- If dictating: Stop dictation

### 2. Floating Overlay

**Appears only during dictation**

**Design**:
```
┌─────────────────────────────────────────────────────┐
│ 🎤 Listening...                         [—] [×]    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  This is live transcription that appears            │
│  as the user speaks into the microphone             │
│                                                     │
│  Previous finalized text appears here              │
│  and stays visible for context                      │
│                                                     │
├─────────────────────────────────────────────────────┤
│ ▓▓▓▓▓▓▓▓░░░░░░  1.5s recorded    ⚡ Fast Mode      │
└─────────────────────────────────────────────────────┘
```

**Key Behaviors**:
- **Frameless**: No window decorations
- **Always-on-top**: Above all other windows
- **No focus steal**: Clicking overlay doesn't take focus from work
- **Position**: Bottom-center of screen (configurable)
- **Auto-hide**: Fades after 5 seconds of inactivity (optional)

**States**:
- Idle: "Press {hotkey} to start dictating"
- Listening: "Listening..." + audio level + live text
- Processing: "Processing..." (between chunks)
- Error: Error message + retry button

### 3. Settings Window

**Opens from tray menu or during onboarding**

**Sections**:
1. **Audio**: Device, sample rate, VAD toggle
2. **Dictation**: Language, punctuation, hotkey
3. **Performance**: Speed/accuracy mode, GPU toggle
4. **Injection**: Method preferences, clipboard settings
5. **Storage**: Model management, cache clearing
6. **Advanced**: Log level, diagnostics, reset
7. **About**: Version, links, license

**Design Principles**:
- Settings apply immediately (no "Save" button)
- Validation inline (invalid hotkey shows error immediately)
- Explanations for technical options
- Restart required indicators where applicable

### 4. Onboarding / First Run

**Shown on first launch**

**Flow**:
1. **Welcome**: "Welcome to OpenWhisper"
2. **Permissions**: Request microphone access
3. **Model Download**: "Downloading speech model..."
   - Progress bar
   - Cancel option
   - Skip option (download later)
4. **Ready**: "You're all set! Press {hotkey} to start"

**Skip Behavior**:
- Can skip model download
- App shows persistent notification: "Model required for dictation"
- Retry download button always available

---

## User Personas

### Primary: Productivity-Focused Professional

**Who**: Writers, developers, knowledge workers
**Needs**: Fast, accurate dictation into any app
**Technical level**: Moderate
**Pain points**: Typing fatigue, wants to work hands-free
**Success metric**: Can dictate emails, code comments, documentation

### Secondary: Accessibility User

**Who**: People with RSI, carpal tunnel, mobility limitations
**Needs**: Reliable dictation, minimal physical interaction
**Technical level**: Varies
**Pain points**: Existing solutions are cloud-based or expensive
**Success metric**: Can work comfortably without typing

### Tertiary: Casual User

**Who**: Occasional dictation for messages, notes
**Needs**: Simple, works when needed
**Technical level**: Low
**Pain points**: Complicated setup, requires internet
**Success metric**: Can dictate occasionally without thinking about it

---

## Success Metrics (UX)

### Quantitative

- **Time to first dictation**: <2 minutes from install
- **Hotkey response time**: <100ms
- **First partial result**: <1 second
- **Session duration**: 30+ minutes without crashes
- **Error recovery**: <5 seconds

### Qualitative

- User can start dictating without reading documentation
- User trusts the app (privacy, reliability)
- User recommends to others
- User uses daily

---

## Anti-Patterns (What NOT to Do)

### UX Anti-Patterns

- ❌ Modal dialogs that block interaction
- ❌ Forced registration or account creation
- ❌ Mandatory cloud features
- ❌ Frequent update notifications
- ❌ Dark patterns (hidden settings, forced opt-in)

### Technical Anti-Patterns

- ❌ Requiring admin for normal use
- ❌ Installing to Program Files (use per-user)
- ❌ Modifying system-wide settings
- ❌ Running background processes when not needed
- ❌ Uploading data without consent

### Performance Anti-Patterns

- ❌ Slow startup (>5 seconds)
- ❌ High memory usage when idle
- ❌ CPU usage when not dictating
- ❌ Blocking the UI during dictation

---

## Accessibility Considerations

### Visual

- High contrast mode support
- Scalable UI (respects Windows text scaling)
- Color-blind friendly indicators

### Motor

- Global hotkey (no mouse needed)
- Configurable hotkey (for users with limited dexterity)
- Large click targets in settings

### Cognitive

- Clear, simple language
- Consistent patterns
- Error messages explain how to fix

---

## Related Documents

- **Architecture**: [01-SUMMARY.md](01-SUMMARY.md) - Technical implementation
- **Tech Stack**: [03-TECH-STACK.md](03-TECH-STACK.md) - Technologies enabling this UX
- **Phase 4**: [08-PHASE-04-PRODUCT-SHELL.md] - UI implementation details
- **Error Handling**: [16-ERROR-HANDLING.md](16-ERROR-HANDLING.md) - Error UX
