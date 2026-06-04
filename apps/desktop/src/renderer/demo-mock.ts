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
} from '../shared/types';
import type { HealthCheckResult, OpenWhisperApi, RestartEngineResult } from './api.d';

type Listener<T> = (data: T) => void;

function createEmitter<T>() {
  const listeners = new Set<Listener<T>>();
  return {
    emit(data: T) {
      listeners.forEach((listener) => listener(data));
    },
    subscribe(listener: Listener<T>) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

const now = Date.now();

const demoHistory: HistoryItem[] = [
  {
    id: 'demo-1',
    text: 'Schedule a product demo for Thursday at 2 PM and send the calendar invite to the team.',
    language: 'en',
    latencyMs: 842,
    createdAt: now - 12 * 60_000,
  },
  {
    id: 'demo-2',
    text: 'Надішли клієнту оновлену пропозицію до кінця тижня.',
    language: 'uk',
    latencyMs: 1104,
    createdAt: now - 47 * 60_000,
  },
  {
    id: 'demo-3',
    text: 'Draft a short README section explaining offline dictation and the Ctrl + Win hotkey.',
    language: 'en',
    latencyMs: 756,
    createdAt: now - 3 * 60 * 60_000,
  },
];

let settings: AppSettings = {
  model: 'large_v3_turbo_q8',
  device: 'auto',
  spokenLanguages: ['en', 'uk', 'de'],
  autoDetectLanguage: false,
  launchAtLogin: true,
  showWindowOnLaunch: false,
};

let history = [...demoHistory];

const dictationStarted = createEmitter<DictationStartedEvent>();
const dictationStopped = createEmitter<DictationStoppedEvent>();
const transcriptPartial = createEmitter<TranscriptPartialEvent>();
const transcriptFinal = createEmitter<TranscriptFinalEvent>();
const transcript = createEmitter<Event>();
const statusUpdate = createEmitter<StatusEvent>();
const error = createEmitter<ErrorEvent>();
const disconnected = createEmitter<void>();
const hotkeyToggle = createEmitter<void>();
const historyChanged = createEmitter<HistoryItem[]>();

const api: OpenWhisperApi = {
  startDictation: async () => ({ success: true }),
  stopDictation: async () => ({ success: true }),
  getStatus: async () => ({
    type: 'status',
    isDictating: false,
    isModelLoaded: true,
    workerHealthy: true,
  }),
  healthCheck: async (): Promise<HealthCheckResult> => ({
    status: 'ok',
    timestamp: Date.now(),
    isDictating: false,
    isModelLoaded: true,
    workerHealthy: true,
  }),

  onTranscript: (callback) => transcript.subscribe(callback),
  onTranscriptPartial: (callback) => transcriptPartial.subscribe(callback),
  onTranscriptFinal: (callback) => transcriptFinal.subscribe(callback),
  onDictationStarted: (callback) => dictationStarted.subscribe(callback),
  onDictationStopped: (callback) => dictationStopped.subscribe(callback),
  onStatusUpdate: (callback) => statusUpdate.subscribe(callback),
  onError: (callback) => error.subscribe(callback),
  onDisconnected: (callback) => disconnected.subscribe(callback),
  onHotkeyToggle: (callback) => hotkeyToggle.subscribe(callback),

  getSettings: async () => ({ ...settings }),
  saveSettings: async (partial) => {
    settings = { ...settings, ...partial };
    return { ...settings };
  },
  restartEngine: async (): Promise<RestartEngineResult> => ({ ok: true }),

  getHistory: async () => [...history],
  clearHistory: async () => {
    history = [];
    historyChanged.emit([...history]);
  },
  deleteHistoryItem: async (id) => {
    history = history.filter((item) => item.id !== id);
    historyChanged.emit([...history]);
  },
  onHistoryChanged: (callback) => historyChanged.subscribe(callback),
};

window.api = api;

/** Overlay page: auto-play recording → transcribing → inserted. */
export function runOverlayDemoSequence() {
  const finalText =
    'OpenWhisper runs fully offline on your machine — hold Ctrl + Win anywhere to dictate.';

  dictationStarted.emit({ type: 'dictation.started', timestamp: Date.now() });

  window.setTimeout(() => {
    dictationStopped.emit({ type: 'dictation.stopped', timestamp: Date.now() });
    transcriptPartial.emit({
      type: 'transcript.partial',
      text: 'OpenWhisper runs fully offline',
      isFinal: false,
      processingLatencyMs: 120,
    });
  }, 2800);

  window.setTimeout(() => {
    transcriptPartial.emit({
      type: 'transcript.partial',
      text: finalText.slice(0, 52),
      isFinal: false,
      processingLatencyMs: 240,
    });
  }, 3600);

  window.setTimeout(() => {
    transcriptFinal.emit({
      type: 'transcript.final',
      text: finalText,
      words: [],
      language: 'en',
      processingLatencyMs: 410,
    });
    const item: HistoryItem = {
      id: `demo-live-${Date.now()}`,
      text: finalText,
      language: 'en',
      latencyMs: 410,
      createdAt: Date.now(),
    };
    history = [item, ...history];
    historyChanged.emit([...history]);
    localStorage.setItem('openwhisper-demo-history', JSON.stringify(history));
  }, 4800);
}

if (typeof window !== 'undefined') {
  window.addEventListener('storage', (event) => {
    if (event.key !== 'openwhisper-demo-history' || !event.newValue) return;
    try {
      history = JSON.parse(event.newValue) as HistoryItem[];
      historyChanged.emit([...history]);
    } catch {
      /* ignore */
    }
  });
}

if (location.pathname.endsWith('overlay.html')) {
  window.setTimeout(runOverlayDemoSequence, 600);
}
