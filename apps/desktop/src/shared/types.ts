// Shared event and app data types for OpenWhisper.

export interface WordResult {
  text: string;
  startMs: number;
  endMs: number;
  confidence?: number;
}

export interface DictationStartedEvent {
  type: 'dictation.started';
  timestamp: number;
}

export interface DictationStoppedEvent {
  type: 'dictation.stopped';
  timestamp: number;
}

export interface TranscriptPartialEvent {
  type: 'transcript.partial';
  text: string;
  isFinal: false;
  processingLatencyMs?: number;
}

export interface TranscriptFinalEvent {
  type: 'transcript.final';
  text: string;
  words: WordResult[];
  language?: string | null;
  processingLatencyMs?: number;
}

export interface StatusEvent {
  type: 'status';
  isDictating: boolean;
  isModelLoaded: boolean;
  workerHealthy: boolean;
}

export interface ErrorEvent {
  type: 'error';
  code: string;
  message: string;
  recoverable: boolean;
  details?: Record<string, unknown>;
}

export type Event =
  | DictationStartedEvent
  | DictationStoppedEvent
  | TranscriptPartialEvent
  | TranscriptFinalEvent
  | StatusEvent
  | ErrorEvent;

// ─── App-level settings & history (Electron <-> renderer) ───

export type AsrModel = 'medium_en_q8' | 'large_v3_turbo_q8';
export type AsrDevice = 'auto' | 'cpu' | 'gpu';

export interface AppSettings {
  /** whisper.cpp model key passed to the ASR worker. */
  model: AsrModel;
  /** Compute device preference for transcription. */
  device: AsrDevice;
  /** Start OpenWhisper automatically when the user signs in. */
  launchAtLogin: boolean;
  /** Open the main window on launch (vs. starting only in the tray). */
  showWindowOnLaunch: boolean;
}

export interface HistoryItem {
  id: string;
  text: string;
  language: string | null;
  latencyMs: number | null;
  /** Epoch milliseconds. */
  createdAt: number;
}
