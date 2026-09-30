// Renderer-side app state shared by the main window's views: settings, transcript
// history, and the speech engine's connection status. `initAppState` wires the
// stores to the preload API once; components read the stores and call the helpers.
import { get, writable } from 'svelte/store';
import type { AppSettings, AsrDevice, AsrModel, HistoryItem } from '../shared/types';
import type { RestartEngineResult } from './api';

/**
 * What the status line can honestly claim. `ready` means the engine answered a
 * status request; the model itself may still load on first dictation.
 */
export type EngineStatus = 'starting' | 'ready' | 'restarting' | 'offline';

export const MODELS: Array<{ id: AsrModel; name: string; detail: string; size: string }> = [
  { id: 'large_v3_turbo_q8', name: 'Multilingual', detail: 'Best accuracy, every language', size: '874 MB' },
  { id: 'medium_en_q8', name: 'English only', detail: 'Smaller fallback for English', size: '823 MB' },
];

export const DEVICES: Array<{ id: AsrDevice; name: string }> = [
  { id: 'auto', name: 'Auto' },
  { id: 'cpu', name: 'CPU' },
  { id: 'gpu', name: 'GPU' },
];

export const settings = writable<AppSettings | null>(null);
export const history = writable<HistoryItem[]>([]);
export const engineStatus = writable<EngineStatus>('starting');

const api = window.api;

// The helper process drops its pipe while restarting; ignore that disconnect
// (and a late one right after) instead of reporting a lost engine.
const RESTART_GRACE_MS = 1_000;
let restartGraceUntil = 0;

function isRestartExpected(): boolean {
  return get(engineStatus) === 'restarting' || Date.now() < restartGraceUntil;
}

async function refreshEngineStatus(): Promise<void> {
  if (!api) return;
  try {
    const status = await api.getStatus();
    engineStatus.set(status.workerHealthy ? 'ready' : 'offline');
  } catch {
    engineStatus.set('offline');
  }
}

/**
 * Loads settings, history, and engine status, and subscribes to their updates.
 * `onUnexpectedDisconnect` fires when the engine drops outside a restart.
 * Returns an unsubscribe function.
 */
export function initAppState(onUnexpectedDisconnect: () => void): () => void {
  if (!api) {
    engineStatus.set('offline');
    return () => {};
  }

  void api.getSettings().then((value) => settings.set(value));
  void api.getHistory().then((items) => history.set(items));
  void refreshEngineStatus();

  const unsubscribes = [
    api.onHistoryChanged((items) => history.set(items)),
    api.onStatusUpdate((status) => {
      if (get(engineStatus) !== 'restarting') engineStatus.set(status.workerHealthy ? 'ready' : 'offline');
    }),
    api.onDisconnected(() => {
      if (isRestartExpected()) return;
      engineStatus.set('offline');
      onUnexpectedDisconnect();
    }),
  ];
  return () => unsubscribes.forEach((unsubscribe) => unsubscribe());
}

/** Persists a settings patch. Optimistic, then replaced by what main actually stored. */
export async function saveSettings(partial: Partial<AppSettings>): Promise<void> {
  const current = get(settings);
  if (!api || !current) return;
  settings.set({ ...current, ...partial });
  settings.set(await api.saveSettings(partial));
}

/** Restarts the speech helper so model, device, or language changes take effect. */
export async function restartEngine(): Promise<RestartEngineResult> {
  if (!api) return { ok: false, error: 'Preload API unavailable' };
  engineStatus.set('restarting');
  const result = await api.restartEngine();
  restartGraceUntil = Date.now() + RESTART_GRACE_MS;
  if (result.ok) await refreshEngineStatus();
  else engineStatus.set('offline');
  return result;
}
