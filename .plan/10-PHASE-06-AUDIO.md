# Phase 6: Audio Pipeline

## Context

This is **Phase 6** of 9 in building OpenWhisper, a Windows-first offline dictation desktop app.

**Purpose**: Capture microphone audio and stream it to the ASR engine for transcription. This is where sound becomes data that the AI can process.

**Corrected approach**:
- Phase 3 may use WAV files and optional Python microphone spikes.
- This phase should implement the production audio path in Rust WASAPI.
- Do not build a full Python `sounddevice` pipeline unless the team intentionally accepts that rework.

---

## Prerequisites

- [ ] Phase 1-5 complete
- [ ] Rust helper can spawn Python worker
- [ ] Protocol between Rust and Python works
- [ ] ASR model loads and accepts audio
- [ ] Read [Performance Budget](17-PERFORMANCE-BUDGET.md) for latency targets
- [ ] Read [Error Handling](16-ERROR-HANDLING.md) for audio error scenarios

---

## Goal

Capture microphone audio in real-time with Rust WASAPI, convert it to the format the ASR model expects, and stream it through the Rust-to-Python protocol for transcription.

**Success means**: User speaks into microphone → Audio is captured → ASR transcribes → Text appears.

---

## Audio Requirements

### Format

| Parameter | Value | Notes |
|-----------|-------|-------|
| Sample Rate | 16kHz | Model requirement |
| Channels | 1 (mono) | Model requirement |
| Format | float32 | [-1.0, 1.0] range |
| Bit Depth | 32-bit | float32 |

### Conversion

If microphone uses different format:
1. Capture at native device rate
2. Convert to mono (average channels)
3. Resample to 16kHz
4. Convert to float32

```python
import numpy as np
import librosa

def convert_audio(audio: np.ndarray, 
                  from_rate: int, 
                  to_rate: int = 16000) -> np.ndarray:
    """Convert audio to model format."""
    # Ensure mono
    if audio.ndim > 1:
        audio = np.mean(audio, axis=1)
    
    # Resample if needed
    if from_rate != to_rate:
        audio = librosa.resample(
            audio, 
            orig_sr=from_rate, 
            target_sr=to_rate
        )
    
    # Ensure float32 in [-1, 1]
    if audio.dtype != np.float32:
        if audio.dtype == np.int16:
            audio = audio.astype(np.float32) / 32768.0
        elif audio.dtype == np.int32:
            audio = audio.astype(np.float32) / 2147483648.0
    
    return audio
```

---

## Spike-Only Path (Python sounddevice)

### Why sounddevice?

- **Fast to test** - Pure Python, simple API
- **Cross-platform** - Works on Windows, macOS, Linux
- **Callback-based** - Real-time streaming
- **PortAudio backend** - Well-tested, reliable

### Spike Architecture

```
Microphone
    ↓ (PortAudio via sounddevice)
Callback receives audio chunks
    ↓
Ring Buffer (thread-safe)
    ↓
Chunk Reader (main thread)
    ↓
VAD (optional gate)
    ↓
Resample if needed (16kHz)
    ↓
Base64 encode
    ↓
NDJSON message to Rust
    ↓
Transcript result
```

### Implementation

```python
# services/asr/scripts/audio_capture_spike.py
import sounddevice as sd
import numpy as np
from collections import deque
import threading
import queue

class AudioCapture:
    def __init__(self, 
                 sample_rate: int = 16000,
                 chunk_duration_ms: int = 100,
                 device: str = None):
        self.sample_rate = sample_rate
        self.chunk_samples = int(sample_rate * chunk_duration_ms / 1000)
        self.device = device
        
        # Ring buffer for audio
        self.buffer = deque(maxlen=int(sample_rate * 10))  # 10s max
        self.buffer_lock = threading.Lock()
        
        self.stream = None
        self.is_capturing = False
        
    def start(self) -> None:
        """Start audio capture."""
        device_info = sd.query_devices(self.device, 'input')
        native_rate = int(device_info['default_samplerate'])
        
        def callback(indata, frames, time_info, status):
            if status:
                print(f"Audio status: {status}")
            
            # Convert to mono float32
            audio = indata[:, 0].astype(np.float32)
            
            # Resample if needed (simple approach)
            if native_rate != self.sample_rate:
                # Use librosa or scipy for quality resampling
                audio = resample(audio, native_rate, self.sample_rate)
            
            with self.buffer_lock:
                self.buffer.extend(audio)
        
        self.stream = sd.InputStream(
            device=self.device,
            channels=1,
            samplerate=native_rate,
            blocksize=self.chunk_samples,
            dtype=np.float32,
            callback=callback
        )
        
        self.stream.start()
        self.is_capturing = True
        
    def stop(self) -> None:
        """Stop audio capture."""
        if self.stream:
            self.stream.stop()
            self.stream.close()
        self.is_capturing = False
        
    def read_chunk(self, duration_ms: int) -> np.ndarray:
        """Read a chunk of audio for ASR."""
        samples_needed = int(self.sample_rate * duration_ms / 1000)
        
        with self.buffer_lock:
            if len(self.buffer) < samples_needed:
                return None  # Not enough audio yet
            
            chunk = np.array(list(self.buffer)[:samples_needed])
            # Remove read samples from buffer
            for _ in range(samples_needed):
                self.buffer.popleft()
            
        return chunk
    
    def get_buffer_fill(self) -> float:
        """Get buffer fill percentage."""
        with self.buffer_lock:
            return len(self.buffer) / self.buffer.maxlen
```

### Integration with ASR Worker

```python
# services/asr/src/openwhisper_asr/__main__.py

class ASRWorker:
    def __init__(self):
        self.audio_capture = AudioCapture()
        self.engine = ParakeetEngine()
        self.is_dictating = False
        
    def start_dictation(self, settings: DictationSettings):
        """Start capturing and transcribing."""
        self.is_dictating = True
        self.audio_capture.start()
        
        # Start processing thread
        self.process_thread = threading.Thread(target=self._process_loop)
        self.process_thread.start()
        
    def _process_loop(self):
        """Main processing loop."""
        while self.is_dictating:
            # Read chunk (e.g., 2 seconds)
            chunk = self.audio_capture.read_chunk(duration_ms=2000)
            
            if chunk is None:
                time.sleep(0.01)  # Wait for more audio
                continue
            
            # Check buffer fill
            fill = self.audio_capture.get_buffer_fill()
            if fill > 0.8:
                log.warning(f"Buffer {fill*100:.0f}% full - processing lag")
            
            # Optional: VAD gate
            if self.vad_enabled and not self.vad.is_speech(chunk):
                continue
            
            # Transcribe
            result = self.engine.transcribe_chunk(chunk)
            
            # Send result
            self.send_transcript(result)
            
    def stop_dictation(self):
        """Stop capturing."""
        self.is_dictating = False
        self.audio_capture.stop()
        self.process_thread.join(timeout=5)
```

---

## Production Path (Rust WASAPI)

Use Rust WASAPI for the product audio path:

### Why WASAPI?

- **Lower latency** - Direct hardware access
- **Better control** - Fine-tune buffer sizes
- **Lower CPU** - Less overhead than PortAudio
- **Event-driven** - Callbacks on audio ready

### Architecture Change

```
Spike only:
  Python captures -> Python processes -> Python sends result

Product:
  Rust captures -> Rust sends to Python -> Python processes -> Python sends result
```

**Benefits**:
- Python becomes inference-only (simpler)
- Rust controls timing (better for low latency)
- Can use shared memory for audio (zero-copy)

### Implementation Sketch

```rust
// crates/openwhisper-native/src/audio.rs
use windows::Win32::Media::Audio::{WASAPI, IAudioClient};

pub struct WasapiCapture {
    client: IAudioClient,
    format: WAVEFORMATEX,
}

impl WasapiCapture {
    pub fn new(device: &Device) -> Result<Self> {
        // Initialize WASAPI
        // Set format to 16kHz mono float32
        // Set buffer size for low latency
    }
    
    pub fn start<F>(&self, callback: F) 
    where F: FnMut(&[f32]) {
        // Start capture thread
        // Call callback with audio chunks
    }
}

// In main loop, send to Python:
fn on_audio_chunk(chunk: &[f32]) {
    let base64 = base64::encode(chunk_as_bytes);
    python.send(ToPython::AudioChunk { 
        data: base64,
        timestamp: now()
    });
}
```

---

## Voice Activity Detection (VAD)

### Purpose

Gate audio - only send speech to ASR, skip silence.

**Benefits**:
- Reduces ASR load (don't transcribe silence)
- Saves memory (don't buffer silence)
- Clearer UX (processing indicator only during speech)

### Implementation Options

**Option 1: Simple Energy-based**
```python
def is_speech_energy(audio: np.ndarray, threshold: float = 0.01) -> bool:
    """Detect speech based on energy level."""
    energy = np.sqrt(np.mean(audio ** 2))
    return energy > threshold
```

**Option 2: WebRTC VAD**
```python
import webrtcvad

vad = webrtcvad.Vad(2)  # Aggressiveness 0-3

def is_speech_webrtc(audio: np.ndarray, sample_rate: int) -> bool:
    """Use Google's WebRTC VAD."""
    # Convert to int16
    pcm = (audio * 32767).astype(np.int16).tobytes()
    return vad.is_speech(pcm, sample_rate)
```

**Option 3: Silero VAD** (more accurate, requires PyTorch)
```python
# Use Silero VAD model
# More accurate, but adds ~50ms latency
```

### Default

Start with **WebRTC VAD** - good balance of accuracy and speed.

---

## Chunk Management

### Chunk Sizes

| Mode | Duration | Use Case |
|------|----------|----------|
| Fast | 0.5s | Real-time feedback, lower accuracy |
| Balanced | 2.0s | Good tradeoff (default) |
| Accurate | 4.0s | Best accuracy, higher latency |

### Overlap Strategy

For streaming, use overlapping chunks to avoid boundary artifacts:

```
Chunk 1: [0.0s - 2.0s]
Chunk 2: [1.6s - 3.6s]  (20% overlap)
Chunk 3: [3.2s - 5.2s]
```

Discard the overlapping regions when combining results.

### Buffer Management

```python
class AudioBuffer:
    """Thread-safe ring buffer for audio."""
    
    def __init__(self, max_duration_s: float = 30.0, 
                 sample_rate: int = 16000):
        max_samples = int(max_duration_s * sample_rate)
        self.buffer = deque(maxlen=max_samples)
        self.lock = threading.Lock()
        
    def write(self, audio: np.ndarray):
        with self.lock:
            self.buffer.extend(audio)
            
    def read(self, samples: int) -> Optional[np.ndarray]:
        with self.lock:
            if len(self.buffer) < samples:
                return None
            result = np.array(list(self.buffer)[:samples])
            for _ in range(samples):
                self.buffer.popleft()
            return result
            
    def fill_level(self) -> float:
        with self.lock:
            return len(self.buffer) / self.buffer.maxlen
```

**Warning threshold**: Log warning at 80% full.
**Critical threshold**: Drop audio at 95% full (processing too slow).

---

## Error Handling

### Audio Device Errors

| Error | Handling | User Notification |
|-------|----------|-------------------|
| Device disconnected | Pause dictation, monitor for reconnect | "Microphone disconnected" |
| Device in use | Retry with backoff | "Microphone in use by another app" |
| Permission denied | Show settings link | "Microphone access denied" |
| Buffer overrun | Increase chunk size, log warning | None (internal) |

See [Error Handling](16-ERROR-HANDLING.md) for complete error taxonomy.

---

## Defaults

- **Sample rate**: 16kHz (fallback: resample from device rate)
- **Chunk size**: 2s (Balanced mode)
- **VAD**: Enabled, WebRTC
- **Device**: System default input
- **Buffer**: 30 seconds max

---

## Success Criteria

- [ ] Speak into microphone → see console transcript output
- [ ] Audio is 16kHz mono float32 format
- [ ] No buffer overruns during normal speech
- [ ] VAD gates silence (pauses transcription when not speaking)
- [ ] Speak into mic → see overlay transcript (via protocol)
- [ ] Speak into mic → inject text into Notepad (full pipeline)
- [ ] Changing audio device in settings takes effect
- [ ] Disconnecting microphone shows error gracefully
- [ ] Reconnecting microphone allows recovery

---

## Testing

```bash
# Test Rust WASAPI capture
cd crates/openwhisper-native
cargo run -- --audio-smoke-test

# Test Python-side VAD/preprocessing on a captured WAV or chunk fixture
cd services/asr
uv run python -c "
from openwhisper_asr.audio.vad import WebRTCVAD
import numpy as np

vad = WebRTCVAD()
audio = np.random.randn(16000) * 0.1  # Noise
print(f'Is speech (noise): {vad.is_speech(audio)}')
"
```

---

## Related Documents

- **Error Handling**: [Document 16](16-ERROR-HANDLING.md) - Audio error scenarios
- **Performance Budget**: [Document 17](17-PERFORMANCE-BUDGET.md) - Buffer and latency targets
- **ASR Engine**: [Phase 3](07-PHASE-03-ASR-ENGINE.md) - Model format requirements
- **Previous Phase**: [Phase 5](09-PHASE-05-RUST-HELPER.md) - Rust spawning Python
- **Next Phase**: [Phase 7](11-PHASE-07-END-TO-END.md) - Full integration
