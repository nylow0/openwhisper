import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron';
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

type Unsubscribe = () => void;

function onChannel<T>(channel: string, callback: (data: T) => void): Unsubscribe {
  const listener = (_event: IpcRendererEvent, data: T) => callback(data);
  ipcRenderer.on(channel, listener);
  return () => ipcRenderer.removeListener(channel, listener);
}

function onSignal(channel: string, callback: () => void): Unsubscribe {
  const listener = () => callback();
  ipcRenderer.on(channel, listener);
  return () => ipcRenderer.removeListener(channel, listener);
}

contextBridge.exposeInMainWorld('api', {
  startDictation: () => ipcRenderer.invoke('dictation:start'),
  stopDictation: () => ipcRenderer.invoke('dictation:stop'),
  getStatus: () => ipcRenderer.invoke('status:get'),
  updateSettings: (settings: UserSettings) => ipcRenderer.invoke('settings:update', settings),
  healthCheck: () => ipcRenderer.invoke('health-check'),

  onTranscript: (callback: (data: Event) => void) => onChannel('asr:event', callback),
  onTranscriptPartial: (callback: (data: TranscriptPartialEvent) => void) =>
    onChannel('transcript:partial', callback),
  onTranscriptFinal: (callback: (data: TranscriptFinalEvent) => void) =>
    onChannel('transcript:final', callback),
  onDictationStarted: (callback: (data: DictationStartedEvent) => void) =>
    onChannel('dictation:started', callback),
  onDictationStopped: (callback: (data: DictationStoppedEvent) => void) =>
    onChannel('dictation:stopped', callback),
  onStatusUpdate: (callback: (data: StatusEvent) => void) => onChannel('status:update', callback),
  onError: (callback: (data: ErrorEvent) => void) => onChannel('error:received', callback),
  onDisconnected: (callback: () => void) => onSignal('rust:disconnected', callback),
});
