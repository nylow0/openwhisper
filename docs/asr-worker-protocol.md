# OpenWhisper ASR Worker Protocol

The ASR worker protocol is newline-delimited JSON over stdio. Every command and
event is one JSON object followed by `\n`. The Rust worker and desktop/native
bridge use the v1 contract below.

## V1 Commands

### `health.check`

Request:

```json
{"type":"health.check","timestamp":7}
```

Response:

```json
{"type":"health.ok","timestamp":7,"status":"ready"}
```

If `timestamp` is missing, the worker responds with its current Unix timestamp
in seconds.

### `model.load`

Request:

```json
{"type":"model.load","model":"medium_en_q8","device":"auto"}
```

Success response:

```json
{"type":"model.loaded","device":"cpu","memory_mb":0.0}
```

Failure response:

```json
{"type":"model.error","error":"model load failed","recoverable":true}
```

`memory_mb` is informational. Tests must verify that it is numeric, not that one
backend reports the same number as another backend.

### `dictation.start`

Request:

```json
{"type":"dictation.start"}
```

Required behavior:

- If no model is loaded, the worker loads one or emits `model.error`.
- If capture starts successfully, no success event is required.
- If capture fails, the worker emits `audio.error`.
- If dictation is already running, the worker emits a recoverable `error` with
  code `DICTATION_ALREADY_RUNNING`.

Streaming workers may emit `transcript.partial` after this command.

### `dictation.stop`

Request:

```json
{"type":"dictation.stop"}
```

Required behavior:

- If dictation is running, the worker stops capture and emits either
  `transcript.final` or `transcript.error`.
- If dictation is not running, the worker emits a recoverable `error` with code
  `DICTATION_NOT_RUNNING`.

### `shutdown`

Request:

```json
{"type":"shutdown"}
```

Required behavior:

- Stop active recording if needed.
- Release worker resources.
- Exit the worker loop.
- No response event is required.

## V1 Events

### `health.ok`

```json
{"type":"health.ok","timestamp":7,"status":"ready"}
```

### `model.loaded`

```json
{"type":"model.loaded","device":"cpu","memory_mb":0.0}
```

### `model.error`

```json
{"type":"model.error","error":"model load failed","recoverable":true}
```

### `transcript.partial`

```json
{"type":"transcript.partial","text":"hello","is_final":false,"processing_latency_ms":12}
```

### `transcript.final`

```json
{"type":"transcript.final","text":"hello world","words":[{"text":"hello","start_ms":0,"end_ms":320,"confidence":0.92}],"language":"en","processing_latency_ms":42}
```

### `transcript.error`

```json
{"type":"transcript.error","error":"decode failed","chunk_timestamp":7}
```

### `audio.error`

```json
{"type":"audio.error","error":"microphone unavailable","code":"AUDIO_RECORDING_FAILED"}
```

### `error`

```json
{"type":"error","code":"PROTOCOL_ERROR","message":"Invalid JSON","recoverable":true}
```

`details` may be present on `error` events, but consumers must not require it.

## Rust Extensions

These are useful diagnostics, but they are not part of the desktop v1 contract
yet:

- `devices.list` -> `devices.result`
- `transcribe.file`

They must stay behind explicit Rust-worker usage until the native bridge knows
which worker it spawned and tests cover both paths.
