const electron = require('electron') as typeof import('electron');
const { contextBridge, ipcRenderer } = electron;

type Unsubscribe = () => void;

function onChannel(channel: string, callback: (data: unknown) => void): Unsubscribe {
  const listener = (_event: unknown, data: unknown) => callback(data);
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
  healthCheck: () => ipcRenderer.invoke('health-check'),

  onTranscript: (callback: (data: unknown) => void) => onChannel('asr:event', callback),
  onTranscriptPartial: (callback: (data: unknown) => void) =>
    onChannel('transcript:partial', callback),
  onTranscriptFinal: (callback: (data: unknown) => void) =>
    onChannel('transcript:final', callback),
  onDictationStarted: (callback: (data: unknown) => void) =>
    onChannel('dictation:started', callback),
  onDictationStopped: (callback: (data: unknown) => void) =>
    onChannel('dictation:stopped', callback),
  onStatusUpdate: (callback: (data: unknown) => void) => onChannel('status:update', callback),
  onError: (callback: (data: unknown) => void) => onChannel('error:received', callback),
  onDisconnected: (callback: () => void) => onSignal('rust:disconnected', callback),
  onHotkeyToggle: (callback: () => void) => onSignal('hotkey:toggle-dictation', callback),

  getSettings: () => ipcRenderer.invoke('settings:read'),
  saveSettings: (partial: unknown) => ipcRenderer.invoke('settings:write', partial),
  restartEngine: () => ipcRenderer.invoke('engine:restart'),

  getHistory: () => ipcRenderer.invoke('history:get'),
  clearHistory: () => ipcRenderer.invoke('history:clear'),
  deleteHistoryItem: (id: unknown) => ipcRenderer.invoke('history:delete', id),
  onHistoryChanged: (callback: (data: unknown) => void) => onChannel('history:changed', callback),
});
