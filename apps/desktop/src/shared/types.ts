// Shared types for OpenWhisper protocol.
// Electron <-> Rust IPC messages.

export interface AudioSettings {
  deviceId: string;
  sampleRate: number;
  chunkDurationMs: number;
  vadEnabled: boolean;
}

export interface DictationSettings {
  hotkey: string;
  injectionMethod: 'auto' | 'keystrokes' | 'clipboard';
  language: string;
  punctuation: boolean;
}

export interface UISettings {
  theme: 'light' | 'dark' | 'system';
  fontSize: number;
  showConfidence: boolean;
}

export interface SystemSettings {
  autoStart: boolean;
  minimizeToTray: boolean;
  diagnosticMode: boolean;
}

export interface UserSettings {
  audio: AudioSettings;
  dictation: DictationSettings;
  ui: UISettings;
  system: SystemSettings;
}

export interface DictationStartCommand {
  type: 'dictation.start';
  settings?: Partial<AudioSettings>;
}

export interface DictationStopCommand {
  type: 'dictation.stop';
}

export interface GetStatusCommand {
  type: 'status.get';
}

export interface UpdateSettingsCommand {
  type: 'settings.update';
  settings: UserSettings;
}

export type Command =
  | DictationStartCommand
  | DictationStopCommand
  | GetStatusCommand
  | UpdateSettingsCommand;

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

export type AsrModel = 'base_en_q8' | 'medium_en_q8';
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
