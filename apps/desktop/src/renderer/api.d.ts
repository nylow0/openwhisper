import type {
  DictationStartedEvent,
  DictationStoppedEvent,
  ErrorEvent,
  Event,
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

declare global {
  interface Window {
    api: {
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
    };
  }
}
