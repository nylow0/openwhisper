// Exercise the real renderer subscriptions around a restart, including late
// disconnects from the old pipe and failures of the newly connected helper.
import { afterAll, afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { get } from 'svelte/store';
import type { AppSettings, StatusEvent } from '../shared/types';

const initialSettings: AppSettings = {
  model: 'large_v3_turbo_q8',
  device: 'auto',
  spokenLanguages: ['en'],
  autoDetectLanguage: false,
  launchAtLogin: false,
  showWindowOnLaunch: true,
};

let connected = true;
let disconnect: (() => void) | undefined;

const api = {
  getSettings: async () => initialSettings,
  getHistory: async () => [],
  getStatus: async (): Promise<StatusEvent> => {
    if (!connected) throw new Error('Rust connection closed');
    return { type: 'status', workerHealthy: true, isDictating: false, isModelLoaded: false };
  },
  onHistoryChanged: () => () => {},
  onStatusUpdate: () => () => {},
  onDisconnected: (callback: () => void) => {
    disconnect = callback;
    return () => { disconnect = undefined; };
  },
  restartEngine: async () => {
    // The intentionally closed old pipe must not be reported as a failure.
    disconnect?.();
    return { ok: true };
  },
};

const previousWindow = Object.getOwnPropertyDescriptor(globalThis, 'window');
Object.defineProperty(globalThis, 'window', { configurable: true, value: { api } });
const { engineStatus, initAppState, restartEngine } = await import('./app-state');

let unexpectedDisconnects = 0;
let cleanup: () => void;

beforeEach(async () => {
  connected = true;
  unexpectedDisconnects = 0;
  engineStatus.set('starting');
  cleanup = initAppState(() => { unexpectedDisconnects += 1; });
  await Bun.sleep(0);
});

afterEach(() => cleanup());
afterAll(() => {
  if (previousWindow) Object.defineProperty(globalThis, 'window', previousWindow);
  else Reflect.deleteProperty(globalThis, 'window');
});

describe('engine restart disconnects', () => {
  test('the old pipe closing during restart does not report a lost connection', async () => {
    await restartEngine();
    expect(get(engineStatus)).toBe('ready');
    expect(unexpectedDisconnects).toBe(0);
  });

  test('a late old-pipe close confirms that the current helper is still ready', async () => {
    await restartEngine();
    disconnect?.();
    await Bun.sleep(0);
    expect(get(engineStatus)).toBe('ready');
    expect(unexpectedDisconnects).toBe(0);
  });

  test('a new helper crashing immediately after restart becomes offline and reports the loss', async () => {
    await restartEngine();
    connected = false;
    disconnect?.();
    await Bun.sleep(0);
    expect(get(engineStatus)).toBe('offline');
    expect(unexpectedDisconnects).toBe(1);
  });
});
