import type {
  AppSettings,
  DictationStartedEvent,
  DictationStoppedEvent,
  ErrorEvent,
  Event,
  HistoryItem,
  StatusEvent,
  TranscriptFinalEvent,
  TranscriptPartialEvent,
  UserSettings,
} from '../shared/types';

export {};

export interface HealthCheckResult {
  status: string;
  timestamp: number;
  isDictating: boolean;
  isModelLoaded: boolean;
  workerHealthy: boolean;
}

export interface RestartEngineResult {
  ok: boolean;
  error?: string;
}

declare global {
  interface OpenWhisperApi {
    startDictation: () => Promise<{ success: boolean }>;
    stopDictation: () => Promise<{ success: boolean }>;
    getStatus: () => Promise<StatusEvent>;
    updateSettings: (settings: UserSettings) => Promise<{ success: boolean }>;
    healthCheck: () => Promise<HealthCheckResult>;

    onTranscript: (callback: (data: Event) => void) => () => void;
    onTranscriptPartial: (callback: (data: TranscriptPartialEvent) => void) => () => void;
    onTranscriptFinal: (callback: (data: TranscriptFinalEvent) => void) => () => void;
    onDictationStarted: (callback: (data: DictationStartedEvent) => void) => () => void;
    onDictationStopped: (callback: (data: DictationStoppedEvent) => void) => () => void;
    onStatusUpdate: (callback: (data: StatusEvent) => void) => () => void;
    onError: (callback: (data: ErrorEvent) => void) => () => void;
    onDisconnected: (callback: () => void) => () => void;
    onHotkeyToggle: (callback: () => void) => () => void;

    getSettings: () => Promise<AppSettings>;
    saveSettings: (partial: Partial<AppSettings>) => Promise<AppSettings>;
    restartEngine: () => Promise<RestartEngineResult>;

    getHistory: () => Promise<HistoryItem[]>;
    clearHistory: () => Promise<void>;
    deleteHistoryItem: (id: string) => Promise<void>;
    onHistoryChanged: (callback: (items: HistoryItem[]) => void) => () => void;
  }

  interface Window {
    api?: OpenWhisperApi;
  }
}
